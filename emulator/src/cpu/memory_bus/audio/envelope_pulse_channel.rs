const ENVELOPE_TICK_CYCLES: u16 = 8192;

const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

pub struct EnvelopePulseChannel {
    nr1: u8,
    nr2: u8,
    nr3: u8,
    nr4: u8,

    enabled: bool,
    duty_step: u8,
    period_timer: u16,
    volume: u8,

    envelope_enabled: bool,
    envelope_timer: u8,
    envelope_counter: u16,
}

impl EnvelopePulseChannel {
    pub fn new() -> Self {
        let mut result = Self {
            nr1: 0x3F,
            nr2: 0x00,
            nr3: 0xFF,
            nr4: 0xBF,

            enabled: false,
            duty_step: 0,
            period_timer: 0,
            volume: 0,

            envelope_enabled: false,
            envelope_timer: 0,
            envelope_counter: 0,
        };

        result.write_nr14(0xBF);
        result
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF16 => Some(self.nr1 | 0x3F),
            0xFF17 => Some(self.nr2),
            0xFF18 => Some(0xFF),
            0xFF19 => Some(self.nr4 | 0xBF),
            _ => None,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0xFF16 => self.nr1 = value,
            0xFF17 => self.nr2 = value,
            0xFF18 => self.nr3 = value,
            0xFF19 => self.write_nr14(value),
            _ => return false,
        }

        true
    }

    pub fn tick(&mut self) {
        if self.period_timer == 0 {
            let period = ((self.nr4 & 0x07) as u16) << 8 | self.nr3 as u16;
            self.period_timer = (2048 - period) * 4;

            self.duty_step = (self.duty_step + 1) % 8;
        } else {
            self.period_timer -= 1;
        }

        self.envelope_counter += 1;
        if self.envelope_counter >= ENVELOPE_TICK_CYCLES {
            self.envelope_counter = 0;
            self.envelope_tick();
        }
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

    pub fn output(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        let duty = (self.nr1 >> 6) as usize;
        let step = self.duty_step as usize;
        let high = DUTY_PATTERNS[duty][step] == 1;

        if high { self.volume as f32 / 15.0 } else { 0.0 }
    }

    fn write_nr14(&mut self, value: u8) {
        self.nr4 = value;

        if value & 0x80 != 0 {
            self.enabled = true;
            self.duty_step = 0;
            self.volume = (self.nr2 >> 4) & 0x0F;

            self.envelope_timer = self.nr2 & 0x07;
            self.envelope_enabled = true;

            let period = ((self.nr4 & 0x07) as u16) << 8 | self.nr3 as u16;
            self.period_timer = (2048 - period) * 4;

            if self.nr2 & 0xF8 == 0 {
                self.enabled = false;
            }
        }
    }
}
