# Solidboy Emulator

A Game Boy emulator written in Rust (Currently in progress).

## Building and Running

### Prerequisites

- Rust (edition 2024 or later)

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run -- path/to/game.gb
```

Replace `path/to/game.gb` with the path to a Game Boy ROM file.

## Architecture

The emulator consists of several components:

- **CPU**: Handles instruction execution, registers, and flags.
- **Memory Bus**: Manages memory access, including ROM, RAM, and I/O.
- **Timer**: Handles timing-related operations.
- **Instructions**: Implements the Game Boy instruction set.

### CPU

The CPU module includes:

- Registers: 8-bit and 16-bit registers (A, B, C, D, E, F, H, L, SP, PC)
- Instruction decoding and execution
- Interrupt handling (IME, halted state)

### Memory Bus

The Memory Bus handles:

- ROM loading and banking
- MBC (Memory Bank Controller) support for NoROM and MBC1
- RAM and VRAM access
- I/O registers

### Instructions

Implements the full Game Boy instruction set, including:

- Standard instructions (LD, ADD, etc.)
- Prefixed instructions (CB prefix for bit operations)
- Control flow (JMP, CALL, RET)

## Supported Features

- Basic instruction set execution
- Memory banking (NoROM, MBC1)
- Timer functionality
- Register and flag management
