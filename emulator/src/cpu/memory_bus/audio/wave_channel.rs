use crate::cpu::memory_bus::types::ModelType;

const WAVE_RAM_SIZE: usize = 0xFF3F - 0xFF30 + 1;

pub struct WaveChannel {
    wave_ram: [u8; WAVE_RAM_SIZE],
    wave_pos: u8,
    period_timer: u16,
    enabled: bool,
    dac_enabled: bool,
    nr0: u8,
    nr1: u8,
    nr2: u8,
    nr3: u8,
    nr4: u8,

    length_enabled: bool,
    length_counter: u16,

    model_type: ModelType,
}

impl WaveChannel {
    pub fn new(model_type: ModelType) -> Self {
        let mut result = Self {
            wave_ram: [0; WAVE_RAM_SIZE],
            wave_pos: 0,
            period_timer: 0,
            enabled: false,
            dac_enabled: false,
            nr0: 0x7F,
            nr1: 0xFF,
            nr2: 0x9F,
            nr3: 0xFF,
            nr4: 0xBF,

            length_enabled: false,
            length_counter: 0,

            model_type,
        };

        result.write_nr0(0x7F);
        result.write_nr1(0xFF);
        result.write_nr4(0, 0xBF);

        result
    }

    pub fn clear(&mut self) {
        self.period_timer = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.length_enabled = false;
        self.length_counter = 0;

        self.write_nr0(0);
        if matches!(self.model_type, ModelType::Color) {
            self.write_nr1(0);
        }
        self.nr2 = 0;
        self.nr3 = 0;
        self.write_nr4(0, 0);
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF1A => Some(self.nr0 | 0x7F),
            0xFF1B => Some(0xFF),
            0xFF1C => Some(self.nr2 | 0x9F),
            0xFF1D => Some(0xFF),
            0xFF1E => Some(self.nr4 | 0xBF),
            0xFF30..=0xFF3F => {
                if self.enabled {
                    Some(self.wave_ram[(self.wave_pos / 2) as usize])
                } else {
                    Some(self.wave_ram[address as usize - 0xFF30])
                }
            }
            _ => None,
        }
    }

    pub fn write(
        &mut self,
        audio_enabled: bool,
        frame_sequencer_step: u8,
        address: u16,
        value: u8,
    ) -> bool {
        if !audio_enabled && (address < 0xFF30 || address > 0xFF3F) {
            if matches!(self.model_type, ModelType::Color) || address != 0xFF1B {
                return address >= 0xFF1A && address <= 0xFF1E;
            }
        }

        match address {
            0xFF1A => self.write_nr0(value),
            0xFF1B => self.write_nr1(value),
            0xFF1C => self.nr2 = value,
            0xFF1D => self.nr3 = value,
            0xFF1E => self.write_nr4(frame_sequencer_step, value),
            0xFF30..=0xFF3F => {
                if self.enabled {
                    self.wave_ram[(self.wave_pos / 2) as usize] = value;
                } else {
                    self.wave_ram[address as usize - 0xFF30] = value;
                }
            }
            _ => return false,
        }

        true
    }

    pub fn length_tick(&mut self) {
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    pub fn tick(&mut self) {
        if self.period_timer == 0 {
            let period = ((self.nr4 & 0x07) as u16) << 8 | self.nr3 as u16;
            self.period_timer = (2048 - period) * 2;

            self.wave_pos = (self.wave_pos + 1) % 32;
        } else {
            self.period_timer -= 1;
        }
    }

    fn write_nr0(&mut self, val: u8) {
        self.nr0 = val;
        self.dac_enabled = val & 0x80 != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    fn write_nr1(&mut self, value: u8) {
        self.nr1 = value;
        self.length_counter = 256 - value as u16;
    }

    fn write_nr4(&mut self, frame_sequencer_step: u8, value: u8) {
        let was_length_enabled = self.length_enabled;
        let new_length_enabled = value & 0x40 != 0;

        if !was_length_enabled && new_length_enabled {
            if frame_sequencer_step % 2 == 1 && self.length_counter > 0 {
                self.length_counter -= 1;
                if self.length_counter == 0 {
                    self.enabled = false;
                }
            }
        }

        self.nr4 = value;
        self.length_enabled = new_length_enabled;

        if value & 0x80 != 0 {
            if self.length_counter == 0 {
                self.length_counter = if new_length_enabled && frame_sequencer_step % 2 == 1 {
                    255
                } else {
                    256
                };
            }

            self.enabled = self.dac_enabled;
            self.wave_pos = 0;

            let period = ((self.nr4 & 0x07) as u16) << 8 | self.nr3 as u16;
            self.period_timer = (2048 - period) * 2;
        }
    }

    fn current_sample(&self) -> u8 {
        let byte = self.wave_ram[(self.wave_pos / 2) as usize];
        if self.wave_pos % 2 == 0 {
            (byte >> 4) & 0x0F
        } else {
            byte & 0x0F
        }
    }

    pub fn output(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        let sample = self.current_sample();
        let shifted = match (self.nr2 >> 5) & 0x03 {
            0 => 0,
            1 => sample,
            2 => sample >> 1,
            3 => sample >> 2,
            _ => unreachable!(),
        };

        shifted as f32 / 15.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_wave_channel(model_type: ModelType) -> WaveChannel {
        WaveChannel::new(model_type)
    }

    #[test]
    fn test_new_initialization() {
        let channel = create_wave_channel(ModelType::DMG);
        assert_eq!(channel.enabled, false);
        assert_eq!(channel.dac_enabled, false);
        assert_eq!(channel.wave_pos, 0);
    }

    #[test]
    fn test_clear_resets_state() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.wave_pos = 5;
        channel.length_counter = 256;

        channel.clear();

        assert_eq!(channel.enabled, false);
        assert_eq!(channel.wave_pos, 5);
        assert_eq!(channel.length_counter, 0);
    }

    #[test]
    fn test_enabled_status() {
        let channel = create_wave_channel(ModelType::DMG);
        assert_eq!(channel.enabled(), false);
    }

    #[test]
    fn test_output_when_disabled() {
        let channel = create_wave_channel(ModelType::DMG);
        assert_eq!(channel.output(), 0.0);
    }

    #[test]
    fn test_output_when_dac_disabled() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = false;
        assert_eq!(channel.output(), 0.0);
    }

    #[test]
    fn test_read_nr0() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.nr0 = 0x80;
        assert_eq!(channel.read(0xFF1A), Some(0x80 | 0x7F));
    }

    #[test]
    fn test_read_nr2() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.nr2 = 0x60;
        assert_eq!(channel.read(0xFF1C), Some(0x60 | 0x9F));
    }

    #[test]
    fn test_read_nr4() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.nr4 = 0x80;
        assert_eq!(channel.read(0xFF1E), Some(0x80 | 0xBF));
    }

    #[test]
    fn test_read_fixed_addresses() {
        let channel = create_wave_channel(ModelType::DMG);
        assert_eq!(channel.read(0xFF1B), Some(0xFF));
        assert_eq!(channel.read(0xFF1D), Some(0xFF));
    }

    #[test]
    fn test_read_wave_ram_when_disabled() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = false;
        channel.wave_ram[0] = 0x12;
        assert_eq!(channel.read(0xFF30), Some(0x12));
    }

    #[test]
    fn test_read_wave_ram_when_enabled() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.wave_pos = 0;
        channel.wave_ram[0] = 0x12;
        assert_eq!(channel.read(0xFF30), Some(0x12));
    }

    #[test]
    fn test_read_invalid_address() {
        let channel = create_wave_channel(ModelType::DMG);
        assert_eq!(channel.read(0xFF00), None);
    }

    #[test]
    fn test_length_tick_disabled() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.length_enabled = false;
        channel.length_counter = 100;

        channel.length_tick();

        assert_eq!(channel.length_counter, 100);
        assert_eq!(channel.enabled, true);
    }

    #[test]
    fn test_length_tick_enabled_decrements() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.length_enabled = true;
        channel.length_counter = 2;

        channel.length_tick();
        assert_eq!(channel.length_counter, 1);

        channel.length_tick();
        assert_eq!(channel.length_counter, 0);
        assert_eq!(channel.enabled, false);
    }

    #[test]
    fn test_tick_decrements_period_timer() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.period_timer = 100;
        channel.enabled = true;

        channel.tick();

        assert_eq!(channel.period_timer, 99);
    }

    #[test]
    fn test_output_with_shift_0() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.nr2 = 0x00; // Shift: 0
        channel.wave_ram[0] = 0x12;
        channel.wave_pos = 0;

        assert_eq!(channel.output(), 0.0);
    }

    #[test]
    fn test_output_with_shift_1() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.nr2 = 0x20; // Shift: 1
        channel.wave_ram[0] = 0xF0;
        channel.wave_pos = 0;

        let output = channel.output();
        assert_eq!(output, 15.0 / 15.0);
    }

    #[test]
    fn test_output_with_shift_2() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.nr2 = 0x40; // Shift: 2
        channel.wave_ram[0] = 0xF0;
        channel.wave_pos = 0;

        let output = channel.output();
        let expected = ((15u8 >> 1) as f32) / 15.0;
        assert_eq!(output, expected);
    }

    #[test]
    fn test_output_with_shift_3() {
        let mut channel = create_wave_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.nr2 = 0x60; // Shift: 3
        channel.wave_ram[0] = 0xF0;
        channel.wave_pos = 0;

        let output = channel.output();
        let expected = ((15u8 >> 2) as f32) / 15.0;
        assert_eq!(output, expected);
    }
}
