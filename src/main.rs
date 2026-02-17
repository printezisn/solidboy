pub mod cpu;

use cpu::CPU;

fn main() {
    let rom = std::fs::read("test-roms/cpu_instrs.gb").unwrap();
    let mut cpu = CPU::new(rom);

    cpu.run();
}
