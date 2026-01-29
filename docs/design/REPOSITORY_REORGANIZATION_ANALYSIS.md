# Repository Reorganization Analysis

## Current Structure Assessment

### Current Organization
```
infocom/
├── src/
│   ├── [40+ interpreter files] - scattered in root
│   ├── bin/ - binary executables
│   ├── grue_compiler/ - separate compiler directory
│   └── quetzal/ - save system directory
```

### Component Analysis

**Interpreter Files (in src/ root)**:
- Core: `interpreter.rs`, `vm.rs`, `instruction.rs`, `game.rs`
- Object System: `zobject*.rs`, `property_defaults.rs`
- Display System: `display_*.rs` (6 files)
- Input System: `input_v3.rs`, `input_v4.rs`, `timed_input.rs`
- Opcodes: `opcodes_*.rs` (6 files)
- Text/Dictionary: `text.rs`, `dictionary.rs`, `parser_engine.rs`, `runtime_parser.rs`
- Utilities: `header.rs`, `gamememorymap.rs`, `util.rs`, `zrand.rs`, `routine.rs`
- Debugging: `debugger.rs`, `debug_symbols.rs`

**Separate Modules**:
- `grue_compiler/` - Complete compiler system
- `quetzal/` - Save/restore system

**Disassembler Files (mixed location)**:
- `disassembler.rs` - in src/ root
- `disasm_txd.rs` - in src/ root (enhanced version)

**Binary Tools**:
- `src/bin/grue_compiler.rs` - compiler binary
- `src/bin/gruedasm-txd.rs` - disassembler binary
- `src/bin/[25+ analysis tools]` - debugging/development tools

### Dependency Analysis

**Key Insight**: Most interpreter files only depend on other interpreter files and common utilities. The `lib.rs` shows all modules are declared at the root level, making reorganization feasible.

**Critical Dependencies**:
- Disassembler depends on interpreter components (vm, instruction, game)
- Quetzal depends on vm and game structures
- Compiler is largely independent (separate directory already)
- Binary tools depend on various components

## Proposed Reorganization Approaches

### Option 1: Modular Separation (Recommended)
```
src/
├── interpreter/           # Main Z-Machine interpreter
│   ├── mod.rs            # Public interface
│   ├── core/             # Core execution
│   │   ├── mod.rs
│   │   ├── vm.rs
│   │   ├── interpreter.rs
│   │   ├── instruction.rs
│   │   └── game.rs
│   ├── display/          # Display subsystem
│   │   ├── mod.rs
│   │   ├── manager.rs    # display_manager.rs
│   │   ├── crossterm.rs  # display_crossterm.rs
│   │   ├── ratatui.rs    # display_ratatui.rs
│   │   ├── trait.rs      # display_trait.rs
│   │   ├── v3.rs         # display_v3.rs
│   │   ├── headless.rs   # display_headless.rs
│   │   └── logging.rs    # display_logging.rs
│   ├── input/            # Input subsystem
│   │   ├── mod.rs
│   │   ├── v3.rs         # input_v3.rs
│   │   ├── v4.rs         # input_v4.rs
│   │   └── timed.rs      # timed_input.rs
│   ├── opcodes/          # Opcode implementations
│   │   ├── mod.rs
│   │   ├── display.rs    # opcodes_display.rs
│   │   ├── io.rs         # opcodes_io.rs
│   │   ├── math.rs       # opcodes_math.rs
│   │   ├── memory.rs     # opcodes_memory.rs
│   │   ├── object.rs     # opcodes_object.rs
│   │   ├── stack.rs      # opcodes_stack.rs
│   │   └── tables.rs     # opcode_tables.rs
│   ├── objects/          # Object system
│   │   ├── mod.rs
│   │   ├── zobject.rs
│   │   ├── interface.rs  # zobject_interface.rs
│   │   ├── v3.rs         # zobject_v3.rs
│   │   ├── v4.rs         # zobject_v4.rs
│   │   └── defaults.rs   # property_defaults.rs
│   ├── text/             # Text processing
│   │   ├── mod.rs
│   │   ├── text.rs
│   │   ├── dictionary.rs
│   │   ├── parser_engine.rs
│   │   └── runtime_parser.rs
│   ├── quetzal/          # Save system (moved under interpreter)
│   │   └── [existing files]
│   └── utils/            # Interpreter utilities
│       ├── mod.rs
│       ├── header.rs
│       ├── gamememorymap.rs
│       ├── util.rs
│       ├── zrand.rs
│       ├── routine.rs
│       ├── debugger.rs
│       └── debug_symbols.rs
├── disassembler/         # Disassembly tools
│   ├── mod.rs
│   ├── disassembler.rs
│   └── txd.rs           # disasm_txd.rs
├── grue_compiler/        # Compiler (unchanged)
│   └── [existing files]
├── bin/                  # Binary executables
│   └── [existing files]
├── lib.rs               # Updated module declarations
└── main.rs              # Main binary
```

**Benefits**:
- Clean separation of concerns
- Quetzal logically under interpreter (saves are interpreter-specific)
- Easier navigation and maintenance
- Clear public interfaces via mod.rs files
- Follows Rust module conventions

**Module Changes Required**:
- Update `lib.rs` to declare new module structure
- Add `mod.rs` files for each subdirectory
- Update import paths throughout codebase
- Update binary tool imports

### Option 2: Flat Component Separation
```
src/
├── interpreter_core/     # Core interpreter files only
│   ├── vm.rs, interpreter.rs, instruction.rs, game.rs
├── interpreter_display/ # Display subsystem
├── interpreter_input/   # Input subsystem
├── interpreter_opcodes/ # Opcodes
├── interpreter_objects/ # Object system
├── interpreter_text/    # Text processing
├── interpreter_utils/   # Utilities
├── disassembler/        # Disassembly tools
├── quetzal/             # Save system (remains separate)
├── grue_compiler/       # Compiler (unchanged)
└── bin/                 # Binaries
```

**Benefits**:
- Less nested structure
- Quetzal remains independent
- Smaller scope of changes

**Drawbacks**:
- More directories at src/ level
- Less clear logical grouping
- Quetzal separation may not make sense (it's interpreter-specific)

## Analysis: Why Put More Under Interpreter?

**Sparky's Question**: Why not put more subdirectories under interpreter in Option 1?

**Answer**: We absolutely should! The current Option 1 is actually conservative. Here's why putting more under interpreter makes sense:

### Logical Ownership
- **Quetzal saves**: Are interpreter state serialization - they belong with the interpreter
- **Display system**: Handles interpreter output - belongs with interpreter
- **Input system**: Handles interpreter input - belongs with interpreter
- **Object system**: Implements Z-Machine object model for interpreter
- **Text system**: Implements Z-Machine text processing for interpreter

### Dependency Flow
- All these systems primarily serve the interpreter
- They don't have independent utility outside the interpreter context
- Moving them under interpreter creates clear ownership

### Namespace Benefits
- `gruesome::interpreter::display::Manager`
- `gruesome::interpreter::objects::ZObject`
- `gruesome::interpreter::quetzal::SaveFile`

This creates clear, logical namespacing.

## Implementation Complexity Assessment

**Module Reference Changes**: Moderate impact
- Need to update ~100+ `use` statements across codebase
- Rust compiler will catch all broken references
- Can be done incrementally by subdirectory

**Binary Tool Updates**: Low impact
- Most binary tools import from top-level lib anyway
- Main changes in `lib.rs` module declarations

**Testing Updates**: Low impact
- Tests mainly use public interfaces
- Module reorganization shouldn't break functionality

**Build System**: Minimal impact
- Cargo automatically discovers modules
- No Cargo.toml changes needed

## Disassembler Placement Challenge

**The Problem**: Disassembler has unclear ownership
- Uses interpreter components (vm, instruction, game)
- But provides analysis tools, not runtime functionality
- Has both library functions and standalone binary

**Option A**: Under interpreter as `interpreter/disasm/`
- Pro: Uses interpreter internals heavily
- Con: Not really part of runtime interpreter

**Option B**: Separate `disassembler/` directory
- Pro: Clear separation of concerns
- Con: Still depends heavily on interpreter types

**Option C**: Under `tools/` or `debug/`
- Pro: Reflects its nature as development tool
- Con: Doesn't match current usage patterns

**Recommendation**: Option B (separate directory) - it's a distinct tool that analyzes interpreter data structures.

## Recommendation

**Proceed with Option 1** with the following refinements:
1. Move even more components under `interpreter/` - it's the dominant component
2. Keep `disassembler/` separate but clearly organized
3. Move `quetzal/` under `interpreter/` since saves are interpreter state
4. Implement incrementally to minimize disruption

The benefits significantly outweigh the reorganization effort, and the current flat structure is becoming unwieldy for maintenance.