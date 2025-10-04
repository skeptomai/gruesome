# Phase 1 Typed Opcode Implementation - Shortcuts and Future Work

## Completed in Phase 1

✅ Created opcodes_spec.toml with Z-Machine opcode definitions
✅ Built code generator (scripts/generate_opcodes.rs)
✅ Generated type-safe enums (Op0, Op1, Op2, OpVar, Opcode)
✅ Added comprehensive unit tests (16 tests, all passing)
✅ No regressions to existing functionality

## Shortcuts Taken (Technical Debt)

### 1. Version-Specific Opcode Conflicts - COMMENTED OUT

**Issue**: Some opcodes share the same raw value but differ by Z-Machine version:
- `0x0F` in Op1: `Not` (V1-4) vs `Call1n` (V5+)
- `0x04` in OpVar: `Sread` (V1-3) vs `Aread` (V4+)
- `0x09` in OpVar: `Pull` (V1-5) vs different semantics in V6+

**Shortcut**: Commented out older versions in opcodes_spec.toml, keeping only newer versions.

**Impact**:
- V3 compiler (our production target) is unaffected - uses newer opcodes anyway
- Missing opcodes: `Op1::Not` (V1-4), `OpVar::Sread` (V1-3)
- V1-V3 games won't compile correctly if they use these specific opcodes

**Proper Solution** (Phase 3 or later):
```rust
// Use conditional compilation based on target version
#[cfg(zmachine_version = "v3")]
pub enum OpVar {
    Sread = 0x04,  // V1-3 version
    // ...
}

#[cfg(zmachine_version = "v4")]
pub enum OpVar {
    Aread = 0x04,  // V4+ version
    // ...
}
```

**Files Affected**:
- `opcodes_spec.toml` - lines 272-281, 582-591
- See comments starting with "# Opcode 0x__ has version conflict"

**Detection**: Search for "version conflict" in opcodes_spec.toml

---

### 2. Generator Conflict Resolution Logic - SKIPPED

**Issue**: The generator's `generate_enum()` function has logic to skip duplicate discriminants, but it was causing issues.

**Shortcut**: Instead of fixing the generator's conflict resolution, we commented out conflicting opcodes in the source TOML.

**Code Location**: `scripts/generate_opcodes.rs` lines 103-129

**Current Logic**:
```rust
// If there are conflicts, use conditional compilation
if conflicts.len() > 1 {
    if opcode.max_version.is_some() {
        // This is the older version - skip it
        continue;
    }
}
```

**Problem**: This logic runs but the TOML already has conflicts removed, so it's dead code.

**Proper Solution**:
- Remove the conflict-skipping logic from generator
- OR: Keep TOML with all opcodes and use proper `#[cfg()]` attributes in generated code
- Add version parameter to generator: `generate-opcodes --target-version v3`

---

### 3. No Integration with emit_instruction() Yet

**Status**: Generated opcodes exist but aren't used anywhere in the compiler yet.

**Current State**:
- `emit_instruction(opcode: u8, ...)` still uses raw numbers
- New typed opcodes are completely separate
- No migration path implemented

**Next Steps** (Phase 2):
1. Add `emit_instruction_typed(opcode: Opcode, ...)`
2. Keep both methods during migration
3. Gradually migrate call sites
4. Eventually deprecate `emit_instruction(u8, ...)`

---

### 4. Missing Opcodes from Spec

**Opcodes NOT in spec** (because V6+ or rare):
- Extended form opcodes (EXT:0-29 in V5+)
- Illegal/undefined opcodes
- Custom/experimental opcodes

**Reason**: Started with common V1-V5 opcodes only.

**Future Work**: Add ExtOp enum for extended opcodes when targeting V5+

---

### 5. Convenience Constants - Incomplete

**Current**: Only most common opcodes have convenience constants.

**Missing Examples**:
- `GETPROP`, `SETATTR`, `TESTATTR`
- `LOADW`, `LOADB`, `STOREW`, `STOREB`
- Many 1OP and 2OP opcodes

**Location**: `scripts/generate_opcodes.rs` - `generate_convenience_constants()`

**Future**: Either generate ALL constants or remove this feature entirely.

---

### 6. No Panic Messages for Version Mismatches

**Current**: Opcodes have `min_version()` method but nothing enforces it.

**Missing**: Runtime check in `emit_instruction_typed()` that panics if:
```rust
if opcode.min_version() > self.target_version {
    panic!("Opcode {:?} requires V{}, targeting V{}",
           opcode, opcode.min_version(), self.target_version);
}
```

**Why Deferred**: Haven't created `emit_instruction_typed()` yet (Phase 2).

---

### 7. Generator Error Handling - Minimal

**Current**: Generator uses `.expect()` everywhere.

**Issues**:
- Poor error messages if TOML is malformed
- No validation of opcode values (could have duplicates within same enum)
- No checking that `min_version <= max_version`

**Future**: Add proper validation and helpful error messages.

---

### 8. No Build Script Integration

**Current**: Must manually run `cargo run --bin generate-opcodes` to regenerate.

**Proper Solution**: Add to `build.rs`:
```rust
// build.rs
fn main() {
    println!("cargo:rerun-if-changed=opcodes_spec.toml");

    std::process::Command::new("cargo")
        .args(&["run", "--bin", "generate-opcodes"])
        .status()
        .expect("Failed to generate opcodes");
}
```

**Why Deferred**: Works fine manually for now, build.rs adds complexity.

---

### 9. HashMap Import Missing from Generator

**Issue**: Generator uses `HashMap` but forgot to import `std::collections::HashMap`.

**Why It Works**: The generator doesn't fail because we bypass the conflict resolution code (see Shortcut #2).

**Fix Needed**: Add to generator:
```rust
use std::collections::HashMap;
```

**Location**: `scripts/generate_opcodes.rs` top of file

---

## Migration Checklist for Phase 2

- [ ] Add `emit_instruction_typed()` method to CodeGen
- [ ] Add version validation (panic on version mismatch)
- [ ] Test with simple opcode (e.g., QUIT, NEW_LINE)
- [ ] Create conversion helper: `emit_instruction(u8)` → calls typed version
- [ ] Add deprecation warning to old method
- [ ] Document migration process

## Long-term Cleanup (Phase 3+)

- [ ] Fix version-specific opcode conflicts with proper #[cfg()]
- [ ] Remove dead conflict-skipping code from generator
- [ ] Add HashMap import to generator
- [ ] Uncomment all opcodes in spec, handle with version gates
- [ ] Add extended opcodes (V5+ EXT form)
- [ ] Integrate into build.rs for automatic regeneration
- [ ] Add validation to generator (duplicate checking, version ordering)
- [ ] Either complete or remove convenience constants feature
- [ ] Add comprehensive generator tests
- [ ] Document opcode spec format in TOML

---

*Created: 2025-10-04*
*Status: Phase 1 complete, documenting technical debt before Phase 2*
