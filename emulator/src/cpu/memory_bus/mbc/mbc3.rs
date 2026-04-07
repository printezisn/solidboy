const EXTERNAL_RAM_SIZE: usize = 0xBFFF - 0xA000 + 1;
const EXTERNAL_RAM_BANKS: usize = 8;
const CYCLES_PER_SECOND: u32 = 4_194_304;

pub struct MBC3 {
    rom: Vec<u8>,
    rom_bank: u8,
    external_ram: [u8; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS],
    ram_enabled: bool,
    ram_bank: u8,
    rtc: RTC,
    rtc_latched: RTC,
    latch_state: u8,
    cycles: u32,
}

#[derive(Clone, Copy)]
struct RTC {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub days_low: u8,
    pub days_high: u8,
}

impl MBC3 {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            rom_bank: 1,
            external_ram: [0; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS],
            ram_enabled: false,
            ram_bank: 0,
            rtc: RTC {
                seconds: 0,
                minutes: 0,
                hours: 0,
                days_low: 0,
                days_high: 0,
            },
            rtc_latched: RTC {
                seconds: 0,
                minutes: 0,
                hours: 0,
                days_low: 0,
                days_high: 0,
            },
            latch_state: 0,
            cycles: 0,
        }
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0x0000..=0x3FFF => Some(self.rom[address as usize]),
            0x4000..=0x7FFF => Some(self.rom[self.rom_bank() * 0x4000 + address as usize - 0x4000]),
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return Some(0xFF);
                }
                match self.ram_bank {
                    0x00..=0x07 => {
                        let offset = self.ram_bank as usize * EXTERNAL_RAM_SIZE + (address as usize - 0xA000);
                        Some(self.external_ram[offset])
                    }
                    0x08 => Some(self.rtc_latched.seconds),
                    0x09 => Some(self.rtc_latched.minutes),
                    0x0A => Some(self.rtc_latched.hours),
                    0x0B => Some(self.rtc_latched.days_low),
                    0x0C => Some(self.rtc_latched.days_high),
                    _ => Some(0xFF)
                }
            }
            _ => None,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0x0000..=0x1FFF => {
                self.ram_enabled = (value & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                let mut bank = value & 0x7F;
                if bank == 0 {
                    bank = 1;
                }
                self.rom_bank = bank;
            }
            0x4000..=0x5FFF => {
                self.ram_bank = value;
            }
            0x6000..=0x7FFF => {
                if self.latch_state == 0 && value == 0x00 {
                    self.latch_state = 1;
                } else if self.latch_state == 1 && value == 0x01 {
                    self.rtc_latched = self.rtc.clone();
                    self.latch_state = 0;
                } else {
                    self.latch_state = 0;
                }
            }
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return true;
                }

                match self.ram_bank {
                    0x00..=0x07 => {
                        let offset = self.ram_bank as usize * EXTERNAL_RAM_SIZE + (address as usize - 0xA000);
                        self.external_ram[offset] = value;
                    }
                    0x08 => self.rtc.seconds = value & 0x3F,
                    0x09 => self.rtc.minutes = value & 0x3F,
                    0x0A => self.rtc.hours = value & 0x1F,
                    0x0B => self.rtc.days_low = value,
                    0x0C => self.rtc.days_high = value & 0xC1,
                    _ => {
                        return false;
                    }
                }
            }
            _ => {
                return false;
            }
        }

        true
    }

    fn rom_bank(&self) -> usize {
        self.rom_bank as usize
    }

    pub fn tick(&mut self, cycles: u32) {
        if self.rtc.days_high & 0x40 != 0 {
            return;
        }

        self.cycles += cycles;
        while self.cycles >= CYCLES_PER_SECOND {
            self.cycles -= CYCLES_PER_SECOND;
            self.increment_rtc();
        }
    }

    fn increment_rtc(&mut self) {
        self.rtc.seconds += 1;
        if self.rtc.seconds >= 60 {
            self.rtc.seconds = 0;
            self.rtc.minutes += 1;
            if self.rtc.minutes >= 60 {
                self.rtc.minutes = 0;
                self.rtc.hours += 1;
                if self.rtc.hours >= 24 {
                    self.rtc.hours = 0;
                    let days = ((self.rtc.days_high & 0x01) as u16) << 8 
                            | self.rtc.days_low as u16;
                    let new_days = days + 1;
                    self.rtc.days_low = new_days as u8;
                    self.rtc.days_high = (self.rtc.days_high & 0xFE) 
                                    | ((new_days >> 8) as u8 & 0x01);
                    if new_days >= 512 {
                        self.rtc.days_high |= 0x80;
                        self.rtc.days_low = 0;
                        self.rtc.days_high &= !0x01;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_rom(size: usize) -> Vec<u8> {
        vec![0u8; size]
    }

    #[test]
    fn test_initialization() {
        let rom = make_test_rom(0x8000);
        let mbc = MBC3::new(rom);

        assert_eq!(mbc.rom_bank, 1);
        assert!(!mbc.ram_enabled);
        assert_eq!(mbc.ram_bank, 0);
        assert_eq!(mbc.rtc.seconds, 0);
        assert_eq!(mbc.rtc.minutes, 0);
        assert_eq!(mbc.rtc.hours, 0);
        assert_eq!(mbc.rtc.days_low, 0);
        assert_eq!(mbc.rtc.days_high, 0);
    }

    #[test]
    fn test_rom_read_low_bank() {
        let mut rom = make_test_rom(0x8000);
        rom[0x0000] = 0x55;
        rom[0x1FFF] = 0xAA;
        let mbc = MBC3::new(rom);

        assert_eq!(mbc.read(0x0000), Some(0x55));
        assert_eq!(mbc.read(0x1FFF), Some(0xAA));
    }

    #[test]
    fn test_rom_read_switchable_bank() {
        let mut rom = make_test_rom(0x18000);
        // Bank 1 (default) at switchable area (0x4000-0x7FFF): rom offset 0x4000
        rom[0x4000] = 0x11;
        // Bank 2 at switchable area (0x4000-0x7FFF): rom offset 0x8000
        rom[0x8000] = 0x22;
        let mut mbc = MBC3::new(rom);

        // Read from address 0x4000 with default bank 1
        assert_eq!(mbc.read(0x4000), Some(0x11));
        // Switch to bank 2
        mbc.write(0x2000, 2);
        // Read from address 0x4000 should now access rom[0x8000]
        assert_eq!(mbc.read(0x4000), Some(0x22));
    }

    #[test]
    fn test_rom_bank_zero_wraps_to_one() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x2000, 0x00);
        assert_eq!(mbc.rom_bank, 1);
    }

    #[test]
    fn test_rom_bank_masking() {
        let rom = make_test_rom(0x18000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x2000, 0xFF);
        assert_eq!(mbc.rom_bank, 0x7F);
    }

    #[test]
    fn test_ram_enable_with_0x0a() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        assert!(mbc.ram_enabled);
    }

    #[test]
    fn test_ram_disable_with_other_values() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        assert!(mbc.ram_enabled);

        mbc.write(0x0000, 0x00);
        assert!(!mbc.ram_enabled);

        mbc.write(0x0000, 0x0A);
        assert!(mbc.ram_enabled);

        mbc.write(0x0000, 0xFF);
        assert!(!mbc.ram_enabled);
    }

    #[test]
    fn test_external_ram_read_when_disabled() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0xA000, 0x42);
        mbc.write(0x0000, 0x00);

        assert_eq!(mbc.read(0xA000), Some(0xFF));
    }

    #[test]
    fn test_external_ram_read_write() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0xA000, 0x42);
        assert_eq!(mbc.read(0xA000), Some(0x42));

        mbc.write(0xA001, 0x99);
        assert_eq!(mbc.read(0xA001), Some(0x99));
    }

    #[test]
    fn test_external_ram_bank_switching() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);

        // Write to bank 0
        mbc.write(0xA000, 0x11);
        assert_eq!(mbc.read(0xA000), Some(0x11));

        // Switch to bank 1
        mbc.write(0x4000, 1);
        assert_eq!(mbc.read(0xA000), Some(0x00));

        // Write to bank 1
        mbc.write(0xA000, 0x22);
        assert_eq!(mbc.read(0xA000), Some(0x22));

        // Switch back to bank 0
        mbc.write(0x4000, 0);
        assert_eq!(mbc.read(0xA000), Some(0x11));
    }

    #[test]
    fn test_rtc_latch_mechanism() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.rtc.seconds = 42;
        mbc.rtc.minutes = 30;

        // Latch the RTC
        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);

        assert_eq!(mbc.rtc_latched.seconds, 42);
        assert_eq!(mbc.rtc_latched.minutes, 30);
    }

    #[test]
    fn test_rtc_latch_requires_sequence() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.rtc.seconds = 42;

        // Wrong sequence - should not latch
        mbc.write(0x6000, 0x01);
        assert_eq!(mbc.rtc_latched.seconds, 0);

        // Correct sequence
        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);
        assert_eq!(mbc.rtc_latched.seconds, 42);
    }

    #[test]
    fn test_rtc_seconds_read() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x08);
        mbc.write(0xA000, 0x25);

        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);

        assert_eq!(mbc.read(0xA000), Some(0x25));
    }

    #[test]
    fn test_rtc_minutes_read() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x09);
        mbc.write(0xA000, 0x3B);

        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);

        assert_eq!(mbc.read(0xA000), Some(0x3B));
    }

    #[test]
    fn test_rtc_hours_read() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x0A);
        mbc.write(0xA000, 0x17);

        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);

        assert_eq!(mbc.read(0xA000), Some(0x17));
    }

    #[test]
    fn test_rtc_days_read() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x0B);
        mbc.write(0xA000, 0xFF);

        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);

        assert_eq!(mbc.read(0xA000), Some(0xFF));
    }

    #[test]
    fn test_rtc_seconds_write_masking() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x08);
        mbc.write(0xA000, 0x7F);

        assert_eq!(mbc.rtc.seconds, 0x3F);
    }

    #[test]
    fn test_rtc_minutes_write_masking() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x09);
        mbc.write(0xA000, 0x7F);

        assert_eq!(mbc.rtc.minutes, 0x3F);
    }

    #[test]
    fn test_rtc_hours_write_masking() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x0A);
        mbc.write(0xA000, 0xFF);

        assert_eq!(mbc.rtc.hours, 0x1F);
    }

    #[test]
    fn test_rtc_days_high_write_masking() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x0C);
        mbc.write(0xA000, 0xFF);

        assert_eq!(mbc.rtc.days_high, 0xC1);
    }

    #[test]
    fn test_tick_increments_seconds() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.tick(CYCLES_PER_SECOND);
        assert_eq!(mbc.rtc.seconds, 1);
    }

    #[test]
    fn test_tick_increment_from_59_to_60() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.rtc.seconds = 59;
        mbc.tick(CYCLES_PER_SECOND);

        assert_eq!(mbc.rtc.seconds, 0);
        assert_eq!(mbc.rtc.minutes, 1);
    }

    #[test]
    fn test_tick_minute_overflow() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.rtc.seconds = 59;
        mbc.rtc.minutes = 59;
        mbc.tick(CYCLES_PER_SECOND);

        assert_eq!(mbc.rtc.seconds, 0);
        assert_eq!(mbc.rtc.minutes, 0);
        assert_eq!(mbc.rtc.hours, 1);
    }

    #[test]
    fn test_tick_hour_overflow() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.rtc.seconds = 59;
        mbc.rtc.minutes = 59;
        mbc.rtc.hours = 23;
        mbc.tick(CYCLES_PER_SECOND);

        assert_eq!(mbc.rtc.seconds, 0);
        assert_eq!(mbc.rtc.minutes, 0);
        assert_eq!(mbc.rtc.hours, 0);
        assert_eq!(mbc.rtc.days_low, 1);
    }

    #[test]
    fn test_tick_day_overflow() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.rtc.days_low = 0xFF;
        mbc.rtc.days_high = 0x00;
        mbc.rtc.seconds = 59;
        mbc.rtc.minutes = 59;
        mbc.rtc.hours = 23;
        mbc.tick(CYCLES_PER_SECOND);

        assert_eq!(mbc.rtc.days_low, 0x00);
        assert_eq!(mbc.rtc.days_high & 0x01, 0x01);
    }

    #[test]
    fn test_tick_day_counter_wraps() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        // Set days to 511 (0x01FF)
        mbc.rtc.days_low = 0xFF;
        mbc.rtc.days_high = 0x01;
        mbc.rtc.seconds = 59;
        mbc.rtc.minutes = 59;
        mbc.rtc.hours = 23;
        mbc.tick(CYCLES_PER_SECOND);

        // Should wrap to 512 with carry flag set
        assert_eq!(mbc.rtc.days_low, 0x00);
        assert_eq!(mbc.rtc.days_high & 0x01, 0x00);
        assert_eq!(mbc.rtc.days_high & 0x80, 0x80);
    }

    #[test]
    fn test_tick_with_halt_flag() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.rtc.seconds = 30;
        mbc.rtc.days_high = 0x40;

        mbc.tick(CYCLES_PER_SECOND);
        assert_eq!(mbc.rtc.seconds, 30);
    }

    #[test]
    fn test_tick_partial_cycles() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.tick(CYCLES_PER_SECOND / 2);
        assert_eq!(mbc.rtc.seconds, 0);

        mbc.tick(CYCLES_PER_SECOND / 2);
        assert_eq!(mbc.rtc.seconds, 1);
    }

    #[test]
    fn test_tick_multiple_increments() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.tick(CYCLES_PER_SECOND * 150);
        assert_eq!(mbc.rtc.seconds, 30);
        assert_eq!(mbc.rtc.minutes, 2);
    }
}
