# StoreVar Silent Fallback Bug Analysis

## Summary
`StoreVar` has a silent fallback to stack storage when variables aren't allocated to locals. This masks allocation failures and causes variables to be treated as ephemeral stack values instead of persistent locals.

## The Bug

### Current Implementation (codegen_instructions.rs:336-361)
```rust
IrInstruction::StoreVar { var_id, source } => {
    let value_operand = self.resolve_ir_id_to_operand(*source)?;

    if let Some(var_num) = self.ir_id_to_local_var.get(var_id) {
        // Store to local variable
        let var_operand = Operand::SmallConstant(*var_num);
        self.emit_instruction_typed(STORE, &[var_operand, value_operand], None, None)?;
    } else {
        // SILENT FALLBACK: Store to stack (variable 0)
        let stack_operand = Operand::SmallConstant(0);
        self.emit_instruction_typed(STORE, &[stack_operand, value_operand], None, None)?;
        self.ir_id_to_stack_var.insert(*var_id, 0);
    }
}
```

### The Problem
1. **Silent failure**: When `var_id` isn't in `ir_id_to_local_var`, code falls back to stack storage WITHOUT ERROR
2. **Wrong semantics**: Variables meant to be persistent locals become ephemeral stack values
3. **Hard to debug**: No indication that variable allocation failed - just silent incorrect behavior
4. **Masking root cause**: Hides that `assign_local_variable()` is never being called

## Evidence: Loop Counter Case

### Loop Counter (var_id 423)
- **Expected**: Loop counter should be a local variable (persistent across loop iterations)
- **Actual**: Not in `ir_id_to_local_var`, falls back to stack storage
- **Result**: Stack value is consumed on first use, underflow on second use

### IR Pattern
```
StoreVar { var_id: 423, source: 424 }  // Initialize loop counter → STACK (wrong!)
LoadVar { target: 428, var_id: 423 }   // Load counter → pushes to stack
GetArrayElement { index: 428 }         // CONSUMES stack value
BinaryOp Add { left: 428 }             // Expects 428 on stack → UNDERFLOW
```

## Dead Code: assign_local_variable()

### Function Exists (codegen.rs:2300-2334)
```rust
fn assign_local_variable(&mut self, var_id: IrId, source: IrId) -> Result<(), CompilerError> {
    // 1. Allocate a local variable slot
    let local_slot = self.allocate_local_variable_slot();

    // 2. Map IR variable ID to the local variable slot
    self.ir_id_to_local_var.insert(var_id, local_slot);

    // 3. Handle constant propagation...
}
```

### Usage
**NEVER CALLED** - This is dead code! The mechanism for allocating locals exists but isn't used.

## LoadVar Inconsistency

### LoadVar Behavior (codegen_instructions.rs:320-334)
```rust
IrInstruction::LoadVar { target, var_id } => {
    // Uses var_id DIRECTLY as Z-Machine variable number
    let var_operand = Operand::SmallConstant(*var_id as u8);
    self.emit_instruction_typed(LOAD, &[var_operand], Some(0), None)?;
}
```

### The Mismatch
- **LoadVar**: Treats `var_id` as a Z-Machine variable number (1-15 for locals, 16+ for globals)
- **StoreVar**: Checks if `var_id` is in `ir_id_to_local_var`, falls back to stack if not

This inconsistency suggests a fundamental confusion about what `var_id` represents:
- Is it an IR ID that needs to be mapped to a Z-Machine variable?
- Or is it already a Z-Machine variable number?

## Impact

### Loop Counter Example (var_id 423)
- `var_id: 423` is way too large to be a Z-Machine variable number (max ~240)
- This is clearly an IR ID, not a variable number
- LoadVar treating it as a variable number (423) would access invalid memory
- StoreVar falling back to stack masks this completely

### What Should Happen
1. Loop counter declared → `assign_local_variable(423, ...)` allocates local slot
2. `ir_id_to_local_var[423] = 1` (e.g., first local slot)
3. `LoadVar { var_id: 423 }` looks up mapping, emits `load local1 → stack`
4. `StoreVar { var_id: 423 }` looks up mapping, emits `store local1 ← stack`

### What Actually Happens
1. Loop counter declared → NO ALLOCATION
2. `ir_id_to_local_var[423]` doesn't exist
3. `LoadVar { var_id: 423 }` emits `load var423 → stack` (INVALID!)
4. `StoreVar { var_id: 423 }` falls back to `store stack ← stack` (WRONG!)

## Fix Strategy

### 1. Remove Silent Fallback (Immediate)
```rust
IrInstruction::StoreVar { var_id, source } => {
    let value_operand = self.resolve_ir_id_to_operand(*source)?;

    if let Some(var_num) = self.ir_id_to_local_var.get(var_id) {
        // Store to local variable
        let var_operand = Operand::SmallConstant(*var_num);
        self.emit_instruction_typed(STORE, &[var_operand, value_operand], None, None)?;
    } else {
        // ERROR: Variable not allocated!
        return Err(CompilerError::CodeGenError(format!(
            "StoreVar: IR variable {} not allocated to local variable slot",
            var_id
        )));
    }
}
```

### 2. Fix Variable Allocation (Root Cause)
- Find where loop counters are created in IR generation
- Call `assign_local_variable()` for loop counters
- Ensure all `var_id` values in StoreVar/LoadVar have local allocations

### 3. Fix LoadVar Consistency
LoadVar should also look up in `ir_id_to_local_var`:
```rust
IrInstruction::LoadVar { target, var_id } => {
    if let Some(var_num) = self.ir_id_to_local_var.get(var_id) {
        let var_operand = Operand::SmallConstant(*var_num);
        self.emit_instruction_typed(LOAD, &[var_operand], Some(0), None)?;
    } else {
        return Err(CompilerError::CodeGenError(format!(
            "LoadVar: IR variable {} not allocated to local variable slot",
            var_id
        )));
    }
}
```

## Relationship to SSA Stack Underflow

### Question: Does this obviate SSA-related stack work?

**Answer: Partially, but not entirely.**

### What This Fixes
If loop counters are properly allocated to locals:
- `LoadVar` loads from local slot → pushes to stack
- Multiple uses of loop counter each do fresh `LoadVar` → fresh stack value
- No more stack underflow from reusing consumed values

### What This Doesn't Fix
The underlying SSA violation in IR generation still exists:
- IR still generates `BinaryOp { left: 428 }` expecting IR 428 on stack
- Even with locals, if IR 428 is consumed by GetArrayElement, it's gone
- Need to generate new IR ID for each use: `LoadVar { target: NEW_ID, var_id: 423 }`

### Combined Fix Required
1. **Allocate loop counters to locals** (this fix)
2. **Reload before each use** (SSA fix) - generate fresh LoadVar instructions

The local variable allocation is necessary but not sufficient. We still need proper SSA form where each use of a variable gets its own LoadVar instruction with a new IR ID.

## Files Involved
- `src/grue_compiler/codegen_instructions.rs:336-361` - StoreVar silent fallback
- `src/grue_compiler/codegen_instructions.rs:320-334` - LoadVar inconsistent behavior
- `src/grue_compiler/codegen.rs:2300-2334` - Dead code: assign_local_variable()
- `src/grue_compiler/ir.rs` - Need to find loop counter generation and add allocation

## Next Steps
1. Add error instead of silent fallback in StoreVar
2. Add error instead of direct use in LoadVar
3. Find where loop counters are created in IR
4. Call assign_local_variable() for loop counters
5. Verify SSA: each use gets fresh LoadVar with new IR ID
