mod registers;

use registers::Register8;
use registers::Register16;
use registers::Registers;
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

    match opcode {
      // NOP
      0x00 => {
        return Instruction { next_pc: pc + 1, cycles: 1 };
      },

      // LD SP, n16
      0x31 => {
        self.registers.set16(Register16::SP, self.opcode_read16(pc));
        return Instruction { next_pc: pc + 3, cycles: 3 };
      },

      // LD (n16), A
      0xEA => {
        let address = self.opcode_read16(pc);
        let value = self.registers.get8(Register8::A);

        self.memory_bus.write(address, value);
        
        return Instruction { next_pc: pc + 3, cycles: 4 };
      },

      // LD A, n8
      0x3E => {
        self.registers.set8(Register8::A, self.rom[pc + 1]);
        return Instruction { next_pc: pc + 2, cycles: 2 };
      },

      // LDH [n16], A
      0xE0 => {
        let offset = self.rom[pc + 1] as u16;
        let address = 0xFF00 + offset;
        let value = self.registers.get8(Register8::A);

        self.memory_bus.write(address, value);

        return Instruction { next_pc: pc + 2, cycles: 3 };
      },

      // LDH A, [n16]
      0xF0 => {
        let offset = self.rom[pc + 1] as u16;
        let address = 0xFF00 + offset;
        let value = self.memory_bus.read(address);

        self.registers.set8(Register8::A, value);

        return Instruction { next_pc: pc + 2, cycles: 3 };
      },

      // LDH A, [C]
      0xF2 => {
        let offset = self.registers.get8(Register8::C) as u16;
        let address = 0xFF00 + offset;
        let value = self.memory_bus.read(address);

        self.registers.set8(Register8::A, value);

        return Instruction { next_pc: pc + 1, cycles: 2 };
      },

      // LD HL, n16
      0x21 => {
        self.registers.set16(Register16::HL, self.opcode_read16(pc));
        return Instruction { next_pc: pc + 3, cycles: 3 };
      },

      // LD BC, n16
      0x01 => {
        self.registers.set16(Register16::BC, self.opcode_read16(pc));
        return Instruction { next_pc: pc + 3, cycles: 3 };
      },

      // LD A, H
      0x7C => {
        let value = self.registers.get8(Register8::H);
        self.registers.set8(Register8::A, value);
        return Instruction { next_pc: pc + 1, cycles: 1 };
      },

      // LD A, L
      0x7D => {
        let value = self.registers.get8(Register8::L);
        self.registers.set8(Register8::A, value);
        return Instruction { next_pc: pc + 1, cycles: 1 };
      },

      // LD A, B
      0x78 => {
        let value = self.registers.get8(Register8::B);
        self.registers.set8(Register8::A, value);
        return Instruction { next_pc: pc + 1, cycles: 1 };
      },

      // LD A, [HLI]
      0x2A => {
        let address = self.registers.get16(Register16::HL);
        let value = self.memory_bus.read(address);

        self.registers.set8(Register8::A, value);
        self.registers.set16(Register16::HL, address + 1);

        return Instruction { next_pc: pc + 1, cycles: 2 };
      },

      // OR A, C
      0xB1 => {
        self.registers.or8(Register8::A, Register8::C);
        return Instruction { next_pc: pc + 1, cycles: 1 };
      }

      // CALL n16
      0xCD => {
        let address = self.opcode_read16(pc);
        let return_address = (pc + 3) as u16;

        self.stack_push16(return_address);

        return Instruction { next_pc: address as usize, cycles: 6 };
      },

      // INC A
      0x3C => {
        self.registers.inc8(Register8::A);
        return Instruction { next_pc: pc + 1, cycles: 1 };
      },

      // INC HL
      0x23 => {
        self.registers.inc16(Register16::HL);
        return Instruction { next_pc: pc + 1, cycles: 2 };
      },

      // INC BC
      0x03 => {
        self.registers.inc16(Register16::BC);
        return Instruction { next_pc: pc + 1, cycles: 2 };
      },

      // JP n16
      0xC3 => {
        return Instruction { next_pc: self.opcode_read16(pc) as usize, cycles: 4 };
      },

      // JR n16
      0x18 => {
        let offset = self.rom[pc + 1] as i8;
        let next_pc = ((pc as i16) + 2 + (offset as i16)) as usize;

        return Instruction { next_pc, cycles: 3 };
      },

      // JR Z, n16
      0x28 => {
        let offset = self.rom[pc + 1] as i8;
        let next_pc = ((pc as i16) + 2 + (offset as i16)) as usize;

        if self.registers.zero() {
          return Instruction { next_pc, cycles: 3 };
        } else {
          return Instruction { next_pc: pc + 2, cycles: 2 };
        }
      },

      // RET
      0xC9 => {
        return Instruction { next_pc: self.stack_pop16() as usize, cycles: 4 };
      },

      // POP HL
      0xE1 => {
        let value = self.stack_pop16();
        self.registers.set16(Register16::HL, value);

        return Instruction { next_pc: pc + 1, cycles: 3 };
      },

      // POP AF
      0xF1 => {
        let value = self.stack_pop16();
        self.registers.set16(Register16::AF, value);

        return Instruction { next_pc: pc + 1, cycles: 3 };
      },

      // PUSH HL
      0xE5 => {
        let value = self.registers.get16(Register16::HL);
        self.stack_push16(value);

        return Instruction { next_pc: pc + 1, cycles: 4 };
      },

      // PUSH BC
      0xC5 => {
        let value = self.registers.get16(Register16::BC);
        self.stack_push16(value);

        return Instruction { next_pc: pc + 1, cycles: 4 };
      },

      // PUSH AF
      0xF5 => {
        let value = self.registers.get16(Register16::AF);
        self.stack_push16(value);

        return Instruction { next_pc: pc + 1, cycles: 4 };
      },

      // DI
      0xF3 => {
        self.accept_interrupts = false;

        return Instruction { next_pc: pc + 1, cycles: 1 };
      }

      _ => {
        panic!("Unknown opcode: {:02X}", opcode);
      }
    }
  }
}