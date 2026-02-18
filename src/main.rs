mod cpu;
mod adapters;
mod emulator;

use adapters::rom_reader::RomReader;
use adapters::Adapters;

fn main() {
    let adapters = Adapters::new(RomReader::File { file_path: "./test-roms/cpu_instrs.gb" });

    emulator::emulate(adapters);
}
