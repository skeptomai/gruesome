# Gruesome

A Rust implementation of the Z-Machine virtual machine for playing classic Infocom text adventure games, with a focus on Zork I compatibility.

## Overview

This project implements a Z-Machine interpreter capable of running Infocom's text adventure games from the 1980s. The Z-Machine was the virtual machine used by Infocom to create portable interactive fiction that could run on multiple platforms.

### Current Status

- ✅ **Fully playable** - Zork I runs from start to finish
- ✅ **Save/Restore** - Full Quetzal format support with compression
- ✅ **Core Z-Machine v3** - All essential opcodes implemented
- ✅ **Object system** - Complete with properties and attributes
- ✅ **Parser** - Dictionary lookup and text parsing
- ✅ **Random events** - Combat and NPC movement work correctly

### Known Limitations

- ⚠️ **No timed interrupts** - Lantern/match timers don't expire (makes game easier)
- ⚠️ **Limited display opcodes** - Basic text output only
- ⚠️ **v3 games only** - Later versions not yet supported

## Quick Start

### Prerequisites

- Rust 1.70 or later
- A Z-Machine game file (e.g., ZORK1.DAT)

### Building and Running

1. Clone the repository:
```bash
git clone https://github.com/skeptomai/gruesome.git
cd gruesome
```

2. Place your game file (e.g., ZORK1.DAT) in `resources/test/zork1/DATA/`

3. Run the game:
```bash
./run_game.sh
```

### Alternative Commands

```bash
# Run with debug output
./run_debug.sh

# Run tests
./run-tests.sh

# Build only
cargo build --release

# Run directly
cargo run --release
```

## Architecture

The interpreter is structured as follows:

- **VM** (`vm.rs`) - Virtual machine state management
- **Interpreter** (`interpreter.rs`) - Main instruction execution loop
- **Instruction** (`instruction.rs`) - Instruction decoding
- **Game** (`game.rs`) - Game file loading and memory management
- **Object System** (`zobject.rs`) - Z-Machine object tree
- **Text** (`text.rs`) - ZSCII encoding/decoding
- **Save/Restore** (`quetzal/`) - Quetzal save format implementation

## Development

### Running Tests

```bash
cargo test
```

### Debug Mode

Enable debug output with:
```bash
RUST_LOG=debug cargo run
```

### Contributing

Contributions are welcome! Key areas for improvement:

1. **Timed interrupts** - Implement timer support for authentic gameplay
2. **Display opcodes** - Add missing display functionality
3. **Version support** - Extend beyond v3 to support more games
4. **Performance** - Optimize hot paths in the interpreter

See [CLAUDE.md](CLAUDE.md) for detailed development guidelines and TODOs.

## Technical Details

This implementation follows the Z-Machine Standards Document 1.1. Notable features:

- Discovers and handles undocumented opcode 0x1F (see [undocumented_opcode_0x1f.md](undocumented_opcode_0x1f.md))
- Implements proper Quetzal save format with XOR-RLE compression
- Handles both Variable and Long forms of 2OP instructions
- Supports the full Z-Machine v3 instruction set

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Infocom for creating the Z-Machine and timeless games
- The Z-Machine Standards Document maintainers
- The interactive fiction community for preserving these classics