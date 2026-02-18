use super::adapters::Adapters;
use super::cpu::CPU;

pub fn emulate(adapters: Adapters) {
  let rom = adapters.rom_reader().read_rom();
  let mut cpu = CPU::new(rom);

  cpu.run();
}