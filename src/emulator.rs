use super::adapters::Adapters;
use super::cpu::CPU;

pub fn emulate() {
  let adapters = Adapters::new();
  
  let rom = adapters.get_rom_reader().read_rom();
  let mut cpu = CPU::new(rom);

  cpu.run();
}