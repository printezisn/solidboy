pub mod traits;
pub mod rom_reader;

use traits::RomReader;

pub struct Adapters {
  rom_reader: Box<dyn RomReader>
}

impl Adapters {
  pub fn new() -> Self {
    Self {
      rom_reader: Box::new(rom_reader::new())
    }
  }

  pub fn get_rom_reader(&self) -> &dyn RomReader {
    self.rom_reader.as_ref()
  }
}