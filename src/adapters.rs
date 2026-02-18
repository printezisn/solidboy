pub mod rom_reader;

use rom_reader::RomReader;

pub struct Adapters {
  rom_reader: RomReader
}

impl Adapters {
  pub fn new(rom_reader: RomReader) -> Self {
    Self {
      rom_reader
    }
  }

  pub fn rom_reader(&self) -> &RomReader {
    &self.rom_reader
  }
}