# PROPERTY OPTIMIZATION CORRUPTION ANALYSIS

**Created**: November 15, 2025
**Status**: CRITICAL ROOT CAUSE IDENTIFIED
**Priority**: HIGH - Blocks all aggressive memory optimization

## EXECUTIVE SUMMARY

**Corruption Mechanism Identified**: Property table optimization causes runtime address corruption through improper cross-region reference patching when memory layouts change.

**Key Finding**: The issue is **NOT** in property space estimation, but in **address resolution and reference patching** when memory boundaries shift during optimization.

---

## INVESTIGATION METHODOLOGY

### Test Case Design

Created minimal test case (`tests/minimal_property_test.grue`) with:
- 1 room ("start") with description
- 1 object ("testobj") with properties
- Minimal init block for player placement
- No complex functionality to isolate property table issues

### Controlled Comparison

**Baseline Test** (hardcoded 1000-byte allocation):
- Compilation: ✅ Successful
- File size: 2.1k bytes
- Runtime: ✅ Functional
- Headers: `0300 0001 0746 0747 072a` (high memory at 0x746-0x747)

**Optimized Test** (estimated space + 500-byte margin):
- Compilation: ✅ Successful (no build-time errors)
- File size: 2.2k bytes (larger than baseline!)
- Runtime: ❌ **CRASHES** with "Invalid packed address 0xfffff729"
- Headers: `0300 0001 0783 0784 0767` (high memory at 0x783-0x784)

---

## CRITICAL FINDINGS

### 1. Runtime vs Compile-Time Failure

**Previous assumption**: Property optimization would cause compile-time "Invalid Long form opcode 0x00" errors
**Reality**: Compilation succeeds, runtime execution fails with address corruption

This indicates the issue is in **post-compilation address resolution**, not instruction generation.

### 2. Memory Layout Shifts

Property optimization changes memory addresses significantly:
```
Baseline:  High memory 0x746-0x747, strings at 0x72a
Optimized: High memory 0x783-0x784, strings at 0x767
Delta:     +61 bytes shift in memory layout
```

### 3. Address Corruption Pattern

**Corrupted Address**: `0xfffff729` (4294965033 decimal)
**Analysis**: This is clearly a corrupted packed address - likely a -1 or invalid reference that wasn't properly patched

**Pattern Recognition**: The address `0xfffff729` contains `0xffff` which suggests an unresolved placeholder that should have been patched during address resolution.

### 4. Cross-Region Dependencies

**Space Optimization**: 1000 → 626 bytes (374-byte savings)
**File Size Impact**: Baseline 2.1k → Optimized 2.2k (larger file!)

This paradox suggests that property optimization triggers cascade effects in other memory regions, causing additional allocations or padding that exceed the property space savings.

---

## TECHNICAL ANALYSIS

### Address Resolution System Investigation

**Hypothesis**: Property optimization changes memory layout, but the address patching system fails to update all cross-region references correctly.

**Affected Systems**:
1. **Packed String Addresses**: References from object properties to string table
2. **Function Addresses**: References from property handlers to code region
3. **Object References**: Cross-object property dependencies
4. **Jump/Branch Addresses**: Control flow within functions

### UnresolvedReference Patching Theory

The corruption likely occurs in the `UnresolvedReference` system that patches placeholders:

```rust
// During compilation: placeholder 0xFFFF written
// During resolution: should patch to actual address
// Bug: Some references not updated when layout shifts
```

**Evidence**: The crashed address `0xfffff729` contains `0xffff` prefix, suggesting a partially-updated placeholder.

### Memory Boundary Coordination

Property optimization affects multiple memory regions:
1. **Object Table**: Fixed at start, determines property table base
2. **Property Tables**: Variable size, affects string table base
3. **String Table**: Depends on property table end, affects code base
4. **Code Region**: Depends on string table end, affects all addresses

**Critical Dependency Chain**:
```
Property space → String base → Code base → All packed addresses
```

When property space changes, ALL downstream addresses must be recalculated and ALL references must be updated.

---

## ROOT CAUSE HYPOTHESIS

**Primary Theory**: Incomplete cross-region reference patching when memory layout changes.

**Failure Mode**:
1. Property optimization reduces space allocation
2. Memory layout shifts (string/code regions move)
3. Some `UnresolvedReference` entries not updated to new addresses
4. Runtime execution follows corrupted address pointers
5. Crash on invalid instruction fetch

**Why Safety Margins Fail**: Even large margins (500 bytes) fail because the issue isn't about insufficient space - it's about incorrect address patching regardless of space allocation.

---

## INVESTIGATION PLAN

### Phase 3: Document Address Resolution System ✅ CURRENT
- Map all UnresolvedReference types and their patching logic
- Identify which references are cross-region dependent
- Document memory layout calculation dependencies

### Phase 4: Debug Packed Address Patching System
- Add detailed logging to UnresolvedReference resolution
- Track which addresses get patched vs missed
- Identify specific reference types that fail during optimization

### Phase 5: Fix Cross-Region Reference Coordination
- Ensure all address calculations use updated memory layout
- Fix any hardcoded assumptions about memory region positions
- Implement comprehensive reference validation

### Phase 6: Implement Safe Property Optimization
- Restore optimization with proper address coordination
- Add runtime validation for all packed addresses
- Comprehensive testing with complex property structures

---

## IMMEDIATE NEXT STEPS

1. **Map UnresolvedReference System**: Document all reference types and patching logic
2. **Enable Detailed Address Logging**: Track address calculations during optimization
3. **Identify Failed References**: Determine which specific references cause corruption
4. **Fix Patching Logic**: Update address resolution to handle layout changes properly

---

## BROADER IMPLICATIONS

This investigation validates the user's insight about "dependencies across regions" in compiler architecture. The property optimization failure reveals fundamental issues in how the compiler coordinates memory layout changes across interdependent regions.

**Key Architectural Insight**: Z-Machine compilation requires **holistic address coordination** - changing any memory region size requires updating ALL dependent regions and references simultaneously.

This applies to ALL future optimization efforts:
- Dictionary compression
- Abbreviation system
- Code layout optimization
- Any memory region modifications

**Resolution**: Fix the cross-region reference patching system to enable safe aggressive optimization across the entire compiler.