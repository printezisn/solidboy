pub enum RomReader {
  File { file_path: &'static str }
}

impl RomReader {
  pub fn read_rom(&self) -> Vec<u8> {
    match self {
      RomReader::File { file_path } => std::fs::read(file_path).unwrap()
    }
  }
}