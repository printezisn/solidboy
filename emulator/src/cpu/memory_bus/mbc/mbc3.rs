const EXTERNAL_RAM_SIZE: usize = 0xBFFF - 0xA000 + 1;
const EXTERNAL_RAM_BANKS: usize = 4;

pub struct MBC3 {
    rom: Vec<u8>,
    rom_bank: u8,
    external_ram: [u8; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS],
    ram_enabled: bool,
    ram_bank: u8,
    rtc_registers: [u8; 5],
    rtc_select: u8,
    latch_clock_state: u8,
}

impl MBC3 {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            rom_bank: 1,
            external_ram: [0; EXTERNAL_RAM_SIZE * EXTERNAL_RAM_BANKS],
            ram_enabled: false,
            ram_bank: 0,
            rtc_registers: [0; 5],
            rtc_select: 0,
            latch_clock_state: 0,
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

                if self.rtc_select < 4 {
                    // RAM bank
                    Some(
                        self.external_ram[self.ram_bank as usize * EXTERNAL_RAM_SIZE
                            + address as usize
                            - 0xA000],
                    )
                } else if self.rtc_select <= 0x0C {
                    // RTC register
                    Some(self.rtc_registers[(self.rtc_select - 0x08) as usize])
                } else {
                    Some(0xFF)
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
                self.rtc_select = value & 0x0F;
                self.ram_bank = value & 0x03;
            }
            0x6000..=0x7FFF => {
                // Latch clock: write 0x00 then 0x01
                if self.latch_clock_state == 0 && value == 0x01 {
                    // Latch the clock on 0x00 -> 0x01 transition
                    // In a real implementation, this would read the system time
                }
                self.latch_clock_state = value & 1;
            }
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    if self.rtc_select < 4 {
                        self.external_ram[self.ram_bank as usize * EXTERNAL_RAM_SIZE
                            + address as usize
                            - 0xA000] = value;
                    } else if self.rtc_select <= 0x0C {
                        self.rtc_registers[(self.rtc_select - 0x08) as usize] = value;
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
        let mbc = MBC3::new(rom);

        assert_eq!(mbc.read(0x0000), Some(0x00));
        assert_eq!(mbc.read(0x3FFF), Some(0xFF));
    }

    #[test]
    fn read_rom_bank_1() {
        let rom = make_rom(0x10000);
        let mbc = MBC3::new(rom);

        assert_eq!(mbc.read(0x4000), Some(0x00));
        assert_eq!(mbc.read(0x7FFF), Some(0xFF));
    }

    #[test]
    fn switch_rom_bank() {
        let rom = make_rom(0x80000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x2000, 0x02);
        assert_eq!(mbc.read(0x4000), Some(0x00));

        mbc.write(0x2000, 0x03);
        assert_eq!(mbc.read(0x4000), Some(0x00));
    }

    #[test]
    fn ram_enable_disable() {
        let rom = make_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        assert_eq!(mbc.read(0xA000), Some(0xFF));

        mbc.write(0x0000, 0x0A);
        assert_eq!(mbc.read(0xA000), Some(0x00));

        mbc.write(0x0000, 0x00);
        assert_eq!(mbc.read(0xA000), Some(0xFF));
    }

    #[test]
    fn write_read_ram_bank() {
        let rom = make_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);
        mbc.write(0x4000, 1);
        mbc.write(0xA000, 0x5A);

        assert_eq!(mbc.read(0xA000), Some(0x5A));
        assert_eq!(mbc.read(0xBFFF), Some(0x00));
    }

    #[test]
    fn rtc_register_select() {
        let rom = make_rom(0x8000);
        let mut mbc = MBC3::new(rom);

        mbc.write(0x0000, 0x0A);

        // Select RTC register for seconds (0x08)
        mbc.write(0x4000, 0x08);
        assert_eq!(mbc.read(0xA000), Some(0x00));

        // Write to RTC seconds
        mbc.write(0xA000, 0x45);
        assert_eq!(mbc.read(0xA000), Some(0x45));
    }

    #[test]
    fn invalid_address() {
        let rom = make_rom(0x8000);
        let mbc = MBC3::new(rom);

        assert_eq!(mbc.read(0x8000), None);
        assert_eq!(mbc.read(0x9FFF), None);
    }
}
