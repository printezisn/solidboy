mod registers;
mod instructions;

use registers::Registers;
use registers::Register;
use instructions::Mnemonic;
use instructions::OperandName;
use instructions::Operand;
use instructions::PREFIXED_INSTRUCTIONS;
use crate::cpu::instructions::CBPREFIXED_INSTRUCTIONS;
use crate::cpu::instructions::Instruction;
use crate::cpu::registers::register_bytes;

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
        Mnemonic::LD => self.ld(&instruction),
        Mnemonic::LDH => self.ldh(&instruction),
        Mnemonic::INC => self.inc(&instruction),
        Mnemonic::JR => self.jr(&instruction),
      _ => panic!("Unknown opcode: {:02X} {:?}", opcode, instruction.mnemonic)
    }
  }

  fn read_n8(&self) -> u8 {
    let pc = self.registers.get(Register::PC) as usize;
    self.rom[pc + 1]
  }

  fn read_n16(&self) -> u16 {
    let pc = self.registers.get(Register::PC) as usize;
    let low = self.rom[pc + 1] as u16;
    let high = self.rom[pc + 2] as u16;
    (high << 8) | low
  }

  fn read_a16(&self, high: bool) -> u8 {
    self.memory_bus.read(self.sanitize_address(self.read_n16(), high))
  }

  fn read_e8(&self) -> u16 {
    let pc = self.registers.get(Register::PC) as usize;
    (self.rom[pc + 1] as i8) as i16 as u16
  }

  fn sanitize_address(&self, address: u16, high: bool) -> u16 {
    if high {
      0xFF00 | (address & 0x00FF)
    } else {
      address
    }
  }

  fn is_condition_true(&self, operand: &Operand) -> bool {
    match operand.name {
      OperandName::NZ => !self.registers.zero(),
      OperandName::Z => self.registers.zero(),
      OperandName::NC => !self.registers.carry(),
      OperandName::C => self.registers.carry(),
      _ => panic!("Invalid condition operand {:?}", operand.name)
    }
  } 

  fn set_add8_half_carry_flag(&mut self, old_value: u16, offset: u16) {
    self.registers.set_half_carry((old_value & 0xF) + (offset & 0xF) > 0xF);
  }

  fn set_add8_carry_flag(&mut self, old_value: u16, offset: u16) {
    self.registers.set_carry((old_value & 0xFF) + (offset & 0xFF) > 0xFF);
  }

  fn set_add8_zero_flag(&mut self, old_value: u16, offset: u16) {
    self.registers.set_zero((old_value + offset) & 0xFF == 0);
  }

  fn read_operand(&self, operand: &Operand, high: bool) -> (u16, u8) {
    match operand.register {
      Some(register) => {
        if operand.immediate {
          return (self.registers.get(register), register_bytes(register));
        }

        let address = self.sanitize_address(self.registers.get(register), high);
        return (self.memory_bus.read(address) as u16, 1);
      },
      None => {
        if !operand.immediate {
          return (self.read_a16(high) as u16, 1);
        }

        if operand.bytes == 1 {
          return (self.sanitize_address(self.read_n8() as u16, high), 1);
        } else {
          return (self.sanitize_address(self.read_n16(), high), 2);
        }
      }
    }
  }

  fn write_operand(&mut self, operand: &Operand, value: u16, register_bytes: u8, high: bool) {
    match operand.register {
      Some(register) => {
        if operand.immediate {
          self.registers.set(register, value);
        } else {
          let address = self.sanitize_address(self.registers.get(register), high);
          self.memory_bus.write(address, value as u8);
        }
      },
      None => {
        if operand.immediate {
          panic!("Cannot write to an immediate operand");
        }

        let address = self.sanitize_address(self.read_n16(), high);
        if register_bytes == 1 {
          self.memory_bus.write(address, value as u8);
        } else {
          self.memory_bus.write(address, (value & 0xFF) as u8);
          self.memory_bus.write(address + 1, (value >> 8) as u8);
        }
      }
    }
  }

  fn noop(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    InstructionResult { cycles: instruction.cycles[0] }
  }

  fn jp(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);

    if instruction.total_operands == 2 {
      if !self.is_condition_true(&instruction.operands[0]) {
          self.registers.set(Register::PC, pc + instruction.bytes as u16);
          return InstructionResult { cycles: instruction.cycles[1] }
      }

      self.registers.set(Register::PC, self.read_n16());
      return InstructionResult { cycles: instruction.cycles[0] }
    }

    match instruction.operands[0].register {
      Some(register) => {
        let target = self.registers.get(register);
        self.registers.set(Register::PC, target);
      },
      None => {
        let target = self.read_n16();
        self.registers.set(Register::PC, target);
      }
    }

    return InstructionResult { cycles: instruction.cycles[0] };
  }

  fn jr(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);

    if instruction.total_operands == 2 {
      if !self.is_condition_true(&instruction.operands[0]) {
          self.registers.set(Register::PC, pc + instruction.bytes as u16);
          return InstructionResult { cycles: instruction.cycles[1] }
      }
    }

    self.registers.set(Register::PC, pc.wrapping_add(instruction.bytes as u16).wrapping_add(self.read_e8()));
    return InstructionResult { cycles: instruction.cycles[0] }
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

  fn ld(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);
    
    let (mut value, register_bytes) = self.read_operand(&instruction.operands[1], false);
    
    if instruction.total_operands == 3 {
      let offset = self.read_e8();

      self.registers.set_zero(false);
      self.registers.set_subtract(false);
      self.set_add8_half_carry_flag(value, offset);
      self.set_add8_carry_flag(value, offset);

      value = value.overflowing_add(offset).0;
    }

    self.write_operand(&instruction.operands[0], value, register_bytes, false);

    for i in 0..(instruction.total_operands as usize) {
      if instruction.operands[i].increment {
        let register = instruction.operands[i].register.unwrap();
        self.registers.set(register, self.registers.get(register).wrapping_add(1));
      } else if instruction.operands[i].decrement {
        let register = instruction.operands[i].register.unwrap();
        self.registers.set(register, self.registers.get(register).wrapping_sub(1));
      }
    }

    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    return InstructionResult { cycles: instruction.cycles[0] };
  }

  fn ldh(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);
    
    let (value, register_bytes) = self.read_operand(&instruction.operands[1], true);
    self.write_operand(&instruction.operands[0], value, register_bytes, true);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    return InstructionResult { cycles: instruction.cycles[0] };
  }

  fn inc(&mut self, instruction: &Instruction) -> InstructionResult {
    let pc = self.registers.get(Register::PC);

    let (value, register_bytes) = self.read_operand(&instruction.operands[0], false);
    let new_value = value.wrapping_add(1);
    if register_bytes == 1 {
      self.set_add8_zero_flag(value, 1);
      self.registers.set_subtract(false);
      self.set_add8_half_carry_flag(value, 1);
    }

    self.write_operand(&instruction.operands[0], new_value, register_bytes, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    return InstructionResult { cycles: instruction.cycles[0] };
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

  #[test]
  fn test_ld_r8_r8() {
    let mut cpu = create_cpu(vec![0x41]);
    cpu.registers.set(Register::B, 0x12);
    cpu.registers.set(Register::C, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.get(Register::B), 0x34);
    assert_eq!(cpu.registers.get(Register::C), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_r8_n8() {
    let mut cpu = create_cpu(vec![0x06, 0x34]);
    cpu.registers.set(Register::B, 0x12);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::B), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_r16_n16() {
    let mut cpu = create_cpu(vec![0x01, 0x34, 0x12]);
    cpu.registers.set(Register::BC, 0x5678);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::BC), 0x1234);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_hl_mem_r8() {
    let mut cpu = create_cpu(vec![0x70]);
    cpu.registers.set(Register::HL, 0xFF00);
    cpu.registers.set(Register::B, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.memory_bus.read(0xFF00), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_hl_mem_n8() {
    let mut cpu = create_cpu(vec![0x36, 0x34]);
    cpu.registers.set(Register::HL, 0xFF00);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0xFF00), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_r8_hl_mem() {
    let mut cpu = create_cpu(vec![0x2A]);
    cpu.registers.set(Register::HL, 0xFF00);
    cpu.memory_bus.write(0xFF00, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::A), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_r16_mem_a() {
    let mut cpu = create_cpu(vec![0x12]);
    cpu.registers.set(Register::DE, 0xFF00);
    cpu.registers.set(Register::A, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.memory_bus.read(0xFF00), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_a16_a() {
    let mut cpu = create_cpu(vec![0xEA, 0x00, 0xFF]);
    cpu.registers.set(Register::A, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.memory_bus.read(0xFF00), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_a_a16() {
    let mut cpu = create_cpu(vec![0xFA, 0x00, 0xFF]);
    cpu.memory_bus.write(0xFF00, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.get(Register::A), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_a_r16() {
    let mut cpu = create_cpu(vec![0x0A]);
    cpu.registers.set(Register::BC, 0xFF00);
    cpu.memory_bus.write(0xFF00, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::A), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_hl_mem_incr_a() {
    let mut cpu = create_cpu(vec![0x22]);
    cpu.registers.set(Register::HL, 0xFF00);
    cpu.registers.set(Register::A, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.memory_bus.read(0xFF00), 0x34);
    assert_eq!(cpu.registers.get(Register::HL), 0xFF01);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_hl_mem_descr_a() {
    let mut cpu = create_cpu(vec![0x32]);
    cpu.registers.set(Register::HL, 0xFF00);
    cpu.registers.set(Register::A, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.memory_bus.read(0xFF00), 0x34);
    assert_eq!(cpu.registers.get(Register::HL), 0xFF00 - 1);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_a_hl_mem_descr() {
    let mut cpu = create_cpu(vec![0x3A]);
    cpu.registers.set(Register::HL, 0xFF00);
    cpu.memory_bus.write(0xFF00, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::A), 0x34);
    assert_eq!(cpu.registers.get(Register::HL), 0xFF00 - 1);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_a_hl_mem_incr() {
    let mut cpu = create_cpu(vec![0x2A]);
    cpu.registers.set(Register::HL, 0xFF00);
    cpu.memory_bus.write(0xFF00, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::A), 0x34);
    assert_eq!(cpu.registers.get(Register::HL), 0xFF00 + 1);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_sp_n16() {
    let mut cpu = create_cpu(vec![0x31, 0x00, 0xFF]);
    cpu.registers.set(Register::SP, 0x0000);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::SP), 0xFF00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_a16_sp() {
    let mut cpu = create_cpu(vec![0x08, 0x00, 0xFF]);
    cpu.registers.set(Register::SP, 0x1234);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 20);
    assert_eq!(cpu.memory_bus.read(0xFF00), 0x34);
    assert_eq!(cpu.memory_bus.read(0xFF01), 0x12);
    assert_eq!(cpu.registers.get(Register::SP), 0x1234);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ld_hl_sp_e8_half_carry() {
    let mut cpu = create_cpu(vec![0xF8, 0x01]);
    cpu.registers.set(Register::SP, 0x000F);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::SP), 0x000F);
    assert_eq!(cpu.registers.get(Register::HL), 0x0010);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  fn test_ld_hl_sp_e8_carry() {
    let mut cpu = create_cpu(vec![0xF8, 0x01]);
    cpu.registers.set(Register::SP, 0x00FF);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::SP), 0x00FF);
    assert_eq!(cpu.registers.get(Register::HL), 0x0100);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  fn test_ld_hl_sp_e8_no_carry() {
    let mut cpu = create_cpu(vec![0xF8, 0x01]);
    cpu.registers.set(Register::SP, 0xFFFE);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::SP), 0xFFFE);
    assert_eq!(cpu.registers.get(Register::HL), 0xFFFF);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  fn test_ld_sp_hl() {
    let mut cpu = create_cpu(vec![0xF9]);
    cpu.registers.set(Register::HL, 0x1234);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::SP), 0x1234);
    assert_eq!(cpu.registers.get(Register::HL), 0x1234);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ldh_a8_a() {
    let mut cpu = create_cpu(vec![0xE0, 0x80]);
    cpu.registers.set(Register::A, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0xFF80), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ldh_c_mem_a() {
    let mut cpu = create_cpu(vec![0xE2]);
    cpu.registers.set(Register::C, 0x80);
    cpu.registers.set(Register::A, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.memory_bus.read(0xFF80), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ldh_a_a8() {
    let mut cpu = create_cpu(vec![0xF0, 0x80]);
    cpu.memory_bus.write(0xFF80, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::A), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ldh_a_c_mem() {
    let mut cpu = create_cpu(vec![0xF2]);
    cpu.registers.set(Register::C, 0x80);
    cpu.memory_bus.write(0xFF80, 0x34);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::A), 0x34);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_r8_no_carry() {
    let mut cpu = create_cpu(vec![0x0C]);
    cpu.registers.set(Register::C, 0x80);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.get(Register::C), 0x81);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_r8_carry() {
    let mut cpu = create_cpu(vec![0x0C]);
    cpu.registers.set(Register::C, 0x0F);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.get(Register::C), 0x10);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_r8_zero() {
    let mut cpu = create_cpu(vec![0x0C]);
    cpu.registers.set(Register::C, 0xFF);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.get(Register::C), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_inc_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0x34]);
    cpu.registers.set(Register::HL, 0x1234);
    cpu.memory_bus.write(0x1234, 0x80);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0x1234), 0x81);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0x34]);
    cpu.registers.set(Register::HL, 0x1234);
    cpu.memory_bus.write(0x1234, 0x0F);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0x1234), 0x10);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0x34]);
    cpu.registers.set(Register::HL, 0x1234);
    cpu.memory_bus.write(0x1234, 0xFF);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0x1234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_inc_r16() {
    let mut cpu = create_cpu(vec![0x03]);
    cpu.registers.set(Register::BC, 0x10FF);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::BC), 0x1100);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_sp() {
    let mut cpu = create_cpu(vec![0x33]);
    cpu.registers.set(Register::SP, 0x10FF);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::SP), 0x1100);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_r16_zero() {
    let mut cpu = create_cpu(vec![0x03]);
    cpu.registers.set(Register::BC, 0xFFFF);
    cpu.registers.set_zero(false);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::BC), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_inc_sp_zero() {
    let mut cpu = create_cpu(vec![0x33]);
    cpu.registers.set(Register::SP, 0xFFFF);
    cpu.registers.set_zero(false);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::SP), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_jr_e8() {
    let mut cpu = create_cpu(vec![0x18, 0xFB]);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC - 3);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_jr_nz_e8_taken() {
    let mut cpu = create_cpu(vec![0x20, 0xFB]);
    cpu.registers.set_zero(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC - 3);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_jr_nz_e8_untaken() {
    let mut cpu = create_cpu(vec![0x20, 0xFB]);
    cpu.registers.set_zero(true);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_jr_z_e8_taken() {
    let mut cpu = create_cpu(vec![0x28, 0xFB]);
    cpu.registers.set_zero(true);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC - 3);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_jr_z_e8_untaken() {
    let mut cpu = create_cpu(vec![0x28, 0xFB]);
    cpu.registers.set_zero(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_jr_nc_e8_taken() {
    let mut cpu = create_cpu(vec![0x30, 0xFB]);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC - 3);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_jr_nc_e8_untaken() {
    let mut cpu = create_cpu(vec![0x30, 0xFB]);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_jr_c_e8_taken() {
    let mut cpu = create_cpu(vec![0x38, 0xFB]);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC - 3);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_jr_c_e8_untaken() {
    let mut cpu = create_cpu(vec![0x38, 0xFB]);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), false);
  }
}