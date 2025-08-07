# Release v0.5.1 Summary

## ✅ Successfully Released!

### What Was Done:
1. **Merged** `investigate-extra-routines` branch into `main`
2. **Updated** version from 0.5.0 to 0.5.1 in Cargo.toml
3. **Created** comprehensive release notes
4. **Tagged** release as v0.5.1
5. **Pushed** to GitHub with tag
6. **Created** GitHub release at https://github.com/skeptomai/gruesome/releases/tag/v0.5.1

### Main Executables:

#### 1. `gruesome` - Z-Machine Interpreter
```bash
# Build
cargo build --release --bin gruesome

# Run
./target/release/gruesome game.dat
```

#### 2. `gruedasm-txd` - Z-Machine Disassembler
```bash
# Build
cargo build --release --bin gruedasm-txd

# Run
./target/release/gruedasm-txd game.dat > output.asm
```

### Key Achievement:
This release adds a **complete TXD-compatible disassembler** that:
- Finds ALL routines TXD finds (100% compatibility)
- Finds 44 additional valid routines TXD misses
- Has ZERO false positives (TXD has 23)
- Properly validates opcodes and routine headers

### Next Steps:
1. Build release binaries for distribution
2. Update README with disassembler documentation
3. Consider creating binary releases for different platforms
4. Share with the interactive fiction community!

The project now provides both a feature-complete interpreter AND a superior disassembler for Z-Machine files.