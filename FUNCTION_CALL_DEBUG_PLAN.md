# Function Call/Return Address Bug - Systematic Analysis Plan

## Problem Summary
The simple test case executes completely (both print statements work) but crashes at address 0x078b after the function returns. The crash happens inside the function's local variable storage instead of returning to the correct address after the function call.

## Current Status
- Function call: `e0 3f 03 c5 00` calls packed address 0x03c5
- Function starts at: 0x078a (unpacked from 0x03c5)
- Expected return address: 0x0783 (after 5-byte call instruction)
- Actual crash location: 0x078b (8 bytes off, inside function's local variables)

## Systematic Analysis Plan

### Phase 1: Verify Call Instruction Encoding ⏳
1. **Check VAR call instruction bytes**: `e0 3f 03 c5 00`
   - Verify opcode 0xe0 is correct VAR form call
   - Confirm operand type byte 0x3f matches 1 LargeConstant operand
   - Validate packed address 0x03c5 calculation

2. **Verify function address packing/unpacking**:
   - Confirm 0x078a ÷ 2 = 0x03c5 (correct packing)
   - Check if function actually starts at 0x078a in final bytecode

### Phase 2: Trace Return Address Calculation
3. **Examine call mechanism**:
   - Determine what return address should be pushed to stack
   - Expected: address after 5-byte call instruction (0x0783)
   - Check if compiler/interpreter accounts for call instruction size correctly

4. **Analyze stack frame setup**:
   - See what gets pushed during function call
   - Verify local variable initialization doesn't interfere

### Phase 3: Verify Return Instruction
5. **Check rtrue generation**:
   - Confirm `rtrue` (0xb0) is at correct position in function
   - Verify it's the last instruction before function end
   - Check if return value handling affects address calculation

### Phase 4: Comparative Analysis
6. **Compare with known working patterns**:
   - Examine how commercial games encode function calls
   - Look for differences in call/return instruction sequences

### Phase 5: Minimal Test Case
7. **Create simplest possible function call**:
   - Empty function with just `rtrue`
   - No local variables, no parameters
   - Isolate the pure call/return mechanism

## Key Hypothesis
The return address calculation is off by 8 bytes, suggesting either:
- Call instruction setup is wrong (pushing wrong return address)
- Function address calculation includes header size when it shouldn't
- Return mechanism is accounting for function header incorrectly

## Files to Examine
- Call instruction generation: `src/grue_compiler/codegen_instructions.rs`
- Function address calculation: `src/grue_compiler/codegen.rs`
- Return instruction: Look for `rtrue` generation

## Test Case
```grue
fn test_property() {
    print("Testing property access");
    let obj = test_obj;
    let is_openable = obj.openable;
    print("Property accessed successfully");
}

init {
    player.location = test_room;
    test_property();  // This call crashes on return
}
```

## Success Criteria
Function executes completely without crash at 0x078b - returns cleanly to continue execution after the function call.