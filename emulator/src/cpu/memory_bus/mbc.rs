mod mbc1;
mod no_rom;

pub enum MBC {
    NoROM(no_rom::NoROM),
    MBC1(mbc1::MBC1),
}

impl MBC {
    pub fn new(rom: Vec<u8>) -> MBC {
        match rom[0x0147] {
            0x00 => MBC::NoROM(no_rom::NoROM::new(rom)),
            0x01 | 0x02 | 0x03 => MBC::MBC1(mbc1::MBC1::new(rom)),
            _ => console_error!("Unsupported MBC type {:02X}", rom[0x0147]),
        }
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match self {
            MBC::NoROM(mbc) => mbc.read(address),
            MBC::MBC1(mbc) => mbc.read(address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match self {
            MBC::NoROM(mbc) => mbc.write(address, value),
            MBC::MBC1(mbc) => mbc.write(address, value),
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
        let mbc = MBC::new(rom);

        match mbc {
            MBC::NoROM(_) => {}
            _ => console_error!("Expected NoROM"),
        }
    }

    #[test]
    fn new_mbc1() {
        let rom = make_rom_with_type(0x01, 0x8000);
        let mbc = MBC::new(rom);

        match mbc {
            MBC::MBC1(_) => {}
            _ => console_error!("Expected MBC1"),
        }
    }

    #[test]
    fn dispatch_read() {
        let rom = make_rom_with_type(0x00, 0x8000);
        let mbc = MBC::new(rom);

        assert_eq!(mbc.read(0x0000), Some(0x00));
    }

    #[test]
    fn dispatch_write() {
        let rom = make_rom_with_type(0x01, 0x8000);
        let mut mbc = MBC::new(rom);

        assert!(mbc.write(0x2000, 2));
    }
}
