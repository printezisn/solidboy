use super::timer::Timer;

pub struct MemoryBus {
  rom: Vec<u8>,
  rom_bank: u8,
  ram_enabled: bool,
  memory: [u8; 0x7FFF + 1],
  timer: Timer,
  total_cycles: u8
}

impl MemoryBus {
  pub fn new(rom: Vec<u8>) -> Self {
    MemoryBus {
      rom,
      rom_bank: 1,
      ram_enabled: false,
      memory: [0; 0x7FFF + 1],
      timer: Timer::new(),
      total_cycles: 0
    }
  }

  pub fn write(&mut self, address: u16, value: u8) {
    match address {
      0x0000..=0x1FFF => {
        self.ram_enabled = (value & 0xF) == 0xA;
      },
      0x2000..=0x3FFF => {
        self.rom_bank = value & 0x1F;
        if self.rom_bank == 0 {
          self.rom_bank = 1;
        }

        let rom_bank_mask = (2 << self.rom[0x0148]) - 1;
        self.rom_bank %= rom_bank_mask;
      },
      0x4000..=0x7FFF => panic!("Invalid write to address {:02X}", address),
      0xA000..=0xBFFF => {
        if self.ram_enabled {
          self.memory[(address - 0x8000) as usize] = value;
        }
      },
      0xFF04 => {
        self.timer.reset_div();
      },
      0xFF05 => {
        self.timer.set_tima(value);
      },
      0xFF06 => {
        self.timer.set_tma(value);
      },
      0xFF07 => {
        self.timer.set_tac(value);
      },
      _ => {
        self.memory[(address - 0x8000) as usize] = value;
        if address == 0xFF02 && value == 0x81 {
          print!("{}", self.memory[0xFF01 - 0x8000] as char);
        }
      }
    }

    self.tick(4);
  }

  pub fn read_without_tick(&self, address: u16) -> u8 {
    match address {
      0x0000..=0x3FFF => self.rom[address as usize],
      0x4000..=0x7FFF => self.rom[((self.rom_bank as u16) * 0x4000 + (address as u16) - 0x4000) as usize],
      0xA000..=0xBFFF => {
        if self.ram_enabled {
          return self.memory[(address - 0x8000) as usize];
        }

        return 0xFF;
      },
      0xFF04 => self.timer.div(),
      0xFF05 => self.timer.tima(),
      0xFF06 => self.timer.tma(),
      0xFF07 => self.timer.tac(),
      _ => self.memory[(address - 0x8000) as usize]
    }
  }

  pub fn read(&mut self, address: u16) -> u8 {
    let result = self.read_without_tick(address);

    self.tick(4);
    result
  }

  pub fn if_flag(&self) -> u8 {
    self.memory[0xFF0F - 0x8000]
  }

  pub fn set_if_flag(&mut self, value: u8) {
    self.memory[0xFF0F - 0x8000] = value;
  }

  pub fn ie_flag(&self) -> u8 {
    self.memory[0xFFFF - 0x8000]
  }

  pub fn set_ie_flag(&mut self, value: u8) {
    self.memory[0xFFFF - 0x8000] = value;
  }

  pub fn reset_total_cycles(&mut self) {
    self.total_cycles = 0;
  }

  pub fn total_cycles(&self) -> u8 {
    self.total_cycles
  }

  pub fn tick(&mut self, cycles: u8) {
    self.total_cycles += cycles;
    if self.timer.tick(cycles).request_interrupt {
      self.set_if_flag(self.if_flag() | 0x04);
    }
  }
}