# Merge and Usage Guide

## Current Status
The `investigate-extra-routines` branch contains the completed disassembler implementation that finds all routines TXD finds plus additional valid ones.

## Safe Merge Process

### 1. Update Main Branch
```bash
git checkout main
git pull origin main
```

### 2. Merge the Feature Branch
```bash
git merge investigate-extra-routines
```

### 3. Run Tests
```bash
cargo test
```

### 4. Build the Project
```bash
cargo build --release
```

## Usage

### Playing Games with the Interpreter

The main interpreter is called `gruesome`:

```bash
# Run from cargo
cargo run -- resources/test/zork1/DATA/ZORK1.DAT

# Or use the built binary
./target/release/gruesome resources/test/zork1/DATA/ZORK1.DAT

# For V4+ games like AMFV
./target/release/gruesome resources/test/amfv/amfv-r79-s851122.z4
```

Features:
- Full Z-Machine v1-v5 support
- Save/restore with Quetzal format
- Timed interrupts (lantern countdown works)
- Character input for menu-driven games
- Smart display with fallback modes

### Disassembling Games

The disassembler is called `gruedasm-txd`:

```bash
# Run from cargo
cargo run --bin gruedasm-txd -- resources/test/zork1/DATA/ZORK1.DAT

# Or use the built binary
./target/release/gruedasm-txd resources/test/zork1/DATA/ZORK1.DAT

# Save output to file
./target/release/gruedasm-txd resources/test/amfv/amfv-r79-s851122.z4 > amfv.disasm
```

Output format matches TXD with improvements:
- Lists all routines with addresses
- Shows routine headers (locals count)
- Finds MORE routines than TXD (no false positives)

### Development/Debug Tools

The project includes many analysis tools in `src/bin/`:

```bash
# Check if specific routines are found
cargo run --bin check_missing_routines -- <game_file>

# Analyze routine patterns
cargo run --bin analyze_routine_patterns -- <game_file>

# Debug specific addresses
cargo run --bin check_addr -- <game_file> <address>
```

## Binary Summary

| Binary | Purpose | Usage |
|--------|---------|-------|
| `gruesome` | Play Z-Machine games | `gruesome game.dat` |
| `gruedasm-txd` | Disassemble games | `gruedasm-txd game.dat` |
| Various debug tools | Analysis/debugging | See `src/bin/` |

## Comparison with Original Tools

### Interpreter (gruesome)
- **Replaces**: Infocom's original interpreters
- **Advantages**: Modern, cross-platform, all versions supported
- **Status**: Feature-complete for v1-v5

### Disassembler (gruedasm-txd)
- **Replaces**: TXD (Mark Howell's disassembler)
- **Advantages**: Finds more valid routines, no false positives
- **Status**: Superior to reference implementation

## Next Steps After Merge

1. Create GitHub release with binaries
2. Update README with usage instructions
3. Consider packaging for distribution (cargo install, homebrew, etc.)
4. Document the improved disassembler features