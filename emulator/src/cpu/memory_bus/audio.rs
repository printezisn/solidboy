mod envelope_pulse_channel;
mod noise_channel;
mod pulse_channel;
mod wave_channel;

use envelope_pulse_channel::EnvelopePulseChannel;
use noise_channel::NoiseChannel;
use pulse_channel::PulseChannel;
use wave_channel::WaveChannel;

const AUDIO_SIZE: usize = 0xFF26 - 0xFF10 + 1;
const SAMPLE_BUFFER_SIZE: usize = 4096;
const SAMPLE_RATE: f32 = 44100.0;
const CLOCK_RATE: f32 = 4194304.0;
const SAMPLE_TICK: f32 = SAMPLE_RATE / CLOCK_RATE;
const CYCLES_PER_FRAME: u32 = 70224;
const ENVELOPE_TICK_CYCLES: u16 = 8192;

pub struct Audio {
    audio: [u8; AUDIO_SIZE],

    sample_buffer: [f32; SAMPLE_BUFFER_SIZE],
    sample_buffer_pos: usize,
    sample_timer: f32,
    cycles: u32,

    nr52: u8,
    pulse_channel: PulseChannel,
    envelope_pulse_channel: EnvelopePulseChannel,
    wave_channel: WaveChannel,
    noise_channel: NoiseChannel,

    frame_sequencer_counter: u16,
    frame_sequencer_step: u8,
}

impl Audio {
    pub fn new() -> Self {
        Self {
            audio: [0; AUDIO_SIZE],

            sample_buffer: [0.0; SAMPLE_BUFFER_SIZE],
            sample_buffer_pos: 0,
            sample_timer: 0.0,
            cycles: 0,

            nr52: 0xF1,
            pulse_channel: PulseChannel::new(),
            envelope_pulse_channel: EnvelopePulseChannel::new(),
            wave_channel: WaveChannel::new(),
            noise_channel: NoiseChannel::new(),

            frame_sequencer_counter: 0,
            frame_sequencer_step: 0,
        }
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match self.pulse_channel.read(address) {
            Some(result) => return Some(result),
            _ => {}
        }

        match self.envelope_pulse_channel.read(address) {
            Some(result) => return Some(result),
            _ => {}
        }

        match self.wave_channel.read(address) {
            Some(result) => return Some(result),
            _ => {}
        }

        match self.noise_channel.read(address) {
            Some(result) => return Some(result),
            _ => {}
        }

        match address {
            0xFF10..=0xFF25 => Some(self.audio[(address - 0xFF10) as usize]),
            0xFF26 => Some(self.nr52),
            _ => None,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        if self.pulse_channel.write(address, value) {
            return true;
        }

        if self.envelope_pulse_channel.write(address, value) {
            return true;
        }

        if self.wave_channel.write(address, value) {
            return true;
        }

        if self.noise_channel.write(address, value) {
            return true;
        }

        match address {
            0xFF10..=0xFF25 => {
                self.audio[(address - 0xFF10) as usize] = value;
            }
            0xFF26 => {
                self.write_nr52(value);
            }
            _ => {
                return false;
            }
        }

        true
    }

    fn write_nr52(&mut self, value: u8) {
        if value & 0x80 == 0 && self.nr52 & 0x80 != 0 {
            self.frame_sequencer_counter = 0;
            self.frame_sequencer_step = 0;
            self.pulse_channel = PulseChannel::new();
            self.envelope_pulse_channel = EnvelopePulseChannel::new();
            self.wave_channel = WaveChannel::new();
            self.noise_channel = NoiseChannel::new();
        }
        self.nr52 = value & 0x80;
    }

    fn length_tick(&mut self) {
        self.pulse_channel.length_tick();
        self.envelope_pulse_channel.length_tick();
        self.wave_channel.length_tick();
        self.noise_channel.length_tick();
    }

    fn envelope_tick(&mut self) {
        self.pulse_channel.envelope_tick();
        self.envelope_pulse_channel.envelope_tick();
        self.noise_channel.envelope_tick();
    }

    fn sweep_tick(&mut self) {
        self.pulse_channel.sweep_tick();
    }

    fn tick_frame_sequencer(&mut self) {
        self.frame_sequencer_counter += 1;

        while self.frame_sequencer_counter >= ENVELOPE_TICK_CYCLES {
            self.frame_sequencer_counter -= ENVELOPE_TICK_CYCLES;

            match self.frame_sequencer_step {
                0 => self.length_tick(),
                1 => {}
                2 => {
                    self.length_tick();
                    self.sweep_tick();
                }
                3 => {}
                4 => self.length_tick(),
                5 => {}
                6 => {
                    self.length_tick();
                    self.sweep_tick();
                }
                7 => self.envelope_tick(),
                _ => unreachable!(),
            }

            self.frame_sequencer_step = (self.frame_sequencer_step + 1) % 8;
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        for _ in 0..cycles {
            self.single_tick();
        }
    }

    fn single_tick(&mut self) {
        if self.nr52 & 0x80 != 0 {
            self.pulse_channel.tick();
            self.envelope_pulse_channel.tick();
            self.wave_channel.tick();
            self.noise_channel.tick();

            self.tick_frame_sequencer();
        }

        self.cycles += 1;
        self.sample_timer += SAMPLE_TICK;

        while self.sample_timer >= 1.0 {
            self.sample_timer -= 1.0;

            let sample = if self.nr52 & 0x80 == 0 {
                0.0
            } else {
                let output = self.pulse_channel.output()
                    + self.envelope_pulse_channel.output()
                    + self.wave_channel.output()
                    + self.noise_channel.output();
                output * 0.15
            };

            self.sample_buffer[self.sample_buffer_pos] = sample;
            self.sample_buffer[self.sample_buffer_pos + 1] = sample;
            self.sample_buffer_pos += 2;
        }

        while self.cycles > CYCLES_PER_FRAME {
            self.cycles -= CYCLES_PER_FRAME;
            append_audio_sample!(self.sample_buffer.as_ptr(), self.sample_buffer_pos);
            self.sample_buffer_pos = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let audio = Audio::new();
        // Check arrays are zeroed
        assert!(audio.audio.iter().all(|&x| x == 0));
    }

    #[test]
    fn test_read_audio() {
        let mut audio = Audio::new();
        audio.audio[0x05] = 0xAB;
        assert_eq!(audio.read(0xFF15), Some(0xAB));
    }

    #[test]
    fn test_write_audio() {
        let mut audio = Audio::new();
        assert!(audio.write(0xFF15, 0xAB));
        assert_eq!(audio.audio[0x05], 0xAB);
    }

    #[test]
    fn test_read_invalid_address() {
        let audio = Audio::new();
        assert_eq!(audio.read(0x0000), None);
        assert_eq!(audio.read(0xFFFF), None);
    }

    #[test]
    fn test_write_invalid_address() {
        let mut audio = Audio::new();
        assert!(!audio.write(0x0000, 0x00));
        assert!(!audio.write(0xFFFF, 0x00));
    }
}
