const ENVELOPE_TICK_CYCLES: u16 = 8192;

pub struct NoiseChannel {
    lfsr: u16,
    period_timer: u16,
    enabled: bool,
    dac_enabled: bool,
    volume: u8,
    envelope_timer: u8,
    envelope_enabled: bool,
    envelope_counter: u16,
    nr1: u8,
    nr2: u8,
    nr3: u8,
    nr4: u8,
}

impl NoiseChannel {
    pub fn new() -> Self {
        let mut result = Self {
            lfsr: 0,
            period_timer: 0,
            enabled: false,
            dac_enabled: false,
            volume: 0,
            envelope_timer: 0,
            envelope_enabled: false,
            envelope_counter: 0,
            nr1: 0xFF,
            nr2: 0x00,
            nr3: 0x00,
            nr4: 0xBF,
        };

        result.write_nr44(0xBF);

        result
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

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0xFF20 => self.nr1 = value,
            0xFF21 => self.nr2 = value,
            0xFF22 => self.nr3 = value,
            0xFF23 => self.write_nr44(value),
            _ => return false,
        }

        true
    }

    fn write_nr44(&mut self, val: u8) {
        self.nr4 = val;

        if val & 0x80 != 0 {
            self.enabled = self.dac_enabled;
            self.lfsr = 0x7FFF;

            self.volume = (self.nr2 >> 4) & 0x0F;
            self.envelope_timer = self.nr2 & 0x07;
            self.envelope_enabled = true;

            if self.nr2 & 0xF8 == 0 {
                self.enabled = false;
            }

            self.period_timer = self.period();
        }
    }

    fn period(&self) -> u16 {
        let s = (self.nr3 >> 4) as u16;
        let r = (self.nr3 & 0x07) as u16;

        let divider = if r == 0 { 8 } else { r * 16 };
        divider << s
    }

    fn envelope_tick(&mut self) {
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

        self.envelope_counter += 1;
        if self.envelope_counter >= ENVELOPE_TICK_CYCLES {
            self.envelope_counter = 0;
            self.envelope_tick();
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
