const EXTERNAL_RAM_SIZE: usize = 0xBFFF - 0xA000 + 1;
const EXTERNAL_RAM_BANKS: usize = 4;

pub struct MBC1 {
    rom: Vec<u8>,
    rom_bank_low: u8,
    rom_bank_high: u8,
    external_ram: [u8; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS],
    ram_enabled: bool,
    banking_mode: u8,
    has_battery_saves: bool,
    has_data_to_save: bool,
}

impl MBC1 {
    pub fn new(rom: Vec<u8>, external_ram: Vec<u8>, has_battery_saves: bool) -> Self {
        let mut result = Self {
            rom,
            rom_bank_low: 1,
            rom_bank_high: 0,
            external_ram: [0; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS],
            ram_enabled: false,
            banking_mode: 0,
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
            0x0000..=0x3FFF => {
                let bank: usize = if self.banking_mode == 0 {
                    0
                } else {
                    self.rom_bank_high << 5
                } as usize;

                Some(self.rom[bank * 0x4000 + address as usize])
            }
            0x4000..=0x7FFF => Some(self.rom[self.rom_bank() * 0x4000 + address as usize - 0x4000]),
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return Some(0xFF);
                }

                Some(
                    self.external_ram
                        [self.ram_bank() * EXTERNAL_RAM_SIZE + address as usize - 0xA000],
                )
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
                let mut bank = value & 0x1F;
                if bank == 0 {
                    bank = 1;
                }
                self.rom_bank_low = bank;
            }
            0x4000..=0x5FFF => {
                self.rom_bank_high = value & 0x03;
            }
            0x6000..=0x7FFF => {
                self.banking_mode = value & 1;
            }
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    self.external_ram
                        [self.ram_bank() * EXTERNAL_RAM_SIZE + address as usize - 0xA000] = value;
                    self.has_data_to_save = self.has_battery_saves;
                }
            }
            _ => {
                return false;
            }
        }

        true
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

    fn rom_bank(&self) -> usize {
        let mut bank: usize = if self.banking_mode == 0 {
            (self.rom_bank_high << 5) | self.rom_bank_low
        } else {
            self.rom_bank_low
        } as usize;

        if bank == 0 {
            bank = 1;
        }

        bank
    }

    fn ram_bank(&self) -> usize {
        if self.banking_mode == 0 {
            0
        } else {
            self.rom_bank_high as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rom(size: usize) -> Vec<u8> {
        let mut rom = vec![0u8; size];
        for i in 0..size {
            rom[i] = (i & 0xFF) as u8;
        }
        rom
    }

    #[test]
    fn read_rom_bank_0() {
        let rom = make_rom(0x8000);
        let mbc = MBC1::new(rom, vec![], false);

        assert_eq!(mbc.read(0x0000), Some(0x00));
        assert_eq!(mbc.read(0x3FFF), Some(0xFF));
    }

    #[test]
    fn read_rom_bank_1() {
        let rom = make_rom(0x10000); // 2 banks
        let mbc = MBC1::new(rom, vec![], false);

        assert_eq!(mbc.read(0x4000), Some(0x00));
        assert_eq!(mbc.read(0x7FFF), Some(0xFF));
    }

    #[test]
    fn switch_rom_bank() {
        let rom = make_rom(0x20000); // 4 banks
        let mut mbc = MBC1::new(rom, vec![], false);

        // Switch to bank 2
        mbc.write(0x2000, 2);
        assert_eq!(mbc.read(0x4000), Some(0x00));
        assert_eq!(mbc.read(0x7FFF), Some(0xFF));

        // Switch to bank 3
        mbc.write(0x2000, 3);
        assert_eq!(mbc.read(0x4000), Some(0x00));
        assert_eq!(mbc.read(0x7FFF), Some(0xFF));
    }

    #[test]
    fn ram_enable_disable() {
        let rom = make_rom(0x8000);
        let mut mbc = MBC1::new(rom, vec![], false);

        // RAM disabled by default
        assert_eq!(mbc.read(0xA000), Some(0xFF));

        // Enable RAM
        mbc.write(0x0000, 0x0A);
        assert_eq!(mbc.read(0xA000), Some(0x00));

        // Disable RAM
        mbc.write(0x0000, 0x00);
        assert_eq!(mbc.read(0xA000), Some(0xFF));
    }

    #[test]
    fn write_read_ram() {
        let rom = make_rom(0x8000);
        let mut mbc = MBC1::new(rom, vec![], false);

        // Enable RAM
        mbc.write(0x0000, 0x0A);

        // Write to RAM
        mbc.write(0xA000, 0x5A);
        assert_eq!(mbc.read(0xA000), Some(0x5A));

        // Write to another address
        mbc.write(0xBFFF, 0xA5);
        assert_eq!(mbc.read(0xBFFF), Some(0xA5));
    }

    #[test]
    fn banking_mode_switch() {
        let rom = make_rom(0x40000); // 16 banks
        let mut mbc = MBC1::new(rom, vec![], false);

        // Set banking mode to RAM banking
        mbc.write(0x6000, 1);

        // Set high bits for RAM bank
        mbc.write(0x4000, 1);

        // RAM bank should be 1
        assert_eq!(mbc.ram_bank(), 1);

        // Switch back to ROM banking
        mbc.write(0x6000, 0);
        assert_eq!(mbc.ram_bank(), 0);
    }

    #[test]
    fn rom_banking_mode_0() {
        let rom = make_rom(0x40000); // 16 banks
        let mut mbc = MBC1::new(rom, vec![], false);

        // Banking mode 0
        mbc.write(0x6000, 0);

        // Set high bits
        mbc.write(0x4000, 0);
        mbc.write(0x2000, 2);

        // Bank 0 should use high bits
        assert_eq!(mbc.read(0x0000), Some(0x00)); // Bank 0
        assert_eq!(mbc.read(0x4000), Some(0x00)); // Bank 2
    }

    #[test]
    fn invalid_address() {
        let rom = make_rom(0x8000);
        let mbc = MBC1::new(rom, vec![], false);

        assert_eq!(mbc.read(0x8000), None);
        assert_eq!(mbc.read(0x9FFF), None);
    }
}
