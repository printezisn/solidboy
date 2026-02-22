mod registers;
mod instructions;

use registers::Registers;
use registers::Register;
use instructions::Mnemonic;
use instructions::PREFIXED_INSTRUCTIONS;
use crate::cpu::instructions::CBPREFIXED_INSTRUCTIONS;
use crate::cpu::instructions::Instruction;

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

  pub fn memory_bus(&self) -> &MemoryBus {
    &self.memory_bus
  }

  pub fn registers(&self) -> &Registers {
    &self.registers
  }

  pub fn execute_instruction(&mut self) -> InstructionResult {
    let pc = self.registers.get(Register::PC) as usize;
    let opcode = self.rom[pc as usize];

    if opcode == 0xCB {
      let cb_opcode = self.rom[pc + 1];
      let cb_instruction = &CBPREFIXED_INSTRUCTIONS[cb_opcode as usize];

      match cb_instruction.mnemonic {
        _ => panic!("Unknown CB-prefixed opcode: {:02X} {:?}", cb_opcode, cb_instruction.mnemonic)
      }
    }

    let instruction = &PREFIXED_INSTRUCTIONS[opcode as usize];
    match instruction.mnemonic {
        Mnemonic::NOP => self.noop(&instruction),
      _ => panic!("Unknown opcode: {:02X} {:?}", opcode, instruction.mnemonic)
    }
  }

  fn noop(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    InstructionResult { cycles: instruction.cycles[0] }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_noop() {
    let mut rom = vec![0x00; 0xFFFF + 1];
    rom[0x0100] = 0x00;

    let mut cpu = CPU::new(rom);
    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers().get(Register::PC), 0x0101);
  }
}