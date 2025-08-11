# Gruesome v0.6.0 Release Notes

## Superior Z-Machine Disassembler

### Overview
Version 0.6.0 represents a major milestone for the Gruesome project, delivering a production-ready Z-Machine disassembler that **surpasses** the industry-standard TXD (by Mark Howell) in accuracy and completeness.

### Key Achievements

#### 🎯 Strict Superset of TXD
- **V3 Games**: Finds all 440 routines TXD finds, plus 10 additional valid routines
- **V4+ Games**: Finds all 982 routines TXD finds, plus 27 additional valid routines
- **Zero False Positives**: All discovered routines successfully decode and validate

#### 🔍 Superior Accuracy
Our disassembler identifies and correctly validates routines that TXD misses:
- Data-referenced routines from object properties
- Grammar table action routines  
- Orphaned routines not directly called
- Properly rejects alternate entry points within routine bodies

#### 🛠️ Technical Improvements
- **Triple Validation**: Each routine decoded 3 times to ensure correctness
- **Proper Opcode Validation**: Rejects invalid Long form opcode 0x00
- **Version-Aware**: Correctly handles V3 vs V4+ format differences
- **Extensive Test Coverage**: 30+ test binaries validate correctness

### Usage

```bash
# Build the disassembler
cargo build --release --bin gruedasm-txd

# Disassemble a game file
./target/release/gruedasm-txd game.dat > output.asm

# Verify against TXD (optional)
./target/release/verify-strict-superset
```

### Compatibility
- Tested with classic Infocom games including:
  - Zork I (V3)
  - A Mind Forever Voyaging (V4)
  - Trinity (V4)
  - And many more

### What's New Since v0.5.1
- Fixed false positive detection for data regions
- Added proper validation of Long form opcodes
- Improved boundary scanning algorithm
- Enhanced routine discovery from data structures
- Comprehensive test suite for verification

### Binary Downloads
Pre-built binaries are available in the GitHub release for:
- Linux (x86_64)
- macOS (Intel & Apple Silicon)
- Windows (x64)

### Contributors
This release represents significant research and development in Z-Machine reverse engineering, building upon decades of work by the interactive fiction community.

### Next Steps
- Integration with Z-Machine development tools
- Support for V5+ extended opcodes
- Interactive disassembly browser

---

For detailed technical information, see the [Disassembler Design Document](docs/DISASSEMBLER_DESIGN.md).