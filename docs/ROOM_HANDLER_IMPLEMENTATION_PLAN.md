# Room Handler Implementation Plan

**Date**: October 14, 2025
**Status**: Planning - Bug identified, implementation needed
**Bug**: Property 13 not found for object 13 when typing "east" from behind_house

## Problem Summary

When user types "north" then "east" in mini_zork, the game crashes with:
```
Error during execution: Property 13 not found for object 13
```

Investigation revealed this is caused by incomplete implementation of room event handlers (`on_enter`, `on_exit`, `on_look`).

## Root Cause Analysis

### 1. Property Number Mismatch

**PropertyManager Assignments** (from IR):
- `visited` → property #13
- `on_look` → property #18
- `on_exit` → property #20
- `on_enter` → property #21

**Runtime Behavior**:
```
[2025-10-14T21:01:00Z ERROR] 🔍 PUT_PROP OPCODE: obj=2, prop=13, value=0x0001, PC=0x0fbe
[2025-10-14T21:01:00Z ERROR] 🔍 PUT_PROP OPCODE: obj=5, prop=13, value=0x0001, PC=0x12bc
[2025-10-14T21:01:00Z ERROR] 🔍 PUT_PROP OPCODE: obj=13, prop=13, value=0x0001, PC=0x12bc
Error during execution: Property 13 not found for object 13
```

Code tries to write property #13 (`visited`) to objects 2, 5, and 13:
- Object 2 (west_of_house): HAS property 13 ✅
- Object 5 (north_of_house?): HAS property 13 ✅
- Object 13 (behind_house): MISSING property 13 ❌

Object 13 only has properties: 15, 7, 2, 1 (from binary analysis)

### 2. Zero Function Addresses

**File**: `src/grue_compiler/codegen_objects.rs:376`

```rust
// Set default property values for rooms
room_properties.set_string(desc_prop, room.description.clone());
room_properties.set_byte(visited_prop, 0); // Initially not visited
room_properties.set_byte(on_look_prop, 0); // No special on_look handler by default
```

**Problem**: ALL rooms get `on_look` property set to 0, even rooms that HAVE `on_look` handlers defined in source code.

Example from mini_zork.grue (lines 68-72):
```grue
room behind_house "Behind House" {
    // ...
    on_look: {
        if window.open {
            print("Through the open window you can see into what appears to be a kitchen.");
        }
    }
}
```

This handler is parsed, analyzed, and stored in IR, but NEVER converted to a callable function with an address.

### 3. Missing Function Generation

**File**: `src/grue_compiler/ir.rs:1854-1870`

```rust
// Now process handlers - objects are available for reference
let on_enter = if let Some(block) = room.on_enter {
    Some(self.generate_block(block)?)
} else {
    None
};

let on_exit = if let Some(block) = room.on_exit {
    Some(self.generate_block(block)?)
} else {
    None
};

let on_look = if let Some(block) = room.on_look {
    Some(self.generate_block(block)?)
} else {
    None
};
```

**Problem**: Room handlers are converted to `Option<IrBlock>` and stored inline in `IrRoom`, NOT converted to `IrFunction` entries. They never get:
- Function headers
- Function addresses
- Added to `ir.functions` vector
- Generated as callable Z-Machine routines

**Data Structure**:
```rust
pub struct IrRoom {
    pub id: IrId,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub exits: IndexMap<String, IrExitTarget>,
    pub on_enter: Option<IrBlock>,  // ❌ Should be function ID
    pub on_exit: Option<IrBlock>,   // ❌ Should be function ID
    pub on_look: Option<IrBlock>,   // ❌ Should be function ID
}
```

### 4. Incomplete Method Call Implementation

**File**: `src/grue_compiler/ir.rs:2796-2824`

```rust
"on_enter" | "on_exit" | "on_look" => {
    // Object handler methods - these call property-based function handlers
    // In Grue, these are properties that contain function addresses
    // The pattern is: if object.property exists, call it as a function

    // Get property number for this handler
    let property_name = method;
    let property_number = self.property_manager.get_property_number(&property_name);

    // Use proper property-based function call
    block.add_instruction(IrInstruction::GetPropertyByNumber {
        target: result_temp,
        object: object_temp,
        property_num: property_number,
    });

    // TODO: In a complete implementation, this would:
    // 1. Get the property value (function address)
    // 2. Check if it's non-zero (function exists)
    // 3. Call the function if it exists
    // For now, we'll return the property value directly
}
```

**Problem**: The code gets the property value (which is 0) and returns it, instead of:
1. Getting the function address from the property
2. Checking if it's non-zero
3. Calling it as a function

### 5. Runtime Crash Cascade

**Execution Flow**:

1. User types "north" → moves to behind_house (object 13)
2. `handle_go()` calls `look_around()` (line 302)
3. `look_around()` checks `if player.location.on_look` (line 172)
4. IR generates GetPropertyByNumber for property #18 (`on_look`)
5. Property returns 0 (no handler)
6. Code treats 0 as "no handler", skips the call ✅ (this works)
7. Later, code tries to set `player.location.visited = true` (line 295)
8. IR generates SetProperty for property #13 (`visited`)
9. Object 13 (behind_house) doesn't have property #13 ❌
10. Runtime crash: "Property 13 not found for object 13"

## Architecture Gap

Room handlers were **designed but never fully implemented**. The architecture assumes:

1. ✅ Room handlers can be defined in source code (parser, AST)
2. ✅ Room handlers are analyzed for correctness (semantic analysis)
3. ✅ Room handlers are stored in IR (IrRoom structure)
4. ❌ Room handlers are converted to functions (MISSING)
5. ❌ Function addresses are stored in properties (MISSING)
6. ❌ Property-based function calls work correctly (INCOMPLETE)

## Detailed Phased Implementation Plan

### **Phase 0: Quick Fix - Make Game Playable (15 minutes)**

**Problem**: Game crashes when trying to set `visited` property because object 13 doesn't have property #13.

**Root Cause**: `analyze_object_property_usage()` (codegen_objects.rs:256) gives ALL objects ALL properties, but something goes wrong for object 13 specifically.

**Fix**: Debug why object 13 is missing property #13. Add logging to property table generation to see what properties each object gets.

**Test**:
```bash
RUST_LOG=debug cargo run --quiet --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep "Object #13\|behind_house" | grep -i property
```

**Success Criteria**:
- Can navigate to behind_house and then east without crash
- Game doesn't crash on `player.location.visited = true`
- Handlers still won't work (that's fine for this phase)

**Why First**: Gets the game playable so we can test other features. Minimal risk.

---

### Phase 1: Convert Room Handlers to Functions (1-2 hours)

**Problem**: Room handlers are stored as inline `IrBlock` instead of callable `IrFunction` entries.

**File**: `src/grue_compiler/ir.rs`

**Changes Needed**:

1. **Add helper method** to `IrGenerator`:
```rust
fn create_function_from_block(
    &mut self,
    name: String,
    block: BlockStmt,
    params: Vec<String>,
) -> Result<IrId, CompilerError> {
    let func_id = self.next_id();
    let body = self.generate_block(block)?;

    let function = IrFunction {
        id: func_id,
        name,
        params,
        body,
        local_count: 0, // Will be calculated later
    };

    self.ir.functions.push(function);
    Ok(func_id)
}
```

2. **Modify `generate_room()`** (ir.rs:1854-1870) to create functions instead of blocks:
```rust
let on_look = if let Some(block) = room.on_look {
    let func_name = format!("{}__on_look", room.identifier);
    let func_id = self.create_function_from_block(func_name, block, vec![])?;
    Some(func_id)
} else {
    None
};
// Similar for on_enter and on_exit
```

3. **Update `IrRoom` structure** (ir.rs:170-179):
```rust
pub struct IrRoom {
    // ...
    pub on_enter: Option<IrId>,  // Changed from Option<IrBlock>
    pub on_exit: Option<IrId>,
    pub on_look: Option<IrId>,
}
```

**Test**:
```bash
cargo run --bin grue-compiler -- examples/mini_zork.grue --print-ir 2>&1 | grep -A5 "Function.*on_look\|Function.*on_enter"
```

**Success Criteria**:
- IR output shows functions named `behind_house__on_look`, `west_of_house__on_enter`
- Functions appear in `ir.functions` vector
- Each function has a unique IrId
- Build succeeds with no compiler errors

**Regression Test**: All 183 tests still pass (handlers won't work yet, but nothing should break)

**Why Second**: Establishes the foundation - functions must exist before we can call them. Pure IR change, doesn't affect codegen yet.

---

### Phase 2: Store Function Addresses in Room Properties (1 hour)

**Problem**: Room properties always set to 0, need to store actual function addresses.

**File**: `src/grue_compiler/codegen_objects.rs:364-376`

**Changes Needed**:

1. **Modify object table generation** to store function addresses:
```rust
// After room_properties.set_string(desc_prop, ...)

// Add handler function addresses if room has handlers
if let Some(on_look_id) = room.on_look {
    let on_look_prop = *self.property_numbers.get("on_look").unwrap();
    if let Some(&func_addr) = self.function_addresses.get(&on_look_id) {
        room_properties.set_word(on_look_prop, func_addr as u16);
    } else {
        // Function not generated yet - will be patched later
        room_properties.set_word(on_look_prop, 0xFFFF); // Placeholder
    }
}
// Similar for on_enter and on_exit
```

2. **May need to reorder codegen phases** if functions aren't generated before objects:
   - Check: Do we call `generate_functions()` before `generate_object_tables()`?
   - If not, need to reorder or use UnresolvedReference pattern

**Test**:
```bash
# Compile and check property values
RUST_LOG=debug cargo run --quiet --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test_handlers.z3 2>&1 | grep "on_look.*property.*address"

# Disassemble to see property values
RUST_LOG=error ./target/debug/gruesome /tmp/test_handlers.z3 --disasm-range 0x0577-0x05a0
```

**Success Criteria**:
- Property #18 (`on_look`) for behind_house contains function address (not 0, not 0xFFFF)
- Function address matches what's in `function_addresses` map
- All 183 tests still pass

**Why Third**: Functions exist (from Phase 1), now we store their addresses. Still doesn't call them, but addresses are in place.

### Phase 3: Implement Property-Based Function Calls (1-2 hours)

**Problem**: When code calls `player.location.on_look()`, it gets the property value (function address) but doesn't call it.

**File**: `src/grue_compiler/ir.rs:2796-2824`

**Changes Needed**:

**Sub-phase 3a**: Check if `IrInstruction::CallIndirect` exists (5 minutes)
```bash
grep -n "CallIndirect" src/grue_compiler/ir.rs
```

If it doesn't exist, add to `IrInstruction` enum:
```rust
CallIndirect {
    target: Option<IrId>,
    function_addr: IrId,  // IR ID containing function address (from property)
    args: Vec<IrId>,
},
```

**Sub-phase 3b**: Implement codegen for CallIndirect (30 minutes)

File: `src/grue_compiler/codegen_instructions.rs`

Add case for CallIndirect that:
1. Loads function address from IR ID (stack or local variable)
2. Emits `call_vs` with function address as first operand
3. Handles return value storage if target is Some

**Sub-phase 3c**: Replace TODO in method call generation (30 minutes)

File: `src/grue_compiler/ir.rs:2796-2824`

Replace the TODO comment with actual implementation:

```rust
"on_enter" | "on_exit" | "on_look" => {
    // Get property number
    let property_name = method;
    let property_number = self.property_manager.get_property_number(&property_name);

    // Step 1: Get the property value (function address)
    let func_addr_temp = self.next_id();
    block.add_instruction(IrInstruction::GetPropertyByNumber {
        target: func_addr_temp,
        object: object_temp,
        property_num: property_number,
    });

    // Step 2: Check if it's non-zero (function exists)
    let no_handler_label = self.next_id();
    let end_label = self.next_id();

    block.add_instruction(IrInstruction::JumpIfZero {
        value: func_addr_temp,
        label: no_handler_label,
    });

    // Step 3: Call the function if it exists
    block.add_instruction(IrInstruction::CallIndirect {
        target: Some(result_temp),
        function_addr: func_addr_temp,
        args: arg_temps,
    });
    block.add_instruction(IrInstruction::Jump { label: end_label });

    // No handler: return 0
    block.add_instruction(IrInstruction::Label { id: no_handler_label });
    block.add_instruction(IrInstruction::LoadImmediate {
        target: result_temp,
        value: IrValue::Integer(0),
    });

    block.add_instruction(IrInstruction::Label { id: end_label });
}
```

**Test**:
```bash
# Compile with IR output
cargo run --bin grue-compiler -- examples/mini_zork.grue --print-ir 2>&1 | grep -A10 "CallIndirect"

# Run game and trigger on_look handler
RUST_LOG=error cargo run --quiet --bin gruesome tests/mini_zork.z3 <<EOF
north
look
EOF
```

**Success Criteria**:
- IR shows CallIndirect instructions for on_enter/on_exit/on_look calls
- Codegen emits call_vs with variable operand (function address from property)
- Game executes behind_house on_look handler when looking
- Window message appears: "Through the open window you can see into what appears to be a kitchen." (if window is open)
- All 183 tests still pass

**Why Fourth**: Functions exist (Phase 1), addresses are stored (Phase 2), now we can call them. This completes the handler functionality.

### Phase 4: Comprehensive Testing and Validation (30 minutes)

**Problem**: After implementing Phases 1-3, need to verify everything works together.

**Testing Strategy**:

**Test 1: IR Function Generation** (5 minutes)
```bash
cargo run --bin grue-compiler -- examples/mini_zork.grue --print-ir 2>&1 | grep "Function.*on_"
```
Expected: See functions like `behind_house__on_look`, `west_of_house__on_enter`, etc.

**Test 2: Property Address Storage** (5 minutes)
```bash
# Compile and disassemble behind_house property table
cargo run --quiet --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3
RUST_LOG=error ./target/debug/gruesome tests/mini_zork.z3 --disasm-range 0x0577-0x05a0
```
Expected: Property #18 (on_look) contains non-zero function address

**Test 3: Handler Execution** (10 minutes)
```bash
# Interactive test - verify handlers execute
RUST_LOG=error cargo run --quiet --bin gruesome tests/mini_zork.z3 <<EOF
north
look
west
east
south
EOF
```
Expected:
- behind_house on_look executes when looking
- No crashes when navigating between rooms
- visited property gets set without errors

**Test 4: Regression Testing** (10 minutes)
```bash
RUST_LOG=error cargo test
```
Expected: All 183 tests pass

**Success Criteria**:
1. ✅ All room handlers converted to functions in IR
2. ✅ Function addresses stored in room properties (not 0, not 0xFFFF)
3. ✅ Handlers execute when called (on_look, on_enter, on_exit)
4. ✅ No crashes when setting visited property
5. ✅ All 183 tests pass
6. ✅ Can navigate between all rooms in mini_zork

**Why Last**: Validates entire implementation is working correctly end-to-end.

---

## Implementation Order Summary

Execute phases in this exact order:

1. **Phase 0**: Quick fix - ensure all rooms have property #13 (PREREQUISITE)
2. **Phase 1**: Convert handlers to functions in IR (FOUNDATION)
3. **Phase 2**: Store function addresses in properties (WIRING)
4. **Phase 3**: Implement indirect function calls (EXECUTION)
5. **Phase 4**: Comprehensive testing (VALIDATION)

**Total Time**: 3-5 hours of focused work

**Critical Path**: Phase 0 must complete first to make game playable. Phases 1-3 are sequential dependencies.

## Dependencies and Prerequisites

**Required Infrastructure**:
- `IrInstruction::CallIndirect` variant (check if exists, add if needed)
- Function generation must happen before object table generation (verify order)
- PropertyManager must assign consistent property numbers across IR and codegen

**Required Knowledge**:
- Z-Machine indirect function call mechanism (call_vs with variable operand)
- Property-based function dispatch pattern
- UnresolvedReference system for function address patching

## Risks and Mitigation

**Risk 1: Breaking Change to IrRoom Structure**
- **Impact**: All code using IrRoom.on_enter/on_exit/on_look must be updated
- **Mitigation**: Compiler will catch all usage sites; fix each one
- **Likelihood**: HIGH (guaranteed to break)
- **Severity**: LOW (compiler errors, not runtime bugs)

**Risk 2: Codegen Phase Ordering**
- **Impact**: Function addresses unavailable when generating object tables
- **Mitigation**: Use UnresolvedReference pattern like exits do
- **Likelihood**: MEDIUM (may already be correct order)
- **Severity**: MEDIUM (requires architectural change if wrong)

**Risk 3: Property Number Consistency**
- **Impact**: Properties assigned different numbers in IR vs codegen
- **Mitigation**: Use PropertyManager as single source of truth
- **Likelihood**: LOW (PropertyManager already used in both places)
- **Severity**: HIGH (runtime errors, hard to debug)

**Risk 4: Test Regression**
- **Impact**: Changes break existing passing tests
- **Mitigation**: Run tests after each phase, rollback if failures occur
- **Likelihood**: LOW (changes are additive, not modifying existing behavior)
- **Severity**: HIGH (can't ship broken tests)

## Related Bugs

- Bug #18: Jump instruction emission (fixed)
- Bug #16: Store instruction form (fixed)
- Bug #15: ir_id_from_property marking (fixed)
- Bug #14: get_prop_len V3 two-byte format (fixed)

## Files Modified

### To Modify:
- `src/grue_compiler/ir.rs` (IrRoom structure, function generation)
- `src/grue_compiler/codegen_objects.rs` (property address storage)
- `src/grue_compiler/codegen.rs` (phase ordering if needed)

### To Review:
- `examples/mini_zork.grue` (handler definitions)
- `src/grue_compiler/semantic.rs` (handler analysis)
- `src/grue_compiler/ast.rs` (RoomDecl structure)

## References

- Z-Machine Specification section on properties and routines
- CLAUDE.md "CRITICAL: Z-Machine Stack vs Local Variable Specification"
- docs/ARCHITECTURE.md "Exit System Architecture"
