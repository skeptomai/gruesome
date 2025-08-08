# Disassembler Development Context and Current State

## Previous Work Summary
We successfully developed a TXD-compatible Z-Machine disassembler that matches the reference implementation's output exactly. The disassembler is in `src/disasm_txd.rs` and the CLI tool is `src/bin/gruedasm-txd.rs`.

### Key Achievements
1. **Full TXD Compatibility**: Our disassembler finds exactly 449 routines for Zork I and 1025 for AMFV, matching TXD
2. **Instruction Decoding**: Complete v3 and v4+ instruction decoding with proper operand handling
3. **Output Modes**: Support for both label mode (default) and address mode (-n flag)
4. **Orphan Detection**: Sophisticated algorithm to find routines not directly called
5. **CI/CD Integration**: Full test suite with integration tests for both gameplay and disassembler

## Current Project State

### Version Information
- Current version: v0.6.2 (last release)
- Z-Machine interpreter fully functional for v3 (Zork I) and v4+ (AMFV) games
- Disassembler matches TXD output exactly

### Test Coverage
- **Unit tests**: Text encoding, VM creation, instruction decoding
- **Integration tests**: 
  - `tests/integration_gameplay.rs` - Zork I and AMFV gameplay tests
  - `tests/integration_disasm.rs` - Disassembler output validation
- **Display tests**: `tests/display_v3_test.rs` and `tests/display_v4_test.rs`

### Key Files Modified Recently
- `CLAUDE.md` - Added automation instructions for "Make it so!", "Engage!", and "Reengage!"
- `.claude/settings.json` - Project-wide tool permissions (checked in)
- `.claude/settings.local.json` - User-specific settings (gitignored)
- Integration tests for AMFV gameplay with DISPLAY_MODE=terminal to avoid ratatui issues

## Automation Commands Configured

### "Make it so!"
Automatically commits and pushes changes:
1. Runs cargo fmt if needed
2. Stages all changes
3. Creates descriptive commit
4. Pushes to origin/main

### "Engage!"
Creates a new release:
1. Everything from "Make it so!"
2. Increments version number
3. Creates and pushes tag
4. Creates GitHub release
5. Monitors CI/release builds

### "Reengage!"
Re-releases with same version:
1. Moves existing tag to latest commit
2. Deletes and recreates GitHub release
3. Triggers fresh CI and binary builds

## Technical Context

### Display System Architecture
- **v3 games**: Use simple terminal display
- **v4+ games**: Default to ratatui TUI, but tests use DISPLAY_MODE=terminal
- **Ratatui issues**: Outputs ANSI escape codes that make testing difficult
- **Solution**: Force terminal mode in tests for clean text output

### Z-Machine Implementation Details
- **Object system**: Version-aware (v3: 255 objects, v4+: 65535 objects)
- **Text encoding**: v3 uses 6 Z-characters, v4+ uses 9 Z-characters
- **Timer support**: Full timed interrupt implementation
- **Save/Restore**: Quetzal format support
- **Display opcodes**: All implemented including split_window, set_cursor, etc.

### Disassembler Algorithm (TXD-compatible)
1. **Phase 1**: Scan from code_base to initial_pc
2. **Phase 2**: Iterative boundary expansion based on call targets
3. **Orphan detection**: Find routines only referenced from data
4. **Validation**: Decode until return instruction found
5. **Output formatting**: Labels (default) or addresses (-n flag)

## Environment Setup
- Z-Machine spec at: `/Users/cb/Projects/Z-Machine-Standard/`
- TXD reference implementation: `../ztools/txd.c`
- Test games:
  - Zork I (v3): `resources/test/zork1/DATA/ZORK1.DAT`
  - AMFV (v4): `resources/test/amfv/amfv-r79-s851122.z4`

## CI/CD Pipeline
- GitHub Actions workflow in `.github/workflows/ci.yml`
- Cross-platform builds (Linux, macOS, Windows)
- Release workflow builds binaries automatically on tag push
- Integration tests run on Ubuntu only

## Recent Bug Fixes
- VAR opcode 0x13 disambiguation (get_next_prop vs output_stream)
- Fixed scrolling issues in small terminals for v4+ games
- Resolved input echo issues with proper Z-Machine spec compliance

## Next Steps (if continuing)
- Consider implementing v5+ extended opcodes (256+)
- Add more v4+ game integration tests
- Potentially add graphical game support (v6)
- Enhanced debugging tools for game development

## Commands to Resume Work
```bash
# Run the interpreter
cargo run --bin gruesome resources/test/zork1/DATA/ZORK1.DAT

# Run the disassembler
cargo run --bin gruedasm-txd resources/test/zork1/DATA/ZORK1.DAT

# Run tests
cargo test
cargo test --test integration_gameplay
cargo test --test integration_disasm

# Check CI locally
./check-ci.sh
```

This context should allow seamless continuation of the project development.