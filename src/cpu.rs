pub mod registers;

use registers::Registers;

pub struct CPU {
  registers: Registers,
  rom: Vec<u8>,
  pc: u16,
  sp: u16,
  accept_interrupts: bool
}

struct Instruction {
  next_pc: u16,
  cycles: u8
}

impl CPU {
  pub fn new(rom: Vec<u8>) -> Self {
    CPU {
      registers: Registers::new(),
      rom,
      pc: 0x0100,
      sp: 0xFFFE,
      accept_interrupts: true
    }
  }

  pub fn run(&mut self) {
    loop {
      let instruction = self.execute_instruction();

      self.pc = instruction.next_pc;
    }
  }

  fn execute_instruction(&mut self) -> Instruction {
    let opcode = self.rom[self.pc as usize];

    match opcode {
      // NOP
      0x00 => {
        return Instruction { next_pc: self.pc + 1, cycles: 1 };
      },

      // LD SP, n16
      0x31 => {
        let byte1 = self.rom[self.pc as usize + 1] as u16;
        let byte2 = self.rom[self.pc as usize + 2] as u16;
        self.sp = (byte2 << 8) | byte1;

        return Instruction { next_pc: self.pc + 3, cycles: 3 };
      },

      // INC A
      0x3C => {
        let (new_value, overflowing) = self.registers.get_a().overflowing_add(1);

        self.registers.set_zero(new_value == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry((self.registers.get_a() & 0xF) + 1 > 0xF);
        self.registers.set_carry(overflowing);
        self.registers.set_a(new_value);

        return Instruction { next_pc: self.pc + 1, cycles: 1 };
      },

      // JP n16
      0xC3 => {
        let byte1 = self.rom[self.pc as usize + 1] as u16;
        let byte2 = self.rom[self.pc as usize + 2] as u16;
        let address = (byte2 << 8) | byte1;

        return Instruction { next_pc: address, cycles: 4 };
      },

      // DI
      0xF3 => {
        self.accept_interrupts = false;

        return Instruction { next_pc: self.pc + 1, cycles: 1 };
      }

      _ => {
        panic!("Unknown opcode: {:02X}", opcode);
      }
    }
  }
}