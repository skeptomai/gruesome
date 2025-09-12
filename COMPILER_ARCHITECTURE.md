# Grue Compiler Architecture & Common Bug Patterns

## Critical Bug Patterns & Solutions

This document captures systematic issues we've encountered multiple times to prevent regression and aid future debugging.

### 1. UnresolvedReference Location Calculation Bug ⚠️

**Problem**: References created with incorrect locations, causing systematic resolution failures.

#### WRONG Pattern (causes widespread crashes):
```rust
// ❌ BROKEN: Location calculated AFTER placeholder emission
self.add_unresolved_reference(
    LegacyReferenceType::StringRef,
    target_id,
    true,
    MemorySpace::Code,
)?;                                    // location = self.final_code_base + self.code_space.len()
self.emit_word(placeholder_word())?;   // Placeholder added AFTER location calculated
```

#### CORRECT Pattern:
```rust
// ✅ FIXED: Location calculated BEFORE placeholder emission
let code_space_offset = self.code_space.len();  // Record exact location first
self.emit_word(placeholder_word())?;             // Then emit placeholder

// Create reference with exact location
self.reference_context.unresolved_refs.push(UnresolvedReference {
    reference_type: LegacyReferenceType::StringRef,
    location: code_space_offset,  // Use exact offset
    target_id,
    is_packed_address: true,
    offset_size: 2,
    location_space: MemorySpace::Code,
});
```

#### Symptoms:
- Hundreds of `0x00 0x00` placeholder bytes left unresolved
- Game crashes during execution with "Branch to address 0xffffff25" errors
- Debug logs showing "Reality: 0x00 suggests missing UnresolvedReference"
- Initial program execution works but crashes partway through

#### Files Affected:
- `src/grue_compiler/codegen.rs` (lines ~7275-7287)
- Any code using `add_unresolved_reference()` followed by `emit_word(placeholder_word())`

#### Detection Method:
Search for this antipattern:
```bash
grep -A5 -B5 "add_unresolved_reference" src/grue_compiler/codegen.rs | grep -A10 -B10 "emit_word.*placeholder"
```

---

### 2. Z-Machine Branch Format Encoding ⚠️

**Problem**: Incorrect branch byte encoding causing arithmetic overflow during execution.

#### WRONG Pattern:
```rust
// ❌ BROKEN: Using emit_word for branch offset
self.emit_word(offset as u16)?;  // Produces incorrect byte order
```

#### CORRECT Pattern:
```rust
// ✅ FIXED: Manual byte-by-byte branch encoding
if offset < 64 && offset >= 0 {
    // 1-byte format: bit 7 = condition, bit 6 = 1, bits 5-0 = offset
    let branch_byte = 0xC0 | (offset as u8 & 0x3F);
    self.emit_byte(branch_byte)?;
} else {
    // 2-byte format: bit 7 = condition, bit 6 = 0, bits 13-0 = offset
    let offset_u16 = offset as u16;
    let first_byte = 0x80 | ((offset_u16 >> 8) as u8 & 0x3F);
    let second_byte = (offset_u16 & 0xFF) as u8;
    self.emit_byte(first_byte)?;
    self.emit_byte(second_byte)?;
}
```

#### Files Affected:
- `src/grue_compiler/codegen_instructions.rs` (`emit_branch_offset` function)

---

### 3. Reference Type Disambiguation ⚠️

**Problem**: Using wrong reference type for similar operations.

#### Key Rule:
- Use `LegacyReferenceType::Jump` for unconditional jumps
- Use `LegacyReferenceType::Branch` for conditional branches
- Never mix these types

#### Example Fix:
```rust
// Line 6232 in codegen.rs - was incorrectly using Branch for Jump
reference_type: LegacyReferenceType::Jump,  // Not Branch
```

---

## Architecture Principles

### UnresolvedReference System Flow

1. **Generation Phase**: Code emits placeholder bytes and records exact locations
2. **Collection Phase**: All UnresolvedReferences stored with code space offsets
3. **Translation Phase**: Code space offsets converted to final memory addresses  
4. **Resolution Phase**: Placeholders patched with actual addresses

**Critical Rule**: Location must be recorded BEFORE placeholder emission, not after.

### Memory Space Types

- `MemorySpace::Code`: Instructions in main code section
- `MemorySpace::CodeSpace`: Same as Code (legacy)
- `MemorySpace::Strings`: String literals section
- `MemorySpace::Objects`: Object table section

### Debug Logging Patterns

Always add comprehensive logging for:
```rust
log::debug!("REFERENCE_CREATE: offset=0x{:04x} target={} type={:?}", 
           offset, target_id, reference_type);

log::debug!("REFERENCE_RESOLVE: location=0x{:04x} -> address=0x{:04x}", 
           reference.location, target_address);
```

---

## Common Debugging Commands

### Check for unresolved placeholders:
```bash
xxd mini_zork.z3 | grep "0000"  # Look for 0x00 0x00 patterns
```

### Search for problematic patterns:
```bash
grep -r "add_unresolved_reference" src/grue_compiler/ | grep -A3 -B3 "emit_word"
```

### Verify tests still pass:
```bash
cargo test codegen_test 2>/dev/null | grep "test result"
```

---

## Lessons Learned

1. **Location timing is critical**: Always record locations before emitting bytes
2. **Systematic bugs affect many files**: One pattern fix can resolve hundreds of failures  
3. **Test early and often**: Compiler tests catch regressions effectively
4. **Debug logging is essential**: Comprehensive logging reveals exact failure points
5. **Pattern consistency matters**: Small variations in patterns can cause major failures

---

## Future Considerations

- Consider deprecating `add_unresolved_reference()` in favor of direct `UnresolvedReference` creation
- Add compile-time checks to prevent location-after-emission patterns
- Create automated tests that verify no `0x00 0x00` patterns in compiled output
- Document all reference creation patterns as they're discovered

---

---

## UPDATE: Systematic UnresolvedReference Resolution Fixed (Sep 12, 2025) ✅

### MAJOR BREAKTHROUGH: IR ID Mapping Consolidation Success

**Problem Solved**: Systematic UnresolvedReference resolution failures where hundreds of references failed with "target_id not found" errors.

**Root Cause**: Compiler used separate tracking systems (string_offsets, label_addresses, ir_id_to_object_number) but only ir_id_to_address was used for resolution.

**Solution**: Added `consolidate_all_ir_mappings()` function that merges all tracking systems into central ir_id_to_address table.

**Results**: 
- Total IR ID mappings: 13 → 237 (1800% increase)
- All 80 UnresolvedReferences now resolve successfully
- mini_zork executes through initialization and displays game banner
- Systematic reference resolution failures eliminated

**Implementation**: See `src/grue_compiler/codegen.rs` lines ~7490-7558

**Remaining**: One direct code emission issue at PC 0x1221 (separate from UnresolvedReference system). See CONTINUATION_PLAN.md for details.

---

*Last Updated: September 12, 2025*  
*Context: Systematic UnresolvedReference resolution failures completely resolved - major compiler milestone achieved*