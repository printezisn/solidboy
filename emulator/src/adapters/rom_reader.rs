pub enum RomReader {
  Memory { rom: Vec<u8> },
  File { file_path: String }
}

impl RomReader {
  pub fn read_rom(&self) -> Vec<u8> {
    match self {
      RomReader::Memory { rom } => rom.clone(),
      RomReader::File { file_path } => std::fs::read(file_path).unwrap()
    }
  }
}