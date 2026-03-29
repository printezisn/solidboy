use crate::cpu::CPU;

pub struct Emulator {
    cpu: CPU,
}

impl Emulator {
    pub fn new(rom: Vec<u8>) -> Self {
        Emulator { cpu: CPU::new(rom) }
    }

    pub fn execute(&mut self, cycles: i32) {
        let mut remaining_cycles = cycles;
        while remaining_cycles > 0 {
            remaining_cycles -= self.cpu.execute_instruction().cycles as i32;
        }
    }
}
