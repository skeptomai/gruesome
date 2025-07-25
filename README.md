# Gruesome

A Rust implementation of the Z-Machine virtual machine for playing classic Infocom text adventure games, with a focus on Zork I compatibility.

## Overview

This project implements a Z-Machine interpreter capable of running Infocom's text adventure games from the 1980s. The Z-Machine was the virtual machine used by Infocom to create portable interactive fiction that could run on multiple platforms.

### Current Status

- ✅ **Fully playable** - Multiple Infocom games run perfectly
- ✅ **Save/Restore** - Full Quetzal format support with compression
- ✅ **Core Z-Machine v3** - All essential opcodes implemented including undocumented ones
- ✅ **Object system** - Complete with properties and attributes
- ✅ **Parser** - Dictionary lookup and text parsing
- ✅ **Random events** - Combat and NPC movement work correctly
- ✅ **Timer support** - Full timer implementation with real-time interrupts for all versions
- ✅ **Non-blocking I/O** - True event-driven input using OS-level notifications
- ✅ **read_char support** - Single character input with timers (v4+ feature)
- ✅ **Status line** - Fully functional status line with automatic updates (v3)
- ✅ **Display opcodes** - split_window, show_status, and set_text_style implemented

### Tested and Working Games

The following Infocom games have been tested and work correctly:

#### Version 3 Games (Fully Supported)
- **Zork I: The Great Underground Empire** - The classic that started it all
- **Deadline** - Mystery game with complex character movements and timed events
- **Enchanter** - First of the magic trilogy with spell system
- **The Hitchhiker's Guide to the Galaxy** - Douglas Adams' sci-fi comedy
- **Suspended** - Unique multi-robot perspective game
- **Seastalker** - Underwater adventure with window splitting
- **The Lurking Horror** - Horror game set at MIT

#### Version 4+ Games (Fully Supported)
- **A Mind Forever Voyaging** (v4) - Dystopian future simulation
- **Trinity** (v4) - Nuclear thriller with complex command processing
- **Bureaucracy** (v4) - Douglas Adams' satire on red tape
- Full gameplay support with advanced display features and proper instruction handling

### Known Limitations

- ⚠️ **Limited sound support** - sound_effect plays beep only (most games don't use sound)
- ⚠️ **No graphics** - Text-only implementation (affects only rare graphical games)
- ⚠️ **No v5+ unicode** - Extended character sets not supported

## Quick Start

### Prerequisites

- Rust 1.70 or later (for building from source)
- A Z-Machine game file (e.g., ZORK1.DAT)

### Installing Pre-built Binaries

Download the appropriate binary for your platform from the [releases page](https://github.com/skeptomai/gruesome/releases).

#### Windows Installation

After downloading `gruesome-x86_64-pc-windows-gnu.exe`:

1. Right-click the file and select "Properties"
2. Check the "Unblock" checkbox at the bottom
3. Click "OK"
4. Run the executable from Command Prompt or PowerShell:
   ```
   gruesome-x86_64-pc-windows-gnu.exe path\to\game.z3
   ```

If you see "Windows protected your PC":
1. Click "More info"
2. Click "Run anyway"

#### macOS Installation

1. Download the appropriate binary:
   - `gruesome-universal-apple-darwin` - Works on both Intel and Apple Silicon
   - `gruesome-aarch64-apple-darwin` - Apple Silicon only (smaller file)
   - `gruesome-x86_64-apple-darwin` - Intel only
2. Make it executable: `chmod +x gruesome-universal-apple-darwin`
3. If macOS blocks the app, go to System Preferences → Security & Privacy and click "Open Anyway"
4. Run: `./gruesome-universal-apple-darwin path/to/game.z3`

### Building from Source

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

### Build Options

The project supports optional features for enhanced display handling:

```bash
# Build with ratatui support (default - better display but larger binary)
cargo build --release

# Build without ratatui (smaller binary, maximum compatibility)
cargo build --release --no-default-features

# Run with specific features
cargo run --release --no-default-features -- path/to/game.z3
```

**Note**: Release binaries are built without ratatui for maximum compatibility across different terminal environments.

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
- **Display** (`display.rs`) - Cross-platform terminal control and status line
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

1. **More display opcodes** - set_cursor, erase_line, erase_window, etc.
2. **Version support** - Full v4/v5/v6 support (v7/v8 are rare)
3. **Sound support** - Implement proper sound_effect for games like Lurking Horror
4. **Performance** - Optimize hot paths in the interpreter
5. **More games** - Test with other Infocom titles
6. **Unicode support** - Z-Machine v5+ Unicode handling
7. **Buffer mode** - Implement proper output buffering
8. **Undo support** - Implement save_undo/restore_undo for v5+ games

See [CODEBASE_GUIDE.md](docs/CODEBASE_GUIDE.md) for getting started and [CLAUDE.md](CLAUDE.md) for implementation notes.

## Technical Details

This implementation follows the Z-Machine Standards Document 1.1. Notable features:

- **Complete instruction decoder** - Handles all instruction forms including complex edge cases
- **Branch detection** - Proper branch handling for all opcodes including scan_table (VAR:0x17)
- **Version-aware execution** - Smart dispatch between v3 and v4+ instruction handling
- **Robust error handling** - Graceful degradation with comprehensive debugging
- Discovers and handles undocumented opcode 0x1F (see [undocumented_opcode_0x1f.md](undocumented_opcode_0x1f.md))
- Implements 2OP:0x1C (not) for v1-v3 games (moved to VAR:143 in v5+)
- Implements proper Quetzal save format with XOR-RLE compression
- Handles both Variable and Long forms of 2OP instructions
- Supports complete Z-Machine v3/v4+ instruction sets including all store and branch operations
- Real non-blocking I/O using OS-level event notification (epoll/kqueue/IOCP)
- Full timer implementation with interrupt callbacks for both SREAD and read_char opcodes
- Cross-platform display system with smart fallback (ratatui → basic → headless)
- Automatic status line updates and proper text styling support

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Infocom for creating the Z-Machine and timeless games
- The Z-Machine Standards Document maintainers
- The interactive fiction community for preserving these classics