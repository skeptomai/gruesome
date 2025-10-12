# Implementation Plan: Location as Containment Only

**Date**: October 12, 2025
**Status**: Ready for Implementation
**Decision**: Remove property 10 (location), use only object tree containment

## Executive Summary

Remove `StandardProperty::Location` from the property system and make `.location` purely an alias for object tree parent relationships. Reads use `get_parent`, writes use `insert_obj`, and initial locations are set via object tree initialization at compile time.

## Current State Analysis

**StandardProperty::Location = 13** is defined in `src/grue_compiler/ir.rs:457`
- Registered as standard property in PropertyManager (ir.rs:336)
- Maps to property number (dynamically assigned, typically 8-10)
- Used in property reads (.location) → GetObjectParent (ir.rs:2960-2967) ✅ CORRECT
- Used in property writes (.location =) → SetPropertyByNumber (ir.rs:2196-2217) ❌ WRONG

**Key Files Involved:**

1. **src/grue_compiler/ir.rs**
   - Line 336: `register_standard_property(StandardProperty::Location)`
   - Line 358: Case for "location" in property name matching
   - Line 444-458: StandardProperty enum definition with `Location = 13`
   - Line 2960-2967: Property READ handling (already uses GetObjectParent ✅)
   - Line 2186-2218: Property WRITE handling (uses SetPropertyByNumber ❌)

2. **src/grue_compiler/codegen_objects.rs**
   - Line 213: Adds "location" to all_properties set
   - Line 369: Gets location property number for player object
   - Line 570: Gets location property number for serialization

3. **examples/*.grue files** - 28 test files use `player.location = room`

## Changes Required

### Phase 1: Remove Location from Property System

**File: src/grue_compiler/ir.rs**

1. **Remove from PropertyManager registration** (Line 336):
   ```rust
   // DELETE THIS LINE:
   manager.register_standard_property(StandardProperty::Location);
   ```

2. **Remove from property name matching** (Line 358):
   ```rust
   // DELETE THIS CASE in match statement:
   StandardProperty::Location => "location",
   ```

3. **Remove from enum** (Line 457):
   ```rust
   // DELETE THIS LINE:
   Location = 13,   // Object location (parent)
   ```

4. **Add assignment handling for .location** (Line 2186, BEFORE property assignment logic):
   ```rust
   crate::grue_compiler::ast::Expr::PropertyAccess { object, property } => {
       // Special handling for .location assignment - use insert_obj instead of property
       if property == "location" {
           log::debug!("🏃 LOCATION_WRITE: Using InsertObj for .location assignment");
           let object_temp = self.generate_expression(*object, block)?;

           block.add_instruction(IrInstruction::InsertObj {
               object: object_temp,
               destination: value_temp,
           });
       } else {
           // Normal property assignment logic (existing code)...
       }
   }
   ```

**File: src/grue_compiler/codegen_objects.rs**

1. **Remove from all_properties** (Line 213):
   ```rust
   // DELETE THIS LINE:
   all_properties.insert("location".to_string());
   ```

2. **Remove location property lookups** (Lines 369, 570):
   ```rust
   // DELETE OR COMMENT OUT THESE LINES:
   let location_prop = *self.property_numbers.get("location").unwrap_or(&8);
   ```
   Note: Check if these are actually used - if so, need to refactor the code that uses them.

### Phase 2: Initialize Object Tree at Compile Time

**File: src/grue_compiler/codegen.rs** (object tree generation section)

**Goal**: Track `.location = room` assignments during IR generation and set initial parent pointers in object tree during codegen.

**Approach**:

1. **During IR generation**: Detect statements like `player.location = west_of_house` in init block
   - Store mapping: `object_id → initial_parent_id`
   - This happens when processing InsertObj instructions in init

2. **During codegen (build_object_tree)**:
   - For each object, check if it has an initial parent in the mapping
   - Set the parent pointer in the object entry based on version:
     - V3: Byte at offset 4
     - V4+: Word at offset 6
   - Also set child/sibling pointers to maintain tree structure

**Implementation Details**:
```rust
// In CodeGenerator struct, add:
initial_locations: HashMap<IrId, IrId>,  // object_id → parent_id

// During IR processing of init block:
if let IrInstruction::InsertObj { object, destination } = instruction {
    self.initial_locations.insert(object, destination);
}

// During object tree generation:
for obj_id in objects {
    if let Some(parent_id) = self.initial_locations.get(&obj_id) {
        let obj_num = self.ir_id_to_object_num[&obj_id];
        let parent_num = self.ir_id_to_object_num[parent_id];

        // Set parent pointer in object entry
        let obj_addr = self.get_object_tree_address(obj_num);
        let parent_offset = if version <= 3 { 4 } else { 6 };

        if version <= 3 {
            self.memory[obj_addr + parent_offset] = parent_num as u8;
        } else {
            self.write_word(obj_addr + parent_offset, parent_num);
        }

        // Also update parent's child pointer and sibling chain
        self.insert_into_tree(obj_num, parent_num);
    }
}
```

**Logging**: Add compilation logs showing initial locations being set:
```rust
log::warn!(
    "🏗️ INITIAL_LOCATION: Setting object {} parent to {} at compile time",
    obj_num, parent_num
);
```

### Phase 3: Test Coverage

**Test Files to Update**: None - syntax stays the same

**Files Using `player.location = room`** (28 files):
- `examples/mini_zork.grue` (line 452) ⚠️ **CRITICAL**
- `examples/test_object_minimal.grue`
- `examples/test_branch_complex.grue`
- `examples/no_grammar.grue`
- `examples/test_minimal.grue`
- `examples/test_quit_command.grue`
- `examples/test_verb_only_grammar.grue`
- `examples/test_label_defer.grue`
- `examples/test_object_branch.grue`
- `examples/test_array_literal_loop.grue`
- `examples/sread_test.grue`
- `examples/test_minimal_reference.grue`
- `examples/test_property_simple.grue`
- `examples/minimal_grammar.grue`
- `examples/test_branch_simple.grue`
- `examples/one_verb.grue`
- `examples/test_loop_with_quit.grue`
- `examples/test_property_minimal.grue`
- Plus 10 more test files

**Test Strategy**:

1. **Compile all test files**: `cargo test` should pass
2. **Run mini_zork navigation test**:
   ```bash
   echo -e "north\nlook\nsouth\nlook\nquit\ny" | RUST_LOG=warn timeout 3 ./target/debug/gruesome tests/mini_zork.z3
   ```
3. **Verify compilation logs**: Check for "INITIAL_LOCATION" messages showing parent pointers being set
4. **Run interpreter with object tree logging**: Add logging to show parent relationships at game start
5. **Regression testing**: All 174 existing tests must pass

### Phase 4: Validation (Using Instrumentation, NOT xxd)

**DO NOT use xxd or reverse engineering. Use our own tools:**

1. **Compiler Instrumentation**:
   ```bash
   RUST_LOG=warn cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3 2>&1 | grep "INITIAL_LOCATION"
   ```
   Should show parent pointers being set at compile time.

2. **Object Tree Dump Tool** (add to interpreter):
   ```bash
   ./target/debug/gruesome tests/mini_zork.z3 --dump-objects
   ```
   Should show:
   ```
   Object 1 (player): parent=2 (west_of_house), sibling=0, child=0
   Object 2 (west_of_house): parent=0, sibling=3, child=1
   ```

3. **Runtime Verification** (add to interpreter startup):
   ```rust
   // In interpreter initialization, log object tree:
   log::error!("🌳 OBJECT_TREE at game start:");
   for obj_num in 1..=num_objects {
       let parent = vm.get_parent(obj_num)?;
       let child = vm.get_child(obj_num)?;
       let sibling = vm.get_sibling(obj_num)?;
       log::error!("  Object {}: parent={}, child={}, sibling={}",
           obj_num, parent, child, sibling);
   }
   ```

4. **Navigation Test with Logging**:
   ```bash
   echo "north" | RUST_LOG=error ./target/debug/gruesome tests/mini_zork.z3 2>&1 | grep "get_parent\|insert_obj"
   ```
   Should show get_parent returning correct room, insert_obj moving player.

5. **Compare Before/After**:
   - Save compilation output before changes
   - Save compilation output after changes
   - Use diff to verify property 13 is gone, parent pointers are set

**NO manual hex inspection. Use logging and built-in tools only.**

## Safety Analysis

**What Could Go Wrong:**

1. **Initial location not set**: Objects have parent=0 at game start
   - **Mitigation**: MUST implement Phase 2 compile-time initialization
   - **Detection**: Compilation logs + object tree dump will show parent=0
   - **Fallback**: Use move() in init block as temporary workaround

2. **Property number reuse**: Removing Location frees up property 13
   - **Impact**: Next property gets number 13 instead of 14
   - **Safety**: ✅ Property numbers are relative, this is safe

3. **Existing Z3 files**: Games compiled before this change have property 13
   - **Impact**: ⚠️ Old Z3 files incompatible with new compiler
   - **Safety**: ✅ Expected - compiler version change

4. **Test file breakage**: Many test files use player.location =
   - **Impact**: ⚠️ Tests will fail if Phase 2 not implemented correctly
   - **Safety**: ✅ Tests will catch bugs before production

5. **codegen_objects.rs dependencies**: Code might read location property number
   - **Impact**: ⚠️ Compilation errors or runtime bugs
   - **Safety**: ✅ Rust compiler will catch missing property number lookups

## Implementation Order

### Step 1: Phase 1 (Remove from Property System)
1. Remove StandardProperty::Location from enum (ir.rs:457)
2. Remove registration call (ir.rs:336)
3. Remove from name mapping match (ir.rs:358)
4. Add special case for .location assignment → InsertObj (ir.rs:2186)
5. Remove from codegen_objects.rs (lines 213, 369, 570)
6. **Verify**: `cargo build` succeeds

### Step 2: Phase 2 (Initialize Object Tree)
1. Add initial_locations HashMap to CodeGenerator
2. Track InsertObj in init block during IR processing
3. Set parent pointers during object tree generation
4. Update child/sibling pointers to maintain tree structure
5. Add compilation logging for initial locations
6. **Verify**: Compilation logs show INITIAL_LOCATION messages

### Step 3: Phase 3 (Testing)
1. Run `cargo test` (expect some failures initially)
2. Fix any broken tests (likely none needed)
3. Compile mini_zork and check logs
4. Run navigation test
5. **Verify**: All 174 tests pass

### Step 4: Phase 4 (Validation)
1. Add --dump-objects flag to interpreter
2. Add object tree logging at startup
3. Run mini_zork with logging enabled
4. Verify parent pointers correct at game start
5. Verify navigation works (player moves between rooms)
6. Verify .location reads return correct room
7. **Verify**: All instrumentation shows correct behavior

## Rollback Plan

If implementation fails at any step:

**Option A - Quick Rollback**:
1. Revert all changes: `git reset --hard HEAD`
2. Keep only the `.location` read fix (GetObjectParent)
3. Document that `.location =` assignment doesn't work

**Option B - Fallback to move()**:
1. Complete Phase 1 (remove property)
2. Skip Phase 2 (no compile-time initialization)
3. Update all test files to use `move(player, room)` instead of `player.location = room`
4. Document that initialization requires move()

This is **Option 3** from the bug document - simplest implementation, requires game code changes.

**Option C - Add Compiler Warning**:
1. Complete Phase 1
2. Add compiler warning when `.location =` used in non-init contexts
3. Make `.location =` work ONLY in init block
4. Use move() for runtime location changes

## Success Criteria

Implementation is successful when:

1. ✅ All 174 tests pass
2. ✅ Mini_zork compiles without errors
3. ✅ Compilation logs show initial locations being set
4. ✅ Object tree dump shows correct parent pointers
5. ✅ Navigation commands work (north, south, etc.)
6. ✅ `.location` reads return correct room
7. ✅ `move()` builtin continues to work
8. ✅ No property 13 ("location") in compiled Z3 files
9. ✅ No synchronization bugs possible (architectural guarantee)

## Timeline Estimate

- **Phase 1**: 30-45 minutes (straightforward deletions + one InsertObj case)
- **Phase 2**: 60-90 minutes (tracking logic + tree initialization)
- **Phase 3**: 15-30 minutes (run tests, verify)
- **Phase 4**: 30-45 minutes (add instrumentation, validate)

**Total**: 2-3.5 hours for full implementation and validation

**Risk Level**: Medium (Phase 2 complexity, but well-scoped)

## Notes

- Property reads (.location) already use GetObjectParent - no change needed ✅
- Property writes (.location =) currently use SetPropertyByNumber - MUST change to InsertObj
- Object tree is the single source of truth for containment
- Initial locations must be set at compile time for game to start correctly
- No runtime overhead - tree operations are native Z-Machine opcodes
