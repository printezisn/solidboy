mod registers;

use registers::Registers;
use registers::Register;
use super::memory_bus::MemoryBus;

pub struct CPU {
  registers: Registers,
  memory_bus: MemoryBus,
  rom: Vec<u8>,
  accept_interrupts: bool
}

pub struct InstructionResult {
  cycles: u8
}

impl CPU {
  pub fn new(rom: Vec<u8>) -> Self {
    CPU {
      registers: Registers::new(),
      memory_bus: MemoryBus::new(),
      rom,
      accept_interrupts: true
    }
  }

  pub fn execute_instruction(&mut self) -> u8 {
    let pc = self.registers.get(Register::PC) as usize;
    let opcode = self.rom[pc as usize];

    panic!("Unknown opcode: {:02X}", opcode);
  }
}