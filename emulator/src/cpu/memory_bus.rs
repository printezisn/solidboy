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

pub struct MemoryBus {
    mbc: MBC,
    ppu: PPU,
    audio: Audio,
    timer: Timer,

    wram: [u8; WRAM_SIZE * (WRAM_TOTAL_BANKS + 1)],
    wram_bank: u8,
    high_ram: [u8; HIGH_RAM_SIZE],

    joypad_input: u8,
    serial_transfer: [u8; SERIAL_TRANSFER_SIZE],
    if_flag: u8,
    ie_flag: u8,
    key0: u8,
    key1: u8,
    boot_rom_mapping_control: u8,
    ir_port: u8,

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

            wram: [0; WRAM_SIZE * (WRAM_TOTAL_BANKS + 1)],
            wram_bank: 1,
            high_ram: [0; HIGH_RAM_SIZE],

            joypad_input: 0,
            serial_transfer: [0; SERIAL_TRANSFER_SIZE],
            if_flag: 0,
            ie_flag: 0,
            key0: 0,
            key1: 0,
            boot_rom_mapping_control: 0,
            ir_port: 0,

            total_cycles: 0,
            model_type,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        let address = match address {
            0xE000..=0xFDFF => address - 0x2000,
            _ => address,
        };

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
                self.joypad_input = value;
            }
            0xFF01..=0xFF02 => {
                self.serial_transfer[address as usize - 0xFF01] = value;
                if address == 0xFF02 && value == 0x81 {
                    console_log!("{}", self.serial_transfer[0] as char);
                }
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

    pub fn read_without_tick(&self, address: u16) -> u8 {
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
            0xFF00 => self.joypad_input,
            0xFF01..=0xFF02 => self.serial_transfer[(address - 0xFF01) as usize],
            0xFF04 => self.timer.div(),
            0xFF05 => self.timer.tima(),
            0xFF06 => self.timer.tma(),
            0xFF07 => self.timer.tac(),
            0xFF0F => self.if_flag,
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

    pub fn set_ie_flag(&mut self, value: u8) {
        self.ie_flag = value;
    }

    pub fn key1(&self) -> u8 {
        self.key1
    }

    pub fn set_key1(&mut self, value: u8) {
        self.key1 = value;
    }

    pub fn reset_total_cycles(&mut self) {
        self.total_cycles = 0;
    }

    pub fn total_cycles(&self) -> u8 {
        self.total_cycles
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
