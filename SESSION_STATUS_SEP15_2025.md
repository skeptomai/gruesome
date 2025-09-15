# Z-Machine Compilation Investigation Status - September 15, 2025

## CRITICAL ISSUE: Label Address Calculation Bug

**Current Status**: Program executes and prints "Pure branch test passed" but crashes with "Branch to address 0x03c3 is outside memory bounds"

**Root Cause Identified**: Label 7 points to wrong code space location
- **Expected**: Label 7 should point to print instruction at code space 0x09
- **Actual**: Label 7 points to code space 0x0011 (wrong location)
- **Impact**: Branch calculation uses wrong target address, causing out-of-bounds jump

## Code Space Layout Analysis

From debug output:
```
Code space first 10 bytes: [0, 224, 63, 255, 255, 186, 1, 0, 0, 13]
Hex representation: [00, e0, 3f, ff, ff, ba, 01, 00, 00, 0d]

Address mapping:
- 0x00: 00 (routine header)
- 0x01-0x08: Various bytes
- 0x09: 0d (print_paddr opcode in 1OP form) ← PRINT INSTRUCTION LOCATION
- 0x0011: Some other instruction ← WHERE LABEL 7 INCORRECTLY POINTS
```

**Key Discovery**: Print instruction is at code space offset 0x09, but Label 7 points to offset 0x0011

## Instruction Analysis Completed

**Je Instruction Analysis**: ✅ VERIFIED CORRECT
- Location: 0x000c -> 0x0011 (exactly 5 bytes as expected)
- Encoding: Proper 2OP long form je instruction
- No length calculation issues

**Final Assembly Phase**: ✅ VERIFIED CORRECT
- Code space copied wholesale to same base address (0x036a)
- No displacement during final assembly
- Issue is in initial code space address recording, not final assembly

## Major Fixes Applied This Session

### 1. ✅ FIXED: Systematic Operand Encoding Bug
**Problem**: `LargeConstant(1)` misinterpreted as IR ID 1
**Location**: `adapt_operand_for_long_form` in `codegen_instructions.rs`
**Impact**: Program now prints "Pure branch test passed" and executes much further
```rust
// REMOVED: Incorrect IR ID resolution logic that caused 0xFFFF placeholders
// OLD: Attempted to resolve LargeConstant values as potential IR IDs
// NEW: Trust that all IR IDs are already resolved by instruction emission time
```

### 2. ✅ FIXED: Wrong Opcode in emit_jz_branch
**Problem**: Using opcode 0x01 (je) instead of 0x00 (jz)
**Location**: `emit_jz_branch` function in `codegen.rs`
**Impact**: All 0xFFFF placeholders eliminated from instruction stream

### 3. ✅ ATTEMPTED: Deferred Label Processing (INEFFECTIVE)
**Problem**: Labels recorded at wrong timing during instruction emission
**Location**: `emit_instruction` in `codegen_instructions.rs`
**Result**: No effect - still records wrong location (0x0011 instead of 0x09)
**Status**: Recognized as ineffective approach

## Files Modified This Session

### Core Compiler Files:
- `src/grue_compiler/codegen_instructions.rs` - Major operand encoding fix + deferred label processing
- `src/grue_compiler/codegen.rs` - opcode fix, pending_label field, public label_addresses
- `src/bin/debug_bytecode.rs` - Corrected 1OP opcode mapping for debugging

### Documentation:
- `COMPILER_ARCHITECTURE.md` - Added Jump vs Branch location calculation patterns
- `SESSION_STATUS_SEP15_2025.md` - This comprehensive status document

### Test Files:
- `examples/test_pure_branch.grue` - Minimal test case for isolating branch issues

## Investigation Commands Used

### Binary Analysis:
```bash
env RUST_LOG=error cargo run --bin grue-compiler -- examples/test_pure_branch.grue --output /tmp/test_systematic_fix.z3
xxd /tmp/test_systematic_fix.z3 | head -20
```

### Debug Tracing:
```bash
env RUST_LOG=debug cargo run --bin grue-compiler -- examples/test_pure_branch.grue --output /tmp/debug.z3
```

### Execution Testing:
```bash
env RUST_LOG=info ./target/debug/gruesome /tmp/test_systematic_fix.z3
```

## Next Steps Required

### IMMEDIATE: Fix Label Address Calculation
**Research needed**: Why is Label 7 being recorded at code space offset 0x0011 when the print instruction is at offset 0x09?

**Investigation approach**:
1. Trace label creation during conditional compilation
2. Examine instruction emission order and address tracking
3. Identify where the 2-byte offset error originates (0x0011 - 0x09 = 0x08 difference)

### SYSTEMATIC: Pattern Analysis
**Check for similar bugs**: Are other labels also pointing to wrong locations?
**Verify**: Do all branch instructions have correct target calculation once labels are fixed?

## Test Program Behavior

**Current execution**:
1. ✅ Program starts correctly
2. ✅ Prints "Pure branch test passed"
3. ❌ Crashes with "Branch to address 0x03c3 is outside memory bounds"

**Expected execution**:
1. ✅ Program starts correctly
2. ✅ Prints "Pure branch test passed"
3. ✅ Program completes and exits normally

## Architecture Context

**Z-Machine Compliance**: All fixes maintain Z-Machine specification compliance
**Interpreter Status**: No interpreter modifications - all changes are compiler fixes
**Test Coverage**: 17 compiler tests still passing after all changes

## Session Continuation Notes

**Ready for Next Session**:
- All investigation work documented
- Critical bug location identified (Label 7 address calculation)
- Test program and debug commands established
- Files ready to commit and push to GitHub

**Key Insight for Next Session**: The fundamental issue is that during conditional compilation (if-else statements), the label for the "else" branch target is being recorded at the wrong code space offset. The print instruction lives at offset 0x09, but Label 7 points to offset 0x0011, causing branch calculations to jump to the wrong location and eventually outside memory bounds.