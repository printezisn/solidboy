mod cpu;
mod emulator;

use emulator::Emulator;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    if args.len() < 2 {
        panic!("Please provide a path to a game rom.");
    }

    let rom = std::fs::read(args[1].clone()).unwrap();
    let mut emulator = Emulator::new(rom);

    loop {
        emulator.execute();
    }
}
