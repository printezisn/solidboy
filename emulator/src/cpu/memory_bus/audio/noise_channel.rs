use crate::cpu::memory_bus::types::ModelType;

pub struct NoiseChannel {
    lfsr: u16,
    period_timer: u16,
    enabled: bool,
    dac_enabled: bool,
    volume: u8,
    envelope_timer: u8,
    envelope_enabled: bool,
    length_counter: u16,
    length_enabled: bool,
    nr1: u8,
    nr2: u8,
    nr3: u8,
    nr4: u8,

    model_type: ModelType,
}

impl NoiseChannel {
    pub fn new(model_type: ModelType) -> Self {
        let mut result = Self {
            lfsr: 0,
            period_timer: 0,
            enabled: false,
            dac_enabled: false,
            volume: 0,
            envelope_timer: 0,
            envelope_enabled: false,
            length_counter: 0,
            length_enabled: false,
            nr1: 0xFF,
            nr2: 0x00,
            nr3: 0x00,
            nr4: 0xBF,

            model_type,
        };

        result.write_nr1(0xFF);
        result.write_nr2(0x00);
        result.write_nr4(0, 0xBF);

        result
    }

    pub fn clear(&mut self) {
        self.lfsr = 0;
        self.period_timer = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.volume = 0;
        self.envelope_timer = 0;
        self.envelope_enabled = false;
        self.length_counter = 0;
        self.length_enabled = false;

        if matches!(self.model_type, ModelType::Color) {
            self.write_nr1(0);
        } else {
            self.write_nr1(self.nr1 & 0x3F);
        }

        self.write_nr2(0);
        self.nr3 = 0;
        self.write_nr4(0, 0);
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF20 => Some(0xFF),
            0xFF21 => Some(self.nr2),
            0xFF22 => Some(self.nr3),
            0xFF23 => Some(self.nr4 | 0xBF),
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
        if !audio_enabled && (matches!(self.model_type, ModelType::Color) || address != 0xFF20) {
            return address >= 0xFF20 && address <= 0xFF23;
        }

        match address {
            0xFF20 => {
                if !audio_enabled {
                    self.write_nr1((self.nr1 & !0x3F) | (value & 0x3F));
                } else {
                    self.write_nr1(value);
                }
            }
            0xFF21 => self.write_nr2(value),
            0xFF22 => self.nr3 = value,
            0xFF23 => self.write_nr4(frame_sequencer_step, value),
            _ => return false,
        }

        true
    }

    fn write_nr1(&mut self, value: u8) {
        self.nr1 = value;
        self.length_counter = 64 - (value & 0x3F) as u16;
    }

    pub fn write_nr2(&mut self, value: u8) {
        self.nr2 = value;
        self.dac_enabled = value & 0xF8 != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
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
                    63
                } else {
                    64
                };
            }

            self.enabled = self.dac_enabled;
            self.lfsr = 0x7FFF;

            self.volume = (self.nr2 >> 4) & 0x0F;
            self.envelope_timer = self.nr2 & 0x07;
            self.envelope_enabled = true;

            self.period_timer = self.period();
        }
    }

    fn period(&self) -> u16 {
        let s = (self.nr3 >> 4) as u16;
        let r = (self.nr3 & 0x07) as u16;

        let divider = if r == 0 { 8 } else { r * 16 };
        divider << s
    }

    pub fn envelope_tick(&mut self) {
        let pace = self.nr2 & 0x07;
        if pace == 0 || !self.envelope_enabled {
            return;
        }

        self.envelope_timer -= 1;
        if self.envelope_timer == 0 {
            self.envelope_timer = pace;
            let direction = (self.nr2 >> 3) & 0x01;
            if direction == 1 && self.volume < 15 {
                self.volume += 1;
            } else if direction == 0 && self.volume > 0 {
                self.volume -= 1;
            } else {
                self.envelope_enabled = false;
            }
        }
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
            self.period_timer = self.period();

            let xor = (self.lfsr & 0x01) ^ ((self.lfsr >> 1) & 0x01);

            self.lfsr = (self.lfsr >> 1) | (xor << 14);

            if self.nr3 & 0x08 != 0 {
                self.lfsr = (self.lfsr & !(1 << 6)) | (xor << 6);
            }
        } else {
            self.period_timer -= 1;
        }
    }

    pub fn output(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        if self.lfsr & 0x01 == 0 {
            self.volume as f32 / 15.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_noise_channel(model_type: ModelType) -> NoiseChannel {
        NoiseChannel::new(model_type)
    }

    #[test]
    fn test_new_initialization() {
        let channel = create_noise_channel(ModelType::DMG);
        assert_eq!(channel.enabled, false);
        assert_eq!(channel.dac_enabled, false);
        assert_eq!(channel.volume, 0);
    }

    #[test]
    fn test_enabled_status() {
        let channel = create_noise_channel(ModelType::DMG);
        assert_eq!(channel.enabled(), false);
    }

    #[test]
    fn test_output_when_disabled() {
        let channel = create_noise_channel(ModelType::DMG);
        assert_eq!(channel.output(), 0.0);
    }

    #[test]
    fn test_output_when_dac_disabled() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = false;
        assert_eq!(channel.output(), 0.0);
    }

    #[test]
    fn test_read_nr1() {
        let channel = create_noise_channel(ModelType::DMG);
        assert_eq!(channel.read(0xFF20), Some(0xFF));
    }

    #[test]
    fn test_read_nr2() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.nr2 = 0xF3;
        assert_eq!(channel.read(0xFF21), Some(0xF3));
    }

    #[test]
    fn test_read_nr3() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.nr3 = 0x42;
        assert_eq!(channel.read(0xFF22), Some(0x42));
    }

    #[test]
    fn test_read_nr4() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.nr4 = 0x80;
        assert_eq!(channel.read(0xFF23), Some(0x80 | 0xBF));
    }

    #[test]
    fn test_read_invalid_address() {
        let channel = create_noise_channel(ModelType::DMG);
        assert_eq!(channel.read(0xFF00), None);
    }

    #[test]
    fn test_write_nr2_dac_enabled() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.write_nr2(0xF0);
        assert_eq!(channel.dac_enabled, true);
    }

    #[test]
    fn test_write_nr2_dac_disabled() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.enabled = true;
        channel.write_nr2(0x00);
        assert_eq!(channel.dac_enabled, false);
        assert_eq!(channel.enabled, false);
    }

    #[test]
    fn test_length_tick_disabled() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.enabled = true;
        channel.length_enabled = false;
        channel.length_counter = 64;

        channel.length_tick();

        assert_eq!(channel.length_counter, 64);
        assert_eq!(channel.enabled, true);
    }

    #[test]
    fn test_length_tick_enabled_decrements() {
        let mut channel = create_noise_channel(ModelType::DMG);
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
    fn test_envelope_tick_disabled() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.envelope_enabled = false;
        channel.volume = 10;

        channel.envelope_tick();

        assert_eq!(channel.volume, 10);
    }

    #[test]
    fn test_tick_decrements_period_timer() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.period_timer = 100;
        channel.enabled = true;

        channel.tick();

        assert_eq!(channel.period_timer, 99);
    }

    #[test]
    fn test_output_with_lfsr_bit_0_low() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.volume = 15;
        channel.lfsr = 0x0000; // LSB is 0

        let output = channel.output();
        assert_eq!(output, 1.0);
    }

    #[test]
    fn test_output_with_lfsr_bit_0_high() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.volume = 15;
        channel.lfsr = 0x0001; // LSB is 1

        assert_eq!(channel.output(), 0.0);
    }

    #[test]
    fn test_output_volume_scaling() {
        let mut channel = create_noise_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.volume = 8;
        channel.lfsr = 0x0000; // LSB is 0

        let output = channel.output();
        assert_eq!(output, 8.0 / 15.0);
    }
}
