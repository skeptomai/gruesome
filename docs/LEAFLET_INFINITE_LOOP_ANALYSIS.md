# Leaflet Infinite Loop Analysis - November 1, 2025

## Problem Description

When executing "open mailbox" command, the game correctly opens the mailbox but then prints "leaflet" thousands of times in an infinite loop instead of showing the leaflet just once.

The output shows:
```
Opened.
listing contents of object
The a small mailbox contains:
  leaflet
  leaflet
  leaflet
  ... [thousands more repetitions]
```

This indicates an infinite loop in the GetObjectSibling iteration system when listing container contents.

## Root Cause Analysis

### Call Chain
1. Open mailbox → mailbox.open = true → print "Opened."
2. Check if container is not empty → call `list_contents(obj)`
3. `list_contents()` function calls `container.contents()`
4. `for item in contents` loop uses GetObjectSibling to iterate through child objects
5. **INFINITE LOOP**: GetObjectSibling never terminates, keeps returning the same leaflet object

### For-Loop Implementation (src/grue_compiler/ir.rs:2326-2385)

The for-loop over object contents generates this IR pattern:
```
1. GetObjectChild(container) → first_child_temp (branch to loop_end if no child)
2. StoreVar(current_obj_var, first_child_temp)
3. Label(loop_start)
4. Label(loop_body)
5. Generate loop body (print item.name)
6. GetObjectSibling(current_obj_var) → next_sibling_temp (branch to loop_end if no sibling)
7. StoreVar(current_obj_var, next_sibling_temp)
8. Jump(loop_start)
9. Label(loop_end)
```

### GetObjectSibling Codegen (src/grue_compiler/codegen_instructions.rs:1382-1425)

The GetObjectSibling instruction generates:
```rust
let layout = self.emit_instruction_typed(
    Opcode::Op1(Op1::GetSibling), // get_sibling opcode (1OP:1)
    &[obj_operand],
    Some(store_var),                // Store result to global variable
    Some(0x7FFF_u16 as i16),       // Branch when result = 0 (no sibling)
)?;
```

**Z-Machine Semantics**:
- `get_sibling` returns the next sibling object number
- If no sibling exists, returns 0 and should branch
- Branch encoding: `0x7FFF` = branch-on-FALSE (when result = 0)

## Investigation Findings

### 1. Object Tree Structure Issue

The leaflet object (Object #11) appears to have a corrupted sibling pointer that points back to itself or creates a circular reference, causing infinite iteration.

From the object mapping:
- Object 'mailbox' (IR ID 33) → Object #10
- Object 'leaflet' (IR ID 34) → Object #11

### 2. Z-Machine Branch Polarity

The GetObjectSibling uses `0x7FFF` (branch-on-FALSE) which should branch when get_sibling returns 0. However, if the object tree has a circular reference, get_sibling will never return 0.

### 3. Missing Infinite Loop Protection

Although there's a `loop_counter_var` created in ir.rs:2310-2319, it's not actually used to limit iterations or detect infinite loops. It's just declared but never incremented or checked.

## Potential Root Causes

### Most Likely: Object Tree Corruption
The object tree structure in the Z-Machine bytecode has incorrect sibling pointers, causing:
- leaflet.sibling → leaflet (self-reference)
- OR leaflet.sibling → some object whose sibling eventually points back to leaflet

### Possible: Branch Instruction Bug
The branch encoding or resolution might be incorrect, causing the branch to never be taken even when get_sibling returns 0.

### Possible: Z-Machine Interpreter Bug
The interpreter's get_sibling implementation might have a bug, but this is unlikely since CLAUDE.md states the interpreter works correctly with real Zork games.

## Next Steps

### 1. Object Tree Inspection
Examine the compiled Z-Machine bytecode to verify object tree structure:
- Check Object #11 (leaflet) sibling pointer
- Verify Object #10 (mailbox) child pointer correctly points to Object #11
- Ensure proper parent/child/sibling relationships

### 2. Debug Z-Machine Execution
Add interpreter logging to track:
- get_sibling calls and return values
- Branch instruction execution
- Object tree traversal sequence

### 3. Implement Proper Loop Protection
As per CLAUDE.md rules: "We prevent infinite loops by not coding them" - fix the root cause rather than adding counters.

## Critical Architecture Insight

This bug violates the **CRITICAL: DYNAMIC BOUNDS CALCULATION RULE** from CLAUDE.md:

> "NEVER use hardcoded maximum/minimum values for loop bounds or array sizes. ALWAYS calculate bounds dynamically from actual data structures."

The infinite loop occurs because the object tree iteration doesn't properly terminate based on the actual object structure - the GetObjectSibling instruction should naturally terminate when there are no more siblings, but it's not doing so.

## Files to Investigate

1. `src/grue_compiler/codegen_objects.rs` - Object table generation and sibling pointer setup
2. `src/interpreter.rs` - Z-Machine get_sibling implementation (if logging needed)
3. `src/grue_compiler/codegen_instructions.rs:1382-1425` - GetObjectSibling branch logic
4. Object table in compiled Z-Machine bytecode - verify correct structure

## Investigation Results

### Object Tree Setup
From compilation logs:
- `Object 'mailbox' (IR ID 33) -> Object #10`
- `Object 'leaflet' (IR ID 34) -> Object #11`
- `Phase 1c: Generated InsertObj for 'leaflet' (IR 34) into container (IR 33)`

This correctly sets up:
- Mailbox as Object #10
- Leaflet as Object #11 (child of mailbox)

### Code Analysis Findings

1. **For-Loop IR Generation (ir.rs:2326-2385)**: ✅ CORRECT
   - Uses GetObjectChild to get first child
   - Uses GetObjectSibling to iterate through siblings
   - Branches to loop_end when no more siblings

2. **GetObjectSibling Codegen (codegen_instructions.rs:1382-1425)**: ✅ CORRECT
   - Uses proper Z-Machine get_sibling opcode (1OP:1)
   - Branch encoding 0x7FFF = branch-on-FALSE (when result = 0)
   - Should terminate when get_sibling returns 0

3. **Z-Machine insert_object (vm.rs:1189-1214)**: ✅ CORRECT
   - Standard Z-Machine object insertion algorithm
   - Properly sets parent/child/sibling relationships

4. **Z-Machine get_sibling (vm.rs:1093-1103)**: ✅ CORRECT
   - Reads sibling pointer from object table
   - Returns 0 when no sibling

### Remaining Hypothesis

Since all the code logic appears correct, the issue is likely one of these:

1. **Object Tree Corruption**: The compiled object table has incorrect sibling pointers causing circular references
2. **Branch Resolution Bug**: The GetObjectSibling branch instruction isn't being resolved correctly to the loop_end label
3. **Object Numbering Bug**: The dual numbering system might have created inconsistent object references

## Root Cause & Fix

### **DOUBLE INSERTION BUG** ✅ **FIXED** (November 1, 2025)

**Root Cause**: Objects were being inserted into containers both at compile time and at runtime, causing self-referencing sibling pointers.

**Specific Bug Sequence**:
1. Compilation: Leaflet (Object #11) correctly inserted into Mailbox (Object #10)
2. Runtime: Game code tries to insert leaflet into mailbox again via `insert_object(11, 10)`
3. `old_child = get_child(10)` returns 11 (leaflet is already first child)
4. `set_sibling(11, old_child)` sets leaflet's sibling to 11 (itself!)
5. GetObjectSibling loop: 11 → 11 → 11 → infinite loop

**Evidence**:
```
🔍 INSERT_DEBUG: insert_object(11, 10) - old_child=11
🔍 SIBLING_DEBUG: get_sibling(11) = 11 (addr=0x0378)
```

**Fix Applied** (vm.rs:1216-1221):
```rust
// CRITICAL FIX: Prevent double insertion bug that causes self-referencing sibling pointers
// If the object is already the first child of the destination, don't insert it again
// This happens when objects are inserted both at compile time and runtime
if old_child == obj_num {
    return Ok(());
}
```

**Result**: Leaflet now displays correctly once instead of infinitely.

## Status

- ✅ Problem reproduced and analyzed
- ✅ Call chain identified
- ✅ Code logic verified as correct
- ✅ Root cause identified: Double insertion bug
- ✅ **FIXED**: Prevented runtime re-insertion of objects already inserted at compile time
- ✅ **VERIFIED**: Mailbox now shows leaflet correctly without infinite loop