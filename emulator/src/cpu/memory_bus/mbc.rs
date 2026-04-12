mod mbc1;
mod mbc3;
mod mbc5;
mod no_rom;

pub enum MBC {
    NoROM(no_rom::NoROM),
    MBC1(mbc1::MBC1),
    MBC3(mbc3::MBC3),
    MBC5(mbc5::MBC5),
}

impl MBC {
    pub fn new(rom: Vec<u8>, external_ram: Vec<u8>) -> MBC {
        match rom[0x0147] {
            0x00 => MBC::NoROM(no_rom::NoROM::new(rom)),
            0x01..=0x02 => MBC::MBC1(mbc1::MBC1::new(rom, external_ram, false)),
            0x03 => MBC::MBC1(mbc1::MBC1::new(rom, external_ram, true)),
            0x0F | 0x10 | 0x13 => MBC::MBC3(mbc3::MBC3::new(rom, external_ram, true)),
            0x11 | 0x12 => MBC::MBC3(mbc3::MBC3::new(rom, external_ram, false)),
            0x1B | 0x1E => MBC::MBC5(mbc5::MBC5::new(rom, external_ram, true)),
            0x19 | 0x1A | 0x1C | 0x1D => MBC::MBC5(mbc5::MBC5::new(rom, external_ram, false)),
            _ => console_error!("Unsupported MBC type"),
        }
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match self {
            MBC::NoROM(mbc) => mbc.read(address),
            MBC::MBC1(mbc) => mbc.read(address),
            MBC::MBC3(mbc) => mbc.read(address),
            MBC::MBC5(mbc) => mbc.read(address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match self {
            MBC::NoROM(mbc) => mbc.write(address, value),
            MBC::MBC1(mbc) => mbc.write(address, value),
            MBC::MBC3(mbc) => mbc.write(address, value),
            MBC::MBC5(mbc) => mbc.write(address, value),
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        match self {
            MBC::MBC3(mbc) => mbc.tick(cycles),
            _ => {}
        }
    }

    pub fn save_data(&mut self) -> (*const u8, usize, bool) {
        match self {
            MBC::NoROM(_) => (0 as *const u8, 0, false),
            MBC::MBC1(mbc) => mbc.save_data(),
            MBC::MBC3(mbc) => mbc.save_data(),
            MBC::MBC5(mbc) => mbc.save_data(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rom_with_type(mbc_type: u8, size: usize) -> Vec<u8> {
        let mut rom = vec![0u8; size];
        for i in 0..size {
            rom[i] = (i & 0xFF) as u8;
        }
        rom[0x0147] = mbc_type;
        rom
    }

    #[test]
    fn new_no_rom() {
        let rom = make_rom_with_type(0x00, 0x8000);
        let mbc = MBC::new(rom, vec![]);

        match mbc {
            MBC::NoROM(_) => {}
            _ => console_error!("Expected NoROM"),
        }
    }

    #[test]
    fn new_mbc1() {
        let rom = make_rom_with_type(0x01, 0x8000);
        let mbc = MBC::new(rom, vec![]);

        match mbc {
            MBC::MBC1(_) => {}
            _ => console_error!("Expected MBC1"),
        }
    }

    #[test]
    fn new_mbc3() {
        let rom = make_rom_with_type(0x0F, 0x8000);
        let mbc = MBC::new(rom, vec![]);

        match mbc {
            MBC::MBC3(_) => {}
            _ => console_error!("Expected MBC3"),
        }
    }

    #[test]
    fn new_mbc5() {
        let rom = make_rom_with_type(0x19, 0x8000);
        let mbc = MBC::new(rom, vec![]);

        match mbc {
            MBC::MBC5(_) => {}
            _ => console_error!("Expected MBC5"),
        }
    }

    #[test]
    fn dispatch_read() {
        let rom = make_rom_with_type(0x00, 0x8000);
        let mbc = MBC::new(rom, vec![]);

        assert_eq!(mbc.read(0x0000), Some(0x00));
    }

    #[test]
    fn dispatch_write() {
        let rom = make_rom_with_type(0x01, 0x8000);
        let mut mbc = MBC::new(rom, vec![]);

        assert!(mbc.write(0x2000, 2));
    }
}
