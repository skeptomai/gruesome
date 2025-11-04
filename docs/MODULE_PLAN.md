# Module Organization Analysis & Plan

**Analysis Date**: November 3, 2025
**Context**: Evaluating module structure for codegen refactoring

## Current Module Structure Analysis

### Grue Compiler Module Organization

The `grue_compiler` module currently uses a flat structure with all submodules declared as `pub mod`:

```rust
// In src/grue_compiler/mod.rs
pub mod ast;
pub mod codegen;
pub mod codegen_builtins;
pub mod codegen_headers;
pub mod codegen_instructions;
pub mod codegen_memory;
pub mod codegen_objects;
pub mod codegen_references;
pub mod codegen_strings;
pub mod codegen_utils;
pub mod error;
pub mod ir;
pub mod lexer;
pub mod opcodes;
pub mod parser;
pub mod semantic;
```

### Current Import Pattern

Modules use explicit crate-relative imports:

```rust
// From codegen.rs
use crate::grue_compiler::codegen_memory::{
    placeholder_word, MemorySpace, HEADER_SIZE, PLACEHOLDER_BYTE,
};
use crate::grue_compiler::codegen_references::{
    LegacyReferenceType, ReferenceContext, UnresolvedReference,
};
use crate::grue_compiler::codegen_utils::CodeGenUtils;
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::opcodes::*;
```

### Extension Pattern in Use

The `codegen_builtins.rs` file shows the established pattern:

```rust
// Extends ZMachineCodeGen with additional impl blocks
impl ZMachineCodeGen {
    pub fn generate_println_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
        // Implementation
    }
}
```

## Idiomatic Rust vs Current Practice

### Current Pattern: Flat Public Modules
**Status**: Not idiomatic but functional

**Problems**:
- Over-exposure of internal modules
- No encapsulation of implementation details
- Namespace pollution with `codegen_*` prefixes
- Implementation details leak to entire crate

### Idiomatic Alternatives

#### Option 1: Private Submodules (Most Idiomatic)
```rust
// In grue_compiler/mod.rs
pub mod codegen;  // Only main module public

// In grue_compiler/codegen.rs (or codegen/mod.rs)
mod builtins;      // Private submodules
mod instructions;
mod objects;
mod extensions;

pub use builtins::*;      // Re-export what needs to be public
```

#### Option 2: Directory Structure
```
src/grue_compiler/
├── mod.rs                    # pub mod codegen;
└── codegen/
    ├── mod.rs               # Main codegen + pub use statements
    ├── builtins.rs          # Private implementation modules
    ├── instructions.rs
    ├── objects.rs
    └── extensions.rs
```

#### Option 3: Current Pattern (Pragmatic)
Keep existing `pub mod` structure for compatibility and consistency.

## Recommendation for Immediate Work

### For Adding `codegen_extensions.rs`

**Approach**: Follow current pattern for consistency

**Steps**:
1. Create `src/grue_compiler/codegen_extensions.rs`
2. Add to `mod.rs`: `pub mod codegen_extensions;`
3. Use same import pattern as other modules
4. Extend `ZMachineCodeGen` with `impl` blocks

**Rationale**:
- ✅ **Works with existing codebase**: No import changes needed
- ✅ **Consistent**: Matches established pattern
- ✅ **Low risk**: Proven approach used by other modules
- ✅ **Quick implementation**: Minimal ceremony

### Import Pattern for New Module
```rust
// In codegen_extensions.rs
use crate::grue_compiler::codegen::{ZMachineCodeGen, Operand};
use crate::grue_compiler::codegen_memory::MemorySpace;
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;

impl ZMachineCodeGen {
    // Extension methods here
}
```

## Future Architectural Improvements

### When to Consider Refactoring Module Structure

**Triggers for refactoring**:
- Making `grue_compiler` a public library crate
- Major architectural overhaul
- Need for stricter encapsulation
- API stability requirements

### Migration Strategy (Future)
1. **Phase 1**: Create new directory structure alongside current
2. **Phase 2**: Move modules one by one with compatibility shims
3. **Phase 3**: Update all import paths
4. **Phase 4**: Remove compatibility shims

## Current Working Examples

### Existing Module Extensions

**`codegen_builtins.rs`**: Extends `ZMachineCodeGen` with builtin functions
**`codegen_instructions.rs`**: Extends with instruction emission methods
**`codegen_objects.rs`**: Extends with object system generation
**`codegen_memory.rs`**: Provides memory management utilities

### Cross-Module Dependencies

Modules can freely import from each other since they're all public:
- `codegen.rs` imports from `codegen_memory`, `codegen_references`, etc.
- `codegen_builtins.rs` imports from `codegen` main module
- No circular dependency issues observed

## Conclusion

**For immediate codegen splitting needs**: Use current `pub mod` pattern
**For long-term architecture**: Consider private submodule structure
**Current status**: Working but not idiomatic - prioritize functionality over perfect structure

The current pattern enables safe, incremental refactoring without breaking the working compiler system.