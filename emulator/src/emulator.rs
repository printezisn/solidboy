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

  pub fn cpu(&self) -> &CPU {
    &self.cpu
  }

  pub fn execute(&mut self) {
    self.cpu.execute_instruction();
  }
}