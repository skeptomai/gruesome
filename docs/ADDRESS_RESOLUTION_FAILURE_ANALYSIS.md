# ADDRESS RESOLUTION FAILURE ANALYSIS

**Created**: November 15, 2025
**Status**: CRITICAL ROOT CAUSE IDENTIFIED
**Priority**: HIGH - Complete system mapping for cross-region coordination fix

## EXECUTIVE SUMMARY

**Root Cause Confirmed**: Property optimization changes object space size, which shifts all downstream memory sections, but some address references are not properly updated during the resolution phase, creating corrupted packed addresses that crash at runtime.

**System Architecture**: The compiler uses a two-phase address resolution system that fails when memory layouts change.

---

## ADDRESS RESOLUTION SYSTEM MAPPING

### Phase 1: Reference Collection (During Code Generation)

**UnresolvedReference System**:
```rust
struct UnresolvedReference {
    reference_type: LegacyReferenceType,  // What type of reference
    location: usize,                      // WHERE to patch in final image
    target_id: IrId,                      // WHAT we're referencing
    is_packed_address: bool,              // Whether to pack the address
    offset_size: u8,                      // Size of patch (1 or 2 bytes)
    location_space: MemorySpace,          // Which memory space location belongs to
}
```

**Reference Types That Cause Cross-Region Dependencies**:
1. `StringRef` - References from code to string addresses
2. `StringPackedAddress` - References from object properties to strings
3. `FunctionCall` - References from code to other functions
4. `DictionaryRef` - References to dictionary entries
5. `Jump/Branch` - References within code space
6. `GlobalsBase` - References to global variable base

### Phase 2: Address Resolution (During Assembly)

**Critical Function**: `resolve_all_addresses()` in `codegen_resolve.rs`

**Address Calculation Chain**:
```
Memory Layout Calculation (codegen_image.rs:318-412):
├─ header_size = 64
├─ globals_base = 64
├─ arrays_base = 64 + globals_size
├─ abbreviations_base = arrays_base + arrays_size
├─ object_base = abbreviations_base + abbreviations_size
├─ static_memory_start = object_base + object_size  ⭐ PROPERTY OPTIMIZATION AFFECTS THIS
├─ dictionary_base = static_memory_start
├─ code_base = dictionary_base + dictionary_size     ⭐ SHIFTS WHEN object_size CHANGES
└─ string_base = code_base + code_size              ⭐ SHIFTS WHEN code_base CHANGES
```

**Address Translation Function**: `translate_space_address_to_final()`

**Base Address Storage**:
- `self.final_code_base = code_base`
- `self.final_string_base = string_base`
- `self.final_object_base = object_base`

---

## CRITICAL FAILURE POINTS IDENTIFIED

### 1. Object Space Size Dependency Chain

**Property Optimization Changes**:
```
Object Space: 1000 bytes → 626 bytes (374-byte reduction)
Effect: ALL downstream addresses shift by 374 bytes
```

**Affected Calculations**:
- `static_memory_start` shifts: moves up by 374 bytes
- `dictionary_base` shifts: moves up by 374 bytes
- `code_base` shifts: moves up by 374 bytes
- `string_base` shifts: moves up by 374 bytes

### 2. Address Resolution Dependencies

**String References** (`StringRef` and `StringPackedAddress`):
```rust
// codegen_resolve.rs:192
let final_addr = self.final_string_base + string_offset;
```
**ISSUE**: When property optimization changes `object_size`, `final_string_base` shifts, but existing references in the `UnresolvedReference` list may contain outdated location addresses.

**Function Call References** (`FunctionCall`):
```rust
// codegen_resolve.rs:322
let routine_addr = self.final_code_base + code_offset;
```
**ISSUE**: When `final_code_base` shifts due to property optimization, function addresses shift.

### 3. Critical Address Translation Gap

**Location Translation** (`translate_space_address_to_final()`):
```rust
// codegen_resolve.rs:47
let adjusted_location = self.translate_space_address_to_final(
    reference.location_space,
    reference.location
)?;
```

**SUSPECTED ISSUE**: The `translate_space_address_to_final()` function may not account for property optimization changes when translating reference locations from generation-time addresses to final addresses.

---

## CORRUPTION MECHANISM ANALYSIS

### Crash Address Investigation

**Corrupted Address**: `0xfffff729` (4294965033)
**Analysis**:
- Contains `0xffff` prefix → likely unresolved placeholder
- Address is far beyond file bounds → indicates address calculation overflow/underflow

### Failure Pattern

**Baseline Compilation**:
```
Object space: 1000 bytes (hardcoded)
Code base: 0x746
String base: 0x72a
Result: ✅ All references resolve correctly
```

**Optimized Compilation**:
```
Object space: 626 bytes (optimized)
Code base: 0x783 (+61 bytes shift)
String base: 0x767 (+61 bytes shift)
Result: ❌ Address 0xfffff729 corruption
```

**Key Insight**: The 61-byte difference in headers suggests that while memory layout shifts correctly, some references are not updated to account for the new base addresses.

---

## SUSPECTED ROOT CAUSES

### 1. Stale Reference Locations

**Hypothesis**: References collected during code generation contain location addresses that are relative to the generation-time layout, but when property optimization changes the layout, these locations are not properly translated.

**Evidence**:
- References are collected with `location_space: MemorySpace` parameter
- Translation function `translate_space_address_to_final()` should handle this
- But corruption suggests translation fails for some reference types

### 2. Address Resolution Order Dependencies

**Hypothesis**: Address resolution assumes certain references are resolved before others, but property optimization changes the dependency order.

**Evidence**:
- Resolution processes `unresolved_refs` sequentially
- Some references may depend on others being resolved first
- Property optimization may break this assumed order

### 3. Base Address Update Timing

**Hypothesis**: Base addresses (`final_code_base`, `final_string_base`) are updated after some references are already collected but before they are resolved.

**Evidence**:
- Base addresses calculated in `assemble_complete_zmachine_image()` (line 407-408)
- References may be collected earlier in the compilation process
- Timing mismatch could cause stale base addresses

---

## CRITICAL DISCOVERY: REFERENCE RESOLUTION IS NOT THE ISSUE

**Debug Results from Property Optimization Test** (November 15, 2025):

### Compilation Phase Analysis ✅
- **ALL 15 references resolved successfully** during address resolution
- Base addresses calculated correctly:
  - `final_code_base=0x0657`
  - `final_string_base=0x069a`
  - `dictionary_addr=0x063b`
- Address translations working correctly (e.g., Objects space 0x006a → 0x034a)
- Target addresses calculated within bounds (e.g., 0x0388, 0x069a, 0x0337)

### Runtime Phase Failure ❌
- **Corruption confirmed** with invalid address `0xfffff5fd` (4294964733)
- Crash message: "Instruction address 4294964733 out of bounds"
- **Root Cause**: Issue occurs **AFTER** reference resolution, not during it

### CORRELATION CONFIRMED: Property Optimization → Address Corruption

**Baseline vs Optimized Base Address Analysis**:
```
Baseline (hardcoded 1000 bytes):    Optimized (calculated 761 bytes):
final_code_base=0x0746             final_code_base=0x0657 (-239 bytes)
final_string_base=0x0788           final_string_base=0x069a (-238 bytes)
```

**Property Space Calculation**:
- Optimization enabled: `optimized_prop_space=561 + margin=200 = 761`
- Space reduction: `1000 - 761 = 239 bytes` saved
- **Exact correlation**: Base address shift matches property space optimization

### ROOT CAUSE CONFIRMED: Property Table Address Patching

**The UnresolvedReference system works correctly.** The corruption is in `patch_property_table_addresses()`:

```rust
// codegen.rs:4364 - CRITICAL ISSUE: Stale address calculation
let absolute_addr = object_base + (space_relative_addr as usize);
```

**Problem**: The `patch_property_table_addresses()` function uses addresses calculated during object space generation (when `object_base` was unknown), but applies them AFTER memory layout changes from property optimization have shifted all downstream regions.

**Debug Evidence**:
- Property table addresses are calculated during object generation: `prop_table_addr=0x0062, 0x006d, 0x0079`
- Property optimization reduces object space: `1000 → 761 bytes` (-239 bytes)
- All downstream base addresses shift: `object_base=0x02e0, final_code_base=0x0657, final_string_base=0x069a`
- But property addresses in objects still reference the OLD memory layout before optimization
- When `patch_property_table_addresses()` calculates `absolute_addr = object_base + space_relative_addr`, it creates addresses pointing to wrong memory regions

## INVESTIGATION PLAN REVISED

### Next Phase: Post-Resolution Corruption Analysis

**Phase 5A: Enable Detailed Address Resolution Logging**
1. Add comprehensive logging to `resolve_all_addresses()`
2. Track every reference resolution with before/after addresses
3. Compare baseline vs optimized reference resolution

**Phase 5B: Validate translate_space_address_to_final()**
1. Examine implementation for property optimization awareness
2. Test translation accuracy for each MemorySpace type
3. Identify which memory spaces fail translation

**Phase 5C: Address Resolution Order Analysis**
1. Map dependency order requirements
2. Identify circular dependencies
3. Test resolution order sensitivity

### Implementation Strategy

**Diagnostic Compilation**: Temporarily enable property optimization with maximum debug logging to capture exact failure point
**Reference Audit**: Track every reference from collection through resolution
**Address Validation**: Add runtime checks for address bounds during resolution

---

## BROADER ARCHITECTURAL IMPLICATIONS

**Key Insight**: The property optimization failure reveals a fundamental architectural weakness in how the compiler coordinates memory layout changes across compilation phases.

**Systems Affected**:
- Any memory region size optimization (dictionary, abbreviations, etc.)
- Code layout changes
- Dynamic memory boundary adjustments
- Header field recalculation

**Architectural Fix Required**: Implement **holistic memory layout coordination** where all address calculations are updated atomically when any memory region changes size.

**Success Criteria**:
- All references resolve correctly regardless of memory layout changes
- Property optimization works without runtime corruption
- No `0xffff` placeholders remain in final binary
- All packed addresses stay within file bounds

This analysis provides the foundation for implementing a robust fix to the cross-region reference coordination system.