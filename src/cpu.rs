pub mod registers;

use registers::Registers;

pub struct CPU {
  registers: Registers,
  rom: Vec<u8>,
  pc: u16,
  sp: u16
}

struct Instruction {
  bytes: u8,
  cycles: u8
}

impl CPU {
  pub fn new(rom: Vec<u8>) -> Self {
    CPU {
      registers: Registers::new(),
      rom,
      pc: 0,
      sp: 0
    }
  }

  pub fn run(&mut self) {
    loop {
      let instruction = self.execute_instruction();

      self.pc += instruction.bytes as u16;
    }
  }

  fn execute_instruction(&mut self) -> Instruction {
    let opcode = self.rom[self.pc as usize];

    match opcode {
      // INC A
      0x3C => {
        let (new_value, overflowing) = self.registers.get_a().overflowing_add(1);

        self.registers.set_zero(new_value == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry((self.registers.get_a() & 0xF) + 1 > 0xF);
        self.registers.set_carry(overflowing);
        self.registers.set_a(new_value);

        return Instruction { bytes: 1, cycles: 1 };
      },
      _ => {
        println!("{:?}", self.registers);
        panic!("Unknown opcode: {:02X}", opcode);
      }
    }
  }
}