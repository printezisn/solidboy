use super::types::ModelType;

const VRAM_TOTAL_BANKS: usize = 2;
const VRAM_SIZE: usize = 0x9FFF - 0x8000 + 1;
const OAM_SIZE: usize = 0xFE9F - 0xFE00 + 1;
const LCD_SIZE: usize = 0xFF4B - 0xFF40 + 1;
const VRAM_DMA_SIZE: usize = 0xFF55 - 0xFF51 + 1;
const BG_OBJ_PALETTES_SIZE: usize = 0xFF6B - 0xFF68 + 1;

pub struct PPU {
  vram: [u8; VRAM_SIZE * VRAM_TOTAL_BANKS],
  vram_bank: u8,
  oam: [u8; OAM_SIZE],
  lcd: [u8; LCD_SIZE],
  vram_dma: [u8; VRAM_DMA_SIZE],
  bg_obj_palettes: [u8; BG_OBJ_PALETTES_SIZE],
  object_priority_mode: u8,
  model_type: ModelType
}

impl PPU {
  pub fn new(model_type: ModelType) -> Self {
    Self {
      vram: [0; VRAM_SIZE * VRAM_TOTAL_BANKS],
      vram_bank: 0,
      oam: [0; OAM_SIZE],
      lcd: [0; LCD_SIZE],
      vram_dma: [0; VRAM_DMA_SIZE],
      bg_obj_palettes: [0; BG_OBJ_PALETTES_SIZE],
      object_priority_mode: 0,
      model_type
    }
  }

  pub fn read(&self, address: u16) -> Option<u8> {
    match address {
      0x8000..=0x9FFF => {
        let bank: usize = if matches!(self.model_type, ModelType::Color) { self.vram_bank as usize } else { 0 };
        Some(self.vram[bank * VRAM_SIZE + address as usize - 0x8000])
      },
      0xFE00..=0xFE9F => Some(self.oam[(address - 0xFE00) as usize]),
      0xFF40..=0xFF4B => Some(self.lcd[(address - 0xFF40) as usize]),
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

  pub fn write(&mut self, address: u16, value: u8) -> bool {
    match address {
      0x8000..=0x9FFF => {
        let bank: usize = if matches!(self.model_type, ModelType::Color) { self.vram_bank as usize } else { 0 };
        self.vram[bank * VRAM_SIZE + address as usize - 0x8000] = value;
      },
      0xFE00..=0xFE9F => {
        self.oam[(address - 0xFE00) as usize] = value;
      },
      0xFF40..=0xFF4B => {
        self.lcd[(address - 0xFF40) as usize] = value;
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
        assert!(ppu.lcd.iter().all(|&x| x == 0));
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
        assert!(ppu.write(0x8100, 0xAB));
        assert_eq!(ppu.vram[0x100], 0xAB);
    }

    #[test]
    fn test_write_vram_color_bank0() {
        let mut ppu = PPU::new(ModelType::Color);
        assert!(ppu.write(0x8100, 0xAB));
        assert_eq!(ppu.vram[0x100], 0xAB);
    }

    #[test]
    fn test_write_vram_color_bank1() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.vram_bank = 1;
        assert!(ppu.write(0x8100, 0xCD));
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
        assert!(ppu.write(0xFE10, 0xEF));
        assert_eq!(ppu.oam[0x10], 0xEF);
    }

    #[test]
    fn test_read_lcd() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcd[0x05] = 0x12;
        assert_eq!(ppu.read(0xFF45), Some(0x12));
    }

    #[test]
    fn test_write_lcd() {
        let mut ppu = PPU::new(ModelType::DMG);
        assert!(ppu.write(0xFF45, 0x12));
        assert_eq!(ppu.lcd[0x05], 0x12);
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
        assert!(ppu.write(0xFF4F, 0x01));
        assert_eq!(ppu.vram_bank, 0); // Should remain 0
    }

    #[test]
    fn test_write_ff4f_color() {
        let mut ppu = PPU::new(ModelType::Color);
        assert!(ppu.write(0xFF4F, 0x01));
        assert_eq!(ppu.vram_bank, 1);
        assert!(ppu.write(0xFF4F, 0x02));
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
        assert!(ppu.write(0xFF53, 0x34));
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
        assert!(ppu.write(0xFF69, 0x56));
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
        assert!(ppu.write(0xFF6C, 0x78));
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
        assert!(!ppu.write(0x0000, 0x00));
        assert!(!ppu.write(0xFFFF, 0x00));
    }
}