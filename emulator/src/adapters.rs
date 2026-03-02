pub mod rom_reader;
pub mod serial_port;

use rom_reader::RomReader;
use serial_port::SerialPort;

pub struct Adapters {
  rom_reader: RomReader,
  serial_port: SerialPort
}

impl Adapters {
  pub fn new(rom_reader: RomReader, serial_port: SerialPort) -> Self {
    Self {
      rom_reader,
      serial_port
    }
  }

  pub fn rom_reader(&self) -> &RomReader {
    &self.rom_reader
  }

  pub fn serial_port(&mut self) -> &mut SerialPort {
    &mut self.serial_port
  }
}