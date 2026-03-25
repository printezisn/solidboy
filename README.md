# Solidboy Emulator

A Game Boy emulator written in Rust (work in progress) with a WASM frontend using Vite.

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

3. Preview production build locally:

```bash
pnpm start:prod
```

## Production build

Build the Rust emulator as optimized WebAssembly and produce a production web bundle:

```bash
pnpm build:prod
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

The emulator consists of several components:

- **CPU**: instruction execution, registers, flags
- **Memory Bus**: ROM, RAM, I/O, MBC handling
- **Timer**: cycle timing and interrupts
- **Instructions**: full Game Boy instruction set

### CPU

- 8-bit / 16-bit registers (A, B, C, D, E, F, H, L, SP, PC)
- decoding/execution
- interrupts (IME, halted state)

### Memory Bus

- ROM loading and banking
- MBC support (NoROM, MBC1)
- RAM and VRAM access
- I/O register handling

### Supported features

- CPU instruction execution
- memory banking (NoROM, MBC1)
- timer and interrupts
- WASM front-end via Vite
