# Gruesome Z-Machine Platform: Complete Developer Architecture Guide

**Version**: 1.0  
**Last Updated**: August 17, 2025  
**Status**: Production Ready

## Executive Summary

The Gruesome Z-Machine Platform is a complete, modern implementation of the Z-Machine virtual machine and associated toolchain, written in Rust. It provides three core capabilities:

1. **🎮 Game Interpretation** (`gruesome`) - Play existing Z-Machine games (V1-V8)
2. **🔍 Code Analysis** (`gruedasm-txd`) - Disassemble and analyze Z-Machine bytecode
3. **⚡ Game Compilation** (`grue-compiler`) - Create new games using the Grue language

**Supported Z-Machine Versions**: V3 ✅ | V4 ✅ | V5 ✅ | V6+ (partial)

## High-Level Platform Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    GRUESOME Z-MACHINE PLATFORM                 │
├─────────────────┬─────────────────┬─────────────────────────────┤
│   🎮 GAMEPLAY   │  🔍 ANALYSIS    │      ⚡ DEVELOPMENT         │
│    (gruesome)   │ (gruedasm-txd)  │    (grue-compiler)          │
├─────────────────┼─────────────────┼─────────────────────────────┤
│ • Play games    │ • Disassemble   │ • Write Grue source         │
│ • V3-V8 support │ • Debug games   │ • Compile to Z-Machine      │
│ • Save/Restore  │ • Analyze code  │ • V3/V4/V5 targets         │
│ • Full features │ • Understand    │ • Modern language features │
└─────────────────┴─────────────────┴─────────────────────────────┘
                            │
                    ┌───────▼───────┐
                    │  SHARED CORE  │
                    │   LIBRARIES   │
                    ├───────────────┤
                    │ • vm.rs       │
                    │ • instruction │
                    │ • text.rs     │
                    │ • zobject.rs  │
                    │ • dictionary  │
                    └───────────────┘
```

## Core Components Deep Dive

### 🎮 Gruesome Interpreter Architecture

**Purpose**: Execute Z-Machine bytecode files (.z3, .z4, .z5, etc.)

```
Game File (.z3/.z4/.z5)
        ↓
┌─────────────────┐
│  game.rs        │ ← File loading & validation
│  header.rs      │ ← Header parsing & version detection
└─────────────────┘
        ↓
┌─────────────────┐
│  vm.rs          │ ← Virtual machine state
│  • Memory       │   (stack, variables, memory)
│  • Stack        │
│  • Variables    │
└─────────────────┘
        ↓
┌─────────────────┐
│ interpreter.rs  │ ← Main execution loop
│ instruction.rs  │ ← Instruction decoding
└─────────────────┘
        ↓
┌─────────────────┐
│ Version-Specific│ ← Display & Input Systems
│ I/O Systems     │
│ • display_v3    │
│ • display_v4+   │
│ • input_v3      │
│ • input_v4+     │
└─────────────────┘
```

**Key Features**:
- **Version Abstraction**: Automatic version detection and feature switching
- **Modular Opcodes**: Each opcode family in separate module (`opcodes_*.rs`)
- **Rich Display**: Status lines, windowing, colors, fonts (V4+)
- **Advanced Input**: Timed input, character-level input, mouse (V6+)
- **Save System**: Quetzal-compatible save/restore

### 🔍 Disassembler Architecture

**Purpose**: Analyze and understand Z-Machine bytecode

```
Z-Machine File
        ↓
┌─────────────────┐
│ disassembler.rs │ ← Core disassembly engine
└─────────────────┘
        ↓
┌─────────────────┐
│  Discovery      │ ← Find routines and data
│  • Routine scan │
│  • Call tracing │
│  • Data analysis│
└─────────────────┘
        ↓
┌─────────────────┐
│  Generation     │ ← Produce readable output
│  • Assembly     │
│  • Comments     │
│  • Labels       │
└─────────────────┘
```

### ⚡ Grue Compiler Architecture

**Purpose**: Transform Grue source code into Z-Machine bytecode

```
Grue Source Code
        ↓
┌─────────────────┐
│ lexer.rs        │ ← Tokenization
└─────────────────┘
        ↓
┌─────────────────┐
│ parser.rs       │ ← Syntax analysis → AST
│ ast.rs          │
└─────────────────┘
        ↓
┌─────────────────┐
│ semantic.rs     │ ← Type checking & validation
└─────────────────┘
        ↓
┌─────────────────┐
│ ir.rs           │ ← Intermediate representation
└─────────────────┘
        ↓
┌─────────────────┐
│ codegen.rs      │ ← Z-Machine bytecode generation
└─────────────────┘
        ↓
Z-Machine File (.z3/.z4/.z5)
```

## Version Support Matrix

| Feature | V3 | V4 | V5 | V6+ |
|---------|----|----|----|----|
| **Interpreter** | ✅ Full | ✅ Full | ✅ Full | 🟡 Partial |
| **Disassembler** | ✅ Full | ✅ Full | ✅ Full | 🟡 Basic |
| **Compiler** | ✅ Full | ✅ Full | ✅ Full | ❌ None |
| **Testing** | ✅ Complete | ✅ Complete | ✅ Complete | 🟡 Limited |

### Version-Specific Features

#### V3 (Classic Infocom)
- **Games**: Zork I, Seastalker, Hitchhiker's Guide
- **Limits**: 255 objects, 32 attributes, simple display
- **Display**: Status line only, no windowing
- **Input**: Line input only

#### V4 (Enhanced Infocom)  
- **Games**: AMFV, Trinity, Bureaucracy
- **Enhanced**: 65,535 objects, 48 attributes
- **Display**: Multiple windows, colors, fonts
- **Input**: Character input, timed input
- **Special**: Sound effects, graphics preparation

#### V5 (Advanced Features)
- **Features**: All V4 plus extended opcodes
- **Memory**: Larger games (256KB), different addressing
- **Display**: Enhanced windowing and graphics support
- **Modern**: Mouse input preparation (V6 transition)

## Directory Structure Guide

```
src/
├── main.rs                 # Gruesome interpreter entry point
├── lib.rs                  # Library exports
├── vm.rs                   # Virtual machine core
├── interpreter.rs          # Main execution loop
├── instruction.rs          # Instruction decoding
├── game.rs                 # Game file handling
├── header.rs               # Z-Machine headers
├── zobject.rs              # Object system
├── dictionary.rs           # Word parsing & encoding
├── text.rs                 # String encoding/decoding
├── disassembler.rs         # Code analysis
├── 
├── opcodes_*.rs           # Opcode implementations
│   ├── opcodes_stack.rs   # Stack operations
│   ├── opcodes_math.rs    # Arithmetic & logic
│   ├── opcodes_display.rs # Text & display
│   ├── opcodes_input.rs   # User input
│   ├── opcodes_flow.rs    # Control flow
│   └── opcodes_var.rs     # Variable operations
│
├── input_*.rs             # Input systems
│   ├── input_v3.rs        # V3 line input
│   └── input_v4.rs        # V4+ advanced input
│
├── display_*.rs           # Display systems  
│   ├── display_v3.rs      # V3 status line
│   └── display_ratatui.rs # V4+ windowing
│
├── grue_compiler/         # Grue language compiler
│   ├── mod.rs             # Public API
│   ├── lexer.rs           # Tokenization
│   ├── parser.rs          # Syntax analysis
│   ├── ast.rs             # Abstract syntax tree
│   ├── semantic.rs        # Type checking
│   ├── ir.rs              # Intermediate representation
│   ├── codegen.rs         # Z-Machine generation
│   └── error.rs           # Error handling
│
├── quetzal/               # Save/restore system
└── bin/                   # Binary entry points
    ├── grue_compiler.rs   # Compiler binary
    └── gruedasm-txd.rs    # Disassembler binary
```

## Development Workflows

### Adding New Opcodes

1. **Identify the opcode** in Z-Machine specification
2. **Choose the appropriate module** (`opcodes_*.rs`)
3. **Implement the opcode** with proper error handling
4. **Add tests** for the opcode behavior
5. **Update documentation** if needed

Example:
```rust
// In opcodes_math.rs
(0x15, OperandCount::OP2) => {
    // add - 0x15 - add operand1 + operand2 → result
    let sum = operands[0].wrapping_add(operands[1]);
    if let Some(store_var) = inst.store_var {
        self.vm.write_variable(store_var, sum)?;
    }
    Ok(ExecutionResult::Continue)
}
```

### Adding Version-Specific Features

1. **Version detection** in header parsing
2. **Feature flags** based on version
3. **Graceful fallbacks** for unsupported features
4. **Testing** across all supported versions

### Extending the Grue Language

1. **Lexer**: Add new tokens if needed
2. **Parser**: Extend grammar rules
3. **AST**: Add new node types
4. **Semantic**: Add validation rules
5. **IR**: Extend intermediate representation
6. **Codegen**: Generate appropriate Z-Machine bytecode

## Testing Strategy

### Unit Tests
- **Each module** has comprehensive unit tests
- **Opcode tests** verify instruction behavior
- **Compiler tests** validate each compilation phase
- **Golden file tests** ensure consistent output

### Integration Tests
- **Real games**: Zork I, AMFV, Trinity
- **Cross-version**: Same game logic across V3/V4/V5
- **Save/restore**: Game state persistence
- **Display**: Visual output verification

### Performance Tests
- **Large games**: Memory usage and execution speed
- **Complex scenes**: Display rendering performance
- **Compilation**: Build time for complex Grue programs

## Common Development Tasks

### Debugging Game Issues
1. **Enable debug logging**: `RUST_LOG=debug cargo run --bin gruesome game.z3`
2. **Use disassembler**: `cargo run --bin gruedasm-txd game.z3 > analysis.txt`
3. **Check specific opcodes**: Focus on failing instruction types
4. **Compare versions**: Test same game in different interpreters

### Extending Compiler Features
1. **Language design**: Define new Grue syntax
2. **Implementation**: Follow the 6-phase pipeline
3. **Testing**: Create comprehensive test cases
4. **Documentation**: Update user guides

### Performance Optimization
1. **Profile**: Use `cargo flamegraph` for performance analysis
2. **Optimize**: Focus on hot paths in interpreter loop
3. **Memory**: Reduce allocations in execution core
4. **Benchmark**: Maintain performance regression tests

## Production Deployment

### Build Configuration
```bash
# Debug build (development)
cargo build

# Release build (production)  
cargo build --release

# Cross-platform builds
cargo build --target x86_64-pc-windows-gnu
cargo build --target x86_64-apple-darwin
```

### Quality Assurance
```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Run all tests
cargo test

# Run specific test categories
cargo test integration
cargo test golden_file
```

### Release Process
1. **Update version numbers** in `Cargo.toml`
2. **Run full test suite** including integration tests
3. **Build release binaries** for all platforms
4. **Generate documentation** updates
5. **Tag release** and update changelog

## Best Practices

### Code Style
- **Follow Rust conventions**: Use `rustfmt` and `clippy`
- **Error handling**: Use `Result<T, E>` consistently
- **Documentation**: Document all public APIs
- **Testing**: Write tests for all new functionality

### Performance
- **Avoid allocations** in interpreter hot paths
- **Use appropriate data structures** for game state
- **Profile regularly** to catch regressions
- **Optimize for common cases** (V3/V4 games)

### Compatibility
- **Test across versions** when making changes
- **Maintain backwards compatibility** in save files
- **Support standard Z-Machine features** before extensions
- **Document deviations** from specification

## Future Development Directions

### Near-term (Next 6 months)
- **Enhanced Grue language** features
- **Advanced object system** (inheritance, complex properties)
- **Improved parser** (multi-word nouns, disambiguation)
- **Better error messages** and debugging tools

### Medium-term (6-12 months)
- **V6 graphics support** for graphical games
- **Advanced display** features (fonts, colors, layout)
- **Performance optimization** for large games
- **Development tools** (debugger, profiler)

### Long-term (1+ years)
- **Complete V1-V8 support** including specialized features
- **IDE integration** for Grue development
- **Game distribution** tools and packaging
- **Community features** (sharing, collaboration)

---

**For Questions**: Check existing documentation in `/docs` or examine test cases in `/tests`  
**For Issues**: Review the issue tracker and testing guidelines  
**For Contributing**: Follow the development workflows and best practices outlined above

This platform represents a complete, production-ready implementation of the Z-Machine ecosystem with modern development practices and comprehensive tooling.