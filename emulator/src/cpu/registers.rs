#[derive(Clone, Copy)]
pub enum Register {
  A,
  B,
  C,
  D,
  E,
  F,
  H,
  L,
  PC,
  SP,
  AF,
  BC,
  DE,
  HL
}

#[derive(Debug)]
pub struct Registers {
  a: u8,
  b: u8,
  c: u8,
  d: u8,
  e: u8,
  h: u8,
  l: u8,

  pc: u16,
  sp: u16,

  zero: bool,
  subtract: bool,
  half_carry: bool,
  carry: bool
}

pub fn register_bytes(name: Register) -> u8 {
  match name {
    Register::A | Register::B | Register::C | Register::D | Register::E | Register::F | Register::H | Register::L => 1,
    Register::PC | Register::SP | Register::AF | Register::BC | Register::DE | Register::HL => 2
  }
}

impl Registers {
  pub fn new() -> Self {
    Self {
      a: 0x01,
      b: 0,
      c: 0x13,
      d: 0,
      e: 0xD8,
      h: 0x01,
      l: 0x4D,

      pc: 0x0100,
      sp: 0xFFFE,

      zero: true,
      subtract: false,
      half_carry: true,
      carry: true
    }
  }

  pub fn get(&self, name: Register) -> u16 {
    match name {
      Register::A => self.a as u16,
      Register::B => self.b as u16,
      Register::C => self.c as u16,
      Register::D => self.d as u16,
      Register::E => self.e as u16,
      Register::F => self.f() as u16,
      Register::H => self.h as u16,
      Register::L => self.l as u16,
      Register::PC => self.pc,
      Register::SP => self.sp,
      Register::AF => ((self.a as u16) << 8) | (self.f() as u16),
      Register::BC => ((self.b as u16) << 8) | (self.c as u16),
      Register::DE => ((self.d as u16) << 8) | (self.e as u16),
      Register::HL => ((self.h as u16) << 8) | (self.l as u16)
    }
  }

  pub fn set(&mut self, name: Register, value: u16) {
    match name {
      Register::A => self.a = value as u8,
      Register::B => self.b = value as u8,
      Register::C => self.c = value as u8,
      Register::D => self.d = value as u8,
      Register::E => self.e = value as u8,
      Register::F => self.set_f(value as u8),
      Register::H => self.h = value as u8,
      Register::L => self.l = value as u8,
      Register::PC => self.pc = value,
      Register::SP => self.sp = value,
      Register::AF => {
        self.a = (value >> 8) as u8;
        self.set_f(value as u8);
      },
      Register::BC => {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
      },
      Register::DE => {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
      },
      Register::HL => {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
      }
    }
  }

  pub fn zero(&self) -> bool {
    self.zero
  }

  pub fn set_zero(&mut self, value: bool) {
    self.zero = value;
  }

  pub fn subtract(&self) -> bool {
    self.subtract
  }

  pub fn set_subtract(&mut self, value: bool) {
    self.subtract = value;
  }

  pub fn half_carry(&self) -> bool {
    self.half_carry
  }

  pub fn set_half_carry(&mut self, value: bool) {
    self.half_carry = value;
  }

  pub fn carry(&self) -> bool {
    self.carry
  }

  pub fn set_carry(&mut self, value: bool) {
    self.carry = value;
  }

  fn f(&self) -> u8 {
    let mut num: u8 = 0;

    if self.zero {
      num |= 1 << 7;
    }
    if self.subtract {
      num |= 1 << 6;
    }
    if self.half_carry {
      num |= 1 << 5;
    }
    if self.carry {
      num |= 1 << 4;
    }

    num
  }

  fn set_f(&mut self, value: u8) {
    self.zero = (value & (1 << 7)) != 0;
    self.subtract = (value & (1 << 6)) != 0;
    self.half_carry = (value & (1 << 5)) != 0;
    self.carry = (value & (1 << 4)) != 0;
  }
}