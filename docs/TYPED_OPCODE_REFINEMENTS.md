# Typed Opcode Enums - Refinement Analysis

## Context
This document captures the detailed design discussions for migrating from `emit_instruction(opcode: u8, ...)` to type-safe opcode enums.

## Decisions Made

### 1. Error Handling Strategy
**Decision**: Use panics for invalid opcode combinations, not Result propagation.

**Rationale**:
- Invalid opcode usage is a compiler bug, not a recoverable error
- Chained error propagation makes debugging harder
- Panics give immediate, clear stack traces
- Tests will catch these during development

**Implementation**:
```rust
pub fn emit_instruction(
    &mut self,
    opcode: Opcode,
    operands: &[Operand],
    store_var: Option<u8>,
    branch_offset: Option<i16>,
) -> Result<InstructionLayout, CompilerError> {
    // Validate operand count - PANIC on mismatch (compiler bug)
    if let Some(expected) = opcode.operand_count() {
        assert_eq!(
            operands.len(), expected,
            "COMPILER BUG: Opcode {:?} expects {} operands, got {}",
            opcode, expected, operands.len()
        );
    }

    // Validate store_var - PANIC if instruction doesn't store
    if store_var.is_some() && !opcode.stores_result() {
        panic!(
            "COMPILER BUG: Opcode {:?} does not store a result, but store_var={:?} was provided",
            opcode, store_var
        );
    }

    // Validate branch_offset - PANIC if instruction doesn't branch
    if branch_offset.is_some() && !opcode.branches() {
        panic!(
            "COMPILER BUG: Opcode {:?} does not branch, but branch_offset={:?} was provided",
            opcode, branch_offset
        );
    }

    // ... rest of implementation
}
```

### 2. Enum Generation Strategy
**Decision**: Generate opcode enums from Z-Machine specification data.

**Rationale**:
- Single source of truth (Z-Machine spec)
- Eliminates manual transcription errors
- Easy to update when targeting new Z-Machine versions
- Metadata (stores/branches/version) can be encoded in spec data

**Implementation Approach**:
Create a build script or code generator that reads Z-Machine spec data and generates the enum definitions.

**Spec Data Format** (e.g., `opcodes_spec.toml`):
```toml
[[op0]]
name = "Rtrue"
value = 0x00
number = 176
stores = false
branches = false
min_version = 1

[[op0]]
name = "Quit"
value = 0x0A
number = 186
stores = false
branches = false
min_version = 1

[[op1]]
name = "Jz"
value = 0x00
number = 128
stores = false
branches = true
min_version = 1

[[op1]]
name = "PrintPaddr"
value = 0x0D
number = 141
stores = false
branches = false
min_version = 1

[[op2]]
name = "Je"
value = 0x01
number = 1
stores = false
branches = true
min_version = 1

[[op2]]
name = "Add"
value = 0x14
number = 20
stores = true
branches = false
min_version = 1

[[var]]
name = "CallVs"
value = 0x00
number = 224
stores = true
branches = false
min_version = 1

[[var]]
name = "OutputStream"
value = 0x13
number = 243
stores = false
branches = false
min_version = 3
```

**Generator Script** (e.g., `build.rs` or `scripts/generate_opcodes.rs`):
```rust
// Reads opcodes_spec.toml and generates src/grue_compiler/opcodes_generated.rs
// with all enum definitions and trait implementations
```

### 3. Exhaustive Enums
**Question**: Should the enums be exhaustive? Or leave room for custom/experimental opcodes?

**Analysis**:
- **Exhaustive (recommended)**: All known opcodes defined in enum
  - Pros: Compiler catches missing match arms, prevents invalid opcodes
  - Cons: Need to add to enum for experimental features
  - **Can still extend**: Just add new variants for experimental opcodes

**Decision**: Use exhaustive enums, add experimental opcodes as variants.

**Example**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpVar {
    // Standard V3 opcodes
    CallVs = 0x00,
    Storew = 0x01,
    // ...

    // V4+ opcodes (experimental in our compiler)
    CallVs2 = 0x0C,
    EraseWindow = 0x0D,

    // Future/experimental - mark clearly
    #[cfg(feature = "experimental")]
    CustomOpcode1 = 0x20,
}
```

### 4. Const Contexts
**Question**: Do you need to construct opcodes in const contexts?

**Explanation of Const Contexts**:

Const contexts are places where Rust evaluates code at compile time:

```rust
// 1. Const declarations
const MY_OPCODE: Opcode = Opcode::Op0(Op0::Quit);

// 2. Static variables
static OPCODE_TABLE: [Opcode; 3] = [
    Opcode::Op0(Op0::Quit),
    Opcode::Op0(Op0::NewLine),
    Opcode::Op0(Op0::Rtrue),
];

// 3. Array sizes
const OPCODE_COUNT: usize = Op2::Throw as usize + 1;
struct OpcodeArray<T>([T; OPCODE_COUNT]);

// 4. Match patterns
fn handle_opcode(op: Op0) {
    match op {
        Op0::Quit => { /* ... */ },
        _ => { /* ... */ }
    }
}

// 5. Const functions (evaluated at compile time)
const fn is_arithmetic(op: Op2) -> bool {
    matches!(op, Op2::Add | Op2::Sub | Op2::Mul | Op2::Div | Op2::Mod)
}

const IS_ADD_ARITHMETIC: bool = is_arithmetic(Op2::Add); // Computed at compile time
```

**Why it matters**:
- If you need lookup tables built at compile time
- If you want compile-time validation
- If you want zero-runtime-cost abstractions

**Current usage in your codebase**:
Looking at your code, you mostly construct opcodes at runtime during code generation. You probably DON'T need const contexts unless you want:
- Compile-time opcode lookup tables
- Const validation in tests
- Static opcode metadata tables

**Recommendation**: Don't worry about const contexts initially. If needed later, we can add `const fn` to methods incrementally.

## Refined Type System Design

### Trait-based Metadata
```rust
pub trait OpcodeMetadata {
    fn raw_value(&self) -> u8;
    fn stores_result(&self) -> bool;
    fn branches(&self) -> bool;
    fn form(&self) -> InstructionForm;
    fn min_version(&self) -> u8;
}

// Auto-generated implementations from spec data
impl OpcodeMetadata for Op0 { /* ... */ }
impl OpcodeMetadata for Op1 { /* ... */ }
impl OpcodeMetadata for Op2 { /* ... */ }
impl OpcodeMetadata for OpVar { /* ... */ }
```

### Version Gating
```rust
impl CodeGen {
    pub fn emit_instruction(
        &mut self,
        opcode: Opcode,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        // Version check
        let min_version = match &opcode {
            Opcode::Op0(op) => op.min_version(),
            Opcode::Op1(op) => op.min_version(),
            Opcode::Op2(op) => op.min_version(),
            Opcode::OpVar(op) => op.min_version(),
        };

        if min_version > self.target_version {
            panic!(
                "COMPILER BUG: Opcode {:?} requires Z-Machine v{}, but targeting v{}",
                opcode, min_version, self.target_version
            );
        }

        // ... validation with panics
        // ... emit instruction
    }
}
```

### Convenience Constants
```rust
// Re-export common opcodes for ergonomics (auto-generated)
pub use crate::grue_compiler::opcodes::{
    // 0OP
    Op0::Quit as QUIT,
    Op0::NewLine as NEW_LINE,
    Op0::Rtrue as RTRUE,
    Op0::Rfalse as RFALSE,

    // 1OP
    Op1::PrintPaddr as PRINT_PADDR,
    Op1::Jz as JZ,
    Op1::Load as LOAD,
    Op1::Ret as RET,
    Op1::Jump as JUMP,

    // 2OP
    Op2::Je as JE,
    Op2::Jl as JL,
    Op2::Jg as JG,
    Op2::Add as ADD,
    Op2::Sub as SUB,
    Op2::Mul as MUL,
    Op2::Div as DIV,
    Op2::Store as STORE,

    // VAR
    OpVar::CallVs as CALL_VS,
    OpVar::PutProp as PUT_PROP,
    OpVar::Sread as SREAD,
    OpVar::PrintChar as PRINT_CHAR,
};
```

## Migration Strategy: Incremental (Chosen)

### Phase 1: Infrastructure (Day 1)
1. Create `opcodes_spec.toml` with all Z-Machine opcodes
2. Create `scripts/generate_opcodes.rs` generator
3. Generate `src/grue_compiler/opcodes.rs`
4. Add unit tests for generated enums
5. Run `cargo test` - should still pass (no usage yet)

### Phase 2: Parallel Implementation (Day 2)
1. Add `emit_instruction_typed()` alongside existing `emit_instruction()`
2. Add conversion layer: `emit_instruction(u8)` tries to map to typed version
3. Add deprecation warnings to old version
4. Run `cargo test` - should still pass

### Phase 3: Module-by-Module Migration (Days 3-7)
1. **Day 3**: Migrate `codegen_builtins.rs` (print, sread, etc.)
2. **Day 4**: Migrate `codegen_instructions.rs` (instruction emission)
3. **Day 5**: Migrate `codegen.rs` arithmetic and comparisons
4. **Day 6**: Migrate `codegen.rs` control flow
5. **Day 7**: Migrate remaining call sites

After each day: Run full test suite, commit working state.

### Phase 4: Cleanup (Day 8)
1. Remove old `emit_instruction(u8)` method
2. Rename `emit_instruction_typed()` to `emit_instruction()`
3. Final test run
4. Update documentation

## Rollback Strategy
- Each phase is committed separately
- Can revert to any phase if issues found
- Migration branch kept until fully validated
- All 170+ tests must pass before merging

## Expected Benefits
1. **Bug Prevention**: 0x13 disambiguation bug impossible
2. **Clear Intent**: `Opcode::Op2(Op2::Add)` vs mysterious `0x14`
3. **Version Safety**: Can't use V5 opcodes in V3 target
4. **Metadata Access**: `opcode.stores_result()` instead of lookup tables
5. **IDE Support**: Autocomplete shows valid opcodes
6. **Refactor Safety**: Compiler catches broken code after changes

## Open Risks
1. **Migration effort**: ~200+ call sites to update
2. **Verbosity**: Code becomes longer (mitigated by convenience constants)
3. **Learning curve**: Team needs to understand enum structure
4. **Performance**: Enum dispatch vs direct u8 (likely negligible)

---

*Created: 2025-10-04*
*Status: Pre-implementation planning*
