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
- ✅ **Timer support** - Turn-based timers for v3, real-time infrastructure ready for v4+
- ✅ **Non-blocking I/O** - True event-driven input using OS-level notifications
- ✅ **read_char support** - Single character input with timers (v4+)

### Known Limitations

- ⚠️ **Limited display opcodes** - split_window, set_cursor, etc. not implemented
- ⚠️ **v3 games primarily** - Basic v4+ support (SREAD/read_char work)
- ⚠️ **No sound support** - sound_effect plays beep only
- ⚠️ **No graphics** - Text-only implementation

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

## Documentation

For developers new to the project:

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - Complete system architecture with diagrams
- **[CODEBASE_GUIDE.md](docs/CODEBASE_GUIDE.md)** - Detailed guide for new contributors
- **[QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md)** - Quick reference for common tasks
- **[TIMER_IMPLEMENTATION.md](docs/TIMER_IMPLEMENTATION.md)** - Timer and real-time support details
- **[NONBLOCKING_IO.md](docs/NONBLOCKING_IO.md)** - Non-blocking I/O architecture
- **[READ_CHAR_IMPLEMENTATION.md](docs/READ_CHAR_IMPLEMENTATION.md)** - Character input implementation
- **[ROUTINE_ADDRESSES.md](docs/ROUTINE_ADDRESSES.md)** - Known Zork I routine documentation
- **[CLAUDE.md](CLAUDE.md)** - Implementation notes and guidelines

## Architecture Overview

The interpreter is structured as follows:

- **VM** (`vm.rs`) - Virtual machine state management
- **Interpreter** (`interpreter.rs`) - Main instruction execution loop
- **Instruction** (`instruction.rs`) - Instruction decoding
- **Game** (`game.rs`) - Game file loading and memory management
- **Object System** (`zobject.rs`) - Z-Machine object tree
- **Text** (`text.rs`) - ZSCII encoding/decoding with packed addresses
- **Dictionary** (`dictionary.rs`) - Word parsing and lookup
- **Timed Input** (`timed_input.rs`) - Non-blocking I/O with timer support
- **Save/Restore** (`quetzal/`) - Quetzal save format implementation

See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed architecture information.

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

1. **Display opcodes** - split_window, set_cursor, set_text_style, etc.
2. **Version support** - Full v4/v5/v6 support (v7/v8 are rare)
3. **Sound support** - Implement proper sound_effect for games like Lurking Horror
4. **Status line** - Implement proper status line handling
5. **Performance** - Optimize hot paths in the interpreter
6. **More games** - Test with other Infocom titles

See [CODEBASE_GUIDE.md](docs/CODEBASE_GUIDE.md) for getting started and [CLAUDE.md](CLAUDE.md) for implementation notes.

## Technical Details

This implementation follows the Z-Machine Standards Document 1.1. Notable features:

- Discovers and handles undocumented opcode 0x1F (see [undocumented_opcode_0x1f.md](undocumented_opcode_0x1f.md))
- Implements proper Quetzal save format with XOR-RLE compression
- Handles both Variable and Long forms of 2OP instructions
- Supports the full Z-Machine v3 instruction set
- Real non-blocking I/O using OS-level event notification (epoll/kqueue/IOCP)
- Timer callbacks for both SREAD and read_char opcodes
- Turn-based timer support for v3 games, real-time ready for v4+

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Infocom for creating the Z-Machine and timeless games
- The Z-Machine Standards Document maintainers
- The interactive fiction community for preserving these classics