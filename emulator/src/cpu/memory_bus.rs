use crate::adapters::Adapters;

pub struct MemoryBus {
  adapters: Adapters,
  rom: Vec<u8>,
  memory: [u8; 0x7FFF + 1]
}

impl MemoryBus {
  pub fn new(adapters: Adapters) -> Self {
    let rom = adapters.rom_reader().read_rom();

    MemoryBus {
      adapters,
      rom,
      memory: [0; 0x7FFF + 1]
    }
  }

  pub fn write(&mut self, address: u16, value: u8) {
    match address {
      0..=0x7FFF => panic!("Invalid memory write {:?}", address),
      0xFF01 => {
        self.memory[(address - 0x8000) as usize] = value;
        self.adapters.serial_port().write(value);
      }
      0xFF02 => {
        self.memory[(address - 0x8000) as usize] = value;
        self.adapters.serial_port().control(value);
      },
      _ => self.memory[(address - 0x8000) as usize] = value
    }
  }

  pub fn read(&self, address: u16) -> u8 {
    match address {
      0..=0x7FFF => self.rom[address as usize],
      _ => self.memory[(address - 0x8000) as usize]
    }
  }

  pub fn if_flag(&self) -> u8 {
    self.memory[0xFF0F - 0x8000]
  }

  pub fn set_if_flag(&mut self, value: u8) {
    self.memory[0xFF0F - 0x8000] = value;
  }

  pub fn ie_flag(&self) -> u8 {
    self.memory[0xFFFF - 0x8000]
  }

  pub fn set_ie_flag(&mut self, value: u8) {
    self.memory[0xFFFF - 0x8000] = value;
  }
}