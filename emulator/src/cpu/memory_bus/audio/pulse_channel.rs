use crate::cpu::memory_bus::types::ModelType;

const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

pub struct PulseChannel {
    nr0: u8,
    nr1: u8,
    nr2: u8,
    nr3: u8,
    nr4: u8,

    enabled: bool,
    dac_enabled: bool,
    duty_step: u8,
    period_timer: u16,
    volume: u8,

    envelope_enabled: bool,
    envelope_timer: u8,

    length_enabled: bool,
    length_counter: u16,

    sweep_enabled: bool,
    sweep_timer: u8,
    sweep_period: u16,
    sweep_negate_used: bool,

    model_type: ModelType,
}

impl PulseChannel {
    pub fn new(model_type: ModelType) -> Self {
        let mut result = Self {
            nr0: 0x80,
            nr1: 0xBF,
            nr2: 0xF3,
            nr3: 0xFF,
            nr4: 0xBF,

            enabled: false,
            dac_enabled: false,
            duty_step: 0,
            period_timer: 0,
            volume: 0,

            envelope_enabled: false,
            envelope_timer: 0,

            length_enabled: false,
            length_counter: 0,

            sweep_enabled: false,
            sweep_timer: 0,
            sweep_period: 0,
            sweep_negate_used: false,

            model_type,
        };

        result.write_nr1(0xBF);
        result.write_nr2(0xF3);
        result.write_nr4(0, 0xBF);
        result
    }

    pub fn clear(&mut self) {
        self.enabled = false;
        self.duty_step = 0;
        self.period_timer = 0;
        self.volume = 0;

        self.envelope_enabled = false;
        self.envelope_timer = 0;

        self.length_enabled = false;
        self.length_counter = 0;

        self.sweep_enabled = false;
        self.sweep_timer = 0;
        self.sweep_period = 0;

        self.write_nr0(0);

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
            0xFF10 => Some(self.nr0 | 0x80),
            0xFF11 => Some(self.nr1 | 0x3F),
            0xFF12 => Some(self.nr2),
            0xFF13 => Some(0xFF),
            0xFF14 => Some(self.nr4 | 0xBF),
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
        if !audio_enabled && (matches!(self.model_type, ModelType::Color) || address != 0xFF11) {
            return address >= 0xFF10 && address <= 0xFF14;
        }

        match address {
            0xFF10 => self.write_nr0(value),
            0xFF11 => {
                if !audio_enabled {
                    self.write_nr1((self.nr1 & !0x3F) | (value & 0x3F));
                } else {
                    self.write_nr1(value);
                }
            }
            0xFF12 => self.write_nr2(value),
            0xFF13 => self.nr3 = value,
            0xFF14 => self.write_nr4(frame_sequencer_step, value),
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

    pub fn tick(&mut self) {
        if self.period_timer == 0 {
            let period = ((self.nr4 & 0x07) as u16) << 8 | self.nr3 as u16;
            self.period_timer = (2048 - period) * 4;

            self.duty_step = (self.duty_step + 1) % 8;
        } else {
            self.period_timer -= 1;
        }
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

    pub fn sweep_tick(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }

        if self.sweep_timer == 0 {
            let pace = (self.nr0 >> 4) & 0x07;
            self.sweep_timer = if pace == 0 { 8 } else { pace };

            if self.sweep_enabled && pace != 0 {
                if let Some(new_period) = self.calculate_new_period() {
                    let shift = self.nr0 & 0x07;
                    if shift != 0 {
                        self.sweep_period = new_period;
                        self.nr3 = new_period as u8;
                        self.nr4 = (self.nr4 & 0xF8) | ((new_period >> 8) as u8 & 0x07);

                        self.calculate_new_period();
                    }
                }
            }
        }
    }

    pub fn output(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        let duty = (self.nr1 >> 6) as usize;
        let step = self.duty_step as usize;
        let high = DUTY_PATTERNS[duty][step] == 1;

        if high { self.volume as f32 / 15.0 } else { 0.0 }
    }

    fn write_nr0(&mut self, value: u8) {
        let old_negate = self.nr0 & 0x08 != 0;
        let new_negate = value & 0x08 != 0;

        if old_negate && !new_negate && self.sweep_negate_used {
            self.enabled = false;
        }

        self.nr0 = value;
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

            self.sweep_negate_used = false;
            self.enabled = self.dac_enabled;
            self.duty_step = 0;
            self.volume = (self.nr2 >> 4) & 0x0F;

            self.envelope_timer = self.nr2 & 0x07;
            self.envelope_enabled = true;

            let period = ((self.nr4 & 0x07) as u16) << 8 | self.nr3 as u16;
            self.period_timer = (2048 - period) * 4;

            let pace = (self.nr0 >> 4) & 0x07;
            let shift = self.nr0 & 0x07;

            self.sweep_period = ((self.nr4 & 0x07) as u16) << 8 | self.nr3 as u16;
            self.sweep_timer = if pace == 0 { 8 } else { pace };
            self.sweep_enabled = pace != 0 || shift != 0;

            if shift != 0 {
                self.calculate_new_period();
            }
        }
    }

    fn calculate_new_period(&mut self) -> Option<u16> {
        let shift = self.nr0 & 0x07;
        let direction = (self.nr0 >> 3) & 0x01;

        let delta = self.sweep_period >> shift;
        let new_period = if direction == 1 {
            self.sweep_negate_used = true;
            if self.sweep_period < delta {
                0
            } else {
                self.sweep_period - delta
            }
        } else {
            self.sweep_period + delta
        };

        if new_period > 2047 {
            self.enabled = false;
            None
        } else {
            Some(new_period)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_pulse_channel(model_type: ModelType) -> PulseChannel {
        PulseChannel::new(model_type)
    }

    #[test]
    fn test_new_initialization() {
        let channel = create_pulse_channel(ModelType::DMG);
        // After write_nr4(0, 0xBF) is called, channel is enabled with dac_enabled=true
        // and volume is set from nr2: (0xF3 >> 4) & 0x0F = 15
        assert_eq!(channel.enabled, true);
        assert_eq!(channel.dac_enabled, true);
        assert_eq!(channel.volume, 15);
    }

    #[test]
    fn test_enabled_status_after_init() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        // Initially enabled after write_nr4(0, 0xBF)
        assert_eq!(channel.enabled, true);

        // Disable dac
        channel.write_nr2(0x00);
        assert_eq!(channel.dac_enabled, false);
        assert_eq!(channel.enabled, false);
    }

    #[test]
    fn test_output_when_not_enabled() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        // Initially enabled with volume 15
        assert_eq!(channel.enabled, true);

        // Disable to test output
        channel.enabled = false;
        assert_eq!(channel.output(), 0.0);
    }

    #[test]
    fn test_output_when_dac_disabled() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = false;
        assert_eq!(channel.output(), 0.0);
    }

    #[test]
    fn test_write_nr2_dac_enabled() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.write_nr2(0xF0);
        assert_eq!(channel.dac_enabled, true);
    }

    #[test]
    fn test_write_nr2_dac_disabled() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.enabled = true;
        channel.write_nr2(0x00);
        assert_eq!(channel.dac_enabled, false);
        assert_eq!(channel.enabled, false);
    }

    #[test]
    fn test_read_returns_correct_values() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.nr1 = 0xC0;
        channel.nr2 = 0xF3;
        channel.nr4 = 0x80;

        // 0xFF10: nr0 = 0x80, so 0x80 | 0x80 = 0x80
        assert_eq!(channel.read(0xFF10), Some(0x80));
        assert_eq!(channel.read(0xFF11), Some(0xC0 | 0x3F));
        assert_eq!(channel.read(0xFF12), Some(0xF3));
        assert_eq!(channel.read(0xFF13), Some(0xFF));
        assert_eq!(channel.read(0xFF14), Some(0x80 | 0xBF));
    }

    #[test]
    fn test_read_invalid_address() {
        let channel = create_pulse_channel(ModelType::DMG);
        assert_eq!(channel.read(0xFF00), None);
    }

    #[test]
    fn test_envelope_tick_increase() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.envelope_enabled = true;
        channel.envelope_timer = 0;
        channel.volume = 5;
        channel.nr2 = 0x08; // Envelope pace = 0, direction = 1 (increase)

        channel.envelope_tick();
        // Should not increase when pace is 0
        assert_eq!(channel.volume, 5);
    }

    #[test]
    fn test_length_tick_disabled() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.enabled = true;
        channel.length_enabled = false;
        channel.length_counter = 10;

        channel.length_tick();

        assert_eq!(channel.length_counter, 10);
        assert_eq!(channel.enabled, true);
    }

    #[test]
    fn test_length_tick_enabled_decrements() {
        let mut channel = create_pulse_channel(ModelType::DMG);
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
    fn test_tick_increments_duty_step() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.period_timer = 0;
        let initial_step = channel.duty_step;

        channel.tick();

        assert_eq!(channel.duty_step, (initial_step + 1) % 8);
    }

    #[test]
    fn test_tick_decrements_period_timer() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.period_timer = 100;

        channel.tick();

        assert_eq!(channel.period_timer, 99);
    }

    #[test]
    fn test_output_with_duty_pattern() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.volume = 15;
        channel.duty_step = 0;
        channel.nr1 = 0x00; // Duty pattern 0: [0, 0, 0, 0, 0, 0, 0, 1]

        assert_eq!(channel.output(), 0.0);

        channel.duty_step = 7;
        assert_eq!(channel.output(), 1.0);
    }

    #[test]
    fn test_output_volume_scaling() {
        let mut channel = create_pulse_channel(ModelType::DMG);
        channel.enabled = true;
        channel.dac_enabled = true;
        channel.volume = 7;
        channel.duty_step = 7;
        channel.nr1 = 0x00; // Duty pattern 0

        let output = channel.output();
        assert_eq!(output, 7.0 / 15.0);
    }
}
