mod mbc;

use super::timer::Timer;
use mbc::MBC;

pub enum ModelType {
  DMG,
  Color
}

const VRAM_TOTAL_BANKS: usize = 2;
const VRAM_SIZE: usize = 0x9FFF - 0x8000 + 1;

const WRAM_TOTAL_BANKS: usize = 7;
const WRAM_SIZE: usize = 0xCFFF - 0xC000 + 1;

const OAM_SIZE: usize = 0xFE9F - 0xFE00 + 1;

const HIGH_RAM_SIZE: usize = 0xFFFE - 0xFF80 + 1;

const AUDIO_SIZE: usize = 0xFF26 - 0xFF10 + 1;
const SERIAL_TRANSFER_SIZE: usize = 0xFF02 - 0xFF01 + 1;
const WAVE_PATTERN_SIZE: usize = 0xFF3F - 0xFF30 + 1;
const LCD_SIZE: usize = 0xFF4B - 0xFF40 + 1;
const VRAM_DMA_SIZE: usize = 0xFF55 - 0xFF51 + 1;
const BG_OBJ_PALETTES_SIZE: usize = 0xFF6B - 0xFF68 + 1;

pub struct MemoryBus {
  mbc: MBC,

  vram: [u8; VRAM_SIZE * VRAM_TOTAL_BANKS],
  vram_bank: u8,

  wram: [u8; WRAM_SIZE * (WRAM_TOTAL_BANKS + 1)],
  wram_bank: u8,

  oam: [u8; OAM_SIZE],

  high_ram: [u8; HIGH_RAM_SIZE],

  joypad_input: u8,
  serial_transfer: [u8; SERIAL_TRANSFER_SIZE],
  if_flag: u8,
  ie_flag: u8,
  key0: u8,
  key1: u8,
  boot_rom_mapping_control: u8,
  ir_port: u8,
  object_priority_mode: u8,
  
  audio: [u8; AUDIO_SIZE],
  wave_pattern: [u8; WAVE_PATTERN_SIZE],
  lcd: [u8; LCD_SIZE],
  vram_dma: [u8; VRAM_DMA_SIZE],
  bg_obj_palettes: [u8; BG_OBJ_PALETTES_SIZE],

  timer: Timer,
  total_cycles: u8
}

impl MemoryBus {
  pub fn new(rom: Vec<u8>) -> Self {
    MemoryBus {
      mbc: MBC::new(rom),

      vram: [0; VRAM_SIZE * VRAM_TOTAL_BANKS],
      vram_bank: 0,

      wram: [0; WRAM_SIZE * (WRAM_TOTAL_BANKS + 1)],
      wram_bank: 1,

      oam: [0; OAM_SIZE],

      high_ram: [0; HIGH_RAM_SIZE],

      joypad_input: 0,
      serial_transfer: [0; SERIAL_TRANSFER_SIZE],
      if_flag: 0,
      ie_flag: 0,
      key0: 0,
      key1: 0,
      boot_rom_mapping_control: 0,
      ir_port: 0,
      object_priority_mode: 0,

      audio: [0; AUDIO_SIZE],
      wave_pattern: [0; WAVE_PATTERN_SIZE],
      lcd: [0; LCD_SIZE],
      vram_dma: [0; VRAM_DMA_SIZE],
      bg_obj_palettes: [0; BG_OBJ_PALETTES_SIZE],
      
      timer: Timer::new(),
      total_cycles: 0
    }
  }

  pub fn write(&mut self, address: u16, value: u8) {
    let address = match address {
      0xE000..=0xFDFF => address - 0x2000,
      _ => address
    };

    if self.mbc.write(address, value) {
      self.tick(4);
      return;
    }

    match address {
      0x8000..=0x9FFF => {
        let bank: usize = if matches!(self.model_type(), ModelType::Color) { self.vram_bank as usize } else { 0 };
        self.vram[bank * VRAM_SIZE + address as usize - 0x8000] = value;
      },
      0xC000..=0xCFFF => {
        self.wram[(address - 0xC000) as usize] = value;
      },
      0xD000..=0xDFFF => {
        let bank: usize = if matches!(self.model_type(), ModelType::Color) { self.wram_bank as usize } else { 1 };
        self.wram[bank * WRAM_SIZE + address as usize - 0xD000] = value;
      },
      0xFE00..=0xFE9F => {
        self.oam[(address - 0xFE00) as usize] = value;
      },
      0xFEA0..=0xFEFF => {},
      0xFF00 => {
        self.joypad_input = value;
      },
      0xFF01..=0xFF02 => {
        self.serial_transfer[address as usize - 0xFF01] = value;
        if address == 0xFF02 && value == 0x81 {
          print!("{}", self.serial_transfer[0] as char);
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
      0xFF0F => {
        self.if_flag = value;
      },
      0xFF10..=0xFF26 => {
        self.audio[(address - 0xFF10) as usize] = value;
      },
      0xFF30..=0xFF3F => {
        self.wave_pattern[(address - 0xFF30) as usize] = value;
      },
      0xFF40..=0xFF4B => {
        self.lcd[(address - 0xFF40) as usize] = value;
      },
      0xFF4C => {
        self.key0 = value;
      },
      0xFF4D => {
        self.key1 = value;
      },
      0xFF4F => {
        if matches!(self.model_type(), ModelType::Color) {
          self.vram_bank = value & 0x01;
        }
      },
      0xFF50 => {
        self.boot_rom_mapping_control = value;
      },
      0xFF51..=0xFF55 => {
        self.vram_dma[(address - 0xFF51) as usize] = value;
      },
      0xFF56 => {
        self.ir_port = value;
      },
      0xFF68..=0xFF6B => {
        self.bg_obj_palettes[(address - 0xFF68) as usize] = value;
      },
      0xFF6C => {
        self.object_priority_mode = value;
      },
      0xFF70 => {
        if matches!(self.model_type(), ModelType::Color) {
          self.wram_bank = value & 0x07;
          if self.wram_bank == 0 {
            self.wram_bank = 1;
          }
        }
      },
      0xFF80..=0xFFFE => {
        self.high_ram[(address - 0xFF80) as usize] = value;
      },
      0xFFFF => {
        self.ie_flag = value;
      }
      _ => panic!("Invalid write to address {:02X}", address)
    }

    self.tick(4);
  }

  pub fn read_without_tick(&self, address: u16) -> u8 {
    let address = match address {
      0xE000..=0xFDFF => address - 0x2000,
      _ => address
    };

    match self.mbc.read(address) {
      Some(result) => { return result; },
      _ => {}
    };

    match address {
      0x8000..=0x9FFF => {
        let bank: usize = if matches!(self.model_type(), ModelType::Color) { self.vram_bank as usize } else { 0 };
        self.vram[bank * VRAM_SIZE + address as usize - 0x8000]
      },
      0xC000..=0xCFFF => self.wram[(address - 0xC000) as usize],
      0xD000..=0xDFFF => {
        let bank: usize = if matches!(self.model_type(), ModelType::Color) { self.wram_bank as usize } else { 1 };
        self.wram[bank * WRAM_SIZE + address as usize - 0xD000]
      },
      0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
      0xFEA0..=0xFEFF => { 0x00 },
      0xFF00 => self.joypad_input,
      0xFF01..=0xFF02 => self.serial_transfer[(address - 0xFF01) as usize],
      0xFF04 => self.timer.div(),
      0xFF05 => self.timer.tima(),
      0xFF06 => self.timer.tma(),
      0xFF07 => self.timer.tac(),
      0xFF0F => self.if_flag,
      0xFF10..=0xFF26 => self.audio[(address - 0xFF10) as usize],
      0xFF30..=0xFF3F => self.wave_pattern[(address - 0xFF30) as usize],
      0xFF40..=0xFF4B => self.lcd[(address - 0xFF40) as usize],
      0xFF4C => self.key0,
      0xFF4D => self.key1,
      0xFF4F =>  {
        if matches!(self.model_type(), ModelType::Color) {
          return 0xFE | self.vram_bank;
        }

        return 0xFF;
      },
      0xFF50 => self.boot_rom_mapping_control,
      0xFF51..=0xFF55 => self.vram_dma[(address - 0xFF51) as usize],
      0xFF56 => self.ir_port,
      0xFF68..=0xFF6B => self.bg_obj_palettes[(address - 0xFF68) as usize],
      0xFF6C => self.object_priority_mode,
      0xFF70 => {
        if matches!(self.model_type(), ModelType::Color) {
          return self.wram_bank;
        }

        return 0xFF;
      },
      0xFF80..=0xFFFE => self.high_ram[(address - 0xFF80) as usize],
      0xFFFF => self.ie_flag,
      _ => panic!("Invalid read from address {:02X}", address)
    }
  }

  pub fn read(&mut self, address: u16) -> u8 {
    let result = self.read_without_tick(address);

    self.tick(4);
    result
  }

  pub fn model_type(&self) -> ModelType {
    match self.mbc.read(0x0143) {
      Some(0xC0) => ModelType::Color,
      _ => ModelType::DMG
    }
  }

  pub fn if_flag(&self) -> u8 {
    self.if_flag
  }

  pub fn set_if_flag(&mut self, value: u8) {
    self.if_flag = value;
  }

  pub fn ie_flag(&self) -> u8 {
    self.ie_flag
  }

  pub fn set_ie_flag(&mut self, value: u8) {
    self.ie_flag = value;
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

#[cfg(test)]
mod tests {
  use super::*;

  fn make_rom(mbc_type: u8, model_type: u8) -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];
    rom[0x0147] = mbc_type;
    rom[0x0143] = model_type;
    rom
  }

  #[test]
  fn vram_bank_switching_color() {
    let rom = make_rom(0x01, 0xC0);
    let mut bus = MemoryBus::new(rom);

    // VRAM bank 0
    bus.write(0xFF4F, 0);
    bus.write(0x8000, 0x55);
    assert_eq!(bus.read(0x8000), 0x55);

    // VRAM bank 1
    bus.write(0xFF4F, 1);
    bus.write(0x8000, 0xAA);
    assert_eq!(bus.read(0x8000), 0xAA);

    // Back to bank 0
    bus.write(0xFF4F, 0);
    assert_eq!(bus.read(0x8000), 0x55);
  }

  #[test]
  fn wram_bank_switching_color() {
    let rom = make_rom(0x01, 0xC0);
    let mut bus = MemoryBus::new(rom);

    // wram bank 0, write to fixed region
    bus.write(0xC000, 0x11);
    assert_eq!(bus.read(0xC000), 0x11);

    // select bank 1 for 0xD000 region
    bus.write(0xFF70, 1);
    bus.write(0xD000, 0x22);
    assert_eq!(bus.read(0xD000), 0x22);

    // ensure it does not clobber bank 0
    assert_eq!(bus.read(0xC000), 0x11);
  }

  #[test]
  fn echo_ram_mirror() {
    let rom = make_rom(0x00, 0x00);
    let mut bus = MemoryBus::new(rom);

    bus.write(0xE000, 0x77);
    assert_eq!(bus.read(0xC000), 0x77);
    assert_eq!(bus.read(0xE000), 0x77);
  }

  #[test]
  fn total_cycles_increment_on_access() {
    let rom = make_rom(0x00, 0x00);
    let mut bus = MemoryBus::new(rom);

    let start = bus.total_cycles();
    let _ = bus.read(0xC000);
    assert_eq!(bus.total_cycles(), start + 4);

    bus.write(0xC000, 0x99);
    assert_eq!(bus.total_cycles(), start + 8);
  }
}
