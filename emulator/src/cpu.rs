mod registers;
mod instructions;
mod memory_bus;
mod timer;

use registers::Registers;
use registers::Register;
use instructions::Mnemonic;
use instructions::OperandName;
use instructions::Operand;
use instructions::PREFIXED_INSTRUCTIONS;
use instructions::CBPREFIXED_INSTRUCTIONS;
use instructions::Instruction;
use registers::register_bytes;
use memory_bus::MemoryBus;

pub struct CPU {
  registers: Registers,
  memory_bus: MemoryBus,
  ime: bool,
  pending_ime_set: bool,
  halted: bool
}

pub struct InstructionResult {
  cycles: u8
}

impl CPU {
  pub fn new(rom: Vec<u8>) -> Self {
    CPU {
      registers: Registers::new(),
      memory_bus: MemoryBus::new(rom),
      ime: false,
      pending_ime_set: false,
      halted: false
    }
  }

  pub fn execute_instruction(&mut self) -> InstructionResult {
    self.memory_bus.reset_total_cycles();

    let interrupt_pending = (self.memory_bus.if_flag() & self.memory_bus.ie_flag() & 0x1F) != 0;
    if interrupt_pending && self.halted {
      self.halted = false;
    }

    if self.ime && interrupt_pending {
      self.interrupt();
      return InstructionResult { cycles: self.memory_bus.total_cycles() };
    }
    if self.halted {
      self.memory_bus.tick(4);
      return InstructionResult { cycles: self.memory_bus.total_cycles() };
    }

    let pc = self.registers.get(Register::PC);
    let opcode = self.memory_bus.read(pc);

    if opcode == 0xCB {
      let cb_opcode = self.memory_bus.read(pc + 1);
      let cb_instruction = &CBPREFIXED_INSTRUCTIONS[cb_opcode as usize];

      match cb_instruction.mnemonic {
        Mnemonic::SRL => self.srl(&cb_instruction),
        Mnemonic::SRA => self.sra(&cb_instruction),
        Mnemonic::SLA => self.sla(&cb_instruction),
        Mnemonic::RRC => self.rrc(&cb_instruction),
        Mnemonic::RR => self.rr(&cb_instruction),
        Mnemonic::RL => self.rl(&cb_instruction),
        Mnemonic::RLC => self.rlc(&cb_instruction),
        Mnemonic::SWAP => self.swap(&cb_instruction),
        Mnemonic::BIT => self.bit(&cb_instruction),
        Mnemonic::RES => self.res(&cb_instruction),
        Mnemonic::SET => self.set(&cb_instruction),
        _ => panic!("Unknown CB-prefixed opcode: {:02X} {:?}", cb_opcode, cb_instruction.mnemonic)
      };

      if self.pending_ime_set {
        self.ime = true;
        self.pending_ime_set = false;
      }
    } else {
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
          Mnemonic::DEC => self.dec(&instruction),
          Mnemonic::CALL => self.call(&instruction),
          Mnemonic::PUSH => self.push(&instruction),
          Mnemonic::POP => self.pop(&instruction),
          Mnemonic::RET => self.ret(&instruction),
          Mnemonic::OR => self.or(&instruction),
          Mnemonic::ADD => self.add(&instruction),
          Mnemonic::SUB => self.sub(&instruction),
          Mnemonic::CP => self.cp(&instruction),
          Mnemonic::AND => self.and(&instruction),
          Mnemonic::XOR => self.xor(&instruction),
          Mnemonic::RRCA => self.rrc(&instruction),
          Mnemonic::RRA => self.rr(&instruction),
          Mnemonic::RLA => self.rl(&instruction),
          Mnemonic::RLCA => self.rlc(&instruction),
          Mnemonic::ADC => self.adc(&instruction),
          Mnemonic::DAA => self.daa(&instruction),
          Mnemonic::HALT => self.halt(&instruction),
          Mnemonic::SBC => self.sbc(&instruction),
          Mnemonic::RETI => self.reti(),
          Mnemonic::RST => self.rst(&instruction),
          Mnemonic::CPL => self.cpl(&instruction),
          Mnemonic::SCF => self.scf(&instruction),
          Mnemonic::CCF => self.ccf(&instruction),
          Mnemonic::STOP => self.stop(&instruction),
        _ => panic!("Unknown opcode: {:02X} {:?}", opcode, instruction.mnemonic)
      };

      if self.pending_ime_set && !matches!(instruction.mnemonic, Mnemonic::EI) && !matches!(instruction.mnemonic, Mnemonic::RETI) {
        self.ime = true;
        self.pending_ime_set = false;
      }
    }

    self.check_external_ram_test_results();

    InstructionResult { cycles: self.memory_bus.total_cycles() }
  }

  fn check_external_ram_test_results(&self) {
    if self.memory_bus.read_without_tick(0xA001) != 0xDE {
      return;
    }
    if self.memory_bus.read_without_tick(0xA002) != 0xB0 {
      return;
    }
    if self.memory_bus.read_without_tick(0xA003) != 0x61 {
      return;
    }
    if self.memory_bus.read_without_tick(0xA000) != 0x80 {
      return;
    }
    
    let mut index = 0xA004;
    while self.memory_bus.read_without_tick(index) != 0x00 {
      print!("{}", self.memory_bus.read_without_tick(index) as char);
      index += 1;
    }
  }

  fn interrupt(&mut self) {
    self.ime = false;

    let pending = self.memory_bus.if_flag() & self.memory_bus.ie_flag() & 0x1F;
    let mut interrupt_num: u8 = 1;
    let mut interrupt_bit: u8 = 0;

    while interrupt_num <= 0x10 {
      if pending & interrupt_num != 0 {
        break;
      }

      interrupt_num <<= 1;
      interrupt_bit += 1;
    }
    if interrupt_num > 0x10 {
      interrupt_num = 1;
      interrupt_bit = 0;
    }

    self.memory_bus.set_if_flag(self.memory_bus.if_flag() & (!interrupt_num));

    self.memory_bus.tick(8);

    self.stack16(self.registers.get(Register::PC));
    self.registers.set(Register::PC, (0x0040 + interrupt_bit * 8) as u16);

    self.memory_bus.tick(4);
  }

  fn read_n8(&mut self) -> u8 {
    let pc = self.registers.get(Register::PC);
    self.memory_bus.read(pc + 1)
  }

  fn read_n16(&mut self) -> u16 {
    let pc = self.registers.get(Register::PC);
    let low = self.memory_bus.read(pc + 1) as u16;
    let high = self.memory_bus.read(pc + 2) as u16;
    (high << 8) | low
  }

  fn read_a16(&mut self, high: bool) -> u8 {
    let address = self.read_n16();
    self.memory_bus.read(self.sanitize_address(address, high))
  }

  fn read_e8(&mut self) -> u16 {
    let pc = self.registers.get(Register::PC);
    (self.memory_bus.read(pc + 1) as i8) as i16 as u16
  }

  fn stack16(&mut self, value: u16) {
    let sp = self.registers.get(Register::SP);
    self.memory_bus.write(sp - 1, (value >> 8) as u8);
    self.memory_bus.write(sp - 2, value as u8);
    self.registers.set(Register::SP, sp - 2);
  }

  fn pop16(&mut self) -> u16 {
    let sp = self.registers.get(Register::SP);
    if sp >= 0xFFFE {
      panic!("Invalid pop operation")
    }

    let low = self.memory_bus.read(sp) as u16;
    let high = self.memory_bus.read(sp + 1) as u16;

    self.registers.set(Register::SP, sp + 2);

    (high << 8) | low
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

  fn set_add8_half_carry_flag(&mut self, old_value: u16, offset: u16, carry: u16) {
    self.registers.set_half_carry((old_value & 0xF) + (offset & 0xF) + (carry & 0xF) > 0xF);
  }

  fn set_add8_carry_flag(&mut self, old_value: u16, offset: u16, carry: u16) {
    self.registers.set_carry((old_value & 0xFF) + (offset & 0xFF) + (carry & 0xFF) > 0xFF);
  }

  fn set_add8_zero_flag(&mut self, old_value: u16, offset: u16, carry: u16) {
    self.registers.set_zero(old_value.wrapping_add(offset).wrapping_add(carry) & 0xFF == 0);
  }

  fn set_add16_half_carry_flag(&mut self, old_value: u16, offset: u16) {
    self.registers.set_half_carry((old_value & 0x0FFF) + (offset & 0x0FFF) > 0x0FFF);
  }

  fn set_add16_carry_flag(&mut self, old_value: u16, offset: u16) {
    self.registers.set_carry(old_value.overflowing_add(offset).1);
  }

  fn set_sub8_half_carry_flag(&mut self, old_value: u16, offset: u16, carry: u16) {
    self.registers.set_half_carry((old_value & 0xF) < (offset & 0xF) + (carry & 0xF));
  }

  fn set_sub8_carry_flag(&mut self, old_value: u16, offset: u16, carry: u16) {
    self.registers.set_carry((old_value & 0xFF) < (offset & 0xFF) + (carry & 0xFF));
  }

  fn set_sub8_zero_flag(&mut self, old_value: u16, offset: u16, carry: u16) {
    self.registers.set_zero(old_value.wrapping_sub(offset).wrapping_sub(carry) & 0xFF == 0);
  }

  fn read_operand(&mut self, operand: &Operand, high: bool) -> (u16, u8) {
    match operand.register {
      Some(register) => {
        if operand.immediate {
          return (self.registers.get(register), register_bytes(register));
        }

        let address = self.sanitize_address(self.registers.get(register), high);
        return (self.memory_bus.read(address) as u16, 1);
      },
      None => {
        match operand.name {
          OperandName::A8 => {
            let address = self.read_n8() as u16;
            return (self.memory_bus.read(self.sanitize_address(address, high)) as u16, 1);
          },
          OperandName::E8 => {
            return (self.read_e8(), 1)
          },
          _ => {
            if !operand.immediate {
              return (self.read_a16(high) as u16, 1);
            }

            if operand.bytes == 1 {
              let address = self.read_n8() as u16;
              return (self.sanitize_address(address, high), 1);
            }

            let address = self.read_n16();
            return (self.sanitize_address(address, high), 2);
          }
        };
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

        let address = match operand.name {
          OperandName::A8 => self.read_n8() as u16,
          _ => self.read_n16()
        };

        let sanitized_address = self.sanitize_address(address, high);
        if register_bytes == 1 {
          self.memory_bus.write(sanitized_address, value as u8);
        } else {
          self.memory_bus.write(sanitized_address, (value & 0xFF) as u8);
          self.memory_bus.write(sanitized_address + 1, (value >> 8) as u8);
        }
      }
    }
  }

  fn noop(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn jp(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    if instruction.total_operands == 2 {
      let address = self.read_n16();

      if !self.is_condition_true(&instruction.operands[0]) {
          self.registers.set(Register::PC, pc + instruction.bytes as u16);
          return;
      }

      self.registers.set(Register::PC, address);
      self.memory_bus.tick(4);
      return;
    }

    match instruction.operands[0].register {
      Some(register) => {
        let target = self.registers.get(register);
        self.registers.set(Register::PC, target);
      },
      None => {
        let target = self.read_n16();
        self.registers.set(Register::PC, target);
        self.memory_bus.tick(4);
      }
    }
  }

  fn call(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    let address = self.read_n16();

    if instruction.total_operands == 2 {
      if !self.is_condition_true(&instruction.operands[0]) {
          self.registers.set(Register::PC, pc + instruction.bytes as u16);
          return;
      }
    }

    self.stack16(pc + instruction.bytes as u16);

    self.registers.set(Register::PC, address);
    self.memory_bus.tick(4);
  }

  fn rst(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    self.stack16(pc + instruction.bytes as u16);

    self.registers.set(Register::PC, (self.memory_bus.read(pc) - 0xC7) as u16);
  }

  fn jr(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    let offset = self.read_e8();

    if instruction.total_operands == 2 {
      if !self.is_condition_true(&instruction.operands[0]) {
          self.registers.set(Register::PC, pc + instruction.bytes as u16);
          return;
      }
    }

    self.registers.set(Register::PC, pc.wrapping_add(instruction.bytes as u16).wrapping_add(offset));
    self.memory_bus.tick(4);
  }

  fn di(&mut self, instruction: &Instruction) {
    self.ime = false;
    self.pending_ime_set = false;
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn ei(&mut self, instruction: &Instruction) {
    self.pending_ime_set = true;
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn ld(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    
    let (mut value, register_bytes) = self.read_operand(&instruction.operands[1], false);
    
    if instruction.total_operands == 3 {
      let offset = self.read_e8();

      self.registers.set_zero(false);
      self.registers.set_subtract(false);
      self.set_add8_half_carry_flag(value, offset, 0);
      self.set_add8_carry_flag(value, offset, 0);

      value = value.overflowing_add(offset).0;

      self.memory_bus.tick(4);
    } else {
      match instruction.operands[0].register {
        Some(Register::SP) => {
          if !matches!(instruction.operands[1].register, None) && instruction.operands[1].immediate {
            self.memory_bus.tick(4);
          }
        },
        _ => {}
      }
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
  }

  fn ldh(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    
    let (value, register_bytes) = self.read_operand(&instruction.operands[1], true);
    self.write_operand(&instruction.operands[0], value, register_bytes, true);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn inc(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value, register_bytes) = self.read_operand(&instruction.operands[0], false);
    let new_value = value.wrapping_add(1);
    if register_bytes == 1 {
      self.set_add8_zero_flag(value, 1, 0);
      self.registers.set_subtract(false);
      self.set_add8_half_carry_flag(value, 1, 0);
    }

    if instruction.operands[0].immediate {
      match instruction.operands[0].register {
        Some(Register::AF) | Some(Register::BC) | Some(Register::DE) | Some(Register::HL)
          | Some(Register::SP) => self.memory_bus.tick(4),
        _ => {}
      };
    }

    self.write_operand(&instruction.operands[0], new_value, register_bytes, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn dec(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value, register_bytes) = self.read_operand(&instruction.operands[0], false);
    let new_value = value.wrapping_sub(1);
    if register_bytes == 1 {
      self.set_sub8_zero_flag(value, 1, 0);
      self.registers.set_subtract(true);
      self.set_sub8_half_carry_flag(value, 1, 0);
    }

    if instruction.operands[0].immediate {
      match instruction.operands[0].register {
        Some(Register::AF) | Some(Register::BC) | Some(Register::DE) | Some(Register::HL)
          | Some(Register::SP) => self.memory_bus.tick(4),
        _ => {}
      };
    }

    self.write_operand(&instruction.operands[0], new_value, register_bytes, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn push(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    let register = instruction.operands[0].register.unwrap();

    self.memory_bus.tick(4);

    self.stack16(self.registers.get(register));
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn pop(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    let register = instruction.operands[0].register.unwrap();
    let value = self.pop16();

    self.registers.set(register, value);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn ret(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    if instruction.total_operands == 1 {
      self.memory_bus.tick(4);

      if !self.is_condition_true(&instruction.operands[0]) {
        self.registers.set(Register::PC, pc + instruction.bytes as u16);
        return;
      }
    }

    let value = self.pop16();
    self.registers.set(Register::PC, value);
    self.memory_bus.tick(4);
  }

  fn reti(&mut self) {
    let value = self.pop16();
    self.registers.set(Register::PC, value);
    self.pending_ime_set = true;
    self.memory_bus.tick(4);
  }

  fn or(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    let value1 = self.registers.get(Register::A);
    let value2 = self.read_operand(&instruction.operands[1], false).0;

    self.registers.set(Register::A, value1 | value2);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    self.registers.set_zero(self.registers.get(Register::A) == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(false);
  }

  fn add(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value1, _) = self.read_operand(&instruction.operands[0], false);
    let (value2, register_bytes) = self.read_operand(&instruction.operands[1], false);
    let new_value = value1.wrapping_add(value2);

    self.registers.set_subtract(false);
    if register_bytes == 1 {
      if !matches!(instruction.operands[1].name, OperandName::E8) {
        self.set_add8_zero_flag(value1, value2, 0);
      } else {
        self.registers.set_zero(false);
      }
      self.set_add8_half_carry_flag(value1, value2, 0);
      self.set_add8_carry_flag(value1, value2, 0);
    } else {
      self.set_add16_half_carry_flag(value1, value2);
      self.set_add16_carry_flag(value1, value2);
    }

    if instruction.operands[0].immediate {
      match instruction.operands[0].register {
        Some(Register::AF) | Some(Register::BC) | Some(Register::DE) | Some(Register::HL) => self.memory_bus.tick(4),
        Some(Register::SP) => self.memory_bus.tick(8),
        _ => {}
      };
    }

    self.write_operand(&instruction.operands[0], new_value, register_bytes, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn sub(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value1, _) = self.read_operand(&instruction.operands[0], false);
    let (value2, _) = self.read_operand(&instruction.operands[1], false);
    let new_value = value1.wrapping_sub(value2);

    self.set_sub8_zero_flag(value1, value2, 0);
    self.registers.set_subtract(true);
    self.set_sub8_half_carry_flag(value1, value2, 0);
    self.set_sub8_carry_flag(value1, value2, 0);

    self.write_operand(&instruction.operands[0], new_value, 1, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn cp(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value1, _) = self.read_operand(&instruction.operands[0], false);
    let (value2, _) = self.read_operand(&instruction.operands[1], false);

    self.set_sub8_zero_flag(value1, value2, 0);
    self.registers.set_subtract(true);
    self.set_sub8_half_carry_flag(value1, value2, 0);
    self.set_sub8_carry_flag(value1, value2, 0);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn and(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value1, _) = self.read_operand(&instruction.operands[0], false);
    let (value2, _) = self.read_operand(&instruction.operands[1], false);
    let new_value = value1 & value2;

    self.registers.set_zero(new_value == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry(true);
    self.registers.set_carry(false);

    self.write_operand(&instruction.operands[0], new_value, 1, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn xor(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value1, _) = self.read_operand(&instruction.operands[0], false);
    let (value2, _) = self.read_operand(&instruction.operands[1], false);
    let new_value = value1 ^ value2;

    self.registers.set_zero(new_value == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(false);

    self.write_operand(&instruction.operands[0], new_value, 1, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn srl(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value, _) = self.read_operand(&instruction.operands[0], false);

    self.registers.set_zero((value >> 1) == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(value & 0x01 != 0);

    self.write_operand(&instruction.operands[0], value >> 1, 1, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn sra(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value, _) = self.read_operand(&instruction.operands[0], false);
    let new_value = (value >> 1) | (value & 0x80);

    self.registers.set_zero((value >> 1) == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(value & 0x01 != 0);

    self.write_operand(&instruction.operands[0], new_value, 1, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn sla(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value, _) = self.read_operand(&instruction.operands[0], false);
    let new_value = (value << 1) & 0xFF;

    self.registers.set_zero(new_value == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(value & 0x80 != 0);

    self.write_operand(&instruction.operands[0], new_value, 1, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn rrc(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let value;

    if instruction.total_operands == 1 {
      value = self.read_operand(&instruction.operands[0], false).0;
    } else {
      value = self.registers.get(Register::A);
    }

    let mut new_value = value >> 1;
    if value & 0x01 != 0 {
      new_value |= 0x80;
    }

    self.registers.set_zero(new_value == 0 && !matches!(instruction.mnemonic, Mnemonic::RRCA));
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(value & 0x01 != 0);

    if instruction.total_operands == 1 {
      self.write_operand(&instruction.operands[0], new_value, 1, false);
    } else {
      self.registers.set(Register::A, new_value);
    }

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn rr(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let value;

    if instruction.total_operands == 1 {
      value = self.read_operand(&instruction.operands[0], false).0;
    } else {
      value = self.registers.get(Register::A);
    }

    let mut new_value = value >> 1;
    if self.registers.carry() {
      new_value |= 0x80;
    }

    self.registers.set_zero(new_value == 0 && !matches!(instruction.mnemonic, Mnemonic::RRA));
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(value & 0x01 != 0);

    if instruction.total_operands == 1 {
      self.write_operand(&instruction.operands[0], new_value, 1, false);
    } else {
      self.registers.set(Register::A, new_value);
    }

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn rl(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let value;

    if instruction.total_operands == 1 {
      value = self.read_operand(&instruction.operands[0], false).0;
    } else {
      value = self.registers.get(Register::A);
    }

    let mut new_value = (value << 1) & 0xFF;
    if self.registers.carry() {
      new_value |= 0x01;
    }

    self.registers.set_zero(new_value == 0 && !matches!(instruction.mnemonic, Mnemonic::RLA));
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(value & 0x80 != 0);

    if instruction.total_operands == 1 {
      self.write_operand(&instruction.operands[0], new_value, 1, false);
    } else {
      self.registers.set(Register::A, new_value);
    }

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn rlc(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let value;

    if instruction.total_operands == 1 {
      value = self.read_operand(&instruction.operands[0], false).0;
    } else {
      value = self.registers.get(Register::A);
    }

    let mut new_value = (value << 1) & 0xFF;
    if value & 0x80 != 0 {
      new_value |= 0x01;
    }

    self.registers.set_zero(new_value == 0 && !matches!(instruction.mnemonic, Mnemonic::RLCA));
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(value & 0x80 != 0);

    if instruction.total_operands == 1 {
      self.write_operand(&instruction.operands[0], new_value, 1, false);
    } else {
      self.registers.set(Register::A, new_value);
    }

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn adc(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value1, _) = self.read_operand(&instruction.operands[0], false);
    let (value2, _) = self.read_operand(&instruction.operands[1], false);
    let mut carry: u16 = 0;

    if self.registers.carry() {
      carry = 1;
    }

    let new_value = value1.wrapping_add(value2).wrapping_add(carry);

    self.set_add8_zero_flag(value1, value2, carry);
    self.registers.set_subtract(false);
    self.set_add8_half_carry_flag(value1, value2, carry);
    self.set_add8_carry_flag(value1, value2, carry);

    self.write_operand(&instruction.operands[0], new_value, 1, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn daa(&mut self, instruction: &Instruction) {
    let mut a = self.registers.get(Register::A) as u8;
    let mut adjust: u8 = 0;
    let mut carry = self.registers.carry();

    if !self.registers.subtract() {
      if self.registers.half_carry() || (a & 0x0F) > 0x9 {
        adjust |= 0x06;
      }
      if carry || a > 0x99 {
        adjust |= 0x60;
        carry = true;
      }
      a = a.wrapping_add(adjust);
    } else {
      if self.registers.half_carry() {
        adjust |= 0x06;
      }
      if carry {
        adjust |= 0x60;
      }
      a = a.wrapping_sub(adjust);
    }

    self.registers.set_zero(a == 0);
    self.registers.set_half_carry(false);
    self.registers.set_carry(carry);

    self.registers.set(Register::A, a as u16);
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn halt(&mut self, instruction: &Instruction) {
    self.halted = true;
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn swap(&mut self, instruction: &Instruction) {
    let (value, _) = self.read_operand(&instruction.operands[0], false);
    let new_value = ((value << 4) | (value >> 4)) & 0xFF;
    self.write_operand(&instruction.operands[0], new_value, 1, false);

    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    self.registers.set_zero(new_value == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(false);
  }

  fn sbc(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let (value1, _) = self.read_operand(&instruction.operands[0], false);
    let (value2, _) = self.read_operand(&instruction.operands[1], false);
    let mut carry = 0;
    
    if self.registers.carry() {
      carry = 1;
    }

    let new_value = value1.wrapping_sub(value2).wrapping_sub(carry);

    self.set_sub8_zero_flag(value1, value2, carry);
    self.registers.set_subtract(true);
    self.set_sub8_half_carry_flag(value1, value2, carry);
    self.set_sub8_carry_flag(value1, value2, carry);

    self.write_operand(&instruction.operands[0], new_value, 1, false);

    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn cpl(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    let a = self.registers.get(Register::A);

    self.registers.set(Register::A, !a);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);

    self.registers.set_subtract(true);
    self.registers.set_half_carry(true);
  }

  fn scf(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(true);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn ccf(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    self.registers.set_subtract(false);
    self.registers.set_half_carry(false);
    self.registers.set_carry(!self.registers.carry());
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn bit(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let bit = match instruction.operands[0].name {
      OperandName::NUM0 => 1,
      OperandName::NUM1 => 2,
      OperandName::NUM2 => 3,
      OperandName::NUM3 => 4,
      OperandName::NUM4 => 5,
      OperandName::NUM5 => 6,
      OperandName::NUM6 => 7,
      OperandName::NUM7 => 8,
      _ => unreachable!()
    };
    let mask = 1 << (bit - 1);

    let (value, _) = self.read_operand(&instruction.operands[1], false);

    self.registers.set_zero((value & mask) == 0);
    self.registers.set_subtract(false);
    self.registers.set_half_carry(true);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn res(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let bit = match instruction.operands[0].name {
      OperandName::NUM0 => 1,
      OperandName::NUM1 => 2,
      OperandName::NUM2 => 3,
      OperandName::NUM3 => 4,
      OperandName::NUM4 => 5,
      OperandName::NUM5 => 6,
      OperandName::NUM6 => 7,
      OperandName::NUM7 => 8,
      _ => unreachable!()
    };
    let mask = !(1 << (bit - 1));

    let (value, register_bytes) = self.read_operand(&instruction.operands[1], false);
    self.write_operand(&instruction.operands[1], value & mask, register_bytes, false);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn set(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);

    let bit = match instruction.operands[0].name {
      OperandName::NUM0 => 1,
      OperandName::NUM1 => 2,
      OperandName::NUM2 => 3,
      OperandName::NUM3 => 4,
      OperandName::NUM4 => 5,
      OperandName::NUM5 => 6,
      OperandName::NUM6 => 7,
      OperandName::NUM7 => 8,
      _ => unreachable!()
    };
    let mask = 1 << (bit - 1);

    let (value, register_bytes) = self.read_operand(&instruction.operands[1], false);
    self.write_operand(&instruction.operands[1], value | mask, register_bytes, false);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }

  fn stop(&mut self, instruction: &Instruction) {
    let pc = self.registers.get(Register::PC);
    self.registers.set(Register::PC, pc + instruction.bytes as u16);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const INITIAL_PC: u16 = 0x0100;
  const INITIAL_SP: u16 = 0xFFFE;
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
      cpu.ime = true;
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 4);
      assert_eq!(cpu.ime, false);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  fn test_ei() {
      let mut cpu = create_cpu(vec![0xFB, 0x00]);
      cpu.ime = false;
      cpu.pending_ime_set = false;
      
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 4);
      assert_eq!(cpu.ime, false);
      assert_eq!(cpu.pending_ime_set, true);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);

      cpu.execute_instruction();

      assert_eq!(cpu.ime, true);
      assert_eq!(cpu.pending_ime_set, false);
  }

  #[test]
  fn test_interrupt() {
    let mut cpu = create_cpu(vec![0xFB, 0x00, 0x00]);
      cpu.ime = false;
      cpu.pending_ime_set = false;
      cpu.memory_bus.write(0xFF0F, 1 << 2);
      cpu.memory_bus.write(0xFFFF, 1 << 2);
      
      cpu.execute_instruction();
      cpu.execute_instruction();
      let result = cpu.execute_instruction();
  
      assert_eq!(result.cycles, 20);
      assert_eq!(cpu.ime, false);
      assert_eq!(cpu.pending_ime_set, false);
      assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
      assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
      assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
      assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);

      assert_eq!(cpu.ime, false);
      assert_eq!(cpu.pending_ime_set, false);
      assert_eq!(cpu.registers.get(Register::PC), 0x50);
      assert_eq!(cpu.memory_bus.read(0xFF0F), 0x00);
      
      let sp = cpu.registers.get(Register::SP);
      assert_eq!(sp, INITIAL_SP - 2);

      let old_pc = cpu.memory_bus.read(sp) as u16 | ((cpu.memory_bus.read(sp + 1) as u16) << 8);
      assert_eq!(old_pc, INITIAL_PC + 2);
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
    let mut cpu = create_cpu(vec![0x08, 0x34, 0xF2]);
    cpu.registers.set(Register::SP, 0x1234);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 20);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x34);
    assert_eq!(cpu.memory_bus.read(0xF235), 0x12);
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
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x80);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x81);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0x34]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x0F);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x10);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_inc_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0x34]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0xFF);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
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

  #[test]
  pub fn test_dec_r8_no_carry() {
    let mut cpu = create_cpu(vec![0x3D]);
    cpu.registers.set(Register::A, 0x81);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.get(Register::A), 0x80);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_dec_r8_carry() {
    let mut cpu = create_cpu(vec![0x3D]);
    cpu.registers.set(Register::A, 0x80);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.get(Register::A), 0x7F);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_dec_r8_zero() {
    let mut cpu = create_cpu(vec![0x3D]);
    cpu.registers.set(Register::A, 0x01);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_dec_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0x35]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x81);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x80);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_dec_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0x35]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x80);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x7F);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_dec_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0x35]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x01);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_dec_r16() {
    let mut cpu = create_cpu(vec![0x1B]);
    cpu.registers.set(Register::DE, 0x10FF);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::DE), 0x10FE);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_dec_sp() {
    let mut cpu = create_cpu(vec![0x3B]);
    cpu.registers.set(Register::SP, 0x10FF);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::SP), 0x10FE);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_dec_r16_zero() {
    let mut cpu = create_cpu(vec![0x1B]);
    cpu.registers.set(Register::DE, 0x0001);
    cpu.registers.set_zero(false);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::DE), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_dec_sp_zero() {
    let mut cpu = create_cpu(vec![0x3B]);
    cpu.registers.set(Register::SP, 0x0001);
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
  pub fn test_call_n16() {
    let mut cpu = create_cpu(vec![0xCD, 0x34, 0x12]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 3) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 3) >> 8) as u8);

    assert_eq!(result.cycles, 24);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_call_nz_n16_taken() {
    let mut cpu = create_cpu(vec![0xC4, 0x34, 0x12]);
    cpu.registers.set_zero(false);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 3) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 3) >> 8) as u8);

    assert_eq!(result.cycles, 24);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_call_nz_n16_untaken() {
    let mut cpu = create_cpu(vec![0xC4, 0x34, 0x12]);
    cpu.registers.set_zero(true);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_call_z_n16_taken() {
    let mut cpu = create_cpu(vec![0xCC, 0x34, 0x12]);
    cpu.registers.set_zero(true);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 3) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 3) >> 8) as u8);

    assert_eq!(result.cycles, 24);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_call_z_n16_untaken() {
    let mut cpu = create_cpu(vec![0xCC, 0x34, 0x12]);
    cpu.registers.set_zero(false);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_call_nc_n16_taken() {
    let mut cpu = create_cpu(vec![0xD4, 0x34, 0x12]);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 3) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 3) >> 8) as u8);

    assert_eq!(result.cycles, 24);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_call_nc_n16_untaken() {
    let mut cpu = create_cpu(vec![0xD4, 0x34, 0x12]);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_call_c_n16_taken() {
    let mut cpu = create_cpu(vec![0xDC, 0x34, 0x12]);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 3) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 3) >> 8) as u8);

    assert_eq!(result.cycles, 24);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_call_c_n16_untaken() {
    let mut cpu = create_cpu(vec![0xDC, 0x34, 0x12]);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 3);

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rst_00() {
    let mut cpu = create_cpu(vec![0xC7]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x0000);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 1) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 1) >> 8) as u8);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_rst_08() {
    let mut cpu = create_cpu(vec![0xCF]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x0008);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 1) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 1) >> 8) as u8);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_rst_10() {
    let mut cpu = create_cpu(vec![0xD7]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x0010);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 1) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 1) >> 8) as u8);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_rst_18() {
    let mut cpu = create_cpu(vec![0xDF]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x0018);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 1) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 1) >> 8) as u8);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_rst_20() {
    let mut cpu = create_cpu(vec![0xE7]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x0020);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 1) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 1) >> 8) as u8);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_rst_28() {
    let mut cpu = create_cpu(vec![0xEF]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x0028);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 1) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 1) >> 8) as u8);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_rst_30() {
    let mut cpu = create_cpu(vec![0xF7]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x0030);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 1) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 1) >> 8) as u8);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_rst_38() {
    let mut cpu = create_cpu(vec![0xFF]);

    let result = cpu.execute_instruction();
    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), 0x0038);
    assert_eq!(cpu.memory_bus.read(sp), (INITIAL_PC + 1) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), ((INITIAL_PC + 1) >> 8) as u8);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_pop_af() {
    let mut cpu = create_cpu(vec![0xF1]);
    cpu.stack16(0x12F0);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::SP), INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.get(Register::AF), 0x12F0);

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_pop_r16() {
    let mut cpu = create_cpu(vec![0xC1]);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::SP), INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(cpu.registers.get(Register::BC), 0x1234);

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_push_af() {
    let mut cpu = create_cpu(vec![0xF5]);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.memory_bus.read(sp), cpu.registers.get(Register::F) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), cpu.registers.get(Register::A) as u8);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_push_r16() {
    let mut cpu = create_cpu(vec![0xC5]);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.memory_bus.read(sp), cpu.registers.get(Register::C) as u8);
    assert_eq!(cpu.memory_bus.read(sp + 1), cpu.registers.get(Register::B) as u8);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_ret() {
    let mut cpu = create_cpu(vec![0xC9]);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_ret_nz_taken() {
    let mut cpu = create_cpu(vec![0xC0]);
    cpu.registers.set_zero(false);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);

    assert_eq!(result.cycles, 20);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_ret_nz_untaken() {
    let mut cpu = create_cpu(vec![0xC0]);
    cpu.registers.set_zero(true);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_ret_z_taken() {
    let mut cpu = create_cpu(vec![0xC8]);
    cpu.registers.set_zero(true);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);

    assert_eq!(result.cycles, 20);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_ret_z_untaken() {
    let mut cpu = create_cpu(vec![0xC8]);
    cpu.registers.set_zero(false);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_ret_nc_taken() {
    let mut cpu = create_cpu(vec![0xD0]);
    cpu.registers.set_carry(false);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);

    assert_eq!(result.cycles, 20);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_ret_nc_untaken() {
    let mut cpu = create_cpu(vec![0xD0]);
    cpu.registers.set_carry(true);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_ret_c_taken() {
    let mut cpu = create_cpu(vec![0xD8]);
    cpu.registers.set_carry(true);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);

    assert_eq!(result.cycles, 20);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_ret_c_untaken() {
    let mut cpu = create_cpu(vec![0xD8]);
    cpu.registers.set_carry(false);
    cpu.stack16(0x1234);

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP - 2);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_reti_basic() {
    let mut cpu = create_cpu(vec![0xD9]);
    cpu.stack16(0x1234);
    cpu.ime = false;

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), 0x1234);
    assert_eq!(cpu.ime, true);
    assert_eq!(cpu.pending_ime_set, false);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_reti_different_address() {
    let mut cpu = create_cpu(vec![0xD9]);
    cpu.stack16(0x5678);
    cpu.ime = false;

    let result = cpu.execute_instruction();

    let sp = cpu.registers.get(Register::SP);

    assert_eq!(sp, INITIAL_SP);
    assert_eq!(cpu.registers.get(Register::PC), 0x5678);
    assert_eq!(cpu.ime, true);
    assert_eq!(cpu.pending_ime_set, false);

    assert_eq!(result.cycles, 16);
  }

  #[test]
  pub fn test_reti_zero_address() {
    let mut cpu = create_cpu(vec![0xD9]);
    cpu.stack16(0x0000);
    cpu.ime = false;

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), 0x0000);
    assert_eq!(cpu.ime, true);
    assert_eq!(cpu.pending_ime_set, false);

    assert_eq!(result.cycles, 16);
  }

  #[test]
  pub fn test_reti_high_address() {
    let mut cpu = create_cpu(vec![0xD9]);
    cpu.stack16(0xFFFF);
    cpu.ime = false;

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), 0xFFFF);
    assert_eq!(cpu.ime, true);
    assert_eq!(cpu.pending_ime_set, false);

    assert_eq!(result.cycles, 16);
  }

  #[test]
  pub fn test_reti_enables_ime_immediately() {
    let mut cpu = create_cpu(vec![0xD9, 0x00]);
    cpu.stack16(0x1234);
    cpu.ime = false;

    let result = cpu.execute_instruction();

    assert_eq!(cpu.ime, true);
    assert_eq!(cpu.pending_ime_set, false);
    assert_eq!(result.cycles, 16);
  }

  #[test]
  pub fn test_reti_preserves_flags() {
    let mut cpu = create_cpu(vec![0xD9]);
    cpu.stack16(0x3000);
    cpu.ime = false;
    cpu.registers.set_zero(true);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);

    let _result = cpu.execute_instruction();

    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_or_a_r8_non_zero() {
    let mut cpu = create_cpu(vec![0xB0]);
    cpu.registers.set(Register::A, 0x0001);
    cpu.registers.set(Register::B, 0x0002);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0003);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_or_a_r8_zero() {
    let mut cpu = create_cpu(vec![0xB0]);
    cpu.registers.set(Register::A, 0x0000);
    cpu.registers.set(Register::B, 0x0000);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_or_a_hl_mem_non_zero() {
    let mut cpu = create_cpu(vec![0xB6]);
    cpu.registers.set(Register::A, 0x0001);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x0002);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0003);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_or_a_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xB6]);
    cpu.registers.set(Register::A, 0x0000);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x0000);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_or_a_n8_non_zero() {
    let mut cpu = create_cpu(vec![0xF6, 0x02]);
    cpu.registers.set(Register::A, 0x0001);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0003);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_or_a_n8_zero() {
    let mut cpu = create_cpu(vec![0xF6, 0x00]);
    cpu.registers.set(Register::A, 0x0000);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_a_r8_no_carry() {
    let mut cpu = create_cpu(vec![0x80]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0009);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_a_r8_half_carry() {
    let mut cpu = create_cpu(vec![0x80]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::B, 0x0D);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0011);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_a_r8_carry() {
    let mut cpu = create_cpu(vec![0x80]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::B, 0xFE);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_a_r8_zero() {
    let mut cpu = create_cpu(vec![0x80]);
    cpu.registers.set(Register::A, 0x02);
    cpu.registers.set(Register::B, 0xFE);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_a_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0x86]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0009);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_a_hl_mem_half_carry() {
    let mut cpu = create_cpu(vec![0x86]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x0D);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0011);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_a_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0x86]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0xFE);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_a_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0x86]);
    cpu.registers.set(Register::A, 0x02);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0xFE);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_a_n8_no_carry() {
    let mut cpu = create_cpu(vec![0xC6, 0x05]);
    cpu.registers.set(Register::A, 0x04);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0009);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_a_n8_half_carry() {
    let mut cpu = create_cpu(vec![0xC6, 0x0D]);
    cpu.registers.set(Register::A, 0x04);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0011);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_a_n8_carry() {
    let mut cpu = create_cpu(vec![0xC6, 0xFE]);
    cpu.registers.set(Register::A, 0x04);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_a_n8_zero() {
    let mut cpu = create_cpu(vec![0xC6, 0xFE]);
    cpu.registers.set(Register::A, 0x02);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_hl_r16_no_carry() {
    let mut cpu = create_cpu(vec![0x09]);
    cpu.registers.set(Register::HL, 0x0F04);
    cpu.registers.set(Register::BC, 0x0005);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::HL), 0x0F09);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_hl_r16_half_carry() {
    let mut cpu = create_cpu(vec![0x09]);
    cpu.registers.set(Register::HL, 0x0FFD);
    cpu.registers.set(Register::BC, 0x0005);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::HL), 0x1002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_hl_r16_carry() {
    let mut cpu = create_cpu(vec![0x09]);
    cpu.registers.set(Register::HL, 0xFFFD);
    cpu.registers.set(Register::BC, 0x0005);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::HL), 0x0002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_hl_r16_zero() {
    let mut cpu = create_cpu(vec![0x09]);
    cpu.registers.set_zero(false);
    cpu.registers.set(Register::HL, 0xFFFD);
    cpu.registers.set(Register::BC, 0x0003);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::HL), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_hl_sp_no_carry() {
    let mut cpu = create_cpu(vec![0x39]);
    cpu.registers.set(Register::HL, 0x0F04);
    cpu.registers.set(Register::SP, 0x0005);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::HL), 0x0F09);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_hl_sp_half_carry() {
    let mut cpu = create_cpu(vec![0x39]);
    cpu.registers.set(Register::HL, 0x0FFD);
    cpu.registers.set(Register::SP, 0x0005);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::HL), 0x1002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_hl_sp_carry() {
    let mut cpu = create_cpu(vec![0x39]);
    cpu.registers.set(Register::HL, 0xFFFD);
    cpu.registers.set(Register::SP, 0x0005);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::HL), 0x0002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_hl_sp_zero() {
    let mut cpu = create_cpu(vec![0x39]);
    cpu.registers.set_zero(false);
    cpu.registers.set(Register::HL, 0xFFFD);
    cpu.registers.set(Register::SP, 0x0003);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::HL), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_sp_e8_no_carry() {
    let mut cpu = create_cpu(vec![0xE8, 0x05]);
    cpu.registers.set(Register::SP, 0x0F04);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::SP), 0x0F09);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_sp_e8_half_carry() {
    let mut cpu = create_cpu(vec![0xE8, 0x05]);
    cpu.registers.set(Register::SP, 0x000D);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::SP), 0x0012);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_add_sp_e8_carry() {
    let mut cpu = create_cpu(vec![0xE8, 0xFF]);
    cpu.registers.set(Register::SP, 0x00FD);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::SP), 0x00FC);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_add_sp_e8_zero() {
    let mut cpu = create_cpu(vec![0xE8, 0xFF]);
    cpu.registers.set_zero(true);
    cpu.registers.set(Register::SP, 0x0001);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::SP), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sub_a_r8_no_carry() {
    let mut cpu = create_cpu(vec![0x90]);
    cpu.registers.set(Register::A, 0x35);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sub_a_r8_half_carry() {
    let mut cpu = create_cpu(vec![0x90]);
    cpu.registers.set(Register::A, 0x30);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x2B);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sub_a_r8_carry() {
    let mut cpu = create_cpu(vec![0x90]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::B, 0x0A);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0xFB);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sub_a_r8_zero() {
    let mut cpu = create_cpu(vec![0x90]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sub_a_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0x96]);
    cpu.registers.set(Register::A, 0x35);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sub_a_hl_mem_half_carry() {
    let mut cpu = create_cpu(vec![0x96]);
    cpu.registers.set(Register::A, 0x30);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x2B);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sub_a_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0x96]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x0A);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0xFB);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sub_a_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0x96]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sub_a_n8_no_carry() {
    let mut cpu = create_cpu(vec![0xD6, 0x05]);
    cpu.registers.set(Register::A, 0x35);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sub_a_n8_half_carry() {
    let mut cpu = create_cpu(vec![0xD6, 0x05]);
    cpu.registers.set(Register::A, 0x30);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x2B);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sub_a_n8_carry() {
    let mut cpu = create_cpu(vec![0xD6, 0x0A]);
    cpu.registers.set(Register::A, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0xFB);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sub_a_n8_zero() {
    let mut cpu = create_cpu(vec![0xD6, 0x05]);
    cpu.registers.set(Register::A, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_r8_no_carry() {
    let mut cpu = create_cpu(vec![0xB8]);
    cpu.registers.set(Register::A, 0x35);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x35);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_r8_half_carry() {
    let mut cpu = create_cpu(vec![0xB8]);
    cpu.registers.set(Register::A, 0x30);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_r8_carry() {
    let mut cpu = create_cpu(vec![0xB8]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::B, 0x0A);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_cp_a_r8_zero() {
    let mut cpu = create_cpu(vec![0xB8]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0xBE]);
    cpu.registers.set(Register::A, 0x35);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x35);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_hl_mem_half_carry() {
    let mut cpu = create_cpu(vec![0xBE]);
    cpu.registers.set(Register::A, 0x30);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0xBE]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x0A);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_cp_a_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xBE]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_n8_no_carry() {
    let mut cpu = create_cpu(vec![0xFE, 0x05]);
    cpu.registers.set(Register::A, 0x35);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x35);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_n8_half_carry() {
    let mut cpu = create_cpu(vec![0xFE, 0x05]);
    cpu.registers.set(Register::A, 0x30);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cp_a_n8_carry() {
    let mut cpu = create_cpu(vec![0xFE, 0x0A]);
    cpu.registers.set(Register::A, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_cp_a_n8_zero() {
    let mut cpu = create_cpu(vec![0xFE, 0x05]);
    cpu.registers.set(Register::A, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_and_a_r8_non_zero() {
    let mut cpu = create_cpu(vec![0xA0]);
    cpu.registers.set(Register::A, 0x35);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_and_a_r8_zero() {
    let mut cpu = create_cpu(vec![0xA0]);
    cpu.registers.set(Register::A, 0x30);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_and_a_hl_mem_non_zero() {
    let mut cpu = create_cpu(vec![0xA6]);
    cpu.registers.set(Register::A, 0x35);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_and_a_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xA6]);
    cpu.registers.set(Register::A, 0x30);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_and_a_n8_non_zero() {
    let mut cpu = create_cpu(vec![0xE6, 0x05]);
    cpu.registers.set(Register::A, 0x35);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x05);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_and_a_n8_zero() {
    let mut cpu = create_cpu(vec![0xE6, 0x05]);
    cpu.registers.set(Register::A, 0x30);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_xor_a_r8_non_zero() {
    let mut cpu = create_cpu(vec![0xA8]);
    cpu.registers.set(Register::A, 0x35);
    cpu.registers.set(Register::B, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_xor_a_r8_zero() {
    let mut cpu = create_cpu(vec![0xA8]);
    cpu.registers.set(Register::A, 0x30);
    cpu.registers.set(Register::B, 0x30);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_xor_a_hl_mem_non_zero() {
    let mut cpu = create_cpu(vec![0xAE]);
    cpu.registers.set(Register::A, 0x35);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_xor_a_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xAE]);
    cpu.registers.set(Register::A, 0x30);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x30);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_xor_a_n8_non_zero() {
    let mut cpu = create_cpu(vec![0xEE, 0x05]);
    cpu.registers.set(Register::A, 0x35);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x30);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_xor_a_n8_zero() {
    let mut cpu = create_cpu(vec![0xEE, 0x30]);
    cpu.registers.set(Register::A, 0x30);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_srl_r8_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x38]);
    cpu.registers.set(Register::B, 0x82);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x41);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_srl_r8_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x38]);
    cpu.registers.set(Register::B, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x41);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_srl_r8_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x38]);
    cpu.registers.set(Register::B, 0x01);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_srl_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x3E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x82);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x41);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_srl_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x3E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x41);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_srl_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x3E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x01);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sra_r8_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x28]);
    cpu.registers.set(Register::B, 0x82);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0xC1);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sra_r8_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x28]);
    cpu.registers.set(Register::B, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0xC1);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sra_r8_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x28]);
    cpu.registers.set(Register::B, 0x01);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sra_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x2E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x82);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0xC1);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sra_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x2E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0xC1);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sra_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x2E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x01);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sla_r8_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x20]);
    cpu.registers.set(Register::B, 0x03);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sla_r8_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x20]);
    cpu.registers.set(Register::B, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sla_r8_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x20]);
    cpu.registers.set(Register::B, 0x80);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sla_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x26]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x03);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sla_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x26]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sla_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x26]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x80);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rrc_r8_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x08]);
    cpu.registers.set(Register::B, 0x06);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x03);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rrc_r8_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x08]);
    cpu.registers.set(Register::B, 0x03);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x81);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rrc_r8_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x08]);
    cpu.registers.set(Register::B, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rrc_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x0E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x06);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x03);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rrc_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x0E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x03);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x81);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rrc_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x0E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rrca_no_carry() {
    let mut cpu = create_cpu(vec![0x0F]);
    cpu.registers.set(Register::A, 0x06);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x03);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rrca_carry() {
    let mut cpu = create_cpu(vec![0x0F]);
    cpu.registers.set(Register::A, 0x03);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x81);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rrca_zero() {
    let mut cpu = create_cpu(vec![0x0F]);
    cpu.registers.set(Register::A, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rr_r8_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x18]);
    cpu.registers.set(Register::B, 0x06);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x83);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rr_r8_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x18]);
    cpu.registers.set(Register::B, 0x03);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x01);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rr_r8_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x18]);
    cpu.registers.set(Register::B, 0x01);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rr_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x1E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x06);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x83);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rr_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x1E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x03);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x01);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rr_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x1E]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x01);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rra_no_carry() {
    let mut cpu = create_cpu(vec![0x1F]);
    cpu.registers.set(Register::A, 0x06);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x83);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rra_carry() {
    let mut cpu = create_cpu(vec![0x1F]);
    cpu.registers.set(Register::A, 0x03);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x01);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rra_zero() {
    let mut cpu = create_cpu(vec![0x1F]);
    cpu.registers.set(Register::A, 0x01);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rl_r8_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x10]);
    cpu.registers.set(Register::B, 0x03);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x07);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rl_r8_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x10]);
    cpu.registers.set(Register::B, 0x83);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rl_r8_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x10]);
    cpu.registers.set(Register::B, 0x80);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rl_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x16]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x03);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x07);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rl_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x16]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x83);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rl_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x16]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x80);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rla_no_carry() {
    let mut cpu = create_cpu(vec![0x17]);
    cpu.registers.set(Register::A, 0x03);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x07);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rla_carry() {
    let mut cpu = create_cpu(vec![0x17]);
    cpu.registers.set(Register::A, 0x83);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rla_zero() {
    let mut cpu = create_cpu(vec![0x17]);
    cpu.registers.set(Register::A, 0x80);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rlc_r8_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x00]);
    cpu.registers.set(Register::B, 0x03);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rlc_r8_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x00]);
    cpu.registers.set(Register::B, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x07);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rlc_r8_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x00]);
    cpu.registers.set(Register::B, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rlc_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x06]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x03);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rlc_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0xCB, 0x06]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x07);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rlc_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x06]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rlca_no_carry() {
    let mut cpu = create_cpu(vec![0x07]);
    cpu.registers.set(Register::A, 0x03);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x06);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_rlca_carry() {
    let mut cpu = create_cpu(vec![0x07]);
    cpu.registers.set(Register::A, 0x83);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x07);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_rlca_zero() {
    let mut cpu = create_cpu(vec![0x07]);
    cpu.registers.set(Register::A, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_adc_a_r8_no_carry() {
    let mut cpu = create_cpu(vec![0x88]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::B, 0x05);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x000A);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_adc_a_r8_half_carry() {
    let mut cpu = create_cpu(vec![0x88]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::B, 0x0D);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0011);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_adc_a_r8_carry() {
    let mut cpu = create_cpu(vec![0x88]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::B, 0xFE);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0003);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_adc_a_r8_zero() {
    let mut cpu = create_cpu(vec![0x88]);
    cpu.registers.set(Register::A, 0x02);
    cpu.registers.set(Register::B, 0xFE);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_adc_a_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0x8E]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x000A);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_adc_a_hl_mem_half_carry() {
    let mut cpu = create_cpu(vec![0x8E]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x0D);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0011);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_adc_a_hl_mem_carry() {
    let mut cpu = create_cpu(vec![0x8E]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0xFE);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0003);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_adc_a_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0x8E]);
    cpu.registers.set(Register::A, 0x02);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0xFE);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_adc_a_n8_no_carry() {
    let mut cpu = create_cpu(vec![0xCE, 0x05]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x000A);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_adc_a_n8_half_carry() {
    let mut cpu = create_cpu(vec![0xCE, 0x0D]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0011);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_adc_a_n8_carry() {
    let mut cpu = create_cpu(vec![0xCE, 0xFE]);
    cpu.registers.set(Register::A, 0x04);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0003);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_adc_a_n8_zero() {
    let mut cpu = create_cpu(vec![0xCE, 0xFE]);
    cpu.registers.set(Register::A, 0x02);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_daa_non_sub_no_adjust() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x15);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x15);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_daa_non_sub_gt9() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x1A);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x20);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_daa_non_sub_half_carry() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x15);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x1B);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_daa_non_sub_carry() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x15);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x75);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
    assert_eq!(result.cycles, 4);
  }

    #[test]
  pub fn test_daa_non_sub_gt99() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0xC0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x20);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_daa_non_sub_both() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x9A);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_daa_sub_no_adjust() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x15);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x15);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_daa_sub_half_carry() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x15);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0F);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_daa_sub_carry() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x15);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0xB5);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_daa_sub_both() {
    let mut cpu = create_cpu(vec![0x27]);
    cpu.registers.set(Register::A, 0x66);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_halt_sets_halted_flag() {
    let mut cpu = create_cpu(vec![0x76]);
    assert_eq!(cpu.halted, false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.halted, true);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_halt_increments_pc() {
    let mut cpu = create_cpu(vec![0x76]);
    
    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_halted_cpu_returns_without_executing() {
    let mut cpu = create_cpu(vec![0x76, 0x27]);
    
    let result1 = cpu.execute_instruction();
    assert_eq!(cpu.halted, true);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(result1.cycles, 4);

    let result2 = cpu.execute_instruction();
    assert_eq!(cpu.halted, true);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
    assert_eq!(result2.cycles, 4);
  }

  #[test]
  pub fn test_halt_wakes_on_interrupt_with_ime_set() {
    let mut cpu = create_cpu(vec![0x76]);
    cpu.ime = true;
    
    let result = cpu.execute_instruction();
    assert_eq!(cpu.halted, true);
    assert_eq!(result.cycles, 4);

    cpu.memory_bus.set_if_flag(0x01);
    cpu.memory_bus.set_ie_flag(0x01);

    let result = cpu.execute_instruction();
    assert_eq!(cpu.halted, false);
    assert_eq!(result.cycles, 20);
  }

  #[test]
  pub fn test_halt_wakes_on_interrupt_without_ime() {
    let mut cpu = create_cpu(vec![0x76, 0x00]);
    cpu.ime = false;
    
    let result = cpu.execute_instruction();
    assert_eq!(cpu.halted, true);

    cpu.memory_bus.set_if_flag(0x01);
    cpu.memory_bus.set_ie_flag(0x01);

    let _result = cpu.execute_instruction();
    assert_eq!(cpu.halted, false);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(result.cycles, 4);
  }

  #[test]
  pub fn test_halt_no_interrupt_stays_halted() {
    let mut cpu = create_cpu(vec![0x76]);
    cpu.ime = true;
    
    let _result = cpu.execute_instruction();
    assert_eq!(cpu.halted, true);

    let _result = cpu.execute_instruction();
    assert_eq!(cpu.halted, true);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);
  }

  #[test]
  pub fn test_halt_interrupt_lower_priority_wakes() {
    let mut cpu = create_cpu(vec![0x76]);
    cpu.ime = true;
    
    let _result = cpu.execute_instruction();
    assert_eq!(cpu.halted, true);

    cpu.memory_bus.set_if_flag(0x1F);
    cpu.memory_bus.set_ie_flag(0x1F);

    let result = cpu.execute_instruction();
    assert_eq!(cpu.halted, false);
    assert_eq!(result.cycles, 20);
  }

  #[test]
  pub fn test_swap_r8_non_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x30]);
    cpu.registers.set(Register::B, 0x01);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.get(Register::B), 0x10);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_swap_r8_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x30]);
    cpu.registers.set(Register::B, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.registers.get(Register::B), 0x00);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_swap_hl_mem_non_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x36]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x01);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x10);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_swap_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0xCB, 0x36]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);
    assert_eq!(cpu.memory_bus.read(0xF234), 0x00);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_r8_no_carry() {
    let mut cpu = create_cpu(vec![0x98]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::B, 0x03);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_r8_with_carry() {
    let mut cpu = create_cpu(vec![0x98]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::B, 0x03);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0001);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_r8_half_carry() {
    let mut cpu = create_cpu(vec![0x98]);
    cpu.registers.set(Register::A, 0x10);
    cpu.registers.set(Register::B, 0x01);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x000F);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_r8_carry_flag() {
    let mut cpu = create_cpu(vec![0x98]);
    cpu.registers.set(Register::A, 0x03);
    cpu.registers.set(Register::B, 0x05);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00FE);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sbc_a_r8_zero() {
    let mut cpu = create_cpu(vec![0x98]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::B, 0x05);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_hl_mem_no_carry() {
    let mut cpu = create_cpu(vec![0x9E]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x03);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_hl_mem_with_carry() {
    let mut cpu = create_cpu(vec![0x9E]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x03);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0001);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_hl_mem_half_carry() {
    let mut cpu = create_cpu(vec![0x9E]);
    cpu.registers.set(Register::A, 0x10);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x01);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x000F);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_hl_mem_carry_flag() {
    let mut cpu = create_cpu(vec![0x9E]);
    cpu.registers.set(Register::A, 0x03);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00FE);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sbc_a_hl_mem_zero() {
    let mut cpu = create_cpu(vec![0x9E]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x05);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_n8_no_carry() {
    let mut cpu = create_cpu(vec![0xDE, 0x03]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0002);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_n8_with_carry() {
    let mut cpu = create_cpu(vec![0xDE, 0x03]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0001);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_n8_half_carry() {
    let mut cpu = create_cpu(vec![0xDE, 0x01]);
    cpu.registers.set(Register::A, 0x10);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x000F);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_sbc_a_n8_carry_flag() {
    let mut cpu = create_cpu(vec![0xDE, 0x05]);
    cpu.registers.set(Register::A, 0x03);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00FE);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_sbc_a_n8_zero() {
    let mut cpu = create_cpu(vec![0xDE, 0x05]);
    cpu.registers.set(Register::A, 0x05);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x0000);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_cpl() {
    let mut cpu = create_cpu(vec![0x2F]);
    cpu.registers.set(Register::A, 0x05);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x00FA);
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), true);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_scf_already_off() {
    let mut cpu = create_cpu(vec![0x37]);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_scf_already_on() {
    let mut cpu = create_cpu(vec![0x37]);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_ccf_switch_on() {
    let mut cpu = create_cpu(vec![0x3F]);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(false);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), true);
  }

  #[test]
  pub fn test_ccf_switch_off() {
    let mut cpu = create_cpu(vec![0x3F]);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(true);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 1);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), false);
    assert_eq!(cpu.registers.carry(), false);
  }

  #[test]
  pub fn test_bit_0_b_set() {
    let mut cpu = create_cpu(vec![0xCB, 0x40]);
    cpu.registers.set(Register::B, 0x01); // bit 0 set

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_bit_0_b_clear() {
    let mut cpu = create_cpu(vec![0xCB, 0x40]);
    cpu.registers.set(Register::B, 0xFE); // bit 0 clear

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_bit_7_a_set() {
    let mut cpu = create_cpu(vec![0xCB, 0x7F]);
    cpu.registers.set(Register::A, 0x80); // bit 7 set

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_bit_7_a_clear() {
    let mut cpu = create_cpu(vec![0xCB, 0x7F]);
    cpu.registers.set(Register::A, 0x7F); // bit 7 clear

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_bit_4_hl_mem_set() {
    let mut cpu = create_cpu(vec![0xCB, 0x66]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x10); // bit 4 set

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.zero(), false);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_bit_4_hl_mem_clear() {
    let mut cpu = create_cpu(vec![0xCB, 0x66]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0xEF); // bit 4 clear

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 12);
    assert_eq!(cpu.registers.zero(), true);
    assert_eq!(cpu.registers.subtract(), false);
    assert_eq!(cpu.registers.half_carry(), true);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_res_0_b() {
    let mut cpu = create_cpu(vec![0xCB, 0x80]);
    cpu.registers.set(Register::B, 0xFF); // all bits set

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::B), 0xFE); // bit 0 cleared
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_res_7_a() {
    let mut cpu = create_cpu(vec![0xCB, 0xBF]);
    cpu.registers.set(Register::A, 0xFF);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::A), 0x7F); // bit 7 cleared
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_res_4_hl_mem() {
    let mut cpu = create_cpu(vec![0xCB, 0xA6]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0xFF);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0xEF); // bit 4 cleared
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_set_0_c() {
    let mut cpu = create_cpu(vec![0xCB, 0xC1]);
    cpu.registers.set(Register::C, 0x00); // all bits clear

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::C), 0x01); // bit 0 set
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_set_7_d() {
    let mut cpu = create_cpu(vec![0xCB, 0xFA]);
    cpu.registers.set(Register::D, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::D), 0x80); // bit 7 set
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 8);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_set_3_hl_mem() {
    let mut cpu = create_cpu(vec![0xCB, 0xDE]);
    cpu.registers.set(Register::HL, 0xF234);
    cpu.memory_bus.write(0xF234, 0x00);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.memory_bus.read(0xF234), 0x08); // bit 3 set
    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 16);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }

  #[test]
  pub fn test_stop() {
    let mut cpu = create_cpu(vec![0x10, 0x00]);

    let result = cpu.execute_instruction();

    assert_eq!(cpu.registers.get(Register::PC), INITIAL_PC + 2);

    assert_eq!(result.cycles, 4);
    assert_eq!(cpu.registers.zero(), INITIAL_ZERO_FLAG);
    assert_eq!(cpu.registers.subtract(), INITIAL_SUBTRACT_FLAG);
    assert_eq!(cpu.registers.half_carry(), INITIAL_HALF_CARRY_FLAG);
    assert_eq!(cpu.registers.carry(), INITIAL_CARRY_FLAG);
  }
}
