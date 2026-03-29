use super::types::ModelType;

const VRAM_TOTAL_BANKS: usize = 2;
const VRAM_SIZE: usize = 0x9FFF - 0x8000 + 1;
const OAM_SIZE: usize = 0xFE9F - 0xFE00 + 1;
const VRAM_DMA_SIZE: usize = 0xFF55 - 0xFF51 + 1;
const BG_OBJ_PALETTES_SIZE: usize = 0xFF6B - 0xFF68 + 1;
const FRAME_BUFFER_ROWS: usize = 160;
const FRAME_BUFFER_COLS: usize = 144;

pub struct PPU {
  vram: [u8; VRAM_SIZE * VRAM_TOTAL_BANKS],
  vram_bank: u8,
  oam: [u8; OAM_SIZE],
  lcdc: u8,
  stat: u8,
  scy: u8,
  scx: u8,
  ly: u8,
  lyc: u8,
  oam_dma_transfer: u8,
  dmg_bgp: u8,
  obp0: u8,
  obp1: u8,
  wy: u8,
  wx: u8,
  vram_dma: [u8; VRAM_DMA_SIZE],
  bg_obj_palettes: [u8; BG_OBJ_PALETTES_SIZE],
  object_priority_mode: u8,
  model_type: ModelType,
  dots: u16,
  mode: u8,
  frame_buffer: [u8; FRAME_BUFFER_ROWS * FRAME_BUFFER_COLS]
}

impl PPU {
  pub fn new(model_type: ModelType) -> Self {
    Self {
      vram: [0; VRAM_SIZE * VRAM_TOTAL_BANKS],
      vram_bank: 0,
      oam: [0; OAM_SIZE],
      lcdc: 0,
      stat: 0,
      scy: 0,
      scx: 0,
      ly: 0,
      lyc: 0,
      oam_dma_transfer: 0,
      dmg_bgp: 0,
      obp0: 0,
      obp1: 0,
      wy: 0,
      wx: 0,
      vram_dma: [0; VRAM_DMA_SIZE],
      bg_obj_palettes: [0; BG_OBJ_PALETTES_SIZE],
      object_priority_mode: 0,
      model_type,
      dots: 0,
      mode: 2,
      frame_buffer: [0; FRAME_BUFFER_ROWS * FRAME_BUFFER_COLS]
    }
  }

  pub fn read(&self, address: u16) -> Option<u8> {
    match address {
      0x8000..=0x9FFF => {
        let bank: usize = if matches!(self.model_type, ModelType::Color) { self.vram_bank as usize } else { 0 };
        Some(self.vram[bank * VRAM_SIZE + address as usize - 0x8000])
      },
      0xFE00..=0xFE9F => Some(self.oam[(address - 0xFE00) as usize]),
      0xFF40 => Some(self.lcdc),
      0xFF41 => Some(self.lcdc),
      0xFF42 => Some(self.scy),
      0xFF43 => Some(self.scx),
      0xFF44 => Some(self.ly),
      0xFF45 => Some(self.lyc),
      0xFF46 => Some(self.oam_dma_transfer),
      0xFF47 => Some(self.dmg_bgp),
      0xFF48 => Some(self.obp0),
      0xFF49 => Some(self.obp1),
      0xFF4A => Some(self.wy),
      0xFF4B => Some(self.wx),
      0xFF4F =>  {
        if matches!(self.model_type, ModelType::Color) {
          return Some(0xFE | self.vram_bank);
        }

        return Some(0xFF);
      },
      0xFF51..=0xFF55 => Some(self.vram_dma[(address - 0xFF51) as usize]),
      0xFF68..=0xFF6B => Some(self.bg_obj_palettes[(address - 0xFF68) as usize]),
      0xFF6C => Some(self.object_priority_mode),
      _ => None
    }
  }

  pub fn write(&mut self, address: u16, value: u8, if_flag: &mut u8) -> bool {
    match address {
      0x8000..=0x9FFF => {
        let bank: usize = if matches!(self.model_type, ModelType::Color) { self.vram_bank as usize } else { 0 };
        self.vram[bank * VRAM_SIZE + address as usize - 0x8000] = value;
      },
      0xFE00..=0xFE9F => {
        self.oam[(address - 0xFE00) as usize] = value;
      },
      0xFF40 => {
        self.lcdc = value;
      },
      0xFF41 => {
        self.update_stat_triggers(
          if_flag,
          self.stat & 0x40 != 0,
          self.stat & 0x20 != 0,
          self.stat & 0x10 != 0,
          self.stat & 0x08 != 0
        );
      },
      0xFF42 => {
        self.scy = value;
      },
      0xFF43 => {
        self.scx = value;
      },
      0xFF44 => {
        self.ly = value;
        self.update_stat_state(if_flag);
      },
      0xFF45 => {
        self.lyc = value;
        self.update_stat_state(if_flag);
      },
      0xFF46 => {
        self.oam_dma_transfer = value;
      },
      0xFF47 => {
        self.dmg_bgp = value;
      },
      0xFF48 => {
        self.obp0 = value;
      },
      0xFF49 => {
        self.obp1 = value;
      },
      0xFF4A => {
        self.wy = value;
      },
      0xFF4B => {
        self.wx = value;
      },
      0xFF4F => {
        if matches!(self.model_type, ModelType::Color) {
          self.vram_bank = value & 0x01;
        }
      },
      0xFF51..=0xFF55 => {
        self.vram_dma[(address - 0xFF51) as usize] = value;
      },
      0xFF68..=0xFF6B => {
        self.bg_obj_palettes[(address - 0xFF68) as usize] = value;
      },
      0xFF6C => {
        self.object_priority_mode = value;
      },
      _ => {
        return false;
      }
    }

    true
  }

  pub fn tick(&mut self, if_flag: &mut u8, cycles: u8) {
    for _ in 0..cycles {
      self.single_tick(if_flag);
    }
  }

  fn single_tick(&mut self, if_flag: &mut u8) {
    if self.lcdc & 0x80 == 0 {
      self.ly = 0;
      self.mode = 2;
      self.dots = 0;
      return;
    }

    self.dots += 1;

    match self.dots {
      0..=79 => {
        self.mode = 2;
      }
      80..=251 => {
        self.mode = 3;
      },
      252..=455 => {
        self.mode = 0;
      },
      456 => {
        self.dots = 0;
        self.ly += 1;
        if self.ly == 144 {
          self.mode = 1;
          *if_flag |= 0x01;
        } else if self.ly == 154 {
          self.ly = 0;
          self.mode = 0;
          render_frame_buffer!(self.frame_buffer.as_ptr(), self.frame_buffer.len());
        }
      },
      _ => {
        console_error!("Invalid ppu state. Dots: {}", self.dots);
      }
    }

    self.update_stat_state(if_flag);
  }

  fn update_stat_state(&mut self, if_flag: &mut u8) {
    if self.ly == self.lyc && (self.stat & 0x04) == 0 {
      self.stat |= 0x04;
      if self.stat & 0x40 != 0 {
        *if_flag |= 0x02;
      }
    } else {
      self.stat &= !0x04;
    }

    if self.stat & 0x03 != self.mode {
      self.stat = (self.stat & !0x03) | self.mode;
      if self.stat & 0x20 != 0 && self.mode == 2 {
        *if_flag |= 0x02;
      } else if self.stat & 0x10 != 0 && self.mode == 1 {
        *if_flag |= 0x02;
      } else if self.stat & 0x08 != 0 && self.mode == 0 {
        *if_flag |= 0x02;
      }
    }
  }

  fn update_stat_triggers(&mut self, if_flag: &mut u8, ly_trigger: bool, mode2_tigger: bool, mode1_trigger: bool, mode0_trigger: bool) {
    if self.stat & 0x40 == 0 && ly_trigger {
      *if_flag |= 0x02;
    }
    if self.stat & 0x20 == 0 && mode2_tigger {
      *if_flag |= 0x02;
    }
    if self.stat & 0x10 == 0 && mode1_trigger {
      *if_flag |= 0x02;
    }
    if self.stat & 0x08 == 0 && mode0_trigger {
      *if_flag |= 0x02;
    }

    self.stat = if ly_trigger { self.stat | 0x40 } else { self.stat & !0x40 };
    self.stat = if mode2_tigger { self.stat | 0x20 } else { self.stat & !0x20 };
    self.stat = if mode1_trigger { self.stat | 0x10 } else { self.stat & !0x10 };
    self.stat = if mode0_trigger { self.stat | 0x08 } else { self.stat & !0x08 };
  }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dmg() {
        let ppu = PPU::new(ModelType::DMG);
        assert_eq!(ppu.vram_bank, 0);
        assert_eq!(ppu.object_priority_mode, 0);
        assert_eq!(ppu.model_type, ModelType::DMG);
        // Check arrays are zeroed
        assert!(ppu.vram.iter().all(|&x| x == 0));
        assert!(ppu.oam.iter().all(|&x| x == 0));
        assert_eq!(ppu.lcdc, 0);
        assert!(ppu.vram_dma.iter().all(|&x| x == 0));
        assert!(ppu.bg_obj_palettes.iter().all(|&x| x == 0));
    }

    #[test]
    fn test_new_color() {
        let ppu = PPU::new(ModelType::Color);
        assert_eq!(ppu.vram_bank, 0);
        assert_eq!(ppu.object_priority_mode, 0);
        assert_eq!(ppu.model_type, ModelType::Color);
    }

    #[test]
    fn test_read_vram_dmg() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.vram[0x100] = 0xAB;
        assert_eq!(ppu.read(0x8100), Some(0xAB));
    }

    #[test]
    fn test_read_vram_color_bank0() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.vram[0x100] = 0xAB;
        assert_eq!(ppu.read(0x8100), Some(0xAB));
    }

    #[test]
    fn test_read_vram_color_bank1() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.vram_bank = 1;
        ppu.vram[VRAM_SIZE + 0x100] = 0xCD;
        assert_eq!(ppu.read(0x8100), Some(0xCD));
    }

    #[test]
    fn test_write_vram_dmg() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0x8100, 0xAB, &mut if_flag));
        assert_eq!(ppu.vram[0x100], 0xAB);
    }

    #[test]
    fn test_write_vram_color_bank0() {
        let mut ppu = PPU::new(ModelType::Color);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0x8100, 0xAB, &mut if_flag));
        assert_eq!(ppu.vram[0x100], 0xAB);
    }

    #[test]
    fn test_write_vram_color_bank1() {
        let mut ppu = PPU::new(ModelType::Color);
        let mut if_flag: u8 = 0;
        ppu.vram_bank = 1;
        assert!(ppu.write(0x8100, 0xCD, &mut if_flag));
        assert_eq!(ppu.vram[VRAM_SIZE + 0x100], 0xCD);
    }

    #[test]
    fn test_read_oam() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.oam[0x10] = 0xEF;
        assert_eq!(ppu.read(0xFE10), Some(0xEF));
    }

    #[test]
    fn test_write_oam() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0xFE10, 0xEF, &mut if_flag));
        assert_eq!(ppu.oam[0x10], 0xEF);
    }

    #[test]
    fn test_read_lcd() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lyc = 0x12;
        assert_eq!(ppu.read(0xFF45), Some(0x12));
    }

    #[test]
    fn test_write_lcd() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0xFF45, 0x12, &mut if_flag));
        assert_eq!(ppu.lyc, 0x12);
    }

    #[test]
    fn test_read_ff4f_dmg() {
        let ppu = PPU::new(ModelType::DMG);
        assert_eq!(ppu.read(0xFF4F), Some(0xFF));
    }

    #[test]
    fn test_read_ff4f_color_bank0() {
        let ppu = PPU::new(ModelType::Color);
        assert_eq!(ppu.read(0xFF4F), Some(0xFE));
    }

    #[test]
    fn test_read_ff4f_color_bank1() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.vram_bank = 1;
        assert_eq!(ppu.read(0xFF4F), Some(0xFF));
    }

    #[test]
    fn test_write_ff4f_dmg() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0xFF4F, 0x01, &mut if_flag));
        assert_eq!(ppu.vram_bank, 0); // Should remain 0
    }

    #[test]
    fn test_write_ff4f_color() {
        let mut ppu = PPU::new(ModelType::Color);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0xFF4F, 0x01, &mut if_flag));
        assert_eq!(ppu.vram_bank, 1);
        assert!(ppu.write(0xFF4F, 0x02, &mut if_flag));
        assert_eq!(ppu.vram_bank, 0); // Only bit 0
    }

    #[test]
    fn test_read_vram_dma() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.vram_dma[0x02] = 0x34;
        assert_eq!(ppu.read(0xFF53), Some(0x34));
    }

    #[test]
    fn test_write_vram_dma() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0xFF53, 0x34, &mut if_flag));
        assert_eq!(ppu.vram_dma[0x02], 0x34);
    }

    #[test]
    fn test_read_bg_obj_palettes() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.bg_obj_palettes[0x01] = 0x56;
        assert_eq!(ppu.read(0xFF69), Some(0x56));
    }

    #[test]
    fn test_write_bg_obj_palettes() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0xFF69, 0x56, &mut if_flag));
        assert_eq!(ppu.bg_obj_palettes[0x01], 0x56);
    }

    #[test]
    fn test_read_ff6c() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.object_priority_mode = 0x78;
        assert_eq!(ppu.read(0xFF6C), Some(0x78));
    }

    #[test]
    fn test_write_ff6c() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        assert!(ppu.write(0xFF6C, 0x78, &mut if_flag));
        assert_eq!(ppu.object_priority_mode, 0x78);
    }

    #[test]
    fn test_read_invalid_address() {
        let ppu = PPU::new(ModelType::DMG);
        assert_eq!(ppu.read(0x0000), None);
        assert_eq!(ppu.read(0xFFFF), None);
    }

    #[test]
    fn test_write_invalid_address() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        assert!(!ppu.write(0x0000, 0x00, &mut if_flag));
        assert!(!ppu.write(0xFFFF, 0x00, &mut if_flag));
    }

    #[test]
    fn test_vblank_interrupt() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        ppu.lcdc = 0x80; // LCD enabled
        ppu.ly = 143;
        ppu.tick(&mut if_flag, 255);
        ppu.tick(&mut if_flag, 201); // Advance to ly=144
        assert_eq!(ppu.ly, 144);
        assert_eq!(if_flag & 0x01, 0x01); // VBlank interrupt
    }

    #[test]
    fn test_lyc_interrupt() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        ppu.lcdc = 0x80;
        ppu.lyc = 10;
        ppu.stat = 0x40; // LYC interrupt enabled
        ppu.ly = 9;
        ppu.tick(&mut if_flag, 255);
        ppu.tick(&mut if_flag, 201); // ly=10
        assert_eq!(ppu.ly, 10);
        assert_eq!(if_flag & 0x02, 0x02); // STAT interrupt
    }

    #[test]
    fn test_mode2_interrupt() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        ppu.lcdc = 0x80;
        ppu.stat = 0x20; // Mode 2 interrupt enabled
        ppu.tick(&mut if_flag, 1); // Enter mode 2
        assert_eq!(ppu.mode, 2);
        assert_eq!(if_flag & 0x02, 0x02); // STAT interrupt
    }

    #[test]
    fn test_mode1_interrupt() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        ppu.lcdc = 0x80;
        ppu.stat = 0x10; // Mode 1 interrupt enabled
        ppu.ly = 143;
        ppu.tick(&mut if_flag, 255);
        ppu.tick(&mut if_flag, 201); // Enter mode 1
        assert_eq!(ppu.mode, 1);
        assert_eq!(if_flag & 0x02, 0x02); // STAT interrupt
    }

    #[test]
    fn test_mode0_interrupt() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        ppu.lcdc = 0x80;
        ppu.stat = 0x08 | 0x03; // Mode 0 interrupt enabled, current mode 3
        ppu.dots = 251; // Just before mode 0
        ppu.tick(&mut if_flag, 1); // Enter mode 0
        assert_eq!(ppu.mode, 0);
        assert_eq!(if_flag & 0x02, 0x02); // STAT interrupt
    }

    #[test]
    fn test_no_interrupt_when_disabled() {
        let mut ppu = PPU::new(ModelType::DMG);
        let mut if_flag: u8 = 0;
        ppu.lcdc = 0x80;
        ppu.lyc = 10;
        ppu.stat = 0x00; // No interrupts enabled
        ppu.ly = 9;
        ppu.tick(&mut if_flag, 255);
        ppu.tick(&mut if_flag, 201); // ly=10
        assert_eq!(ppu.ly, 10);
        assert_eq!(if_flag, 0); // No interrupt
    }
}