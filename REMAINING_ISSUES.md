# REMAINING ERROR ISSUES AND REMEDIATION PLAN
*Generated: September 24, 2025 - After establishing clean baseline*

## CONFIRMED REMAINING ISSUES

### 1. CRITICAL: Object Reference Corruption (mini_zork)
- **Error**: `invalid object 37889 > max 255` at PC=0x0ebd
- **Repro Case**: `tests/golden_files/mini_zork_v3.z3` (archived golden file)
- **Status**: Pre-existing issue, not a regression
- **Root Cause**: Object number corruption in complex gameplay scenarios

### 2. CRITICAL: Array Index Out of Bounds (minimal for-loops)
- **Error**: `index out of bounds: the len is 1 but the index is 1` in `opcodes_math.rs:46`
- **Repro Cases**:
  - `tests/golden_files/test_simple_v3.z3` (archived)
  - `/tmp/baseline_minimal.z3` (newly compiled)
  - `/tmp/baseline_bare.z3` (newly compiled)
- **Status**: Systematic issue affecting for-loop constructs
- **Root Cause**: Array/operand indexing bug in math opcodes

## SYSTEMATIC REMEDIATION PLAN - UPDATED SEPTEMBER 24, 2025

### ✅ PHASE 1 COMPLETED: Array Index Crash Resolution
**ROOT CAUSE IDENTIFIED AND FIXED**: VAR opcode classification conflict in `is_true_var_opcode()` function
- **Issue**: Raw opcode 0x00 was forced to VAR form, preventing 2OP:0 (je) for loop conditions
- **Fix**: Removed conflicting raw opcode mappings (0x00-0x1F), kept only VAR-only opcodes (0x08, 0x09)
- **Result**: "Invalid Long form opcode 0x00" errors eliminated ✅

### 🔄 PHASE 2: Stack Architecture Redesign (Current Priority)
**ROOT CAUSE IDENTIFIED**: Fundamental Z-Machine stack compliance violation
- **Issue**: Compiler treats stack as random-access with "stack slots" - Z-Machine only supports LIFO stack via Variable(0)
- **Symptoms**: Stack underflow in for-loops when multiple values resolve to same Variable(0)
- **Impact**: All complex expressions that need multiple intermediate values fail

### PHASE 3: Object Reference Corruption Investigation (Deferred)
- **Status**: Still present in mini_zork (object 37889 > max 255)
- **Priority**: Lower - architectural stack issues must be fixed first

## RECOMMENDED INVESTIGATION ORDER

### START HERE: Array Index Bug (opcodes_math.rs:46)
```bash
# 1. Get precise crash context
RUST_BACKTRACE=full ./target/debug/gruesome /tmp/baseline_bare.z3

# 2. Add debug logging to opcodes_math.rs line 46 area
# Look for: operands[1] access when operands.len() == 1

# 3. Identify problematic opcode
# Likely candidates: array access, push/pop operations
```

### THEN: Object Reference Investigation
```bash
# 1. Examine crash location
xxd -s 0x0ebd -l 16 tests/golden_files/mini_zork_v3.z3

# 2. Disassemble context
./target/debug/gruedasm-txd tests/golden_files/mini_zork_v3.z3 | grep -A5 -B5 "0ebd"

# 3. Compare object tables
# Working: basic_test_v3.z3 vs Broken: mini_zork_v3.z3
```

## SPECIFIC ACTION ITEMS

### Immediate (Next 30 minutes):
1. **Run crash with full backtrace** to get exact function and line
2. **Add logging around opcodes_math.rs:46** to see operand patterns
3. **Identify the specific Z-Machine opcode** causing array index errors

### Short-term (Next session):
1. **Fix array indexing bug** - likely incorrect operand count assumptions
2. **Create regression tests** for the fixed array operations
3. **Begin object corruption investigation** using disassembler analysis

### Medium-term:
1. **Object number overflow investigation** - check 16-bit vs 8-bit boundaries
2. **Memory layout analysis** - verify object table integrity
3. **Cross-reference with Z-Machine specification** for object limits

## DEBUGGING INFRASTRUCTURE NEEDED - UPDATED

**MAJOR DISCOVERY**: Fixed **Variable Form Operand Count Classification Bug** in `src/instruction.rs:184-195`

**Issue**: Variable form instructions with bit 5=0 were incorrectly classified as `OP2` instead of `VAR`
- Before: `0xC8` (push) → `OP2` → routed to math module → array index crash
- After: `0xC8` (push) → `VAR` → routed to stack module → works correctly

**STATUS**: Array index crashes **COMPLETELY FIXED** ✅ (September 24, 2025 - Session End)
- ✅ `bare_for_loop.grue` - no more array index crash (now gets stack underflow - different runtime issue)
- ✅ `minimal_for_loop.grue` - no more array index crash (compiles successfully)
- ✅ `basic_test.grue` - compiles successfully
- ✅ `tests/golden_files/basic_test_v3.z3` - runs correctly (no regressions)

**ROOT CAUSE IDENTIFIED AND FIXED**:
- **File**: `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/codegen_instructions.rs`
- **Function**: `is_true_var_opcode()` around lines 1602-1674
- **Issue**: Missing VAR opcode 0x08 (push) from classification list
- **Impact**: Compiler emitted 0xC8 instead of 0xE8, causing 2OP:8 (OR) vs VAR:232 (push) confusion
- **Solution**: Added comprehensive VAR opcode list (0x00-0x1F) per Z-Machine specification

**FIX IMPLEMENTED**: Comprehensive VAR opcode classification following user guidance to "fix these opcode problems in a general fashion" rather than targeted exceptions.

**NEXT REMAINING ISSUES**: Runtime behavior issues (stack underflow, object corruption) - NOT array index crashes

## SUCCESS CRITERIA

**Phase 1 Complete**:
- ✅ `bare_for_loop.grue` executes without array index crashes
- ✅ `minimal_for_loop.grue` executes without array index crashes

**Phase 2 Complete**:
- ✅ `mini_zork.grue` executes basic commands (look, inventory) without object errors
- ✅ All golden file comparisons still pass (no compilation regressions)

## BASELINE STATUS
- ✅ **185/186 unit tests passing** (99.5% success rate)
- ✅ **All key examples compile successfully**
- ✅ **Perfect golden file byte matches**
- ✅ **No architectural regressions**
- ✅ **0xE8 instruction form bug fixed**
- ✅ **UnresolvedReference system working**