# Conditional Compilation Bug Analysis - September 6, 2025

## Executive Summary

**CRITICAL BUG IDENTIFIED**: The Grue compiler has a fundamental architectural flaw in how it handles comparison operations (`==`, `<`, `>`, etc.) in conditional statements. The issue affects **all conditional logic** in compiled programs.

**Status**: 
- ✅ HashMap to IndexMap conversion successful - compiler now produces deterministic output
- ❌ **BLOCKING**: Comparison operations generate malformed Z-Machine bytecode causing runtime crashes
- ✅ Root cause identified with full scope analysis completed

## Root Cause Analysis

### The Core Problem

The compiler treats Z-Machine **branch instructions** (`je`, `jl`, `jg`) as **store instructions**, generating malformed bytecode:

**What the compiler generates**:
```rust
// WRONG: Treating je (jump-if-equal) as a store instruction
self.emit_instruction(
    0x01,                      // je opcode
    &[left_operand, right_operand],
    Some(0),                   // Store to stack - INVALID for branch instructions!
    None,                      // No branch offset - INVALID for branch instructions!
)?;
```

**What should be generated**:
```rust  
// CORRECT: je as a branch instruction
self.emit_instruction(
    0x01,                      // je opcode  
    &[left_operand, right_operand],
    None,                      // No store - branch instructions don't store
    Some(branch_offset),       // Branch target - required for branch instructions!
)?;
```

### Evidence from Runtime Execution

**Bytecode Analysis** (test_conditional.z3):
```
0x0366: 00        // Routine header (0 locals) ✅ CORRECT
0x0367: 8d 01 97  // print_paddr ✅ CORRECT  
0x036A: 11 01 02 00  // get_prop obj=1, prop=2, store to stack ✅ CORRECT
0x036E: 41 01 01  // je V01, #0001 ❌ WRONG - should read from stack, not V01
```

**Runtime Error**:
```
WARNING: Reading local variable 1 but routine only has 0 locals - returning 0
je V01, #0001 [FALSE +160] trying to read from V01 but routine only has 0 locals
BRANCH_OUT_OF_BOUNDS: new_PC=0x0411 >= memory_len=900 - INVALID BRANCH!
```

### The IR Mapping Inconsistency

**Problem**: IR mapping and Z-Machine instruction generation are inconsistent:

1. **get_prop instruction**: 
   - Debug: `"Using local variable 1 for get_prop result"`
   - Generated: `store_var=Some(0)` (stores to stack)
   - **Inconsistent**: IR thinks result is in local var 1, Z-Machine stores to stack

2. **je instruction**:
   - IR resolves operand: `"IR ID 6 resolved to Variable(1) [Local parameter]"`
   - Generated: `je Variable(1), constant` 
   - **Problem**: Variable 1 doesn't exist (routine has 0 locals)

## Affected Instructions & Scope

**All comparison operators are broken**:
- `IrBinaryOp::Equal` → `je` (0x01) ❌
- `IrBinaryOp::NotEqual` → `je` with negation ❌  
- `IrBinaryOp::Less` → `jl` (0x02) ❌
- `IrBinaryOp::Greater` → `jg` (0x03) ❌
- `IrBinaryOp::LessEqual` → `jg` with negation ❌
- `IrBinaryOp::GreaterEqual` → `jl` with negation ❌

**Impact on Grue language features**:
- ✅ String literals and printing work  
- ✅ Function calls work
- ✅ Property access (`get_prop`) works
- ❌ **If statements** - broken
- ❌ **While loops** - broken  
- ❌ **For loops** - broken
- ❌ **Ternary conditionals** - broken
- ❌ **Any conditional logic** - broken

## Multiple Conflicting Implementations Found

The codebase has **three different approaches** to handling comparisons:

### Approach 1: Store Instruction (Current, Broken)
```rust
// Lines with IrBinaryOp::Equal => 0x01 
self.emit_instruction(
    opcode, &[left_operand, right_operand],
    Some(0), // Store to stack - WRONG for branch instructions
    None,    // No branch - WRONG for branch instructions  
)?;
```

### Approach 2: Branch Instruction (Correct, But Not Used)
```rust
// emit_comparison_branch method
self.emit_comparison_branch(
    0x01, &[left_op, right_op], 
    true_label, false_label  // Proper branch targets
)?;
```

### Approach 3: Error Approach (Defensive, Unused)
```rust
// Later in code
return Err(CompilerError::CodeGenError(
    "Comparison operations should not be generated as direct binary operations"
));
```

**The conflict**: Different code paths use different approaches, causing inconsistent behavior.

## The "Zero Bytes Are NOT The Problem" Discovery

**Initial Misdiagnosis**: Debug output showed many `🚨 ZERO_BYTE_WRITE` errors, leading to suspicion of padding issues.

**Actual Reality**: The zero bytes are **legitimate Z-Machine bytecode**:
- `0x00` at address 0x0000 = Routine header (0 locals) ✅
- `0x00` in store variable positions = Stack variable (Variable 0) ✅  
- `0x00` in operand positions = Valid constant or variable values ✅

**The real issue**: Branch vs store instruction confusion, not zero bytes.

## Deterministic Compilation Verification

**Testing Results** (HashMap → IndexMap conversion):
```bash
# Multiple compilation runs of same file:
shasum test_conditional_run*.z3
6583047cd8c354a6bc105c2f2f3ecdb42bb86563  test_conditional_run1.z3
6583047cd8c354a6bc105c2f2f3ecdb42bb86563  test_conditional_run2.z3  
6583047cd8c354a6bc105c2f2f3ecb42bb86563  test_conditional_run3.z3

# Complex files also deterministic:
diff test_complex_run1.z3 test_complex_run2.z3         # No differences
diff test_progressive_run1.z3 test_progressive_run2.z3 # No differences
```

✅ **Deterministic compilation achieved** - HashMap conversion was successful.

## Fix Strategy & Architecture

### Required Changes

1. **Remove Store Instruction Approach**: Delete all code that treats comparison operators as store instructions

2. **Unify on Branch Instruction Approach**: Use only the `emit_comparison_branch` method for all comparisons

3. **Fix IR Mapping Consistency**: Ensure that:
   - If `get_prop` stores to stack (`store_var=Some(0)`), then IR should map result to `Variable(0)`
   - If `get_prop` stores to local var (`store_var=Some(1)`), then IR should map result to `Variable(1)`

4. **Validate Z-Machine Spec Compliance**: 
   - Branch instructions (`je`, `jl`, `jg`) MUST have branch offsets, never store variables
   - Store instructions (`add`, `sub`, `get_prop`) MUST have store variables, never branch offsets

### Implementation Plan

**Phase 1**: Fix the fundamental branch vs store confusion
- Audit all uses of opcodes 0x01, 0x02, 0x03 
- Ensure they're only generated with proper branch offsets
- Remove any store instruction usage of these opcodes

**Phase 2**: Fix IR mapping consistency  
- Trace get_prop result storage and ensure IR mapping matches
- Verify all comparison operand resolution is consistent

**Phase 3**: Test all conditional constructs
- If statements
- While loops  
- For loops
- Ternary conditionals
- Complex nested conditions

## Test Cases for Validation

**Simple conditional** (test_conditional.grue):
```grue
if player.location == 1 then
    print("Testing conditional...")
end
```

**Expected Z-Machine sequence**:
```
get_prop player, location -> stack    // Store result to stack
je stack, #1 ?(branch_true)          // Branch if equal  
print "Testing conditional..."        // True branch
branch_true: ...                     // Continue
```

**Current broken sequence**:
```
get_prop player, location -> stack    // Store result to stack  
je local_var_1, #1 -> stack          // WRONG: read non-existent local var, store result
```

## Files to Examine for Fix

**Primary**:
- `src/grue_compiler/codegen.rs` - Main compilation logic  
- `examples/test_conditional.grue` - Primary test case
- `examples/test_*conditional*.grue` - All conditional test cases

**Secondary**:
- `src/grue_compiler/ir.rs` - IR definitions
- `src/grue_compiler/semantic.rs` - IR generation  

## Historical Context

**Previous Session Status** (from CLAUDE.md):
```
### 🔧 REMAINING ISSUE - Variable Corruption:
- Problem: Variable 5 contains invalid object ID 38469 instead of valid object number  
- Location: PC=0x0bd2, get_prop instruction accessing corrupted variable
- Status: 90% functional - only variable handling bug remains
```

**Updated Understanding**: The "variable corruption" was actually this **comparison instruction bug**. The interpreter was crashing not due to corrupted variables, but due to malformed comparison instructions trying to read non-existent local variables.

## Next Session Action Items  

1. **Implement the fix** using the branch instruction approach
2. **Test fix** on all conditional test cases
3. **Verify** that complex conditionals and property access now work  
4. **Update CLAUDE.md** with success status once fixed

## Debug Information Preservation

**Key compiler debug patterns to watch**:
- `🚨 JE_INSTRUCTION_TRACE:` - Shows when je is incorrectly generated as store instruction
- `✅ resolve_ir_id_to_operand:` - Shows IR to operand mapping
- `🔧 UNIVERSAL_FIX: Using local variable X for get_prop result` - Shows inconsistent mapping

**Interpreter error patterns**:
- `WARNING: Reading local variable X but routine only has 0 locals` - Indicates IR mapping bug
- `BRANCH_OUT_OF_BOUNDS:` - Indicates invalid branch offset calculation

---

*Analysis completed September 6, 2025 - Ready for implementation phase*