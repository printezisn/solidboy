use crate::cpu::CPU;

pub struct Emulator {
  cpu: CPU
}

impl Emulator {
  pub fn new(rom: Vec<u8>) -> Self {
    Emulator {
      cpu: CPU::new(rom)
    }
  }

  pub fn run(&mut self) {
    loop {
      self.cpu.execute_instruction();
    }
  }
}