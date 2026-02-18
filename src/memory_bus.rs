pub struct MemoryBus {
  memory: [u8; 0xFFFF + 1]
}

impl MemoryBus {
  pub fn new() -> Self {
    MemoryBus {
      memory: [0; 0xFFFF + 1]
    }
  }

  pub fn write(&mut self, address: u16, value: u8) {
    self.memory[address as usize] = value;
  }

  pub fn read(&self, address: u16) -> u8 {
    self.memory[address as usize]
  }
}