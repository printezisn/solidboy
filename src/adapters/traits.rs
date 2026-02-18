pub trait RomReader {
  fn read_rom(&self) -> Vec<u8>;
}