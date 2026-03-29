pub struct Timer {
  counter: u16,
  tima: u8,
  tma: u8,
  tac: u8,
  previous_tma: u8,
  reload_delay: u8
}

impl Timer {
  pub fn new() -> Timer {
    Timer { 
      counter: 0,
      tima: 0,
      tma: 0,
      tac: 0,
      previous_tma: 0,
      reload_delay: 0
    }
  }

  pub fn div(&self) -> u8 {
    (self.counter >> 8) as u8
  }

  pub fn reset_div(&mut self) {
    self.counter = 0;
  }

  pub fn tima(&self) -> u8 {
    self.tima
  }

  pub fn set_tima(&mut self, value: u8) {
    self.tima = value;
    self.reload_delay = 0;
  }

  pub fn tma(&self) -> u8 {
    self.tma
  }

  pub fn set_tma(&mut self, value: u8) {
    self.tma = value;
  }

  pub fn tac(&self) -> u8 {
    self.tac
  }

  pub fn set_tac(&mut self, value: u8) {
    self.tac = value;
  }

  pub fn tick(&mut self, if_flag: &mut u8, cycles: u8) {
    for _ in 0..cycles {
      if self.single_tick() {
        *if_flag |= 0x04;
      }
    }

    self.previous_tma = self.tma;
  }

  fn single_tick(&mut self) -> bool {
    let mut result = false;

    let old_counter = self.counter;
    self.counter = self.counter.wrapping_add(1);

    if self.reload_delay > 0 {
      self.reload_delay -= 1;
      if self.reload_delay == 0 {
        self.tima = self.previous_tma;
        result = true;
      }
    }

    if self.tac & 0x04 == 0 {
      return result;
    }

    let bit_index: u8 = match self.tac & 0b11 {
        0b00 => 9,
        0b01 => 3,
        0b10 => 5,
        0b11 => 7,
        _ => unreachable!(),
    };

    let old_bit = (old_counter >> bit_index) & 0x01;
    let new_bit = (self.counter >> bit_index) & 0x01;

    if old_bit == 1 && new_bit == 0 {
      let (new_value, overflow) = self.tima.overflowing_add(1);
      self.tima = new_value;

      if overflow {
        self.reload_delay = 4;
      }
    }

    result
  }
}

#[cfg(test)]
mod timer {
  use super::*;

  fn create_timer() -> Timer {
    Timer::new()
  }

  #[test]
  fn test_div_increment() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;

    for _ in 0..20 {
      timer.tick(&mut if_flag, 50);
    }

    assert_eq!(timer.div(), 3);
  }

  #[test]
  fn test_tima_disabled_no_increment() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x00);
    timer.set_tima(0x00);

    for _ in 0..2 {
      timer.tick(&mut if_flag, 128);
    }

    assert_eq!(timer.tima(), 0x00);
    assert_eq!(if_flag & 0x04, 0);
  }

  #[test]
  fn test_tima_increment_frequency_1024() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04);
    timer.set_tima(0x00);

    for _ in 0..2 {
      timer.tick(&mut if_flag, 255);
    }
    timer.tick(&mut if_flag, 2);
    assert_eq!(timer.tima(), 0x00);

    for _ in 0..2 {
      timer.tick(&mut if_flag, 255);
    }
    timer.tick(&mut if_flag, 2);
    assert_eq!(timer.tima(), 0x01);

    for _ in 0..4 {
      timer.tick(&mut if_flag, 255);
    }
    timer.tick(&mut if_flag, 4);
    assert_eq!(timer.tima(), 0x02);
  }

  #[test]
  fn test_tima_increment_frequency_16() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x01);
    timer.set_tima(0x00);

    timer.tick(&mut if_flag, 8);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 8);
    assert_eq!(timer.tima(), 0x01);

    timer.tick(&mut if_flag, 16);
    assert_eq!(timer.tima(), 0x02);
  }

  #[test]
  fn test_tima_increment_frequency_64() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x02);
    timer.set_tima(0x00);

    timer.tick(&mut if_flag, 32);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 32);
    assert_eq!(timer.tima(), 0x01);

    timer.tick(&mut if_flag, 64);
    assert_eq!(timer.tima(), 0x02);
  }

  #[test]
  fn test_tima_increment_frequency_256() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x03);
    timer.set_tima(0x00);

    timer.tick(&mut if_flag, 128);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 128);
    assert_eq!(timer.tima(), 0x01);

    for _ in 0..2 {
      timer.tick(&mut if_flag, 128);
    }
    assert_eq!(timer.tima(), 0x02);
  }

  #[test]
  fn test_tima_overflow_triggers_reload() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x01);
    timer.set_tima(0xFF);
    timer.set_tma(0x42);

    timer.tick(&mut if_flag, 16);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 3);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 1);
    assert_eq!(timer.tima(), 0x42);
    assert_eq!(if_flag & 0x04, 0x04);
  }

  #[test]
  fn test_overflow_interrupt_request() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x01);
    timer.set_tima(0xFF);
    timer.set_tma(0x00);

    timer.tick(&mut if_flag, 16);
    assert_eq!(if_flag & 0x04, 0);

    timer.tick(&mut if_flag, 4);
    assert_eq!(if_flag & 0x04, 0x04);
  }

  #[test]
  fn test_multiple_overflows_in_one_tick() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x01);
    timer.set_tima(0xFF);
    timer.set_tma(0xFF);

    timer.tick(&mut if_flag, 32);
    assert_eq!(if_flag & 0x04, 0x04);
  }

  #[test]
  fn test_tac_frequency_change() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x01);
    timer.set_tima(0x00);

    timer.tick(&mut if_flag, 16);
    assert_eq!(timer.tima(), 0x01);

    timer.set_tac(0x04);
    timer.set_tima(0x00);

    timer.tick(&mut if_flag, 32);
    assert_eq!(timer.tima(), 0x00);
  }

  #[test]
  fn test_timer_disable_midway() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x01);
    timer.set_tima(0x00);

    timer.tick(&mut if_flag, 8);
    
    timer.set_tac(0x00);
    
    timer.tick(&mut if_flag, 16);
    assert_eq!(timer.tima(), 0x00);
  }

  #[test]
  fn test_reload_delay_exact_timing() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0b01);
    timer.set_tima(0xFF);
    timer.set_tma(0x99);

    timer.tick(&mut if_flag, 16);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 1);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 1);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 1);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 1);
    assert_eq!(timer.tima(), 0x99);
  }

  #[test]
  fn test_tima_rollover_sequence() {
    let mut timer = create_timer();
    let mut if_flag: u8 = 0;
    timer.set_tac(0x04 | 0x01);
    timer.set_tima(0xFE);
    timer.set_tma(0xFE);

    timer.tick(&mut if_flag, 16);
    assert_eq!(timer.tima(), 0xFF);

    timer.tick(&mut if_flag, 16);
    assert_eq!(timer.tima(), 0x00);

    timer.tick(&mut if_flag, 4);
    assert_eq!(timer.tima(), 0xFE);

    timer.tick(&mut if_flag, 16);
    assert_eq!(timer.tima(), 0xFF);
  }
}