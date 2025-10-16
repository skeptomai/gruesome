# Dynamic Descriptions Side Quest

**Date**: October 16, 2025
**Status**: DEFERRED - Documented for future implementation
**Priority**: Low (static descriptions work fine for now)

## What Happened

During debugging session, work began on converting `description: String` to `description: Expr` across the AST/Parser/IR/Codegen layers. This was **NOT needed to fix the active bugs** and was a side quest that should be deferred.

## What It Would Enable

**Current limitation (static descriptions only):**
```grue
object mailbox {
    desc "A small mailbox."  // Can't change based on state
}
```

**Future capability (dynamic descriptions):**
```grue
object mailbox {
    desc "The mailbox is " + (mailbox.open ? "open" : "closed") + "."
    // Description changes based on object.open property
}

room dark_room "Dark Room" {
    desc player.has_lamp ?
        "A dimly lit room with stone walls." :
        "It is pitch black. You are likely to be eaten by a grue."
}
```

## Implementation Status

**Partially Complete Infrastructure:**

1. ✅ AST: `RoomDecl.description` and `ObjectDecl.description` changed to `Expr` type
2. ✅ Parser: Uses `parse_expression()` instead of `consume_string()`
3. ✅ IR: `IrRoom.description` and `IrObject.description` changed to `Expr` type
4. ⚠️ Codegen: Pattern matches on `Expr::String` but **silently ignores** other expressions
5. ❌ Runtime evaluation: **Not implemented**

**The Problem:**
```rust
match &room.description {
    Expr::String(s) => {
        room_properties.set_string(desc_prop, s.clone()); // ✅ Works
    }
    _ => {
        // Runtime expression - will be evaluated at runtime when property is accessed
        // ❌ This is a LIE - nothing happens, property is never set!
    }
}
```

The parser **accepts** runtime expressions but codegen **silently ignores** them, resulting in missing description properties.

## What's Needed for Full Implementation

### Architectural Decision Required

Three possible approaches:

**Option 1: Synthetic Getter Functions**
- For each object with runtime desc, generate a synthetic `get_desc_<object>()` function
- Store function address in property 7 instead of string address
- Modify property access to call function if value is in code space range

**Option 2: Runtime Expression Evaluation**
- Store expression AST as bytecode instructions
- Generate evaluation code inline when property is accessed
- Complex but flexible

**Option 3: Computed Property System**
- Add new property type: "computed" (vs "static")
- Computed properties store function pointers
- Property access checks type and either returns value or calls function

### Implementation Tasks

1. **Choose architectural approach** (see above)
2. **Codegen changes:**
   - Detect non-String Expr in descriptions
   - Generate appropriate bytecode for evaluation
   - Store proper reference in property 7
3. **Runtime changes:**
   - Modify property access to handle computed properties
   - Ensure proper scoping (access to object.property within expression)
4. **Testing:**
   - Unit tests for expression parsing
   - Integration tests for runtime evaluation
   - Gameplay tests with dynamic descriptions

### Complexity Estimate

- Medium-High complexity (requires runtime evaluation infrastructure)
- Touches multiple compiler layers
- Needs careful design to avoid performance issues

## Files That Would Change

From the incomplete work:
- `src/grue_compiler/ast.rs` - Already changed (revert if deferred)
- `src/grue_compiler/parser.rs` - Already changed (revert if deferred)
- `src/grue_compiler/ir.rs` - Already changed (revert if deferred)
- `src/grue_compiler/codegen_objects.rs` - Needs full implementation
- `src/grue_compiler/codegen_instructions.rs` - May need property access changes
- Test files - Need updates for Expr types

## Why This Was Deferred

1. **Not needed for core functionality** - Static descriptions work fine for initial release
2. **Incomplete implementation worse than no implementation** - Silent failures are confusing
3. **Active bugs take priority** - Property 7 garbled text bug is blocking
4. **Good to have, not must have** - Zork-style dynamic descriptions are nice but not critical

## When to Revisit

Consider implementing when:
1. ✅ Core game is fully playable with static descriptions
2. ✅ All critical bugs are fixed
3. ✅ Test coverage is comprehensive
4. User feedback indicates need for dynamic descriptions
5. Have 2-3 days for focused implementation and testing

## References

- **Session Date**: October 16, 2025
- **Context**: Was investigating "Cannot insert object 0" crash, got sidetracked into desc expressions
- **Related Bugs**: Bug #18 (opcode 0x04), Property 7 garbled text (active)
- **Test Case**: `mini_zork.grue` works fine with static descriptions

## Recommendation

**Revert the partial changes and defer this feature.** The infrastructure is documented here for future reference. Focus on fixing the property 7 garbled text bug (the actual active bug) instead.
