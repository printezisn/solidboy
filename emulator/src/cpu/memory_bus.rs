pub struct MemoryBus {
  rom: Vec<u8>,
  memory: [u8; 0x7FFF + 1]
}

impl MemoryBus {
  pub fn new(rom: Vec<u8>) -> Self {
    MemoryBus {
      rom,
      memory: [0; 0x7FFF + 1]
    }
  }

  pub fn write(&mut self, address: u16, value: u8) {
    match address {
      0..=0x7FFF => panic!("Invalid memory write {:?}", address),
      _ => self.memory[(address - 0x8000) as usize] = value
    }
  }

  pub fn read(&self, address: u16) -> u8 {
    match address {
      0..=0x7FFF => self.rom[address as usize],
      _ => self.memory[(address - 0x8000) as usize]
    }
  }
}