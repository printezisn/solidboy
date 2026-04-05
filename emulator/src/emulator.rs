use crate::cpu::CPU;

pub struct Emulator {
    cpu: CPU,
}

impl Emulator {
    pub fn new(rom: Vec<u8>) -> Self {
        Emulator { cpu: CPU::new(rom) }
    }

    pub fn execute(
        &mut self,
        cycles: i32,
        joypad_pressed_directions: u8,
        joypad_pressed_buttons: u8,
    ) {
        self.cpu
            .set_joypad_pressed_directions(joypad_pressed_directions);
        self.cpu.set_joypad_pressed_buttons(joypad_pressed_buttons);

        let mut remaining_cycles = cycles;
        while remaining_cycles > 0 {
            remaining_cycles -= self.cpu.execute_instruction().cycles as i32;
        }
    }
}
