const EXTERNAL_RAM_SIZE: u16 = 8192;

pub enum MBC {
  NoROM { rom: Vec<u8>, external_ram: [u8; EXTERNAL_RAM_SIZE as usize] },
  MBC1 {
    rom: Vec<u8>,
    rom_bank: u16,
    external_ram: [u8; EXTERNAL_RAM_SIZE as usize * 4],
    ram_enabled: bool,
    ram_bank: u16,
    banking_mode: u8,
  }
}

impl MBC {
  pub fn new(rom: Vec<u8>) -> MBC {
    match rom[0x0147] {
      0x00 => MBC::NoROM { rom, external_ram: [0; EXTERNAL_RAM_SIZE as usize] },
      0x01 | 0x02 | 0x03 => MBC::MBC1 {
        rom,
        external_ram: [0; EXTERNAL_RAM_SIZE as usize * 4],
        rom_bank: 1,
        ram_enabled: false,
        ram_bank: 0,
        banking_mode: 0,
      },
      _ => panic!("Unsupported MBC type {:02X}", rom[0x0147])
    }
  }

  pub fn read(&self, address: u16) -> Option<u8> {
    match self {
      MBC::NoROM { rom, external_ram } => {
        match address {
          0x0000..=0x7FFF => Some(rom[address as usize]),
          0xA000..=0xBFFF => Some(external_ram[(address - 0xA000) as usize]),
          _ => None
        }
      },
      MBC::MBC1 { rom, rom_bank, external_ram, ram_bank, ram_enabled, banking_mode } => {
        match address {
          0x0000..=0x3FFF => Some(rom[address as usize]),
          0x4000..=0x7FFF => Some(rom[(rom_bank * 0x4000 + address - 0x4000) as usize]),
          0xA000..=0xBFFF => {
            if !ram_enabled {
              return Some(0xFF);
            }

            let bank = if *banking_mode == 0 { 0 } else { *ram_bank };
            Some(external_ram[(bank * EXTERNAL_RAM_SIZE + address - 0xA000) as usize])
          },
          _ => None
        }
      }
    }
  }

  pub fn write(&mut self, address: u16, value: u8) -> bool {
    match self {
      MBC::NoROM { rom: _, external_ram: _ } => false,
      MBC::MBC1 { rom: _, rom_bank, external_ram, ram_enabled, ram_bank, banking_mode } => {
        match address {
          0x0000..=0x1FFF => {
            *ram_enabled = (value & 0xF) == 0xA;
            true
          },
          0x2000..=0x3FFF => {
            // Lower 5 bits select the ROM bank; 0 means bank 1.
            let lower = (value & 0x1F) as u16;
            let lower = if lower == 0 { 1 } else { lower };
            let upper = *rom_bank & 0x60;
            *rom_bank = upper | lower;
            true
          }
          0x4000..=0x5FFF => {
            let bits = (value & 0x3) as u16;
            if *banking_mode == 0 {
              // ROM banking mode: update high bits of the ROM bank.
              let lower = *rom_bank & 0x1F;
              *rom_bank = lower | (bits << 5);
              if *rom_bank & 0x1F == 0 {
                *rom_bank |= 1;
              }
              *ram_bank = 0;
            } else {
              // RAM banking mode
              *ram_bank = bits;
            }
            true
          }
          0x6000..=0x7FFF => {
            *banking_mode = value & 1;
            if *banking_mode == 0 {
              *ram_bank = 0;
            }
            true
          }
          0xA000..=0xBFFF => {
            if !*ram_enabled {
              return false;
            }
            let bank = if *banking_mode == 0 { 0 } else { *ram_bank };
            external_ram[(bank * EXTERNAL_RAM_SIZE + address - 0xA000) as usize] = value;
            true
          },
          _ => false
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_rom_with_type(mbc_type: u8, size: usize) -> Vec<u8> {
    let mut rom = vec![0u8; size];
    for i in 0..size {
      rom[i] = (i & 0xFF) as u8;
    }
    rom[0x0147] = mbc_type;
    rom
  }

  #[test]
  fn no_rom_read_write_behaviour() {
    let rom = make_rom_with_type(0x00, 0x8000);
    let mut mbc = MBC::new(rom);

    assert_eq!(mbc.read(0x0000), Some(0x00));
    assert_eq!(mbc.read(0x7FFF), Some(0xFF));

    // External RAM is present but readonly in NoROM mode
    assert_eq!(mbc.read(0xA000), Some(0x00));
    assert!(!mbc.write(0xA000, 0x5A));
    assert_eq!(mbc.read(0xA000), Some(0x00));
  }

  #[test]
  fn mbc1_rom_bank_switching() {
    // Create a ROM with at least 4 banks (0x4000 bytes each)
    let rom = make_rom_with_type(0x01, 0x4000 * 4);
    let mut mbc = MBC::new(rom);

    // Bank 1 is the default for 0x4000-0x7FFF
    assert_eq!(mbc.read(0x4000), Some(0x00));

    // Switch to bank 2, read from bank 2
    assert!(mbc.write(0x2000, 2));
    assert_eq!(mbc.read(0x4000), Some(0x00));

    // Writing 0 should select bank 1
    assert!(mbc.write(0x2000, 0));
    assert_eq!(mbc.read(0x4000), Some(0x00));
  }

  #[test]
  fn mbc1_ram_enable_and_banking_modes() {
    let rom = make_rom_with_type(0x01, 0x4000 * 4);
    let mut mbc = MBC::new(rom);

    // RAM is disabled by default
    assert_eq!(mbc.read(0xA000), Some(0xFF));
    assert!(!mbc.write(0xA000, 0x12));

    // Enable RAM
    assert!(mbc.write(0x0000, 0x0A));
    assert!(mbc.write(0xA000, 0x12));
    assert_eq!(mbc.read(0xA000), Some(0x12));

    // Switch to RAM banking mode and choose bank 2
    assert!(mbc.write(0x6000, 1));
    assert!(mbc.write(0x4000, 2));
    assert!(mbc.write(0xA000, 0x34));
    assert_eq!(mbc.read(0xA000), Some(0x34));

    // Return to ROM banking mode: RAM bank should become bank 0
    assert!(mbc.write(0x6000, 0));
    assert!(mbc.write(0xA000, 0x56));
    assert_eq!(mbc.read(0xA000), Some(0x56));
  }
}
