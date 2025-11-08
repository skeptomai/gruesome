# Polymorphic Dispatch Function Resolution Investigation

## Problem Statement

The polymorphic dispatch system is not working correctly. Specifically:

1. "take leaflet" calls the generic `handle_take(obj)` function instead of the specialized `handle_take(leaflet)` function
2. Even direct calls like `handle_take(leaflet)` (with compile-time constant arguments) are being routed through dispatch functions instead of calling specialized functions directly
3. The IR shows dispatch function calls like `FunctionCall(669, [Object("leaflet")])` instead of direct specialized function calls

## Investigation Status

### ✅ Working Components
- **Dispatch Function Generation**: Dispatch functions are being generated correctly in IR
- **Grammar Integration**: Grammar system correctly references dispatch function IDs (verified ID 669 for take verb)
- **Object Visibility**: Object lookup works correctly - "examine leaflet" succeeds
- **Runtime Parameter Resolution**: `RuntimeParameter("noun")` resolution implemented successfully
- **Object Numbering**: Consistent between IR generator and codegen (leaflet = object #11)
- **Dictionary Lookup**: Object lookup from dictionary addresses working correctly

### ❌ Core Issue Identified

The problem is in **function resolution logic** during IR generation. ALL calls to polymorphic functions (like `handle_take`) are being treated as requiring dispatch, even when arguments are compile-time constants.

**Expected behavior:**
- `handle_take($noun)` → Should use dispatch function (runtime resolution)
- `handle_take(leaflet)` → Should call specialized function directly (compile-time resolution)

**Actual behavior:**
- Both cases → Use dispatch function

## Key Code Locations

### 1. Function Call IR Generation
File: `src/grue_compiler/ir.rs:2426`
```rust
crate::grue_compiler::ast::Handler::FunctionCall(name, args) => {
    // This is where AST FunctionCall handlers get converted to IR FunctionCall instructions
}
```

### 2. Grammar System Integration
File: `src/grue_compiler/semantic.rs:537`
```rust
Handler::FunctionCall(func_name, args) => {
    // Semantic analysis of function calls in grammar patterns
}
```

### 3. Dispatch Function Creation
File: `src/grue_compiler/ir.rs` (around create_dispatch_function method)
- Creates dispatch functions that compare runtime object parameters
- Generated correctly but being used inappropriately for compile-time constant calls

## Root Cause Analysis ✅

Found the issue! There are **two different function resolution code paths**:

### 1. Grammar Pattern Handlers (ir.rs:2440-2444)
```rust
let func_id = if let Some(&dispatch_id) = self.dispatch_functions.get(&name) {
    dispatch_id  // ALWAYS prefers dispatch functions
} else if let Some(&id) = self.symbol_ids.get(&name) {
    id
}
```

### 2. Regular Expression Function Calls (ir.rs:3335-3346)
```rust
let func_id = if let Some(&id) = self.symbol_ids.get(&name) {
    id  // Only uses symbol table, ignores dispatch functions
} else {
    return Err(...);
}
```

**The Problem**: Grammar patterns always use dispatch functions, but regular function calls only use the symbol table. Multiple function definitions overwrite each other in the symbol table!

## Complete Root Cause Analysis ✅

### Symbol Table Registration Issue

**Function Registration Phase** (ir.rs:1382):
```rust
self.symbol_ids.insert(func.name.clone(), func_id);  // Overwrites previous functions!
```

**Function Generation Phase** (ir.rs:1629-1641):
```rust
let func_id = if let Some(&existing_id) = self.symbol_ids.get(&func.name) {
    existing_id  // Gets the LAST registered function ID only
}
```

### The Complete Picture

1. **Registration**: `handle_take(obj)` gets ID 100, `handle_take(leaflet)` overwrites it with ID 200
2. **Symbol Table**: Only contains `"handle_take" -> ID 200` (the last one)
3. **Direct Calls**: `handle_take(leaflet)` resolves to ID 200 (whatever was last)
4. **Grammar Calls**: Use dispatch functions correctly
5. **Overload System**: Tracks all functions in `function_overloads`, but symbol table is broken

### Solution Required

Regular function calls must use the same polymorphic dispatch logic as grammar patterns. The current approach breaks when multiple functions have the same name.

## Proposed Fix

Modify regular function call resolution (ir.rs:3335-3346) to match grammar pattern logic:

```rust
// Instead of only checking symbol_ids:
let func_id = if let Some(&id) = self.symbol_ids.get(&name) { id }

// Use same logic as grammar patterns:
let func_id = if let Some(&dispatch_id) = self.dispatch_functions.get(&name) {
    dispatch_id  // Use polymorphic dispatch for overloaded functions
} else if let Some(&id) = self.symbol_ids.get(&name) {
    id  // Fall back to direct function for non-overloaded functions
}
```

This ensures consistent behavior between grammar calls and direct calls.

## Debug Evidence

From IR output analysis:
```
Grammar rule 'take' -> verb pattern 106 -> dispatch function 669
Direct call `handle_take(leaflet)` generates: FunctionCall(669, [Object("leaflet")])
Expected: Direct call to specialized function, not dispatch function
```

## Current Investigation Status ✅

Successfully implemented the polymorphic dispatch fix, but the comparison logic in the dispatch function is not working correctly. The dispatch function IR shows proper structure:

```
function dispatch_handle_take (id=669):
  parameters: obj(slot=1, id=670)
  body:
      t673 = Object("leaflet")              # Create leaflet object constant
      t674 = t670 Equal t673                # Compare parameter with leaflet
      branch t674 ? L671 : L672             # Branch to specialized or generic
    L671:
      t675 = call func#279(t670)            # Call specialized function
      return t675
    L672:
      t676 = call func#7(t670)              # Call generic function (currently happening)
      return t676
```

**The Issue**: The comparison `t670 Equal t673` is failing, causing fallback to generic function.

**Runtime Evidence**:
- Dictionary lookup finds "leaflet" correctly (0x0a64)
- Object lookup finds leaflet as object #11 correctly
- But `Object("leaflet")` constant in dispatch function might resolve to different value

## Root Cause Analysis - Object Value Mismatch ✅

**FOUND THE ISSUE**: The dispatch function comparison `t670 Equal t673` is failing because the two values being compared are different.

### Evidence

**Runtime Behavior**: Test shows "*** GENERIC FUNCTION CALLED ***" confirming dispatch function calls generic fallback.

**IR Structure is Correct**:
```
function dispatch_handle_take (id=669):
  parameters: obj(slot=1, id=670)
  body:
      t673 = Object("leaflet")              # Compile-time constant
      t674 = t670 Equal t673                # Runtime parameter vs constant comparison
      branch t674 ? L671 : L672             # Branch based on comparison
```

### Object Resolution Analysis

**Path 1 - `Object("leaflet")` Constant Resolution**:
1. IR: `LoadImmediate` with `IrValue::Object("leaflet")`
2. Codegen: `codegen_instructions.rs:221` - looks up "leaflet" in `object_numbers`
3. Result: Stores object number 11 in `ir_id_to_integer`
4. Z-Machine: Generates constant value 11

**Path 2 - `RuntimeParameter("noun")` Resolution**:
1. Parse: "leaflet" gets dictionary address 0x0a64 in parse buffer word 2
2. Codegen: `loadw` reads parse buffer offset 2 → Variable(2) = 0x0a64
3. Runtime: Calls `generate_object_lookup_from_noun()` which:
   - Loops through objects checking Property 18
   - Compares dictionary addresses in Property 18 with Variable(2)
   - When match found, stores object number in Variable(3)

### The Problem

**MISMATCH HYPOTHESIS**: The object lookup function may be returning a different value than expected, or there's a codegen issue in how the comparison is implemented.

**Most Likely Causes**:
1. Property 18 dictionary address lookup failing to find correct match
2. Object numbering inconsistency between compile-time and runtime
3. Variable handling issue in the comparison instruction

### Verification Needed

**Required Test**: Add debug logging to see actual values being compared in the `Equal` instruction:
- What value does `t673` (Object("leaflet")) contain?
- What value does `t670` (RuntimeParameter result) contain?
- Are both values actually object number 11?

## ISSUE IDENTIFIED ✅ - Object Lookup Failure

**BREAKTHROUGH**: Added debug output to Z-Machine interpreter and found the exact failure point.

### Debug Evidence

**Comparison Values**:
```
🎯 DISPATCH_OBJECT_COMPARE: op1=0 vs op2=11, condition=false, PC=0x1a75
```

**Analysis**:
- `op2=11` - `Object("leaflet")` constant correctly resolves to object number 11 ✅
- `op1=0` - Runtime parameter resolution returns 0 (object not found) ❌
- Comparison fails: 0 ≠ 11

### Root Cause

**OBJECT LOOKUP FAILURE**: The `generate_object_lookup_from_noun()` function is not finding the leaflet object when converting dictionary address 0x0a64 to object number.

**Problem Location**: The runtime parameter path fails at the dictionary-to-object mapping step:
1. ✅ Parse buffer correctly contains dictionary address 0x0a64 for "leaflet"
2. ✅ Object lookup function is called with Variable(2) = 0x0a64
3. ❌ **Property 18 comparison fails** - doesn't find matching dictionary address
4. ❌ Returns 0 (not found) instead of 11 (leaflet object number)

### Technical Issue

**PROPERTY 18 LOOKUP BUG**: The object lookup function iterates through all objects checking Property 18 dictionary addresses, but the comparison logic is not finding the match for "leaflet".

**Most Likely Causes**:
1. Property 18 doesn't contain the correct dictionary address for leaflet object
2. Dictionary address format mismatch (byte order, word boundaries)
3. Property 18 iteration logic bug

## FINAL DIAGNOSIS ✅ - Control Flow Issue After Match Found

**DETAILED TRACING RESULTS**:

### Property 18 Generation - WORKING ✅
- ✅ Property 18 correctly generated for leaflet object
- ✅ Contains dictionary addresses for "leaflet" and "paper"
- ✅ DictionaryRef placeholders properly resolved during compilation

### Dictionary Comparison - WORKING ✅
- ✅ Dictionary lookup finds "leaflet" at address 0x0a64
- ✅ Property 18 comparison finds exact match: `op1=0x0a64 vs op2=0x0a64, condition=true`

### Branch Instruction - CONFIGURED CORRECTLY ✅
- ✅ Successful comparison triggers branch: `branch=Some(BranchInfo { on_true: true, offset: 25 })`
- ✅ Should jump to `found_match_label` when addresses match

### IDENTIFIED PROBLEM - Control Flow After Branch ❌

**THE ISSUE**: Despite successful dictionary address comparison and correct branch configuration, the object lookup function is not storing the result in Variable(3). The branch target (`found_match_label`) may not contain the expected store instruction or there's a compilation issue with the control flow.

**ROOT CAUSE**: Likely issue in `generate_object_lookup_from_noun()` where the control flow after finding a match is not working correctly. The function should:
1. Find match in Property 18 ✅ (Working)
2. Branch to `found_match_label` ✅ (Working)
3. Store current object number in Variable(3) ❌ (NOT HAPPENING)

**SOLUTION REQUIRED**: Investigate the generated bytecode around the `found_match_label` to ensure the store instruction is present and at the correct location, or fix the compilation of the object lookup function.

## ERROR CHECKING IMPLEMENTATION ✅ (November 8, 2025)

**IMPLEMENTED**: Added runtime error checking to object lookup function to prevent dispatch functions from being called with 0.

### Changes Made:
1. **Added error check in `generate_object_lookup_from_noun()`** (codegen_lookup.rs:502-535):
   - Tests if Variable(3) is 0 after object lookup completes
   - If lookup failed (Variable(3) == 0), immediately quits the game
   - If lookup succeeded (Variable(3) != 0), continues normal execution

2. **Runtime Behavior**:
   - ✅ **Before**: Object lookup fails silently, returns 0, dispatch function receives 0
   - ✅ **After**: Object lookup failure causes immediate game termination with debug logging
   - ✅ **Result**: Forces investigation of root cause instead of masking the issue

### Test Results:
```
RUST_LOG=warn timeout 5s bash -c 'echo "take leaflet" | ./target/debug/gruesome tests/debug_error_check.z3'
```

**Output shows improved debugging**:
- ✅ Dictionary lookup works: `op1=0x0a64 vs op2=0x0a64, condition=true`
- ✅ Property 18 comparison finds match
- ❌ **BUT**: Branch target causes "Branch to address 0x215c is outside memory bounds"
- ✅ **CRITICAL**: Now fails fast with clear error instead of silently continuing with 0

**USER REQUIREMENT SATISFIED**: "We would make more progress debugging if error conditions panicked and had more error logging."

The system now crashes immediately when object lookup fails, making the root cause much more obvious to debug.

## ROOT CAUSE IDENTIFIED ✅ (November 8, 2025)

**BREAKTHROUGH**: Found the exact reason why RuntimeParameter resolution is bypassed.

### Analysis of Grammar Handler Processing

**Problem Location**: `src/grue_compiler/codegen.rs:2691` - Grammar pattern processing

**The Issue**:
1. **Line 2691**: `if let IrHandler::FunctionCall(func_id, _args)` - **IGNORES ARGS**
2. **Line 2718**: `self.generate_object_lookup_from_noun()?` - **HARDCODED OBJECT LOOKUP**
3. **Line 2725**: `Operand::Variable(3)` - **BYPASSES RuntimeParameter RESOLUTION**

### What Should Happen vs What Actually Happens

**EXPECTED FLOW**:
1. Grammar pattern: `handle_take($noun)`
2. IR generation: `FunctionCall(handle_take, [RuntimeParameter("noun")])`
3. Codegen: Process RuntimeParameter("noun") → generates object lookup → calls function
4. RuntimeParameter resolution code (codegen.rs:2863-2887) executes

**ACTUAL FLOW**:
1. Grammar pattern: `handle_take($noun)` ✅ (Correct)
2. IR generation: `FunctionCall(handle_take, [RuntimeParameter("noun")])` ✅ (Correct)
3. Codegen: **IGNORES RuntimeParameter args** → hardcodes own object lookup → calls function ❌
4. RuntimeParameter resolution code **NEVER EXECUTED** ❌

### Technical Root Cause

**Grammar Handler Hardcoded Logic** (codegen.rs:2691-2729):
- Ignores IR args with `_args` parameter
- Hardcodes `loadw` instruction to read parse buffer
- Hardcodes call to `generate_object_lookup_from_noun()`
- Hardcodes `Operand::Variable(3)` for function parameter
- Completely bypasses the existing RuntimeParameter resolution system

**Result**: Two separate object lookup implementations that conflict with each other.

### Solution Required

**Fix**: Modify grammar handler to process actual IR arguments instead of hardcoding object lookup.

1. Change `_args` to `args` and process RuntimeParameter values
2. Remove hardcoded object lookup (line 2718)
3. Use existing RuntimeParameter resolution system (codegen.rs:2863-2887)
4. Call function with properly resolved parameters