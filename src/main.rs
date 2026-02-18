mod cpu;
mod adapters;
mod emulator;

use adapters::rom_reader::RomReader;
use adapters::Adapters;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    if args.len() < 2 {
        panic!("Please provide a path to a game rom.");
    }

    let adapters = Adapters::new(RomReader::File { file_path: args[1].clone() });

    emulator::emulate(adapters);
}
