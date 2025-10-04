# Phase 3: Migration to Typed Opcodes - Plan and Progress

## Overview

Phase 1 & 2 created infrastructure for type-safe opcodes. Phase 3 is the actual migration
of call sites from `emit_instruction(u8, ...)` to `emit_instruction_typed(Opcode, ...)`.

## Migration Strategy: Incremental by Module

Migrate one module at a time, running tests after each migration to ensure no breakage.

### Module Priority (in order):

1. **codegen_builtins.rs** - Built-in functions (print, quit, etc.)
   - Estimated: ~20-30 call sites
   - Low risk: Well-tested functions
   - High value: Clear examples for future migrations

2. **codegen_instructions.rs** - Test/validation code
   - Estimated: ~10 call sites (mostly in tests)
   - Low risk: Tests will validate themselves

3. **codegen.rs** - Main code generation (arithmetic, comparisons, control flow)
   - Estimated: ~100-150 call sites
   - Medium risk: Core compiler logic
   - Split into sub-sections:
     - 3a. Arithmetic operations (ADD, SUB, MUL, DIV, MOD)
     - 3b. Comparison/branch operations (JE, JL, JG)
     - 3c. Control flow (JUMP, function calls)
     - 3d. Variable/stack operations (LOAD, STORE, PUSH, PULL)
     - 3e. Object/property operations
     - 3f. Remaining operations

4. **codegen_objects.rs** - Object table generation
   - Estimated: ~5-10 call sites
   - Low risk: Less frequently changed code

5. **codegen_strings.rs** - String encoding
   - Estimated: ~5-10 call sites
   - Low risk: Isolated functionality

## Migration Pattern

### Before (raw u8):
```rust
self.emit_instruction(0x14, &[op1, op2], Some(0), None)?;  // What is 0x14?
```

### After (typed):
```rust
use super::opcodes::*;

self.emit_instruction_typed(
    Opcode::Op2(Op2::Add),  // Clear intent
    &[op1, op2],
    Some(0),
    None
)?;

// Or using convenience constant:
self.emit_instruction_typed(ADD, &[op1, op2], Some(0), None)?;
```

### Search/Replace Strategy

1. **Find all emit_instruction calls in file**:
   ```bash
   grep -n "emit_instruction(" src/grue_compiler/codegen_builtins.rs
   ```

2. **Common patterns to migrate**:
   ```rust
   // Pattern 1: 0OP instructions
   0x0A => QUIT
   0x0B => NEWLINE
   0x00 (0OP) => RTRUE
   0x01 (0OP) => RFALSE

   // Pattern 2: 1OP instructions
   0x0D => PRINTPADDR
   0x0C => JUMP
   0x0E => LOAD
   0x0B => RET

   // Pattern 3: 2OP instructions
   0x01 => JE
   0x02 => JL
   0x03 => JG
   0x14 => ADD
   0x15 => SUB
   0x16 => MUL
   0x17 => DIV
   0x18 => MOD
   0x0D => STORE

   // Pattern 4: VAR instructions
   0x00 (VAR) => CALLVS
   0x03 => PUTPROP
   0x04 => SREAD (V3) / AREAD (V4+)
   0x05 => PRINTCHAR
   0x06 => PRINTNUM
   ```

3. **Context-dependent mappings** (need to check form):
   ```rust
   // Opcode 0x00 could be:
   - Op0::Rtrue (0OP form, no operands)
   - Op1::Jz (1OP form, 1 operand)
   - OpVar::CallVs (VAR form, variable operands)

   // Opcode 0x01 could be:
   - Op0::Rfalse (0OP form, no operands)
   - Op1::GetSibling (1OP form, 1 operand)
   - Op2::Je (2OP form, 2 operands)
   - OpVar::Storew (VAR form, variable operands)
   ```

## Validation After Each Migration

After migrating each module:

1. **Run full test suite**:
   ```bash
   cargo test --lib
   ```

2. **Check for panics** (validation catching bugs):
   ```bash
   cargo test --lib 2>&1 | grep "COMPILER BUG"
   ```

3. **Verify no regressions**:
   - All existing tests still pass
   - No new failures introduced

4. **Commit the module**:
   ```bash
   git add -A
   git commit -m "refactor: Migrate [module] to typed opcodes"
   ```

## Expected Benefits After Migration

### Immediate (during migration):
- Validation catches incorrect store_var usage
- Validation catches incorrect branch_offset usage
- Version mismatches caught at compile time

### Long-term:
- Self-documenting code (QUIT vs 0x0A)
- IDE autocomplete for available opcodes
- Refactoring safety (renaming caught by compiler)
- Form confusion eliminated (can't use 2OP opcode as VAR)

## Tracking Progress

### Phase 3.1: codegen_builtins.rs
- [ ] Count total emit_instruction calls
- [ ] Identify unique opcode patterns
- [ ] Migrate print-related functions
- [ ] Migrate quit/control functions
- [ ] Run tests
- [ ] Commit

### Phase 3.2: codegen_instructions.rs tests
- [ ] Count total emit_instruction calls in tests
- [ ] Migrate test helpers
- [ ] Run tests
- [ ] Commit

### Phase 3.3a: codegen.rs - Arithmetic
- [ ] Migrate ADD, SUB, MUL, DIV, MOD
- [ ] Run tests
- [ ] Commit

### Phase 3.3b: codegen.rs - Comparisons/Branches
- [ ] Migrate JE, JL, JG, JZ
- [ ] Migrate conditional branches
- [ ] Run tests
- [ ] Commit

### Phase 3.3c: codegen.rs - Control Flow
- [ ] Migrate JUMP, CALL_VS
- [ ] Migrate function calls
- [ ] Run tests
- [ ] Commit

### Phase 3.3d: codegen.rs - Variables/Stack
- [ ] Migrate LOAD, STORE, PUSH, PULL
- [ ] Run tests
- [ ] Commit

### Phase 3.3e: codegen.rs - Objects/Properties
- [ ] Migrate object operations
- [ ] Migrate property operations
- [ ] Run tests
- [ ] Commit

### Phase 3.3f: codegen.rs - Remaining
- [ ] Migrate all remaining calls
- [ ] Run tests
- [ ] Commit

### Phase 3.4: codegen_objects.rs
- [ ] Count calls
- [ ] Migrate all
- [ ] Run tests
- [ ] Commit

### Phase 3.5: codegen_strings.rs
- [ ] Count calls
- [ ] Migrate all
- [ ] Run tests
- [ ] Commit

## Phase 4: Cleanup (After All Migrations)

Once ALL modules are migrated:

1. **Deprecate old method**:
   ```rust
   #[deprecated(
       since = "0.7.0",
       note = "Use emit_instruction_typed() for type safety"
   )]
   pub fn emit_instruction(
       &mut self,
       opcode: u8,
       // ...
   ) -> Result<InstructionLayout, CompilerError> {
       // Keep implementation for compatibility
   }
   ```

2. **Eventually remove** (major version bump):
   - Remove `emit_instruction(u8)` entirely
   - Rename `emit_instruction_typed()` to `emit_instruction()`
   - Update all documentation

## Risk Mitigation

### Low Risk Indicators:
- ✅ All migrations are local (single file at a time)
- ✅ Full test suite validates after each change
- ✅ Git commits allow easy rollback
- ✅ Typed version delegates to tested u8 version

### High Risk Areas (extra caution):
- ⚠️ Context-dependent opcodes (0x00, 0x01, 0x0D, 0x13)
- ⚠️ Dynamic opcode selection (if any)
- ⚠️ Opcodes in loops/conditionals

### Rollback Plan:
If any migration causes issues:
```bash
git revert HEAD  # Undo last commit
# Or:
git reset --soft HEAD~1  # Unstage changes but keep edits
```

## Current Status

- **Phase 1**: ✅ Complete (bcb7951)
- **Phase 2**: ✅ Complete (2516916)
- **Phase 3**: 🔄 In Progress
  - Starting with: codegen_builtins.rs

---

*Last Updated: 2025-10-04*
*Next: Begin Phase 3.1 - codegen_builtins.rs migration*
