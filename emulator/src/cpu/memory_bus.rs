mod audio;
mod mbc;
mod ppu;
mod timer;
pub mod types;

use audio::Audio;
use mbc::MBC;
use ppu::PPU;
use timer::Timer;
use types::ModelType;

const WRAM_TOTAL_BANKS: usize = 7;
const WRAM_SIZE: usize = 0xCFFF - 0xC000 + 1;

const HIGH_RAM_SIZE: usize = 0xFFFE - 0xFF80 + 1;

const SERIAL_TRANSFER_SIZE: usize = 0xFF02 - 0xFF01 + 1;

struct DMA {
    pub active: bool,
    pub init_delay: u8,
    pub pause: u8,
    pub byte: u8,
    pub source: u16,
}

pub struct MemoryBus {
    mbc: MBC,
    ppu: PPU,
    audio: Audio,
    timer: Timer,
    dma: DMA,

    wram: [u8; WRAM_SIZE * (WRAM_TOTAL_BANKS + 1)],
    wram_bank: u8,
    high_ram: [u8; HIGH_RAM_SIZE],

    joypad_selection: u8,
    joypad_pressed_directions: u8,
    joypad_pressed_buttons: u8,

    serial_transfer: [u8; SERIAL_TRANSFER_SIZE],

    if_flag: u8,
    ie_flag: u8,
    key0: u8,
    key1: u8,
    boot_rom_mapping_control: u8,
    ir_port: u8,
    oam_dma_transfer: u8,

    total_cycles: u8,
    model_type: ModelType,
}

impl MemoryBus {
    pub fn new(rom: Vec<u8>) -> Self {
        let model_type = match rom[0x0143] {
            0xC0 => ModelType::Color,
            _ => ModelType::DMG,
        };

        MemoryBus {
            mbc: MBC::new(rom),
            ppu: PPU::new(model_type.clone()),
            audio: Audio::new(),
            timer: Timer::new(),
            dma: DMA {
                active: false,
                init_delay: 0,
                pause: 0,
                byte: 0,
                source: 0,
            },

            wram: [0; WRAM_SIZE * (WRAM_TOTAL_BANKS + 1)],
            wram_bank: 1,
            high_ram: [0; HIGH_RAM_SIZE],

            joypad_selection: 0,
            joypad_pressed_directions: 0x0F,
            joypad_pressed_buttons: 0x0F,
            serial_transfer: [0; SERIAL_TRANSFER_SIZE],
            if_flag: 0xE1,
            ie_flag: 0,
            key0: 0,
            key1: 0,
            boot_rom_mapping_control: 0,
            ir_port: 0,
            oam_dma_transfer: 0,

            total_cycles: 0,
            model_type,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        let address = match address {
            0xE000..=0xFDFF => address - 0x2000,
            _ => address,
        };

        if self.dma.active && (address < 0xFF80 || address > 0xFFFE) {
            console_error!(
                "Invalid write to address {:04X} during OAM DMA Transfer\n",
                address
            );
        }

        if self.mbc.write(address, value) {
            self.tick(4);
            return;
        }

        if self.ppu.write(address, value, &mut self.if_flag) {
            self.tick(4);
            return;
        }

        if self.audio.write(address, value) {
            self.tick(4);
            return;
        }

        match address {
            0xC000..=0xCFFF => {
                self.wram[(address - 0xC000) as usize] = value;
            }
            0xD000..=0xDFFF => {
                let bank: usize = if matches!(self.model_type, ModelType::Color) {
                    self.wram_bank as usize
                } else {
                    1
                };
                self.wram[bank * WRAM_SIZE + address as usize - 0xD000] = value;
            }
            0xFEA0..=0xFEFF => {}
            0xFF00 => {
                self.joypad_selection = (value & 0x30) >> 4;
            }
            0xFF01..=0xFF02 => {
                self.serial_transfer[address as usize - 0xFF01] = value;
            }
            0xFF04 => {
                self.timer.reset_div();
            }
            0xFF05 => {
                self.timer.set_tima(value);
            }
            0xFF06 => {
                self.timer.set_tma(value);
            }
            0xFF07 => {
                self.timer.set_tac(value);
            }
            0xFF0F => {
                self.if_flag = value;
            }
            0xFF46 => {
                self.oam_dma_transfer = value;
                self.dma = DMA {
                    active: false,
                    init_delay: 8,
                    pause: 0,
                    byte: 0,
                    source: (value as u16) << 8,
                }
            }
            0xFF4C => {
                self.key0 = value;
            }
            0xFF4D => {
                self.key1 = value;
            }
            0xFF50 => {
                self.boot_rom_mapping_control = value;
            }
            0xFF56 => {
                self.ir_port = value;
            }
            0xFF70 => {
                if matches!(self.model_type, ModelType::Color) {
                    self.wram_bank = value & 0x07;
                    if self.wram_bank == 0 {
                        self.wram_bank = 1;
                    }
                }
            }
            0xFF80..=0xFFFE => {
                self.high_ram[(address - 0xFF80) as usize] = value;
            }
            0xFFFF => {
                self.ie_flag = value;
            }
            _ => console_error!("Invalid write to address {:02X}", address),
        }

        self.tick(4);
    }

    fn read_without_tick(&self, address: u16) -> u8 {
        let address = match address {
            0xE000..=0xFDFF => address - 0x2000,
            _ => address,
        };

        match self.mbc.read(address) {
            Some(result) => {
                return result;
            }
            _ => {}
        };

        match self.ppu.read(address) {
            Some(result) => {
                return result;
            }
            _ => {}
        }

        match self.audio.read(address) {
            Some(result) => {
                return result;
            }
            _ => {}
        }

        match address {
            0xC000..=0xCFFF => self.wram[(address - 0xC000) as usize],
            0xD000..=0xDFFF => {
                let bank: usize = if matches!(self.model_type, ModelType::Color) {
                    self.wram_bank as usize
                } else {
                    1
                };
                self.wram[bank * WRAM_SIZE + address as usize - 0xD000]
            }
            0xFEA0..=0xFEFF => 0x00,
            0xFF00 => {
                let mut value = 0xC0 | (self.joypad_selection << 4);
                if self.joypad_selection == 0x01 {
                    value |= self.joypad_pressed_buttons;
                } else if self.joypad_selection == 0x02 {
                    value |= self.joypad_pressed_directions;
                } else {
                    value |= 0x0F;
                }

                value
            }
            0xFF01..=0xFF02 => self.serial_transfer[(address - 0xFF01) as usize],
            0xFF04 => self.timer.div(),
            0xFF05 => self.timer.tima(),
            0xFF06 => self.timer.tma(),
            0xFF07 => self.timer.tac(),
            0xFF0F => self.if_flag,
            0xFF46 => self.oam_dma_transfer,
            0xFF4C => self.key0,
            0xFF4D => self.key1,
            0xFF50 => self.boot_rom_mapping_control,
            0xFF56 => self.ir_port,
            0xFF70 => {
                if matches!(self.model_type, ModelType::Color) {
                    return self.wram_bank;
                }

                return 0xFF;
            }
            0xFF80..=0xFFFE => self.high_ram[(address - 0xFF80) as usize],
            0xFFFF => self.ie_flag,
            _ => console_error!("Invalid read from address {:02X}", address),
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        let result = self.read_without_tick(address);

        if self.dma.active && (address < 0xFF80 || address > 0xFFFE) {
            console_error!(
                "Invalid read from address {:04X} during OAM DMA Transfer\n",
                address
            );
        }

        self.tick(4);
        result
    }

    pub fn model_type(&self) -> ModelType {
        self.model_type
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

    #[allow(dead_code)]
    pub fn set_ie_flag(&mut self, value: u8) {
        self.ie_flag = value;
    }

    pub fn key1(&self) -> u8 {
        self.key1
    }

    pub fn set_key1(&mut self, value: u8) {
        self.key1 = value;
    }

    pub fn set_joypad_pressed_directions(&mut self, joypad_pressed_directions: u8) {
        self.joypad_pressed_directions = joypad_pressed_directions;
    }

    pub fn set_joypad_pressed_buttons(&mut self, joypad_pressed_buttons: u8) {
        self.joypad_pressed_buttons = joypad_pressed_buttons;
    }

    pub fn reset_total_cycles(&mut self) {
        self.total_cycles = 0;
    }

    pub fn total_cycles(&self) -> u8 {
        self.total_cycles
    }

    fn tick_dma(&mut self, cycles: u8) {
        for _ in 0..cycles {
            if !self.dma.active && self.dma.init_delay == 0 {
                break;
            }

            if self.dma.init_delay > 0 {
                self.dma.init_delay -= 1;
                if self.dma.init_delay == 0 {
                    self.dma.active = true;
                } else {
                    continue;
                }
            }

            if self.dma.pause > 0 {
                self.dma.pause -= 1;
                if self.dma.pause > 0 {
                    continue;
                }
            }

            let source = self.dma.source + self.dma.byte as u16;
            let destination = 0xFE00 + self.dma.byte as u16;
            let value = self.read_without_tick(source);
            self.ppu.write(destination, value, &mut self.if_flag);

            self.dma.byte += 1;
            if self.dma.byte == 160 {
                self.dma.active = false;
            } else {
                self.dma.pause = 4;
            }
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        self.total_cycles += cycles;
        self.timer.tick(&mut self.if_flag, cycles);

        let real_speed: u8 =
            if matches!(self.model_type(), ModelType::Color) && (self.key1() & 0x80) != 0 {
                cycles / 2
            } else {
                cycles
            };

        self.ppu.tick(&mut self.if_flag, real_speed);
        self.mbc.tick(real_speed as u32);
        self.tick_dma(cycles);
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

    #[test]
    fn oam_dma_transfer_starts_correctly() {
        let rom = make_rom(0x00, 0x00);
        let mut bus = MemoryBus::new(rom);

        // Write to DMA register to start transfer from 0x8000
        bus.write(0xFF46, 0x80);

        assert_eq!(bus.dma.active, false);
        bus.tick(4);

        assert!(bus.dma.active);
        assert_eq!(bus.dma.source, 0x8000);
        assert_eq!(bus.dma.byte, 1); // After 4 ticks, 1 byte transferred
    }

    #[test]
    fn oam_dma_transfer_copies_data() {
        let rom = make_rom(0x00, 0x00);
        let mut bus = MemoryBus::new(rom);
        bus.ppu.write(0xFF40, 0, &mut bus.if_flag);

        bus.tick(252);
        bus.reset_total_cycles();

        // Set up source data in WRAM
        for i in 0..160 {
            bus.write(0xC000 + i, i as u8);
            bus.reset_total_cycles();
        }

        // Start DMA from 0xC000
        bus.write(0xFF46, 0xC0);
        bus.reset_total_cycles();

        // Tick enough cycles to complete DMA (160 bytes * 4 cycles each)
        for _ in 0..160 {
            bus.tick(4);
            bus.reset_total_cycles();
        }

        // Verify data was copied to OAM
        for i in 0..160 {
            assert_eq!(bus.read(0xFE00 + i), i as u8);
            bus.reset_total_cycles();
        }

        // DMA should be inactive after completion
        assert!(!bus.dma.active);
    }

    #[test]
    fn oam_dma_transfer_from_vram() {
        let rom = make_rom(0x00, 0x00);
        let mut bus = MemoryBus::new(rom);
        bus.ppu.write(0xFF40, 0, &mut bus.if_flag);

        // Set up source data in VRAM
        for i in 0..160 {
            bus.write(0x8000 + i, (i + 100) as u8);
            bus.reset_total_cycles();
        }

        // Start DMA from 0x8000
        bus.write(0xFF46, 0x80);
        bus.reset_total_cycles();

        // Tick enough cycles to complete DMA
        for _ in 0..160 {
            bus.tick(4);
            bus.reset_total_cycles();
        }

        // Verify data was copied to OAM
        for i in 0..160 {
            assert_eq!(bus.read(0xFE00 + i), (i + 100) as u8);
            bus.reset_total_cycles();
        }
    }

    #[test]
    fn oam_dma_transfer_partial_progress() {
        let rom = make_rom(0x00, 0x00);
        let mut bus = MemoryBus::new(rom);
        bus.ppu.write(0xFF40, 0, &mut bus.if_flag);

        // Set up source data
        for i in 0..160 {
            bus.write(0xC000 + i, i as u8);
            bus.reset_total_cycles();
        }

        // Start DMA
        bus.write(0xFF46, 0xC0);
        bus.reset_total_cycles();

        // Tick for 44 bytes (176 cycles)
        bus.tick(176);
        bus.reset_total_cycles();

        // DMA should still be active
        assert!(bus.dma.active);
        assert_eq!(bus.dma.byte, 44);

        // Tick remaining cycles (116 bytes)
        for _ in 0..116 {
            bus.tick(4);
            bus.reset_total_cycles();
        }

        // Check all bytes copied
        for i in 0..160 {
            assert_eq!(bus.read(0xFE00 + i), i as u8);
            bus.reset_total_cycles();
        }

        // DMA should be complete
        assert!(!bus.dma.active);
    }
}
