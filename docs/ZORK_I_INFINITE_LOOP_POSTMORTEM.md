# Zork I Infinite Loop Bug - Post-Mortem Analysis

## Executive Summary

**Date**: November 11, 2025
**Bug**: Zork I infinite loops during property access operations
**Root Cause**: Incorrect V3 property size calculation in `get_prop_len` opcode
**Impact**: Complete game-breaking failure, 1 month of undetected production issues
**Status**: FIXED ✅

## Timeline of Events

### October 9, 2025 - **THE BREAKING CHANGE**
**Commit 830fca89**: "fix: Add exit_data packed addresses and get_prop_len two-byte format support"

- **Intent**: Fix exit system by adding V3 two-byte property format support
- **Implementation**: Replaced simple 3-line `get_prop_len` with complex 60+ line logic
- **Critical Error**: Line 101 used `size_byte as usize` instead of `((size_byte >> 5) & 0x07) + 1`
- **Result**: `get_prop_len` returned raw bytes (242, 114, 50) instead of decoded sizes (1-8)

**SMOKING GUN**:
```rust
// BROKEN (Oct 9, 2025):
size_byte as usize  // Returns 242 instead of proper size

// WORKING (before Oct 9):
((size_byte >> 5) & 0x07) + 1  // Returns 8 for V3 properties
```

### October 9-November 11, 2025 - **SYMPTOM WHACK-A-MOLE**

Multiple commits attempted to fix infinite loop **symptoms** without identifying root cause:
- **bf4134b**: "fix: Prevent double insertion bug causing infinite loops in object iteration"
- **6845e3b**: "fix: CRITICAL - Object lookup infinite loop resolved with temporary workaround"
- **3567e4b**: "fix: Grammar system infinite loop - increment instruction bytecode generation"
- **42b968e**: "fix: Grammar system infinite loop - add object names to dictionary"

**Pattern**: Each fix addressed superficial symptoms while the underlying property calculation bug persisted.

### November 11, 2025 - **ROOT CAUSE DISCOVERY**
- Deep debugging with `RUST_LOG=debug` revealed property sizes of 242, 114, 50
- Git blame traced issue to exact line in commit 830fca89
- Fixed bit extraction logic to properly decode V3 property size field

## Root Cause Analysis

### What Went Wrong

1. **Insufficient Testing Protocol**: No Zork I gameplay testing after interpreter changes
2. **Complex Change Without Validation**: 60+ line rewrite of critical opcode without regression testing
3. **Symptom-Based Debugging**: Fixed consequences instead of investigating root causes
4. **Missing Z-Machine Spec Compliance**: Bit field extraction logic was incorrect

### The Technical Bug

**V3 Z-Machine Property Format**:
- Properties have size encoded in bits 7-5 of size byte as `(actual_size - 1)`
- Proper decoding: `((size_byte >> 5) & 0x07) + 1`
- Bug returned raw byte value instead of extracting 3-bit field

**Impact Cascade**:
```
get_prop_len returns 242 → property enumeration loops 242 times →
infinite memory access → object iteration hangs → game unplayable
```

## Process Improvements

### 1. **MANDATORY: Zork I Regression Testing**
- **Rule**: ANY change to `src/interpreter.rs`, `src/opcodes_*.rs`, or `src/vm.rs` MUST pass Zork I gameplay test
- **Test Sequence**: `open mailbox → take leaflet → read leaflet → north → east → inventory → score`
- **Automation**: Add Zork I regression test to CI pipeline
- **Enforcement**: Pre-commit hook prevents interpreter commits without Zork I validation

### 2. **Interpreter Change Review Protocol**
- **Complex Changes** (>20 lines): Require mandatory code review
- **Critical Functions** (`get_prop_len`, property access, object iteration): Extra scrutiny
- **Z-Machine Spec Compliance**: Cross-reference changes against official specification

### 3. **Root Cause Investigation Process**
- **Debug-First Rule**: Use `RUST_LOG=debug` traces before implementing fixes
- **Symptom Analysis**: When seeing "infinite loops", investigate property/memory access patterns
- **Historical Analysis**: Use `git blame` to identify when regressions were introduced

### 4. **Documentation Requirements**
- **Breaking Changes**: Document Z-Machine spec deviations in commit messages
- **Complex Logic**: Inline comments explaining bit field operations and format detection
- **Test Coverage**: Document which commercial games validate each interpreter change

## Lessons Learned

### ✅ **What Worked**
- Systematic debugging with detailed logging
- Git forensics to identify exact breaking commit
- Focused fix addressing root cause rather than symptoms

### ❌ **What Failed**
- Month-long period with broken core functionality
- Multiple commits fixing symptoms instead of root cause
- Lack of regression testing for critical game compatibility
- Complex interpreter changes without adequate validation

### 🎯 **Key Insight**
**Commercial Z-Machine games are our most reliable regression test.** Any interpreter change that breaks Zork I indicates a fundamental Z-Machine specification violation that will affect other games.

## Prevention Measures

### Immediate Actions (Implemented)
- ✅ Fixed root cause with proper V3 bit field extraction
- ✅ Added comprehensive debug logging for property operations
- ✅ Verified full Zork I gameplay restoration

### Future Safeguards (Required)
- [ ] Add Zork I gameplay test to CI pipeline
- [ ] Implement pre-commit hooks for interpreter changes
- [ ] Create Z-Machine compliance review checklist
- [ ] Establish debug-first investigation protocol for infinite loops

**The Rule**: **If it breaks Zork I, it breaks the Z-Machine specification.**