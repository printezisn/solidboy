const EXTERNAL_RAM_SIZE: usize = 0xBFFF - 0xA000 + 1;
const EXTERNAL_RAM_BANKS: usize = 16;

pub struct MBC5 {
    rom: Vec<u8>,
    rom_bank_low: u8,
    rom_bank_high: u8,
    external_ram: [u8; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS],
    ram_enabled: bool,
    ram_bank: u8,
    has_battery_saves: bool,
    has_data_to_save: bool,
}

impl MBC5 {
    pub fn new(rom: Vec<u8>, external_ram: Vec<u8>, has_battery_saves: bool) -> Self {
        let mut result = Self {
            rom,
            rom_bank_low: 1,
            rom_bank_high: 0,
            external_ram: [0; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS],
            ram_enabled: false,
            ram_bank: 0,
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
            0x2000..=0x2FFF => {
                self.rom_bank_low = value;
            }
            0x3000..=0x3FFF => {
                self.rom_bank_high = value & 0x01;
            }
            0x4000..=0x5FFF => {
                self.ram_bank = value & 0x0F;
            }
            0x6000..=0x7FFF => {
                // Optional rumble / mode register on some cartridges; ignore.
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
        let bank = (self.rom_bank_low as usize) | ((self.rom_bank_high as usize) << 8);

        let num_banks = self.rom.len() / 0x4000;
        bank & (num_banks - 1)
    }

    fn ram_bank(&self) -> usize {
        self.ram_bank as usize
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
        let mbc = MBC5::new(rom, vec![], false);

        assert_eq!(mbc.read(0x0000), Some(0x00));
        assert_eq!(mbc.read(0x3FFF), Some(0xFF));
    }

    #[test]
    fn read_rom_bank_1() {
        let rom = make_rom(0x10000);
        let mbc = MBC5::new(rom, vec![], false);

        assert_eq!(mbc.read(0x4000), Some(0x00));
        assert_eq!(mbc.read(0x7FFF), Some(0xFF));
    }

    #[test]
    fn switch_rom_bank() {
        let rom = make_rom(0x80000);
        let mut mbc = MBC5::new(rom, vec![], false);

        mbc.write(0x2000, 0x02);
        assert_eq!(mbc.read(0x4000), Some(0x00));

        mbc.write(0x2000, 0x03);
        assert_eq!(mbc.read(0x4000), Some(0x00));
    }

    #[test]
    fn switch_high_rom_bank_bit() {
        let rom = make_rom(0x404000);
        let mut mbc = MBC5::new(rom, vec![], false);

        mbc.write(0x2000, 0x00);
        mbc.write(0x3000, 0x01);
        assert_eq!(mbc.read(0x4000), Some(0x00));
        assert_eq!(mbc.read(0x7FFF), Some(0xFF));
    }

    #[test]
    fn ram_enable_disable() {
        let rom = make_rom(0x8000);
        let mut mbc = MBC5::new(rom, vec![], false);

        assert_eq!(mbc.read(0xA000), Some(0xFF));

        mbc.write(0x0000, 0x0A);
        assert_eq!(mbc.read(0xA000), Some(0x00));

        mbc.write(0x0000, 0x00);
        assert_eq!(mbc.read(0xA000), Some(0xFF));
    }

    #[test]
    fn write_read_ram_bank() {
        let rom = make_rom(0x8000);
        let mut mbc = MBC5::new(rom, vec![], false);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 1);
        mbc.write(0xA000, 0x5A);

        assert_eq!(mbc.read(0xA000), Some(0x5A));
        assert_eq!(mbc.read(0xBFFF), Some(0x00));
    }

    #[test]
    fn invalid_address() {
        let rom = make_rom(0x8000);
        let mbc = MBC5::new(rom, vec![], false);

        assert_eq!(mbc.read(0x8000), None);
        assert_eq!(mbc.read(0x9FFF), None);
    }
}
