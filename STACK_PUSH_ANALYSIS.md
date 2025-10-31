# Stack Push Analysis: Errant push_pull_ir_ids Entries

## Problem Summary

The `push_pull_ir_ids` set incorrectly contains IR IDs for local variables from for-loops, causing `resolve_ir_id_to_operand` to emit `pull` instructions when it should directly access local variables.

## Key Evidence

**LoadVar Implementation (codegen_instructions.rs:1089-1124):**
```rust
IrInstruction::LoadVar { target, var_id } => {
    // LoadVar correctly stores to allocated global variable, NOT stack
    self.emit_instruction_typed(LOAD, &[var_operand], Some(result_var as u8), None)?;
    self.ir_id_to_stack_var.insert(*target, result_var as u8);
}
```

**StoreVar Implementation (codegen_instructions.rs:1126-1155):**
```rust
IrInstruction::StoreVar { var_id, source } => {
    // StoreVar calls resolve_ir_id_to_operand(*source)
    let value_operand = self.resolve_ir_id_to_operand(*source)?;
    // This is where the problem occurs!
}
```

**resolve_ir_id_to_operand Issue (codegen.rs:7017-7034):**
```rust
if self.push_pull_ir_ids.contains(&ir_id) {
    // ERROR: Emits pull instruction for local variable values
    let pull_operands = vec![Operand::Variable(temp_global)];
    self.emit_instruction_typed(Opcode::OpVar(OpVar::Pull), &pull_operands, None, None)?;
}
```

## For-Loop IR Pattern (Correct)

**IR Generation for list_objects function:**
1. `GetObjectChild { target: 457, object: 447, branch_if_no_child: 456 }`
2. `StoreVar { var_id: 453, source: 457 }`  ← source IR ID 457 should NOT be in push_pull_ir_ids
3. `LoadVar { target: 458, var_id: 453 }`
4. `StoreVar { var_id: 452, source: 458 }`  ← source IR ID 458 should NOT be in push_pull_ir_ids

## The Problem

For-loop local variable IR IDs (457, 458, etc.) are incorrectly added to `push_pull_ir_ids` set, causing:

1. **GetObjectChild** stores result in Variable(0)
2. **use_push_pull_for_result(457)** is called → adds 457 to `push_pull_ir_ids` and emits `push`
3. **StoreVar** calls `resolve_ir_id_to_operand(457)`
4. Because 457 is in `push_pull_ir_ids`, it emits `pull` instruction
5. **Stack imbalance**: Push + Pull should balance, but pattern repeats multiple times

## Root Cause Hypothesis

**GetObjectChild** instruction likely calls `use_push_pull_for_result(target)` incorrectly. Object tree operations (GetObjectChild, GetObjectSibling) should store results directly to local variables, NOT use stack push/pull.

## Next Steps

1. Find where `GetObjectChild` calls `use_push_pull_for_result`
2. Modify object tree operations to bypass push/pull for local variable storage
3. Verify for-loop iteration uses direct local variable access

## Stack Operations from "look" Command

From runtime logs, the 6 push operations occur at:
- PC 0x14b9: `push 1`
- PC 0x14c7: `push 1`
- PC 0x14f3: `push 10`
- PC 0x14b9: `push 10` (second iteration)
- PC 0x14c7: `push 10` (second iteration)
- PC 0x14f3: `push 1` (second iteration)

These correspond to the 3 push operations per loop iteration × 2 iterations = 6 total pushes.