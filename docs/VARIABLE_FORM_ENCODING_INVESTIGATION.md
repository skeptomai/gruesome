# Variable Form Encoding Investigation & Memory Layout Issues

**Date**: August 21, 2025  
**Status**: Partially Resolved - Major breakthrough on NOT instruction encoding, remaining memory layout issue  
**Priority**: High - Blocking 100% compilation success rate

## Executive Summary

Investigation into Variable form instruction encoding issues in the Grue compiler that were causing runtime failures. Successfully identified and fixed critical NOT instruction generation bugs, achieving major progress toward 100% compilation success. One remaining memory layout issue identified for future investigation.

## Issues Investigated

### 1. ✅ **RESOLVED: NOT Instruction Generation Bug**
**Problem**: Unary NOT operations were generating `je` (0x01) instructions instead of proper `not` (0x18) instructions.

**Root Cause**: In `src/grue_compiler/codegen.rs:2255`, the `generate_unary_op` function was incorrectly implemented:
```rust
// INCORRECT (before fix)
self.emit_instruction(0x01, &operands, store_var, None)?; // je (equals) instruction
```

**Fix Applied**:
```rust
// CORRECT (after fix)
self.emit_instruction(0x18, &operands, store_var, None)?; // not instruction
```

**Impact**: This was causing "Missing dynamic expression for IR ID" errors and property table corruption during runtime.

### 2. ✅ **RESOLVED: Variable Form Determination Bug** 
**Problem**: VAR:0x18 instructions were being encoded as Short form instead of Variable form.

**Root Cause**: In `src/grue_compiler/codegen.rs:4060`, the form determination logic was defaulting single-operand instructions to Short form:
```rust
1 => InstructionForm::Short, // 1OP - This was incorrect for VAR:0x18
```

**Fix Applied**: Added 0x18 to the special cases list:
```rust
// Special cases: certain opcodes are always VAR form regardless of operand count
match opcode {
    0x03 => InstructionForm::Variable, // put_prop is always VAR
    0x04 => InstructionForm::Variable, // sread is always VAR  
    0x07 => InstructionForm::Variable, // random is always VAR
    0x18 => InstructionForm::Variable, // not is always VAR - ADDED THIS
    0x20 => InstructionForm::Variable, // call_1n is always VAR
    // ...
}
```

**Impact**: This fixed the instruction byte generation from 0x98 (Short form) to 0xd8 (Variable form).

### 3. ✅ **RESOLVED: Property Table Memory Corruption**
**Previous Fix**: Earlier investigation resolved memory layout collision where property tables were overwriting dictionary/string data. Fixed by increasing property table allocation from 50 to 150 bytes per object.

## 🔄 **REMAINING: Memory Layout Gap Issue**

### Current Problem
- **Error**: "Invalid Long form opcode 0x00 at address 0069"
- **Observation**: Address 0x0069 contains null bytes instead of valid instructions
- **Scope**: Different from the Variable form encoding issues - appears to be a memory layout/address resolution problem

### Evidence
```bash
$ xxd -s 0x65 -l 16 mini_zork.z3
00000065: 0000 0000 0000 0000 0000 0000 0000 0000  ................
```

### Likely Causes

**1. Jump Target Miscalculation**
- Jump instructions calculating incorrect target addresses
- Unresolved references remaining as 0x0000
- Label-to-address mapping errors during address resolution phase

**2. Memory Layout Gaps**
- Gaps between header and first function
- Gaps between functions or code sections
- Incorrect memory allocation causing uninitialized regions

**3. Unresolved Reference**
- Address 0x0069 should point to actual code but remains unresolved
- Reference resolution phase not properly patching this address

## Investigation Steps for Memory Layout Issue

### 1. Trace Jump Instructions
```bash
# Search for jumps targeting 0x0069
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep "0069\|patch.*69"
```

### 2. Check Address Resolution
```bash
# Check address resolution phase for unresolved references
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep "resolve.*69\|unresolved"
```

### 3. Examine Memory Layout
```bash
# Look at memory layout decisions
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep "address.*69\|location.*69"
```

### 4. Validate Code Section Allocation
```bash
# Check if all code sections are properly allocated
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep "section\|allocation\|layout"
```

### 5. Jump Target Analysis
Examine the disassembly or debug output to find:
- Which instruction is trying to jump to 0x0069
- What that address should actually contain
- Whether it's a branch, call, or other control flow instruction

## Technical Context

### Z-Machine Instruction Forms
- **Short Form**: 0OP (bits 11) or 1OP (bits 10), operand types in bits 5-4
- **Long Form**: 2OP (bits 01), operand types in bits 6 and 5  
- **Variable Form**: VAR (bits 11 + bit 5 = 1), operand types follow in separate bytes
- **Extended Form**: 0xBE prefix (V5+), extended opcode follows

### VAR:0x18 Specification
- **Opcode**: 0x18 in Variable form
- **Name**: `not` (logical NOT)
- **Operands**: 1 operand (value to negate)
- **Store**: Yes (stores result)
- **Branch**: No
- **Version**: V5+ (but available in some V3 interpreters)

## Files Modified

### `src/grue_compiler/codegen.rs`
1. **Line 2253**: Fixed NOT instruction generation from 0x01 to 0x18
2. **Line 4057**: Added 0x18 to Variable form special cases

## Success Metrics

### ✅ Achieved
- NOT instructions now generate correct VAR:0x18 opcodes
- Variable form determination works correctly for 0x18
- Property table corruption eliminated
- "Missing dynamic expression" errors resolved

### 🔄 In Progress  
- Address 0x0069 memory layout gap resolution
- 100% compilation success rate for mini_zork.grue

## Debug Commands Reference

### Compilation Debug
```bash
# Full debug compilation
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue

# Focus on Variable form 0x18 instructions
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep -A3 -B1 "emit_instruction opcode=0x18"

# Check address resolution
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep "resolve\|patch\|unresolved"
```

### Runtime Debug
```bash
# Runtime execution with full debug
RUST_BACKTRACE=1 timeout 10s cargo run --bin gruesome mini_zork.z3

# Instruction decoding debug
RUST_LOG=debug timeout 5s cargo run --bin gruesome mini_zork.z3 2>&1 | grep "decode\|instruction"
```

### Memory Analysis
```bash
# Examine specific addresses in compiled file
xxd -s 0x65 -l 32 mini_zork.z3

# Check file size and layout
ls -la mini_zork.z3
```

## Related Documentation

- `CLAUDE.md` - Project instructions and VAR opcode 0x13 disambiguation 
- `docs/var-opcode-0x13-disambiguation.md` - Previous Variable form investigations
- `src/opcode_tables.rs:378` - Expected operand count for VAR:0x18
- Z-Machine Standard sections 4.4 (instruction forms) and 15 (opcodes)

## Next Steps

1. **Immediate**: Investigate the 0x0069 memory layout gap using the investigation steps above
2. **Short-term**: Implement proper address resolution validation to prevent null byte regions
3. **Long-term**: Add comprehensive memory layout validation to compiler pipeline

## Impact Assessment

**High Impact Fixes**: The NOT instruction generation and Variable form determination fixes resolved the core issues that were preventing proper Z-Machine bytecode generation for logical operations. This eliminates a major class of runtime failures.

**Remaining Impact**: The 0x0069 memory layout issue appears to be isolated and may be related to specific control flow patterns in mini_zork.grue rather than a systemic problem affecting all compilation.