# Solidboy Emulator

A Game Boy emulator written in Rust with a WASM frontend using Vite.

## Prerequisites

- Node.js >= 24
- pnpm
- Rust (edition 2024 or later)
- wasm-pack

## Local development

1. Install dependencies:

```bash
pnpm install
```

2. Build the WASM binding + UI and run dev server:

```bash
pnpm start:dev
```

This executes:

- `wasm-pack build ./emulator --target bundler --release`
- `pnpm i --force ./emulator/pkg`
- `vite build`

## Running tests

Run Rust tests in the emulator crate:

```bash
cargo test --manifest-path ./emulator/Cargo.toml
```

Run full repo pre-commit checks (from `.husky/pre-commit`):

```bash
cargo test --manifest-path ./emulator/Cargo.toml
pnpm format
pnpm lint
pnpm build:prod
git update-index --again
```

## Build commands (scripts)

- `pnpm build:wasm` - build Rust WASM package and refresh symlinked package
- `pnpm build:prod` - wasm + Vite production bundle
- `pnpm format` - format source files with Prettier
- `pnpm lint` - static lint checks via ESLint

## Architecture

The emulator is structured as a modular Rust crate compiled to WebAssembly, interfacing with a JavaScript frontend for user interaction and rendering. The core components are:

- **CPU**: Handles instruction decoding, execution, register management, and interrupt processing.
- **Memory Bus**: Manages memory access, including ROM banking, RAM, VRAM, I/O registers, and cartridge memory bank controllers (MBC).
- **Timer**: Implements the Game Boy's internal timer for cycle-based timing and interrupt generation.
- **Audio**: Emulates the Game Boy's audio processing unit (APU) with multiple channels and effects.
- **PPU (Graphics)**: Handles pixel processing and frame rendering.
- **Joypad**: Manages input from game controls.

### Technical Details

#### CPU Emulation

The CPU module emulates the Sharp LR35902 processor, an 8-bit CPU with a hybrid 8/16-bit architecture. Key features:

- **Registers**: 8 general-purpose 8-bit registers (A, B, C, D, E, F, H, L) that can be paired into 16-bit registers (AF, BC, DE, HL), plus 16-bit stack pointer (SP) and program counter (PC).
- **Flags**: The F register contains flags for zero (Z), subtract (N), half-carry (H), and carry (C).
- **Instruction Set**: Full implementation of the Game Boy's 500+ instructions, including prefixed (CB) instructions for bit operations.
- **Interrupts**: Supports interrupt master enable (IME), halted state, and pending interrupt handling.
- **Initialization**: Different initial register values for DMG (original Game Boy) and CGB (Game Boy Color) models.

Instructions are decoded from memory, executed cycle-accurately, and affect registers and memory as per the Game Boy specification.

#### Memory Bus

The memory bus provides a unified interface for all memory operations:

- **ROM**: Cartridge ROM with banking support for larger games.
- **RAM**: Internal RAM, external cartridge RAM, and video RAM (VRAM).
- **I/O Registers**: Memory-mapped registers for hardware control (timers, audio, graphics, joypad).
- **MBC Support**: Implements NoROM (no banking), MBC1, MBC3 and MBC5 controllers for cartridge compatibility.
- **Memory Mapping**: Translates logical addresses to physical memory locations, handling special regions like echo RAM and OAM.

#### Audio System

The audio processing unit (APU) emulation includes:

- **Channels**:
  - **Pulse Channels (1 & 2)**: Square wave generators with duty cycles, frequency control, and envelope/sweep effects.
  - **Wave Channel (3)**: Custom waveform playback from RAM.
  - **Noise Channel (4)**: Pseudo-random noise generation.
- **Effects**: Volume envelope, frequency sweep, and length counters.
- **Master Control**: Overall volume, channel enable/disable, and mute functionality.
- **Sequencer**: Frequency and volume updates at 512Hz and 64Hz rates.

Audio samples are generated in real-time and passed to the JavaScript frontend for playback.

#### Timer

The timer module emulates the Game Boy's internal timer:

- **Registers**: DIV (divider), TIMA (timer counter), TMA (timer modulo), TAC (timer control).
- **Interrupts**: Generates timer interrupts when TIMA overflows.
- **Cycle Accuracy**: Updates based on CPU cycles for precise timing.

#### WebAssembly Interface

The Rust core exposes functions via `wasm-bindgen`:

- `execute(cycles, joypad_directions, joypad_buttons)`: Runs the emulator for a specified number of cycles with input.
- Memory access functions for ROM loading and save data handling.
- Callbacks for rendering frame buffers and appending audio samples.

The JavaScript frontend (using Vite) handles UI, input events, and Web Audio API for sound output.

### Supported Features

- Full CPU instruction set execution
- Memory banking (NoROM, MBC1, MBC3, MBC5)
- Timer and interrupt handling
- Graphics processing unit (PPU) for video output
- Complete audio emulation (pulse, wave, noise channels with effects)
- Joypad input
- Save data persistence (with IndexedDB)
- WebAssembly-based web frontend
- Support for both DMG (original Game Boy) and GBC (Game Boy Color) games.

## Contributing

Solidboy is an open-source project. Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass and code is formatted
6. Submit a pull request

For major changes, please open an issue first to discuss the proposed changes.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
