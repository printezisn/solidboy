const AUDIO_SIZE: usize = 0xFF26 - 0xFF10 + 1;
const WAVE_PATTERN_SIZE: usize = 0xFF3F - 0xFF30 + 1;
const SAMPLE_BUFFER_SIZE: usize = 4096;
const SAMPLE_RATE: f32 = 44100.0;
const CLOCK_RATE: f32 = 4194304.0;
const SAMPLE_TICK: f32 = SAMPLE_RATE / CLOCK_RATE;
const CYCLES_PER_FRAME: u32 = 70224;

pub struct Audio {
    audio: [u8; AUDIO_SIZE],
    wave_pattern: [u8; WAVE_PATTERN_SIZE],

    sample_buffer: [f32; SAMPLE_BUFFER_SIZE],
    sample_buffer_pos: usize,
    sample_timer: f32,
    cycles: u32,
}

impl Audio {
    pub fn new() -> Self {
        Self {
            audio: [0; AUDIO_SIZE],
            wave_pattern: [0; WAVE_PATTERN_SIZE],

            sample_buffer: [0.0; SAMPLE_BUFFER_SIZE],
            sample_buffer_pos: 0,
            sample_timer: 0.0,
            cycles: 0,
        }
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF10..=0xFF26 => Some(self.audio[(address - 0xFF10) as usize]),
            0xFF30..=0xFF3F => Some(self.wave_pattern[(address - 0xFF30) as usize]),
            _ => None,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0xFF10..=0xFF26 => {
                self.audio[(address - 0xFF10) as usize] = value;
            }
            0xFF30..=0xFF3F => {
                self.wave_pattern[(address - 0xFF30) as usize] = value;
            }
            _ => {
                return false;
            }
        }

        true
    }

    pub fn tick(&mut self, cycles: u8) {
        for _ in 0..cycles {
            self.single_tick();
        }
    }

    fn single_tick(&mut self) {
        self.cycles += 1;
        self.sample_timer += SAMPLE_TICK;
        
        while self.sample_timer >= 1.0 {
            self.sample_timer -= 1.0;

            self.sample_buffer[self.sample_buffer_pos] = 0.0;
            self.sample_buffer[self.sample_buffer_pos + 1] = 0.0;
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
        assert!(audio.wave_pattern.iter().all(|&x| x == 0));
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
    fn test_read_wave_pattern() {
        let mut audio = Audio::new();
        audio.wave_pattern[0x05] = 0xCD;
        assert_eq!(audio.read(0xFF35), Some(0xCD));
    }

    #[test]
    fn test_write_wave_pattern() {
        let mut audio = Audio::new();
        assert!(audio.write(0xFF35, 0xCD));
        assert_eq!(audio.wave_pattern[0x05], 0xCD);
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
