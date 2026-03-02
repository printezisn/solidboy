pub enum SerialPort {
  None,
  Debug { byte: u8 }
}

impl SerialPort {
  pub fn write(&mut self, value: u8) {
    match self {
      SerialPort::None => {},
      SerialPort::Debug { byte } => {
        *byte = value;
      }
    }
  }

  pub fn control(&self, value: u8) {
    match self {
      SerialPort::None => {}
      SerialPort::Debug { byte} => {
        if value == 0x81 {
          print!("{}", *byte as char);
        }
      }
    }
  }
}