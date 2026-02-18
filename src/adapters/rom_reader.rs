use super::traits::RomReader;

struct FileRomReader {
  file_path: &'static str,
}

impl FileRomReader {
  fn new(file_path: &'static str) -> Self {
    Self { file_path }
  }
}

impl RomReader for FileRomReader {
  fn read_rom(&self) -> Vec<u8> {
    std::fs::read(&self.file_path).unwrap()
  }
}

pub fn new() -> impl RomReader {
  FileRomReader::new("test-roms/cpu_instrs.gb")
}