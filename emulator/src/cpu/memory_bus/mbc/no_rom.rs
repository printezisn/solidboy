const EXTERNAL_RAM_SIZE: usize = 0xBFFF - 0xA000 + 1;

pub struct NoROM {
    rom: Vec<u8>,
    external_ram: [u8; EXTERNAL_RAM_SIZE],
}

impl NoROM {
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            external_ram: [0; EXTERNAL_RAM_SIZE],
        }
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0x0000..=0x7FFF => Some(self.rom[address as usize]),
            0xA000..=0xBFFF => Some(self.external_ram[(address - 0xA000) as usize]),
            _ => None,
        }
    }

    pub fn write(&mut self, _address: u16, _value: u8) -> bool {
        false
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
    fn read_rom() {
        let rom = make_rom(0x8000);
        let mbc = NoROM::new(rom);

        assert_eq!(mbc.read(0x0000), Some(0x00));
        assert_eq!(mbc.read(0x7FFF), Some(0xFF));
    }

    #[test]
    fn read_write_external_ram() {
        let rom = make_rom(0x8000);
        let mut mbc = NoROM::new(rom);

        // RAM is readable
        assert_eq!(mbc.read(0xA000), Some(0x00));
        assert_eq!(mbc.read(0xBFFF), Some(0x00));

        // But writes are ignored
        assert!(!mbc.write(0xA000, 0x5A));
        assert_eq!(mbc.read(0xA000), Some(0x00));
    }

    #[test]
    fn invalid_address() {
        let rom = make_rom(0x8000);
        let mbc = NoROM::new(rom);

        assert_eq!(mbc.read(0x8000), None);
        assert_eq!(mbc.read(0x9FFF), None);
    }
}
