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
}

impl WaveChannel {
    pub fn new() -> Self {
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
        self.write_nr1(0);
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
            },
            _ => None,
        }
    }

    pub fn write(&mut self, frame_sequencer_step: u8, address: u16, value: u8) -> bool {
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
            },
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

        (shifted as f32 / 7.5) - 1.0
    }
}
