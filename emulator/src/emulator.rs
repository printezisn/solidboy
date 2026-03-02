use super::adapters::Adapters;
use super::cpu::CPU;

pub fn emulate(adapters: Adapters) {
  let mut cpu = CPU::new(adapters);

  loop {
    cpu.execute_instruction();
  }
}