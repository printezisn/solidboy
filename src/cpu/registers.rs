#[derive(Clone, Copy)]
pub enum Register16 {
  PC,
  SP,
  AF,
  BC,
  DE,
  HL
}

#[derive(Clone, Copy)]
pub enum Register8 {
  A,
  B,
  C,
  D,
  E,
  F,
  H,
  L
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

  pub fn get8(&self, name: Register8) -> u8 {
    match name {
      Register8::A => self.a,
      Register8::B => self.b,
      Register8::C => self.c,
      Register8::D => self.d,
      Register8::E => self.e,
      Register8::F => self.f(),
      Register8::H => self.h,
      Register8::L => self.l
    }
  }

  pub fn set8(&mut self, name: Register8, value: u8) {
    match name {
      Register8::A => self.a = value,
      Register8::B => self.b = value,
      Register8::C => self.c = value,
      Register8::D => self.d = value,
      Register8::E => self.e = value,
      Register8::F => self.set_f(value),
      Register8::H => self.h = value,
      Register8::L => self.l = value
    }
  }

  pub fn get16(&self, name: Register16) -> u16 {
    match name {
      Register16::PC => self.pc,
      Register16::SP => self.sp,
      Register16::AF => ((self.a as u16) << 8) | (self.f() as u16),
      Register16::BC => ((self.b as u16) << 8) | (self.c as u16),
      Register16::DE => ((self.d as u16) << 8) | (self.e as u16),
      Register16::HL => ((self.h as u16) << 8) | (self.l as u16)
    }
  }

  pub fn set16(&mut self, name: Register16, value: u16) {
    match name {
      Register16::PC => self.pc = value,
      Register16::SP => self.sp = value,
      Register16::AF => {
        self.a = (value >> 8) as u8;
        self.set_f((value & 0xFF) as u8);
      },
      Register16::BC => {
        self.b = (value >> 8) as u8;
        self.c = (value & 0xFF) as u8;
      },
      Register16::DE => {
        self.d = (value >> 8) as u8;
        self.e = (value & 0xFF) as u8;
      },
      Register16::HL => {
        self.h = (value >> 8) as u8;
        self.l = (value & 0xFF) as u8;
      }
    }
  }

  pub fn add8(&mut self, name: Register8, value: u8) {
    let old_value = self.get8(name);
    let (new_value, overflowing) = old_value.overflowing_add(value);

    self.zero = new_value == 0;
    self.subtract = false;
    self.half_carry = (old_value & 0xF) + (value & 0xF) > 0xF;
    self.carry = overflowing;
    self.set8(name, new_value);
  }

  pub fn inc8(&mut self, name: Register8) {
    let old_value = self.get8(name);
    let new_value = old_value.wrapping_add(1);

    self.zero = new_value == 0;
    self.subtract = false;
    self.half_carry = (old_value & 0xF) + 1 > 0xF;
    self.set8(name, new_value);
  }

  pub fn inc16(&mut self, name: Register16) {
    let old_value = self.get16(name);
    let new_value = old_value.wrapping_add(1);
    self.set16(name, new_value);
  }

  pub fn or8(&mut self, name1: Register8, name2: Register8) {
    let value1 = self.get8(name1);
    let value2 = self.get8(name2);
    let new_value = value1 | value2;

    self.zero = new_value == 0;
    self.subtract = false;
    self.half_carry = false;
    self.carry = false;
    self.set8(name1, new_value);
  }

  pub fn zero(&self) -> bool {
    self.zero
  }

  pub fn subtract(&self) -> bool {
    self.subtract
  }

  pub fn half_carry(&self) -> bool {
    self.half_carry
  }

  pub fn carry(&self) -> bool {
    self.carry
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