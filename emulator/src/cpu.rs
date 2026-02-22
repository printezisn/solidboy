mod registers;
mod instructions;

use registers::Registers;
use registers::Register;
use instructions::Mnemonic;
use instructions::OperandName;
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
        Mnemonic::JP => self.jp(&instruction),
        Mnemonic::DI => self.di(&instruction),
        Mnemonic::EI => self.ei(&instruction),
      _ => panic!("Unknown opcode: {:02X} {:?}", opcode, instruction.mnemonic)
    }
  }

  fn read16(&self) -> u16 {
    let pc = self.registers.get(Register::PC) as usize;
    let low = self.rom[pc + 1] as u16;
    let high = self.rom[pc + 2] as u16;
    (high << 8) | low
  }

  fn noop(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    InstructionResult { cycles: instruction.cycles[0] }
  }

  fn jp(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);

    if instruction.total_operands == 2 {
      if (matches!(instruction.operands[0].name, OperandName::NZ) && self.registers.zero())
        || (matches!(instruction.operands[0].name, OperandName::Z) && !self.registers.zero())
        || (matches!(instruction.operands[0].name, OperandName::NC) && self.registers.carry())
        || (matches!(instruction.operands[0].name, OperandName::C) && !self.registers.carry()) {
          self.registers.set(Register::PC, pc + instruction.bytes as u16);
          return InstructionResult { cycles: instruction.cycles[1] }
      }

      if (matches!(instruction.operands[0].name, OperandName::NZ) && !self.registers.zero())
        || (matches!(instruction.operands[0].name, OperandName::Z) && self.registers.zero())
        || (matches!(instruction.operands[0].name, OperandName::NC) && !self.registers.carry())
        || (matches!(instruction.operands[0].name, OperandName::C) && self.registers.carry()) {
          self.registers.set(Register::PC, self.read16());
          return InstructionResult { cycles: instruction.cycles[0] }
      }
    }

    match instruction.operands[0].register {
      Some(register) => {
        let target = self.registers.get(register);
        self.registers.set(Register::PC, target);
      },
      None => {
        let target = self.read16();
        self.registers.set(Register::PC, target);
      }
    }

    return InstructionResult { cycles: instruction.cycles[0] };
  }

  fn di(&mut self, instruction: &Instruction) -> InstructionResult {
    self.accept_interrupts = false;
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    InstructionResult { cycles: instruction.cycles[0] }
  }

  fn ei(&mut self, instruction: &Instruction) -> InstructionResult {
    self.accept_interrupts = true;
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    InstructionResult { cycles: instruction.cycles[0] }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const INITIAL_PC: u16 = 0x0100;
  const INITIAL_ZERO_FLAG: bool = true;
  const INITIAL_SUBTRACT_FLAG: bool = false;
  const INITIAL_HALF_CARRY_FLAG: bool = true;
  const INITIAL_CARRY_FLAG: bool = true;

  fn create_cpu(bytes: Vec<u8>) -> CPU {
    let mut rom = vec![0x00; 0xFFFF + 1];
    for i in 0..bytes.len() {
      rom[INITIAL_PC as usize + i] = bytes[i];
    }

    CPU::new(rom)
  }

  #[test]
  fn test_noop() {
    let mut cpu = create_cpu(vec![0x00]);
    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);

  }

  #[test]
  fn test_jp_cond_non_zero_taken() {
      let mut cpu = create_cpu(vec![0xC2, 0x34, 0x12]);
      cpu.registers.set_zero(false);

      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 16);
      assert_eq!(cpu.registers.get(Register::PC), 0x1234);
      assert_eq!(cpu.registers.zero(), false);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_jp_cond_non_zero_untaken() {
      let mut cpu = create_cpu(vec![0xC2, 0x34, 0x12]);
      cpu.registers.set_zero(true);
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 12);
      assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
      assert_eq!(cpu.registers.zero(), true);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_jp_cond_zero_taken() {
      let mut cpu = create_cpu(vec![0xCA, 0x34, 0x12]);
      cpu.registers.set_zero(true);

      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 16);
      assert_eq!(cpu.registers.get(Register::PC), 0x1234);
      assert_eq!(cpu.registers.zero(), true);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_jp_cond_zero_untaken() {
      let mut cpu = create_cpu(vec![0xCA, 0x34, 0x12]);
      cpu.registers.set_zero(false);
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 12);
      assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
      assert_eq!(cpu.registers.zero(), false);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_jp_cond_non_carry_taken() {
      let mut cpu = create_cpu(vec![0xD2, 0x34, 0x12]);
      cpu.registers.set_carry(false);

      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 16);
      assert_eq!(cpu.registers.get(Register::PC), 0x1234);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  fn test_jp_cond_non_carry_untaken() {
      let mut cpu = create_cpu(vec![0xD2, 0x34, 0x12]);
      cpu.registers.set_carry(true);
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 12);
      assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  fn test_jp_cond_carry_taken() {
      let mut cpu = create_cpu(vec![0xDA, 0x34, 0x12]);
      cpu.registers.set_carry(true);

      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 16);
      assert_eq!(cpu.registers.get(Register::PC), 0x1234);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  fn test_jp_cond_carry_untaken() {
      let mut cpu = create_cpu(vec![0xDA, 0x34, 0x12]);
      cpu.registers.set_carry(false);
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 12);
      assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  fn test_jp_memory() {
      let mut cpu = create_cpu(vec![0xC3, 0x34, 0x12]);
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 16);
      assert_eq!(cpu.registers.get(Register::PC), 0x1234);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_jp_register() {
      let mut cpu = create_cpu(vec![0xE9]);
      cpu.registers.set(Register::HL, 0x1234);
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 4);
      assert_eq!(cpu.registers.get(Register::PC), 0x1234);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_di() {
      let mut cpu = create_cpu(vec![0xF3]);
      cpu.accept_interrupts = true;
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 4);
      assert_eq!(cpu.accept_interrupts, false);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ei() {
      let mut cpu = create_cpu(vec![0xFB]);
      cpu.accept_interrupts = false;
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 4);
      assert_eq!(cpu.accept_interrupts, true);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }
}