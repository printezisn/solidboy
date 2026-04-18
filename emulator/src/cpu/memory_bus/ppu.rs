use super::types::ModelType;

const VRAM_TOTAL_BANKS: usize = 2;
const VRAM_SIZE: usize = 0x9FFF - 0x8000 + 1;
const OAM_SIZE: usize = 0xFE9F - 0xFE00 + 1;
const CGB_BG_PALETTE_DATA_SIZE: usize = 64;
const CGB_OBJ_PALETTE_DATA_SIZE: usize = 64;
const FRAME_BUFFER_ROWS: usize = 160;
const FRAME_BUFFER_COLS: usize = 144;

struct Sprite {
    x: u8,
    y: u8,
    tile_index: u8,
    attributes: u8,
    priority: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PixelPriority {
    BG,
    BGIfSpriteDisabled,
    Sprite,
}

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
    dmg_bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    bg_palette_spec: u8,
    bg_palette_data: [u8; CGB_BG_PALETTE_DATA_SIZE],
    obj_palette_spec: u8,
    obj_palette_data: [u8; CGB_OBJ_PALETTE_DATA_SIZE],
    object_priority_mode: u8,
    model_type: ModelType,
    dots: u16,
    window_line: u8,
    mode: u8,
    sprites: Vec<Sprite>,
    frame_buffer: [u8; FRAME_BUFFER_ROWS * FRAME_BUFFER_COLS * 4],
    frame_buffer_pixel_priorities: [PixelPriority; FRAME_BUFFER_ROWS * FRAME_BUFFER_COLS],
    skip_frame: bool,
}

impl PPU {
    pub fn new(model_type: ModelType) -> Self {
        Self {
            vram: [0; VRAM_SIZE * VRAM_TOTAL_BANKS],
            vram_bank: 0,
            oam: [0; OAM_SIZE],
            lcdc: 0x91,
            stat: 0x85,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dmg_bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            bg_palette_spec: 0,
            bg_palette_data: [0; CGB_BG_PALETTE_DATA_SIZE],
            obj_palette_spec: 0,
            obj_palette_data: [0; CGB_OBJ_PALETTE_DATA_SIZE],
            object_priority_mode: 0,
            model_type,
            dots: 0,
            window_line: 0,
            mode: 2,
            sprites: Vec::new(),
            frame_buffer: [0; FRAME_BUFFER_ROWS * FRAME_BUFFER_COLS * 4],
            frame_buffer_pixel_priorities: [PixelPriority::Sprite;
                FRAME_BUFFER_ROWS * FRAME_BUFFER_COLS],
            skip_frame: false,
        }
    }

    pub fn mode(&self) -> u8 {
        self.mode
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0x8000..=0x9FFF => {
                if self.mode == 3 {
                    return Some(0xFF);
                }

                let bank: usize = if matches!(self.model_type, ModelType::Color) {
                    self.vram_bank as usize
                } else {
                    0
                };
                Some(self.vram[bank * VRAM_SIZE + address as usize - 0x8000])
            }
            0xFE00..=0xFE9F => {
                if self.mode == 3 {
                    return Some(0xFF);
                }

                Some(self.oam[(address - 0xFE00) as usize])
            }
            0xFF40 => Some(self.lcdc),
            0xFF41 => Some(self.stat),
            0xFF42 => Some(self.scy),
            0xFF43 => Some(self.scx),
            0xFF44 => Some(self.ly),
            0xFF45 => Some(self.lyc),
            0xFF47 => Some(self.dmg_bgp),
            0xFF48 => Some(self.obp0),
            0xFF49 => Some(self.obp1),
            0xFF4A => Some(self.wy),
            0xFF4B => Some(self.wx),
            0xFF4F => {
                if matches!(self.model_type, ModelType::Color) {
                    return Some(0xFE | self.vram_bank);
                }

                return Some(0xFF);
            }
            0xFF68 => Some(self.bg_palette_spec),
            0xFF69 => Some(self.bg_palette_data[(self.bg_palette_spec & 0x3F) as usize]),
            0xFF6A => Some(self.obj_palette_spec),
            0xFF6B => Some(self.obj_palette_data[(self.obj_palette_spec & 0x3F) as usize]),
            0xFF6C => Some(self.object_priority_mode),
            _ => None,
        }
    }

    pub fn write(&mut self, address: u16, value: u8, if_flag: &mut u8) -> bool {
        match address {
            0x8000..=0x9FFF => {
                if self.mode != 3 {
                    let bank: usize = if matches!(self.model_type, ModelType::Color) {
                        self.vram_bank as usize
                    } else {
                        0
                    };
                    self.vram[bank * VRAM_SIZE + address as usize - 0x8000] = value;
                }
            }
            0xFE00..=0xFE9F => {
                if self.mode != 3 {
                    self.oam[(address - 0xFE00) as usize] = value;
                }
            }
            0xFF40 => {
                if self.lcdc & 0x80 != 0 && value & 0x80 == 0 {
                    self.mode = 0;
                } else if self.lcdc & 0x80 == 0 && value & 0x80 != 0 {
                    self.ly = 0;
                    self.dots = 0;
                    self.window_line = 0;
                    self.skip_frame = true;
                }

                self.lcdc = value;
            }
            0xFF41 => {
                self.update_stat_triggers(
                    if_flag,
                    value & 0x40 != 0,
                    value & 0x20 != 0,
                    value & 0x10 != 0,
                    value & 0x08 != 0,
                );
            }
            0xFF42 => {
                self.scy = value;
            }
            0xFF43 => {
                self.scx = value;
            }
            0xFF44 => {
                self.ly = value;
                self.update_stat_state(if_flag);
            }
            0xFF45 => {
                self.lyc = value;
                self.update_stat_state(if_flag);
            }
            0xFF47 => {
                self.dmg_bgp = value;
            }
            0xFF48 => {
                self.obp0 = value;
            }
            0xFF49 => {
                self.obp1 = value;
            }
            0xFF4A => {
                self.wy = value;
            }
            0xFF4B => {
                self.wx = value;
            }
            0xFF4F => {
                if matches!(self.model_type, ModelType::Color) {
                    self.vram_bank = value & 0x01;
                }
            }
            0xFF68 => {
                self.bg_palette_spec = value;
            }
            0xFF69 => {
                let index = self.bg_palette_spec & 0x3F;
                self.bg_palette_data[index as usize] = value;
                if self.bg_palette_spec & 0x80 != 0 {
                    let new_index = (index + 1) & 0x3F;
                    self.bg_palette_spec = (self.bg_palette_spec & 0x80) | new_index;
                }
            }
            0xFF6A => {
                self.obj_palette_spec = value;
            }
            0xFF6B => {
                let index = self.obj_palette_spec & 0x3F;
                self.obj_palette_data[index as usize] = value;
                if self.obj_palette_spec & 0x80 != 0 {
                    let new_index = (index + 1) & 0x3F;
                    self.obj_palette_spec = (self.obj_palette_spec & 0x80) | new_index;
                }
            }
            0xFF6C => {
                self.object_priority_mode = value;
            }
            _ => {
                return false;
            }
        }

        true
    }

    pub fn write_to_vram(&mut self, address: u16, value: u8) {
        let bank: usize = if matches!(self.model_type, ModelType::Color) {
            self.vram_bank as usize
        } else {
            0
        };
        self.vram[bank * VRAM_SIZE + address as usize] = value;
    }

    pub fn tick(&mut self, if_flag: &mut u8, cycles: u8) {
        for _ in 0..cycles {
            self.single_tick(if_flag);
        }
    }

    fn single_tick(&mut self, if_flag: &mut u8) {
        if self.lcdc & 0x80 == 0 {
            return;
        }

        if self.dots == 0 && self.mode != 1 && !self.skip_frame {
            self.oam_scan();
        }

        self.dots += 1;

        let scx_penalty = self.scx % 8;
        let sprite_penalty = self.sprites.len() as u8 * 6;
        let mode3_end = 80 + 172 + scx_penalty as u16 + sprite_penalty as u16 - 1;
        let mode0_start = mode3_end + 1;
        let mode0_end = 455;

        if self.dots <= 79 {
            if self.mode != 1 {
                self.mode = 2;
            }
        } else if self.dots >= 80 && self.dots <= mode3_end {
            if self.mode != 1 {
                self.mode = 3;
                if self.dots == 80 && self.ly < 144 && !self.skip_frame {
                    self.render_scanline();
                }
            }
        } else if self.dots >= mode0_start && self.dots <= mode0_end {
            if self.mode != 1 {
                self.mode = 0;
            }
        } else if self.dots == mode0_end + 1 {
            self.dots = 0;
            self.ly += 1;
            if self.ly == 144 {
                self.mode = 1;
                *if_flag |= 0x01;
            } else if self.ly == 154 {
                self.ly = 0;
                self.window_line = 0;
                self.mode = 0;
                self.skip_frame = false;
                render_frame_buffer!(self.frame_buffer.as_ptr(), self.frame_buffer.len());
            }
        } else {
            console_error!("Invalid ppu state. Dots: {}", self.dots);
        }

        self.update_stat_state(if_flag);
    }

    fn render_scanline(&mut self) {
        let window_enabled = self.lcdc & 0x20 != 0 && self.ly >= self.wy;
        let mut window_drawn = false;

        for i in 0u8..160 {
            let use_window = window_enabled && i + 7 >= self.wx;
            if use_window {
                window_drawn = true;
            }

            if self.lcdc & 0x01 == 0 && !matches!(self.model_type, ModelType::Color) {
                let frame_buffer_index = (self.ly as usize * 160 + i as usize) * 4;

                self.frame_buffer[frame_buffer_index] = 0x9B;
                self.frame_buffer[frame_buffer_index + 1] = 0xBC;
                self.frame_buffer[frame_buffer_index + 2] = 0x0F;
                self.frame_buffer[frame_buffer_index + 3] = 0xFF;
                self.frame_buffer_pixel_priorities[self.ly as usize * 160 + i as usize] =
                    PixelPriority::Sprite;

                continue;
            }

            let timemap_base_bit = if use_window { 0x40 } else { 0x08 };
            let timemap_base: u16 = if self.lcdc & timemap_base_bit != 0 {
                0x1C00
            } else {
                0x1800
            };

            let x = if use_window {
                (i + 7 - self.wx) as u16
            } else {
                i.wrapping_add(self.scx) as u16
            };
            let y = if use_window {
                self.window_line as u16
            } else {
                self.ly.wrapping_add(self.scy) as u16
            };
            let tile_row = (y / 8) as u16;
            let tile_col = x / 8;

            let tilemap_address = timemap_base + tile_row * 32 + tile_col;
            let tile_index = self.vram[tilemap_address as usize];
            let tile_data_address = if self.lcdc & 0x10 != 0 {
                (tile_index as u16) * 16
            } else {
                (0x1000 + (tile_index as i8 as i16) * 16) as u16
            };
            let tile_attributes = if matches!(self.model_type, ModelType::Color) {
                self.vram[VRAM_SIZE + tilemap_address as usize]
            } else {
                0x00
            };

            let palette_num = (tile_attributes & 0x07) as usize;
            let tile_bank = if tile_attributes & 0x08 != 0 { 1 } else { 0 };
            let x_flip = tile_attributes & 0x20 != 0;
            let y_flip = tile_attributes & 0x40 != 0;
            let bg_priority = tile_attributes & 0x80 != 0;

            let inner_tile_row = if y_flip { 7 - (y % 8) } else { y % 8 } as u16;
            let inner_tile_col = if x_flip { 7 - (x % 8) } else { x % 8 } as u16;
            let inner_row_address =
                VRAM_SIZE as u16 * tile_bank + tile_data_address + inner_tile_row * 2;

            let color_index = self.calculate_pixel_color_index(inner_row_address, inner_tile_col);
            let (r, g, b, a) = if matches!(self.model_type, ModelType::Color) {
                let base = palette_num * 8 + color_index as usize * 2;
                let low = self.bg_palette_data[base];
                let high = self.bg_palette_data[base + 1];

                self.calculate_cgb_color(low, high)
            } else {
                self.calculate_dmg_color(self.dmg_bgp, color_index)
            };
            let frame_buffer_index = (self.ly as usize * 160 + i as usize) * 4;

            self.frame_buffer[frame_buffer_index] = r;
            self.frame_buffer[frame_buffer_index + 1] = g;
            self.frame_buffer[frame_buffer_index + 2] = b;
            self.frame_buffer[frame_buffer_index + 3] = a;
            self.frame_buffer_pixel_priorities[self.ly as usize * 160 + i as usize] =
                if self.lcdc & 0x01 == 0 {
                    PixelPriority::Sprite
                } else if bg_priority && color_index > 0 {
                    PixelPriority::BG
                } else if color_index > 0 {
                    PixelPriority::BGIfSpriteDisabled
                } else {
                    PixelPriority::Sprite
                };
        }

        if window_drawn {
            self.window_line += 1;
        }

        self.render_sprites();
    }

    fn render_sprites(&mut self) {
        if self.lcdc & 0x02 == 0 {
            return;
        }

        let sprite_height = if self.lcdc & 0x04 != 0 { 16 } else { 8 };

        for sprite in &self.sprites {
            let mut tile_row = self.ly.wrapping_sub(sprite.y) as u16;

            if sprite.attributes & 0x40 != 0 {
                tile_row = sprite_height - 1 - tile_row;
            }

            let tile_index = if sprite_height == 16 {
                if tile_row < 8 {
                    sprite.tile_index & !0x01
                } else {
                    tile_row -= 8;
                    sprite.tile_index | 0x01
                }
            } else {
                sprite.tile_index
            } as u16;

            let tile_bank =
                if matches!(self.model_type, ModelType::Color) && sprite.attributes & 0x08 != 0 {
                    1
                } else {
                    0
                };
            let tile_address = VRAM_SIZE as u16 * tile_bank + tile_index * 16 + tile_row * 2;
            let byte0 = self.vram[tile_address as usize];
            let byte1 = self.vram[tile_address as usize + 1];

            for bit in 0..8u8 {
                let screen_x = sprite.x.wrapping_add(bit);

                if screen_x >= 160 {
                    continue;
                }

                let flipped_bit = if sprite.attributes & 0x20 != 0 {
                    bit
                } else {
                    7 - bit
                };

                let low = (byte0 >> flipped_bit) & 0x01;
                let high = (byte1 >> flipped_bit) & 0x01;
                let color_index = (high << 1) | low;

                if color_index == 0 {
                    continue;
                }

                let pixel_priority =
                    self.frame_buffer_pixel_priorities[self.ly as usize * 160 + screen_x as usize];
                if matches!(pixel_priority, PixelPriority::BG) {
                    continue;
                }
                if matches!(pixel_priority, PixelPriority::BGIfSpriteDisabled)
                    && sprite.attributes & 0x80 != 0
                {
                    continue;
                }

                let (r, g, b, a) = if matches!(self.model_type, ModelType::Color) {
                    let palette_num = (sprite.attributes & 0x07) as usize;
                    let base = palette_num * 8 + color_index as usize * 2;
                    let low = self.obj_palette_data[base];
                    let high = self.obj_palette_data[base + 1];

                    self.calculate_cgb_color(low, high)
                } else {
                    let palette = if sprite.attributes & 0x10 != 0 {
                        self.obp1
                    } else {
                        self.obp0
                    };

                    self.calculate_dmg_color(palette, color_index)
                };

                let i = (self.ly as usize * 160 + screen_x as usize) * 4;
                self.frame_buffer[i] = r;
                self.frame_buffer[i + 1] = g;
                self.frame_buffer[i + 2] = b;
                self.frame_buffer[i + 3] = a;
            }
        }
    }

    fn oam_scan(&mut self) {
        self.sprites.clear();
        let sprite_height = if self.lcdc & 0x04 != 0 { 16 } else { 8 };

        for i in 0..40 {
            if self.sprites.len() == 10 {
                break;
            }

            let base = i * 4;
            let sprite_y = self.oam[base].wrapping_sub(16);
            let sprite_x = self.oam[base + 1].wrapping_sub(8);
            let tile_index = self.oam[base + 2];
            let attributes = self.oam[base + 3];

            let diff = self.ly.wrapping_sub(sprite_y);
            if diff < sprite_height {
                self.sprites.push(Sprite {
                    y: sprite_y,
                    x: sprite_x,
                    tile_index,
                    attributes,
                    priority: i as u8,
                });
            }
        }

        let sort_only_by_priority =
            matches!(self.model_type, ModelType::Color) && self.object_priority_mode & 0x01 == 0;

        self.sprites.sort_by(|a, b| {
            if a.x == b.x || sort_only_by_priority {
                b.priority.cmp(&a.priority)
            } else {
                b.x.cmp(&a.x)
            }
        });
    }

    fn calculate_pixel_color_index(&self, row_address: u16, x: u16) -> u8 {
        let byte0 = self.vram[row_address as usize];
        let byte1 = self.vram[row_address as usize + 1];
        let low_bit = (byte0 >> (7 - x)) & 0x01;
        let high_bit = (byte1 >> (7 - x)) & 0x01;

        (high_bit << 1) | low_bit
    }

    fn calculate_dmg_color(&self, palette: u8, color_index: u8) -> (u8, u8, u8, u8) {
        let shade = (palette >> (color_index * 2)) & 0x03;

        match shade {
            0 => (0x9B, 0xBC, 0x0F, 0xFF),
            1 => (0x8B, 0xAC, 0x0F, 0xFF),
            2 => (0x30, 0x62, 0x30, 0xFF),
            3 => (0x0F, 0x38, 0x0F, 0xFF),
            _ => unreachable!(),
        }
    }

    fn calculate_cgb_color(&self, low: u8, high: u8) -> (u8, u8, u8, u8) {
        let r5 = (low & 0x1F) as u16;
        let g5 = (((low >> 5) | (high << 3)) & 0x1F) as u16;
        let b5 = ((high >> 2) & 0x1F) as u16;

        let r8 = ((r5 << 3) | (r5 >> 2)) as u8;
        let g8 = ((g5 << 3) | (g5 >> 2)) as u8;
        let b8 = ((b5 << 3) | (b5 >> 2)) as u8;

        (r8, g8, b8, 255)
    }

    fn update_stat_state(&mut self, if_flag: &mut u8) {
        let previous_value = self.stat_on();

        if self.ly == self.lyc {
            self.stat |= 0x04;
        } else {
            self.stat &= !0x04;
        }

        self.stat = (self.stat & !0x03) | self.mode;

        let new_value = self.stat_on();

        if !previous_value && new_value && self.lcdc & 0x80 != 0 {
            *if_flag |= 0x02;
        }
    }

    fn update_stat_triggers(
        &mut self,
        if_flag: &mut u8,
        ly_trigger: bool,
        mode2_tigger: bool,
        mode1_trigger: bool,
        mode0_trigger: bool,
    ) {
        let previous_value = self.stat_on();

        self.stat = if ly_trigger {
            self.stat | 0x40
        } else {
            self.stat & !0x40
        };
        self.stat = if mode2_tigger {
            self.stat | 0x20
        } else {
            self.stat & !0x20
        };
        self.stat = if mode1_trigger {
            self.stat | 0x10
        } else {
            self.stat & !0x10
        };
        self.stat = if mode0_trigger {
            self.stat | 0x08
        } else {
            self.stat & !0x08
        };

        let new_value = self.stat_on();

        if !previous_value && new_value && self.lcdc & 0x80 != 0 {
            *if_flag |= 0x02;
        }
    }

    fn stat_on(&self) -> bool {
        if self.stat & 0x40 != 0 && self.stat & 0x04 != 0 {
            return true;
        }
        if self.stat & 0x20 != 0 && self.stat & 0x03 == 2 {
            return true;
        }
        if self.stat & 0x10 != 0 && self.stat & 0x03 == 1 {
            return true;
        }
        if self.stat & 0x08 != 0 && self.stat & 0x03 == 0 {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dmg() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0;
        ppu.stat = 0;
        assert_eq!(ppu.vram_bank, 0);
        assert_eq!(ppu.object_priority_mode, 0);
        assert_eq!(ppu.model_type, ModelType::DMG);
        // Check arrays are zeroed
        assert!(ppu.vram.iter().all(|&x| x == 0));
        assert!(ppu.oam.iter().all(|&x| x == 0));
        assert_eq!(ppu.lcdc, 0);
        assert!(ppu.bg_palette_data.iter().all(|&x| x == 0));
        assert!(ppu.obj_palette_data.iter().all(|&x| x == 0));
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
        ppu.mode = 1;
        ppu.oam[0x10] = 0xEF;
        assert_eq!(ppu.read(0xFE10), Some(0xEF));
    }

    #[test]
    fn test_write_oam() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.mode = 1;
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
    fn test_read_bg_palette_data() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.bg_palette_spec = 0x02;
        ppu.bg_palette_data[0x02] = 0x56;
        assert_eq!(ppu.read(0xFF69), Some(0x56));
    }

    #[test]
    fn test_read_obj_palette_data() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.obj_palette_spec = 0x02;
        ppu.obj_palette_data[0x02] = 0x56;
        assert_eq!(ppu.read(0xFF6B), Some(0x56));
    }

    #[test]
    fn test_write_bg_palette_data() {
        let mut ppu = PPU::new(ModelType::Color);
        let mut if_flag: u8 = 0;
        ppu.bg_palette_spec = 0x81;
        assert!(ppu.write(0xFF69, 0x56, &mut if_flag));
        assert_eq!(ppu.bg_palette_data[0x01], 0x56);
        assert_eq!(ppu.bg_palette_spec, 0x82);
    }

    #[test]
    fn test_write_obj_palette_data() {
        let mut ppu = PPU::new(ModelType::Color);
        let mut if_flag: u8 = 0;
        ppu.obj_palette_spec = 0x81;
        assert!(ppu.write(0xFF6B, 0x56, &mut if_flag));
        assert_eq!(ppu.obj_palette_data[0x01], 0x56);
        assert_eq!(ppu.obj_palette_spec, 0x82);
    }

    #[test]
    fn test_write_bg_palette_data_auto_increment_wraps() {
        let mut ppu = PPU::new(ModelType::Color);
        let mut if_flag: u8 = 0;
        ppu.bg_palette_spec = 0x80 | 0x3F;
        assert!(ppu.write(0xFF69, 0xAA, &mut if_flag));
        assert_eq!(ppu.bg_palette_data[0x3F], 0xAA);
        assert_eq!(ppu.bg_palette_spec, 0x80);
    }

    #[test]
    fn test_render_scanline_cgb_bg_palette_section_uses_palette_number() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.lcdc = 0x80 | 0x01 | 0x10;
        ppu.ly = 0;
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.vram[0x1800] = 0;
        ppu.vram[0] = 0b1000_0000;
        ppu.vram[1] = 0x00;
        ppu.vram[VRAM_SIZE + 0x1800] = 0x02;

        let palette_num = 2;
        let color_index = 1;
        let base = palette_num * 8 + color_index * 2;
        ppu.bg_palette_data[base] = 0x1F;
        ppu.bg_palette_data[base + 1] = 0x00;

        ppu.render_scanline();

        let expected = ppu.calculate_cgb_color(0x1F, 0x00);
        let color_offset = 0;
        assert_eq!(ppu.frame_buffer[color_offset], expected.0);
        assert_eq!(ppu.frame_buffer[color_offset + 1], expected.1);
        assert_eq!(ppu.frame_buffer[color_offset + 2], expected.2);
        assert_eq!(ppu.frame_buffer[color_offset + 3], expected.3);
        assert_eq!(
            ppu.frame_buffer_pixel_priorities[0],
            PixelPriority::BGIfSpriteDisabled
        );
    }

    #[test]
    fn test_render_scanline_cgb_bg_priority_flag_sets_bg_priority() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.lcdc = 0x80 | 0x01 | 0x10;
        ppu.ly = 0;
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.vram[0x1800] = 0;
        ppu.vram[0] = 0b1000_0000;
        ppu.vram[1] = 0x00;
        ppu.vram[VRAM_SIZE + 0x1800] = 0x82;

        let palette_num = 2;
        let color_index = 1;
        let base = palette_num * 8 + color_index * 2;
        ppu.bg_palette_data[base] = 0x1F;
        ppu.bg_palette_data[base + 1] = 0x00;

        ppu.render_scanline();

        assert_eq!(ppu.frame_buffer_pixel_priorities[0], PixelPriority::BG);
    }

    #[test]
    fn test_oam_scan_color_sort_by_priority_when_object_priority_mode_zero() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.ly = 8;
        ppu.object_priority_mode = 0;

        ppu.oam[0] = 24;
        ppu.oam[1] = 18;
        ppu.oam[2] = 0;
        ppu.oam[3] = 0;

        ppu.oam[4] = 24;
        ppu.oam[5] = 18;
        ppu.oam[6] = 0;
        ppu.oam[7] = 0;

        ppu.oam_scan();

        assert_eq!(ppu.sprites.len(), 2);
        assert_eq!(ppu.sprites[0].priority, 1);
        assert_eq!(ppu.sprites[1].priority, 0);
    }

    #[test]
    fn test_oam_scan_color_sort_by_x_when_object_priority_mode_one() {
        let mut ppu = PPU::new(ModelType::Color);
        ppu.ly = 8;
        ppu.object_priority_mode = 1;

        ppu.oam[0] = 24;
        ppu.oam[1] = 18;
        ppu.oam[2] = 0;
        ppu.oam[3] = 0;

        ppu.oam[4] = 24;
        ppu.oam[5] = 26;
        ppu.oam[6] = 0;
        ppu.oam[7] = 0;

        ppu.oam_scan();

        assert_eq!(ppu.sprites.len(), 2);
        assert_eq!(ppu.sprites[0].x, 18);
        assert_eq!(ppu.sprites[1].x, 10);
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

    #[test]
    fn test_calculate_dmg_color_shade_white() {
        let mut ppu = PPU::new(ModelType::DMG);
        // Set palette to map color_index 0 to shade 0
        ppu.dmg_bgp = 0b00_00_00_00;
        let (r, g, b, a) = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        assert_eq!((r, g, b, a), (0x9B, 0xBC, 0x0F, 0xFF));
    }

    #[test]
    fn test_calculate_dmg_color_shade_light_gray() {
        let mut ppu = PPU::new(ModelType::DMG);
        // Set palette to map color_index 0 to shade 1
        ppu.dmg_bgp = 0b00_00_00_01;
        let (r, g, b, a) = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        assert_eq!((r, g, b, a), (0x8B, 0xAC, 0x0F, 0xFF));
    }

    #[test]
    fn test_calculate_dmg_color_shade_dark_gray() {
        let mut ppu = PPU::new(ModelType::DMG);
        // Set palette to map color_index 0 to shade 2
        ppu.dmg_bgp = 0b00_00_00_10;
        let (r, g, b, a) = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        assert_eq!((r, g, b, a), (0x30, 0x62, 0x30, 0xFF));
    }

    #[test]
    fn test_calculate_dmg_color_shade_black() {
        let mut ppu = PPU::new(ModelType::DMG);
        // Set palette to map color_index 0 to shade 3
        ppu.dmg_bgp = 0b00_00_00_11;
        let (r, g, b, a) = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        assert_eq!((r, g, b, a), (0x0F, 0x38, 0x0F, 0xFF));
    }

    #[test]
    fn test_calculate_dmg_color_with_custom_palette() {
        let mut ppu = PPU::new(ModelType::DMG);
        // Palette: 0b11_10_01_00
        // color_index 0 -> bits [1:0] = 00 -> shade 0
        // color_index 1 -> bits [3:2] = 01 -> shade 1
        // color_index 2 -> bits [5:4] = 10 -> shade 2
        // color_index 3 -> bits [7:6] = 11 -> shade 3
        ppu.dmg_bgp = 0b11_10_01_00;

        let (r, g, b, a) = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        assert_eq!((r, g, b, a), (0x9B, 0xBC, 0x0F, 0xFF));

        let (r, g, b, a) = ppu.calculate_dmg_color(ppu.dmg_bgp, 1);
        assert_eq!((r, g, b, a), (0x8B, 0xAC, 0x0F, 0xFF));

        let (r, g, b, a) = ppu.calculate_dmg_color(ppu.dmg_bgp, 2);
        assert_eq!((r, g, b, a), (0x30, 0x62, 0x30, 0xFF));

        let (r, g, b, a) = ppu.calculate_dmg_color(ppu.dmg_bgp, 3);
        assert_eq!((r, g, b, a), (0x0F, 0x38, 0x0F, 0xFF));
    }

    #[test]
    fn test_calculate_pixel_color_index_single_bit() {
        let mut ppu = PPU::new(ModelType::DMG);
        // Set tile data at VRAM address 0
        // Byte 0 (low bits): 0b10101010
        // Byte 1 (high bits): 0b01010101
        ppu.vram[0] = 0b10101010;
        ppu.vram[1] = 0b01010101;

        // At x=0: low=1, high=0 -> color_index=1
        let color_index = ppu.calculate_pixel_color_index(0, 0);
        assert_eq!(color_index, 1);

        // At x=1: low=0, high=1 -> color_index=2
        let color_index = ppu.calculate_pixel_color_index(0, 1);
        assert_eq!(color_index, 2);

        // At x=2: low=1, high=0 -> color_index=1
        let color_index = ppu.calculate_pixel_color_index(0, 2);
        assert_eq!(color_index, 1);

        // At x=7: low=0, high=1 -> color_index=2
        let color_index = ppu.calculate_pixel_color_index(0, 7);
        assert_eq!(color_index, 2);
    }

    #[test]
    fn test_calculate_pixel_color_index_all_zeros() {
        let ppu = PPU::new(ModelType::DMG);
        // All bytes are 0
        for x in 0..8 {
            let color_index = ppu.calculate_pixel_color_index(0, x as u16);
            assert_eq!(color_index, 0);
        }
    }

    #[test]
    fn test_calculate_pixel_color_index_all_ones() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.vram[0] = 0xFF;
        ppu.vram[1] = 0xFF;
        // All pixels should be color_index=3 (high=1, low=1)
        for x in 0..8 {
            let color_index = ppu.calculate_pixel_color_index(0, x as u16);
            assert_eq!(color_index, 3);
        }
    }

    #[test]
    fn test_render_scanline_basic() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01; // LCD enabled, BG enabled
        ppu.dmg_bgp = 0x00; // All pixels use shade 0
        ppu.ly = 0;
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.render_scanline();

        let expected = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        for x in 0..160 {
            let offset = x * 4;
            assert_eq!(ppu.frame_buffer[offset], expected.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected.3);
        }
    }

    #[test]
    fn test_render_scanline_fills_frame_buffer() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01; // LCD enabled, BG enabled
        ppu.dmg_bgp = 0x00; // All pixels use shade 0
        ppu.ly = 0;
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.render_scanline();

        let expected = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        for x in 0..160 {
            let index = x * 4;
            assert_eq!(ppu.frame_buffer[index], expected.0);
            assert_eq!(ppu.frame_buffer[index + 1], expected.1);
            assert_eq!(ppu.frame_buffer[index + 2], expected.2);
            assert_eq!(ppu.frame_buffer[index + 3], expected.3);
        }
    }

    #[test]
    fn test_frame_buffer_size() {
        let ppu = PPU::new(ModelType::DMG);
        // Frame buffer should be 160*144*4 = 92160 bytes
        let expected_size = 160 * 144 * 4;
        assert_eq!(ppu.frame_buffer.len(), expected_size);
    }

    #[test]
    fn test_render_scanline_different_ly() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01;
        ppu.dmg_bgp = 0x00;
        ppu.ly = 50; // Different scanline
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.render_scanline();

        let expected = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        let scanline_offset = 50 * 160 * 4;
        for x in 0..160 {
            let index = scanline_offset + x * 4;
            assert_eq!(ppu.frame_buffer[index], expected.0);
            assert_eq!(ppu.frame_buffer[index + 1], expected.1);
            assert_eq!(ppu.frame_buffer[index + 2], expected.2);
            assert_eq!(ppu.frame_buffer[index + 3], expected.3);
        }
    }

    #[test]
    fn test_render_scanline_with_scrolling() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01; // LCD enabled, BG enabled
        ppu.dmg_bgp = 0x00; // All pixels use shade 0 when background is empty
        ppu.ly = 0;
        ppu.scx = 8; // Scroll 8 pixels right
        ppu.scy = 0;

        // Create a tile with alternating pattern at tile 0
        ppu.vram[0] = 0b10101010; // Low bits
        ppu.vram[1] = 0b01010101; // High bits

        ppu.render_scanline();

        let expected_empty = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        for x in 0..8 {
            let offset = x * 4;
            assert_eq!(ppu.frame_buffer[offset], expected_empty.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected_empty.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected_empty.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected_empty.3);
        }
    }

    #[test]
    fn test_render_scanline_with_window() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01 | 0x20; // LCD enabled, BG enabled, window enabled
        ppu.dmg_bgp = 0b11_11_11_11; // All palette entries map to shade 3
        ppu.ly = 0;
        ppu.wy = 0;
        ppu.wx = 10;

        ppu.render_scanline();

        let expected_black = ppu.calculate_dmg_color(ppu.dmg_bgp, 3);
        for x in 0..160 {
            let offset = x * 4;
            assert_eq!(ppu.frame_buffer[offset], expected_black.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected_black.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected_black.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected_black.3);
        }
    }

    #[test]
    fn test_render_scanline_window_not_visible() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01 | 0x20; // LCD enabled, BG enabled, window enabled
        ppu.dmg_bgp = 0x00; // All pixels use shade 0
        ppu.ly = 0;
        ppu.wy = 10; // Window starts at line 10, so not visible on line 0
        ppu.wx = 10;

        ppu.render_scanline();

        let expected = ppu.calculate_dmg_color(ppu.dmg_bgp, 0);
        for x in 0..160 {
            let offset = x * 4;
            assert_eq!(ppu.frame_buffer[offset], expected.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected.3);
        }
    }

    #[test]
    fn test_render_scanline_8800_tile_addressing() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01; // LCD enabled, BG enabled, 8800 addressing mode
        ppu.dmg_bgp = 0b11_11_11_11; // All palette entries map to shade 3
        ppu.ly = 0;
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.vram[0x1800] = 0x00; // Tile index in tilemap
        ppu.vram[0x1000] = 0xFF; // Low byte all 1s
        ppu.vram[0x1001] = 0xFF; // High byte all 1s

        ppu.render_scanline();

        let expected_black = ppu.calculate_dmg_color(ppu.dmg_bgp, 3);
        for x in 0..8 {
            let offset = x * 4;
            assert_eq!(ppu.frame_buffer[offset], expected_black.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected_black.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected_black.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected_black.3);
        }
    }

    #[test]
    fn test_render_scanline_8000_tile_addressing() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01 | 0x10; // LCD enabled, BG enabled, 8000 addressing mode
        ppu.dmg_bgp = 0b11_11_11_11; // All palette entries map to shade 3
        ppu.ly = 0;
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.vram[0x1800] = 0x00; // Tile index 0
        ppu.vram[0] = 0xFF; // Low byte all 1s
        ppu.vram[1] = 0xFF; // High byte all 1s

        ppu.render_scanline();

        let expected_black = ppu.calculate_dmg_color(ppu.dmg_bgp, 3);
        for x in 0..8 {
            let offset = x * 4;
            assert_eq!(ppu.frame_buffer[offset], expected_black.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected_black.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected_black.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected_black.3);
        }
    }

    #[test]
    fn test_render_scanline_alternate_tilemap() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01 | 0x08 | 0x10; // LCD enabled, BG enabled, alternate tilemap, 8000 addressing
        ppu.dmg_bgp = 0b11_11_11_11; // All palette entries map to shade 3
        ppu.ly = 0;
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.vram[0x1C00] = 0x00; // Tile index 0
        ppu.vram[0] = 0xFF; // Low byte all 1s
        ppu.vram[1] = 0xFF; // High byte all 1s

        ppu.render_scanline();

        let expected_black = ppu.calculate_dmg_color(ppu.dmg_bgp, 3);
        for x in 0..8 {
            let offset = x * 4;
            assert_eq!(ppu.frame_buffer[offset], expected_black.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected_black.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected_black.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected_black.3);
        }
    }

    #[test]
    fn test_render_scanline_bg_disabled() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80; // LCD enabled, BG disabled
        ppu.ly = 0;

        ppu.render_scanline();

        let expected = (0x9B, 0xBC, 0x0F, 0xFF);
        for x in 0..160 {
            let offset = x * 4;
            assert_eq!(ppu.frame_buffer[offset], expected.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected.3);
        }
    }

    #[test]
    fn test_window_line_increment() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01 | 0x20; // LCD enabled, BG enabled, window enabled
        ppu.dmg_bgp = 0x00;
        ppu.ly = 0;
        ppu.wy = 0;
        ppu.wx = 0; // Window visible from pixel 0

        assert_eq!(ppu.window_line, 0);
        ppu.render_scanline();
        assert_eq!(ppu.window_line, 1); // Window was drawn, line should increment

        ppu.ly = 1;
        ppu.render_scanline();
        assert_eq!(ppu.window_line, 2);
    }

    #[test]
    fn test_window_line_no_increment_when_not_drawn() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01 | 0x20; // LCD enabled, BG enabled, window enabled
        ppu.dmg_bgp = 0x00;
        ppu.ly = 0;
        ppu.wy = 10; // Window not visible on line 0
        ppu.wx = 0;

        assert_eq!(ppu.window_line, 0);
        ppu.render_scanline();
        assert_eq!(ppu.window_line, 0); // Window not drawn, line should not increment
    }

    #[test]
    fn test_render_scanline_complex_pattern() {
        let mut ppu = PPU::new(ModelType::DMG);
        ppu.lcdc = 0x80 | 0x01 | 0x10; // LCD enabled, BG enabled, 8000 addressing
        ppu.dmg_bgp = 0b11100100;
        ppu.ly = 0;
        ppu.scx = 0;
        ppu.scy = 0;

        ppu.vram[0] = 0b10101010; // Low bits
        ppu.vram[1] = 0b01010101; // High bits

        ppu.render_scanline();

        let expected_shade1 = ppu.calculate_dmg_color(ppu.dmg_bgp, 1);
        let expected_shade2 = ppu.calculate_dmg_color(ppu.dmg_bgp, 2);
        let expected_colors = [
            expected_shade1,
            expected_shade2,
            expected_shade1,
            expected_shade2,
            expected_shade1,
            expected_shade2,
            expected_shade1,
            expected_shade2,
        ];

        for x in 0..8 {
            let offset = x * 4;
            let expected = expected_colors[x];
            assert_eq!(ppu.frame_buffer[offset], expected.0);
            assert_eq!(ppu.frame_buffer[offset + 1], expected.1);
            assert_eq!(ppu.frame_buffer[offset + 2], expected.2);
            assert_eq!(ppu.frame_buffer[offset + 3], expected.3);
        }
    }
}
