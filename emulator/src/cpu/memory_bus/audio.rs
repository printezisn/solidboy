mod envelope_pulse_channel;
mod noise_channel;
mod pulse_channel;
mod wave_channel;

use envelope_pulse_channel::EnvelopePulseChannel;
use noise_channel::NoiseChannel;
use pulse_channel::PulseChannel;
use wave_channel::WaveChannel;

const SAMPLE_BUFFER_SIZE: usize = 4096;
const SAMPLE_RATE: f32 = 44100.0;
const CLOCK_RATE: f32 = 4194304.0;
const SAMPLE_TICK: f32 = SAMPLE_RATE / CLOCK_RATE;
const CYCLES_PER_FRAME: u32 = 70224;
const ENVELOPE_TICK_CYCLES: u16 = 8192;

pub struct Audio {
    sample_buffer: [f32; SAMPLE_BUFFER_SIZE],
    sample_buffer_pos: usize,
    sample_timer: f32,
    cycles: u32,

    nr50: u8,
    nr51: u8,
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
            sample_buffer: [0.0; SAMPLE_BUFFER_SIZE],
            sample_buffer_pos: 0,
            sample_timer: 0.0,
            cycles: 0,

            nr50: 0x77,
            nr51: 0xF3,
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
            0xFF10..=0xFF23 => Some(0xFF),
            0xFF24 => Some(self.nr50),
            0xFF25 => Some(self.nr51),
            0xFF26 => Some(self.nr52 
                    | (if self.pulse_channel.enabled() { 0x01 } else { 0 })
                    | (if self.envelope_pulse_channel.enabled() { 0x02 } else { 0 })
                    | (if self.wave_channel.enabled() { 0x04 } else { 0 })
                    | (if self.noise_channel.enabled() { 0x08 } else { 0 })
                    | 0x70),
            0xFF27..=0xFF3F => Some(0xFF),
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
            0xFF10..=0xFF23 => {},
            0xFF24 => self.nr50 = value,
            0xFF25 => self.nr51 = value,
            0xFF26 => self.write_nr52(value),
            0xFF27..=0xFF3F => {},
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

    fn mix_samples(&self) -> (f32, f32) {
        let ch1 = self.pulse_channel.output();
        let ch2 = self.envelope_pulse_channel.output();
        let ch3 = self.wave_channel.output();
        let ch4 = self.noise_channel.output();
        
        let mut left = 0.0f32;
        if self.nr51 & 0x10 != 0 { left += ch1; }
        if self.nr51 & 0x20 != 0 { left += ch2; }
        if self.nr51 & 0x40 != 0 { left += ch3; }
        if self.nr51 & 0x80 != 0 { left += ch4; }
        
        let mut right = 0.0f32;
        if self.nr51 & 0x01 != 0 { right += ch1; }
        if self.nr51 & 0x02 != 0 { right += ch2; }
        if self.nr51 & 0x04 != 0 { right += ch3; }
        if self.nr51 & 0x08 != 0 { right += ch4; }
        
        let left_vol  = ((self.nr50 >> 4) & 0x07) as f32 + 1.0;
        let right_vol = (self.nr50 & 0x07) as f32 + 1.0;
        
        left  *= left_vol  / 8.0;
        right *= right_vol / 8.0;
        
        (left / 4.0, right / 4.0)
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

            let (left, right) = if self.nr52 & 0x80 == 0 {
                (0.0, 0.0)
            } else {
                self.mix_samples()
            };

            self.sample_buffer[self.sample_buffer_pos] = left;
            self.sample_buffer[self.sample_buffer_pos + 1] = right;
            self.sample_buffer_pos += 2;
        }

        while self.cycles > CYCLES_PER_FRAME {
            self.cycles -= CYCLES_PER_FRAME;
            append_audio_sample!(self.sample_buffer.as_ptr(), self.sample_buffer_pos);
            self.sample_buffer_pos = 0;
        }
    }
}