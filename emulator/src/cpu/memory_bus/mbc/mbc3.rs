const EXTERNAL_RAM_SIZE: usize = 0xBFFF - 0xA000 + 1;
const EXTERNAL_RAM_BANKS: usize = 8;
const CYCLES_PER_SECOND: u32 = 4_194_304;
const RTC_OFFSET: usize = EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS;
const LATCHED_RTC_OFFSET: usize = RTC_OFFSET + 5;
const CYCLES_OFFSET: usize = LATCHED_RTC_OFFSET + 5;

pub struct MBC3 {
    rom: Vec<u8>,
    rom_bank: u8,
    external_ram: [u8; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS + 15],
    ram_enabled: bool,
    ram_bank: u8,
    latch_state: u8,
    has_battery_saves: bool,
    has_data_to_save: bool,
}

impl MBC3 {
    fn load_u32(&self, offset: usize) -> u32 {
        let mut num: u32 = 0;

        num |= self.external_ram[offset] as u32;
        num |= (self.external_ram[offset + 1] as u32) << 8;
        num |= (self.external_ram[offset + 2] as u32) << 16;
        num |= (self.external_ram[offset + 3] as u32) << 24;

        num
    }

    fn store_u32(&mut self, offset: usize, value: u32) {
        self.external_ram[offset] = value as u8;
        self.external_ram[offset + 1] = (value >> 8) as u8;
        self.external_ram[offset + 2] = (value >> 16) as u8;
        self.external_ram[offset + 3] = (value >> 24) as u8;
    }

    fn rtc_seconds(&self) -> u8 {
        self.external_ram[RTC_OFFSET]
    }

    fn set_rtc_seconds(&mut self, value: u8) {
        self.external_ram[RTC_OFFSET] = value;
    }

    fn rtc_minutes(&self) -> u8 {
        self.external_ram[RTC_OFFSET + 1]
    }

    fn set_rtc_minutes(&mut self, value: u8) {
        self.external_ram[RTC_OFFSET + 1] = value;
    }

    fn rtc_hours(&self) -> u8 {
        self.external_ram[RTC_OFFSET + 2]
    }

    fn set_rtc_hours(&mut self, value: u8) {
        self.external_ram[RTC_OFFSET + 2] = value;
    }

    fn rtc_days_low(&self) -> u8 {
        self.external_ram[RTC_OFFSET + 3]
    }

    fn set_rtc_days_low(&mut self, value: u8) {
        self.external_ram[RTC_OFFSET + 3] = value;
    }

    fn rtc_days_high(&self) -> u8 {
        self.external_ram[RTC_OFFSET + 4]
    }

    fn set_rtc_days_high(&mut self, value: u8) {
        self.external_ram[RTC_OFFSET + 4] = value;
    }

    fn rtc_latched_seconds(&self) -> u8 {
        self.external_ram[LATCHED_RTC_OFFSET]
    }

    fn set_rtc_latched_seconds(&mut self, value: u8) {
        self.external_ram[LATCHED_RTC_OFFSET] = value;
    }

    fn rtc_latched_minutes(&self) -> u8 {
        self.external_ram[LATCHED_RTC_OFFSET + 1]
    }

    fn set_rtc_latched_minutes(&mut self, value: u8) {
        self.external_ram[LATCHED_RTC_OFFSET + 1] = value;
    }

    fn rtc_latched_hours(&self) -> u8 {
        self.external_ram[LATCHED_RTC_OFFSET + 2]
    }

    fn set_rtc_latched_hours(&mut self, value: u8) {
        self.external_ram[LATCHED_RTC_OFFSET + 2] = value;
    }

    fn rtc_latched_days_low(&self) -> u8 {
        self.external_ram[LATCHED_RTC_OFFSET + 3]
    }

    fn set_rtc_latched_days_low(&mut self, value: u8) {
        self.external_ram[LATCHED_RTC_OFFSET + 3] = value;
    }

    fn rtc_latched_days_high(&self) -> u8 {
        self.external_ram[LATCHED_RTC_OFFSET + 4]
    }

    fn set_rtc_latched_days_high(&mut self, value: u8) {
        self.external_ram[LATCHED_RTC_OFFSET + 4] = value;
    }

    pub fn new(rom: Vec<u8>, external_ram: Vec<u8>, has_battery_saves: bool) -> Self {
        let mut result = Self {
            rom,
            rom_bank: 1,
            external_ram: [0; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS + 15],
            ram_enabled: false,
            ram_bank: 0,
            latch_state: 0,
            has_battery_saves,
            has_data_to_save: false,
        };

        for i in 0..external_ram.len() {
            result.external_ram[i] = external_ram[i];
        }

        result
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
                        let offset = self.ram_bank as usize * EXTERNAL_RAM_SIZE
                            + (address as usize - 0xA000);
                        Some(self.external_ram[offset])
                    }
                    0x08 => Some(self.rtc_latched_seconds()),
                    0x09 => Some(self.rtc_latched_minutes()),
                    0x0A => Some(self.rtc_latched_hours()),
                    0x0B => Some(self.rtc_latched_days_low()),
                    0x0C => Some(self.rtc_latched_days_high()),
                    _ => Some(0xFF),
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
                    self.set_rtc_latched_seconds(self.rtc_seconds());
                    self.set_rtc_latched_minutes(self.rtc_minutes());
                    self.set_rtc_latched_hours(self.rtc_hours());
                    self.set_rtc_latched_days_low(self.rtc_days_low());
                    self.set_rtc_latched_days_high(self.rtc_days_high());
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
                        let offset = self.ram_bank as usize * EXTERNAL_RAM_SIZE
                            + (address as usize - 0xA000);
                        self.external_ram[offset] = value;
                    }
                    0x08 => self.set_rtc_seconds(value & 0x3F),
                    0x09 => self.set_rtc_minutes(value & 0x3F),
                    0x0A => self.set_rtc_hours(value & 0x1F),
                    0x0B => self.set_rtc_days_low(value),
                    0x0C => self.set_rtc_days_high(value & 0xC1),
                    _ => {
                        return false;
                    }
                }

                self.has_data_to_save = self.has_battery_saves;
            }
            _ => {
                return false;
            }
        }

        true
    }

    fn rom_bank(&self) -> usize {
        let num_banks = self.rom.len() / 0x4000;
        self.rom_bank as usize & (num_banks - 1)
    }

    pub fn save_data(&mut self) -> (*const u8, usize, bool) {
        let result = (
            self.external_ram.as_ptr(),
            self.external_ram.len(),
            self.has_data_to_save,
        );
        self.has_data_to_save = false;

        result
    }

    pub fn tick(&mut self, cycles: u32) {
        if self.rtc_days_high() & 0x40 != 0 {
            return;
        }

        let mut stored_cycles = self.load_u32(CYCLES_OFFSET);
        stored_cycles += cycles;

        while stored_cycles >= CYCLES_PER_SECOND {
            stored_cycles -= CYCLES_PER_SECOND;
            self.increment_rtc();
        }

        self.store_u32(CYCLES_OFFSET, stored_cycles);
        self.has_data_to_save = self.has_battery_saves;
    }

    fn increment_rtc(&mut self) {
        let seconds = self.rtc_seconds().wrapping_add(1);
        if seconds >= 60 {
            self.set_rtc_seconds(0);
            let minutes = self.rtc_minutes().wrapping_add(1);
            if minutes >= 60 {
                self.set_rtc_minutes(0);
                let hours = self.rtc_hours().wrapping_add(1);
                if hours >= 24 {
                    self.set_rtc_hours(0);
                    let days_high = self.rtc_days_high();
                    let days = ((days_high & 0x01) as u16) << 8 | self.rtc_days_low() as u16;
                    let new_days = days + 1;
                    self.set_rtc_days_low(new_days as u8);
                    self.set_rtc_days_high((days_high & 0xFE) | ((new_days >> 8) as u8 & 0x01));
                    if new_days >= 512 {
                        self.set_rtc_days_high((days_high | 0x80) & !0x01);
                        self.set_rtc_days_low(0);
                    }
                } else {
                    self.set_rtc_hours(hours);
                }
            } else {
                self.set_rtc_minutes(minutes);
            }
        } else {
            self.set_rtc_seconds(seconds);
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
        let mbc = MBC3::new(rom, vec![], false);

        assert_eq!(mbc.rom_bank, 1);
        assert!(!mbc.ram_enabled);
        assert_eq!(mbc.ram_bank, 0);
        assert_eq!(mbc.rtc_seconds(), 0);
        assert_eq!(mbc.rtc_minutes(), 0);
        assert_eq!(mbc.rtc_hours(), 0);
        assert_eq!(mbc.rtc_days_low(), 0);
        assert_eq!(mbc.rtc_days_high(), 0);
    }

    #[test]
    fn test_rom_read_low_bank() {
        let mut rom = make_test_rom(0x8000);
        rom[0x0000] = 0x55;
        rom[0x1FFF] = 0xAA;
        let mbc = MBC3::new(rom, vec![], false);

        assert_eq!(mbc.read(0x0000), Some(0x55));
        assert_eq!(mbc.read(0x1FFF), Some(0xAA));
    }

    #[test]
    fn test_rom_read_switchable_bank() {
        let mut rom = make_test_rom(65536);
        // Bank 1 (default) at switchable area (0x4000-0x7FFF): rom offset 0x4000
        rom[0x4000] = 0x11;
        // Bank 2 at switchable area (0x4000-0x7FFF): rom offset 0x8000
        rom[0x8000] = 0x22;
        let mut mbc = MBC3::new(rom, vec![], false);

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
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x2000, 0x00);
        assert_eq!(mbc.rom_bank, 1);
    }

    #[test]
    fn test_rom_bank_masking() {
        let rom = make_test_rom(0x18000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x2000, 0xFF);
        assert_eq!(mbc.rom_bank, 0x7F);
    }

    #[test]
    fn test_ram_enable_with_0x0a() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        assert!(mbc.ram_enabled);
    }

    #[test]
    fn test_ram_disable_with_other_values() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

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
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.write(0xA000, 0x42);
        mbc.write(0x0000, 0x00);

        assert_eq!(mbc.read(0xA000), Some(0xFF));
    }

    #[test]
    fn test_external_ram_read_write() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.write(0xA000, 0x42);
        assert_eq!(mbc.read(0xA000), Some(0x42));

        mbc.write(0xA001, 0x99);
        assert_eq!(mbc.read(0xA001), Some(0x99));
    }

    #[test]
    fn test_external_ram_bank_switching() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

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
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.set_rtc_seconds(42);
        mbc.set_rtc_minutes(30);

        // Latch the RTC
        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);

        assert_eq!(mbc.rtc_latched_seconds(), 42);
        assert_eq!(mbc.rtc_latched_minutes(), 30);
    }

    #[test]
    fn test_rtc_latch_requires_sequence() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.set_rtc_seconds(42);

        // Wrong sequence - should not latch
        mbc.write(0x6000, 0x01);
        assert_eq!(mbc.rtc_latched_seconds(), 0);

        // Correct sequence
        mbc.write(0x6000, 0x00);
        mbc.write(0x6000, 0x01);
        assert_eq!(mbc.rtc_latched_seconds(), 42);
    }

    #[test]
    fn test_rtc_seconds_read() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

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
        let mut mbc = MBC3::new(rom, vec![], false);

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
        let mut mbc = MBC3::new(rom, vec![], false);

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
        let mut mbc = MBC3::new(rom, vec![], false);

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
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x08);
        mbc.write(0xA000, 0x7F);

        assert_eq!(mbc.rtc_seconds(), 0x3F);
    }

    #[test]
    fn test_rtc_minutes_write_masking() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x09);
        mbc.write(0xA000, 0x7F);

        assert_eq!(mbc.rtc_minutes(), 0x3F);
    }

    #[test]
    fn test_rtc_hours_write_masking() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x0A);
        mbc.write(0xA000, 0xFF);

        assert_eq!(mbc.rtc_hours(), 0x1F);
    }

    #[test]
    fn test_rtc_days_high_write_masking() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 0x0C);
        mbc.write(0xA000, 0xFF);

        assert_eq!(mbc.rtc_days_high(), 0xC1);
    }

    #[test]
    fn test_tick_increments_seconds() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.tick(CYCLES_PER_SECOND);
        assert_eq!(mbc.rtc_seconds(), 1);
    }

    #[test]
    fn test_tick_increment_from_59_to_60() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.set_rtc_seconds(59);
        mbc.tick(CYCLES_PER_SECOND);

        assert_eq!(mbc.rtc_seconds(), 0);
        assert_eq!(mbc.rtc_minutes(), 1);
    }

    #[test]
    fn test_tick_minute_overflow() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.set_rtc_seconds(59);
        mbc.set_rtc_minutes(59);
        mbc.tick(CYCLES_PER_SECOND);

        assert_eq!(mbc.rtc_seconds(), 0);
        assert_eq!(mbc.rtc_minutes(), 0);
        assert_eq!(mbc.rtc_hours(), 1);
    }

    #[test]
    fn test_tick_hour_overflow() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.set_rtc_seconds(59);
        mbc.set_rtc_minutes(59);
        mbc.set_rtc_hours(23);
        mbc.tick(CYCLES_PER_SECOND);

        assert_eq!(mbc.rtc_seconds(), 0);
        assert_eq!(mbc.rtc_minutes(), 0);
        assert_eq!(mbc.rtc_hours(), 0);
        assert_eq!(mbc.rtc_days_low(), 1);
    }

    #[test]
    fn test_tick_day_overflow() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.set_rtc_days_low(0xFF);
        mbc.set_rtc_days_high(0x00);
        mbc.set_rtc_seconds(59);
        mbc.set_rtc_minutes(59);
        mbc.set_rtc_hours(23);
        mbc.tick(CYCLES_PER_SECOND);

        assert_eq!(mbc.rtc_days_low(), 0x00);
        assert_eq!(mbc.rtc_days_high() & 0x01, 0x01);
    }

    #[test]
    fn test_tick_day_counter_wraps() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        // Set days to 511 (0x01FF)
        mbc.set_rtc_days_low(0xFF);
        mbc.set_rtc_days_high(0x01);
        mbc.set_rtc_seconds(59);
        mbc.set_rtc_minutes(59);
        mbc.set_rtc_hours(23);
        mbc.tick(CYCLES_PER_SECOND);

        // Should wrap to 512 with carry flag set
        assert_eq!(mbc.rtc_days_low(), 0x00);
        assert_eq!(mbc.rtc_days_high() & 0x01, 0x00);
        assert_eq!(mbc.rtc_days_high() & 0x80, 0x80);
    }

    #[test]
    fn test_tick_with_halt_flag() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.set_rtc_seconds(30);
        mbc.set_rtc_days_high(0x40);

        mbc.tick(CYCLES_PER_SECOND);
        assert_eq!(mbc.rtc_seconds(), 30);
    }

    #[test]
    fn test_tick_partial_cycles() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.tick(CYCLES_PER_SECOND / 2);
        assert_eq!(mbc.rtc_seconds(), 0);

        mbc.tick(CYCLES_PER_SECOND / 2);
        assert_eq!(mbc.rtc_seconds(), 1);
    }

    #[test]
    fn test_tick_multiple_increments() {
        let rom = make_test_rom(0x8000);
        let mut mbc = MBC3::new(rom, vec![], false);

        mbc.tick(CYCLES_PER_SECOND * 150);
        assert_eq!(mbc.rtc_seconds(), 30);
        assert_eq!(mbc.rtc_minutes(), 2);
    }
}
