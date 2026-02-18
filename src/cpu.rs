mod registers;

use registers::Register8;
use registers::Register16;
use registers::Registers;

pub struct CPU {
  registers: Registers,
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
        let byte1 = self.rom[pc + 1] as u16;
        let byte2 = self.rom[pc + 2] as u16;
        self.registers.set16(Register16::SP, (byte2 << 8) | byte1);

        return Instruction { next_pc: pc + 3, cycles: 3 };
      },

      // INC A
      0x3C => {
        self.registers.add8(Register8::A, 1);
        return Instruction { next_pc: pc + 1, cycles: 1 };
      },

      // JP n16
      0xC3 => {
        let byte1 = self.rom[pc + 1] as u16;
        let byte2 = self.rom[pc + 2] as u16;
        let address = (byte2 << 8) | byte1;

        return Instruction { next_pc: address as usize, cycles: 4 };
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