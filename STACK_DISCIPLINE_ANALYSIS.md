# Stack Discipline Violation Analysis

## Problem Summary

The "look" command works but "open mailbox" fails with stack underflow. Analysis shows "look" accidentally masks systematic stack imbalance by leaving 6 extra values on the stack.

## Root Cause: For Loop Stack Violations

### Z-Machine Stack Specification (Section 6.3)

- **6.3.1**: "The stack is considered as empty at the start of each routine"
- **6.3.2**: "The stack is left empty at the end of each routine: when a return occurs, any values pushed during the routine are thrown away"

### Stack Operations During "look" Command

Found 6 unbalanced push operations in `list_objects` function:

1. PC 0x14b9: `push 1`
2. PC 0x14c7: `push 1`
3. PC 0x14f3: `push 10`
4. PC 0x14b9: `push 10` (second iteration)
5. PC 0x14c7: `push 10` (second iteration)
6. PC 0x14f3: `push 1` (second iteration)

### Verification Test Results

```bash
# Direct "open mailbox" - FAILS
echo "open mailbox" | ./target/debug/gruesome tests/demo.z3
# Result: Stack underflow

# "look" then "open mailbox" - WORKS
echo -e "look\nopen mailbox" | ./target/debug/gruesome tests/demo.z3
# Result: "You can't open that." (no stack underflow)
```

## Source Code Analysis

### list_objects Function (mini_zork.grue:436-444)

```grue
fn list_objects(location) {
    let objects = location.contents();

    for obj in objects {        // ← FOR LOOP ISSUE
        if obj != player {
            print("There is " + obj.name + " here.");
        }
    }
}
```

### Compiler IR for list_objects

```
function list_objects (id=16):
  parameters: location(slot=1, id=447)
  locals:
    location (slot=1, id=447)
    objects (slot=2, id=449)
    obj (slot=3, id=452)              // ← Should use this local variable
    __current_obj_453 (slot=4, id=453) // ← Should use this local variable
```

## Stack vs Local Variable Usage

**CORRECT**: For loop iteration should use local variables (slots 1-15)
- `obj` (slot=3) for current object
- `__current_obj_453` (slot=4) for iteration state

**INCORRECT**: Current implementation pushes values to stack (variable 0)
- Violates Z-Machine stack discipline
- Stack should be empty at routine start/end
- Stack is only for temporary expression evaluation and function call arguments/returns

## Root Cause Found: push_pull_ir_ids Set Error

**Located in `resolve_ir_id_to_operand` (codegen.rs:7017-7034)**

The issue is NOT in the IR generation (which correctly uses local variables), but in the codegen phase. The `push_pull_ir_ids` set incorrectly contains IR IDs for local variables from for-loops.

**What happens:**
1. IR generates proper `StoreVar { var_id, source }` for local variables
2. StoreVar calls `resolve_ir_id_to_operand(*source)`
3. Because `source` IR ID is in `push_pull_ir_ids` set, it emits `pull` instruction
4. This treats local variable value as if it was on the stack
5. But the value was never pushed - causing stack underflow

**Code Evidence:**
```rust
// Line 7017-7034 in codegen.rs
if self.push_pull_ir_ids.contains(&ir_id) {
    let pull_operands = vec![Operand::Variable(temp_global)];
    self.emit_instruction_typed(Opcode::OpVar(OpVar::Pull), &pull_operands, None, None)?;
```

## Fix Required

Find where `push_pull_ir_ids` is populated and ensure for-loop local variable IR IDs are NOT added to this set. The for-loop iteration should use direct local variable access, not stack push/pull sequences.

## Impact

This systematic violation affects all for loops in the compiler, not just `list_objects`. The "look" command accidentally compensates for stack underflow by incorrectly pushing values that other commands expect to find on the stack.