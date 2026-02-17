#[derive(Debug)]
pub struct Registers {
  b: u8,
  c: u8,
  d: u8,
  e: u8,
  h: u8,
  l: u8,
  a: u8,

  zero: bool,
  subtract: bool,
  half_carry: bool,
  carry: bool
}

impl Registers {
  pub fn new() -> Self {
    Self {
      b: 0,
      c: 0,
      d: 0,
      e: 0,
      h: 0,
      l: 0,
      a: 0,
      zero: false,
      subtract: false,
      half_carry: false,
      carry: false
    }
  }

  pub fn get_a(&self) -> u8 {
    self.a
  }

  pub fn set_a(&mut self, value: u8) {
    self.a = value;
  }

  pub fn get_zero(&self) -> bool {
    self.zero
  }

  pub fn get_subtract(&self) -> bool {
    self.subtract
  }

  pub fn get_half_carry(&self) -> bool {
    self.half_carry
  }

  pub fn get_carry(&self) -> bool {
    self.carry
  }

  pub fn set_zero(&mut self, value: bool) {
    self.zero = value;
  }

  pub fn set_subtract(&mut self, value: bool) {
    self.subtract = value;
  }

  pub fn set_half_carry(&mut self, value: bool) {
    self.half_carry = value;
  }

  pub fn set_carry(&mut self, value: bool) {
    self.carry = value;
  }
}