mod registers;

use registers::Register8;
use registers::Register16;
use registers::Registers;
use registers::register8;
use registers::register16;
use registers::register16_stk;
use registers::register16_mem;
use super::memory_bus::MemoryBus;

pub struct CPU {
  registers: Registers,
  memory_bus: MemoryBus,
  rom: Vec<u8>,
  accept_interrupts: bool
}

struct Instruction {
  next_pc: usize,
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

  pub fn run(&mut self) {
    loop {
      let instruction = self.execute_instruction();

      self.registers.set16(Register16::PC, instruction.next_pc as u16);
    }
  }

  fn opcode_read16(&self, pc: usize) -> u16 {
    let byte1 = self.rom[pc + 1] as u16;
    let byte2 = self.rom[pc + 2] as u16;
    (byte2 << 8) | byte1
  }

  fn stack_pop16(&mut self) -> u16 {
    let sp = self.registers.get16(Register16::SP);
    let byte1 = self.memory_bus.read(sp) as u16;
    let byte2 = self.memory_bus.read(sp + 1) as u16;

    self.registers.set16(Register16::SP, sp + 2);

    (byte2 << 8) | byte1
  }

  fn stack_push16(&mut self, value: u16) {
    let sp = self.registers.get16(Register16::SP);
    let byte1 = (value & 0xFF) as u8;
    let byte2 = (value >> 8) as u8;

    self.memory_bus.write(sp - 1, byte2);
    self.memory_bus.write(sp - 2, byte1);

    self.registers.set16(Register16::SP, sp - 2);
  }

  fn execute_instruction(&mut self) -> Instruction {
    let pc = self.registers.get16(Register16::PC) as usize;
    let opcode = self.rom[pc as usize];

    if opcode == 0x00 {
      // noop
      return Instruction { next_pc: pc + 1, cycles: 1 };
    } else if opcode == 0xC3 {
      // JP n16
      return Instruction { next_pc: self.opcode_read16(pc) as usize, cycles: 3 };
    } else if opcode == 0xF3 {
      // DI
      self.accept_interrupts = false;
      return Instruction { next_pc: pc + 1, cycles: 1 };
    } else if opcode & 0xC0 == 0x00 && opcode & 0x0F == 0x01 {
      // LD r16, n16
      let register_index = (opcode >> 4) & 0x03;
      let register = register16(register_index);
      let value = self.opcode_read16(pc);
      self.registers.set16(register, value);

      return Instruction { next_pc: pc + 3, cycles: 3 };
    } else if opcode == 0xEA {
      // LD [n16], A
      let address = self.opcode_read16(pc);
      let value = self.registers.get8(Register8::A);
      self.memory_bus.write(address, value);

      return Instruction { next_pc: pc + 3, cycles: 4 };
    } else if opcode & 0xC0 == 0x00 && opcode & 0x07 == 0x06 && (opcode >> 3) & 0x07 != 0x06 {
      // LD r8, n8
      let register_index = (opcode >> 3) & 0x07;
      let register = register8(register_index);
      let value = self.rom[pc + 1];
      self.registers.set8(register, value);

      return Instruction { next_pc: pc + 2, cycles: 2 };
    } else if opcode == 0xE0 {
      // LDH [n16], A
      let address = 0xFF00 + self.rom[pc + 1] as u16;
      let value = self.registers.get8(Register8::A);
      self.memory_bus.write(address, value);

      return Instruction { next_pc: pc + 2, cycles: 3 };
    } else if opcode == 0xCD {
      // CALL n16
      let address = self.opcode_read16(pc);
      self.stack_push16((pc + 3) as u16);

      return Instruction { next_pc: address as usize, cycles: 6 };
    } else if opcode & 0xC0 == 0x40 && opcode != 0x76 {
      // LD r8, r8
      let dest_register_index = (opcode >> 3) & 0x07;
      let src_register_index = opcode & 0x07;
      let dest_register = register8(dest_register_index);
      let src_register = register8(src_register_index);
      let value = self.registers.get8(src_register);
      self.registers.set8(dest_register, value);

      return Instruction { next_pc: pc + 1, cycles: 1 };
    } else if opcode == 0x18 {
      // JR n16
      let offset = self.rom[pc + 1] as i8;
      let next_pc = ((pc as i32) + 2 + (offset as i32)) as usize;
      return Instruction { next_pc, cycles: 3 };
    } else if opcode == 0xC9 {
      // RET
      let next_pc = self.stack_pop16() as usize;
      return Instruction { next_pc, cycles: 4 };
    } else if opcode & 0xC0 == 0xC0 && opcode & 0xF == 0x05 {
      // PUSH r16
      let register_index = (opcode >> 4) & 0x03;
      let register = register16_stk(register_index);
      let value = self.registers.get16(register);
      self.stack_push16(value);

      return Instruction { next_pc: pc + 1, cycles: 4 };
    } else if opcode & 0xC0 == 0xC0 && opcode & 0x0F == 0x01 {
      // POP r16
      let register_index = (opcode >> 4) & 0x03;
      let register = register16_stk(register_index);
      let value = self.stack_pop16();
      self.registers.set16(register, value);

      return Instruction { next_pc: pc + 1, cycles: 3 };
    } else if opcode & 0xC0 == 0x00 && opcode & 0x03 == 0x03 {
      // INC r16
      let register_index = (opcode >> 4) & 0x03;
      let register = register16(register_index);
      self.registers.inc16(register);

      return Instruction { next_pc: pc + 1, cycles: 2 };
    } else if opcode & 0xC0 == 0x00 && opcode & 0x0F == 0x0A {
      // LD A, [r16]
      let register_index = (opcode >> 4) & 0x03;
      let (register, offset) = register16_mem(register_index);
      let address = self.registers.get16(register);
      let value = self.memory_bus.read(address);
      self.registers.set8(Register8::A, value);

      self.registers.set16(register, ((address as i16) + (offset as i16)) as u16);

      return Instruction { next_pc: pc + 1, cycles: 2 };
    } else if opcode & 0xF8 == 0xB0 && opcode & 0x07 != 0x06 {
      // OR A, r8
      let register_index = opcode & 0x07;
      let register = register8(register_index);
      self.registers.or8(Register8::A, register);

      return Instruction { next_pc: pc + 1, cycles: 1 };
    }

    panic!("Unknown opcode: {:02X} {:08b}", opcode, opcode);
  }
}