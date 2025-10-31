# Errant push_pull_ir_ids Tracking Analysis

## Exact Location of Bug

**File:** `/src/grue_compiler/codegen_builtins.rs`
**Function:** `generate_get_object_contents_builtin`
**Line:** 912 - `self.use_push_pull_for_result(store_var, "get_object_contents builtin")?;`

## The Errant Push Sequence

### Step 1: GetObjectChild Operation
```rust
// Line 873-878: GetObjectChild stores result to stack (Variable 0)
let layout = self.emit_instruction(
    0x02, // get_child opcode (1OP:2)
    &[container_operand.clone()],
    Some(0),      // Store result to stack (Variable 0) ← CORRECT
    Some(0x7FFF), // Branch on FALSE (no child)
)?;
```

### Step 2: Errant use_push_pull_for_result Call
```rust
// Line 912: PROBLEM - calls use_push_pull_for_result
self.use_push_pull_for_result(store_var, "get_object_contents builtin")?;
```

### Step 3: What use_push_pull_for_result Does Wrong
```rust
// From codegen.rs:8176 - adds IR ID to push_pull_ir_ids set
self.push_pull_ir_ids.insert(target_id);

// From codegen.rs:8164 - emits unnecessary push instruction
self.emit_instruction_typed(Opcode::OpVar(OpVar::Push), &[push_operand], None, None)?;
```

## Specific IR IDs Added Errantly

**From compilation logs:**
- IR ID 457: Added to push_pull_ir_ids in "get_object_contents builtin" context
- IR ID 450: Added to push_pull_ir_ids in "get_object_contents builtin" context
- IR ID 477: Added to push_pull_ir_ids in "get_object_contents builtin" context

## The Stack Imbalance Pattern

### Correct Z-Machine Pattern for GetObjectChild:
1. `get_child container` → stores result directly to Variable(0)
2. **NO PUSH** - value consumed immediately by caller
3. **NO PULL** - direct Variable(0) access

### Current Errant Pattern:
1. `get_child container` → stores result to Variable(0) ✓
2. **ERRANT PUSH** - pushes Variable(0) to stack
3. StoreVar calls `resolve_ir_id_to_operand(IR_ID)`
4. Because IR_ID is in `push_pull_ir_ids`, it emits **PULL**
5. **Result: Push without matching Pull = Stack Imbalance**

## Evidence from Runtime Logs

**Six errant pushes during "look" command:**
- PC 0x14b9: `push 1` (first loop iteration, IR ID 457)
- PC 0x14c7: `push 1` (first loop iteration, IR ID 450)
- PC 0x14f3: `push 10` (first loop iteration, IR ID 477)
- PC 0x14b9: `push 10` (second iteration, IR ID 457)
- PC 0x14c7: `push 10` (second iteration, IR ID 450)
- PC 0x14f3: `push 1` (second iteration, IR ID 477)

**Pattern:** 3 pushes per loop iteration × 2 iterations = 6 total unbalanced pushes

## The Fix Required

**Remove the errant use_push_pull_for_result call in get_object_contents:**

```rust
// CURRENT (WRONG):
// Use push/pull to move result from stack to target variable
self.use_push_pull_for_result(store_var, "get_object_contents builtin")?;

// SHOULD BE (CORRECT):
// Map result directly to target IR ID - no push/pull needed
if let Some(target_id) = target {
    self.ir_id_to_stack_var.insert(target_id, 0); // Result already in Variable(0)
}
```

## Why This Fixes the Stack Discipline Issue

1. **GetObjectChild** correctly stores result in Variable(0)
2. **No push** operation - eliminates unbalanced stack operations
3. **Direct Variable(0) access** via `ir_id_to_stack_var` mapping
4. **No pull needed** - `resolve_ir_id_to_operand` returns `Operand::Variable(0)`
5. **Stack remains balanced** - adheres to Z-Machine Section 6.3 specification

## Impact Assessment

**Functions Affected:**
- `list_objects` function (examples/mini_zork.grue:436-444)
- Any for-loop using `object.contents()` method
- All object tree iteration patterns

**Commands Fixed:**
- "look" command will no longer leave 6 values on stack
- "open mailbox" command will no longer fail with stack underflow
- All object iteration will use proper local variable access

## Verification Strategy

1. Remove `use_push_pull_for_result` call from `get_object_contents`
2. Compile and test "open mailbox" directly - should work
3. Test "look" command - should still work but with balanced stack
4. Verify no other stack instrumentation logs show unbalanced operations