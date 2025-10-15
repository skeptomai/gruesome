# Mini Zork Object Structure Analysis

## Object vs Room Distinction

**CRITICAL**: In the Z-Machine object table, **rooms are NOT objects**. Only actual game objects (items, scenery, etc.) are stored in the object table.

### Why This Matters

When debugging object-related issues, you must understand that:
- **Rooms** are referenced by their identifiers (e.g., `west_of_house`, `forest_path`) in code but are NOT in the object tree
- **Objects** (mailbox, leaflet, tree, etc.) ARE in the object tree and have object IDs (1, 2, 3, ...)
- **The player** is always object ID 1 in the Z-Machine

This distinction affects:
1. Object ID calculations during debugging
2. Property lookups (rooms don't have properties in the object table)
3. Object tree navigation (parent/child/sibling relationships)
4. Dictionary word resolution (object names must be in dictionary, room names don't)

## Mini Zork Object Table Structure

Based on `examples/mini_zork.grue` as of October 15, 2025:

### Object ID Assignments

1. **player** (implicit, always object 1)
2. **mailbox** (in west_of_house)
   - Names: ["small mailbox", "mailbox", "box"]
   - Properties: openable, container
3. **leaflet** (inside mailbox)
   - Names: ["leaflet", "paper"]
   - Parent: mailbox
4. **window** (in behind_house)
   - Names: ["window", "small window"]
   - Properties: openable
5. **tree** (in forest_path) ✨ **NEWLY ADDED**
   - Names: ["tree", "large tree"]
   - Properties: takeable: false
   - Added to fix "examine tree" bug
6. **nest** (in up_a_tree room)
   - Names: ["bird's nest", "nest"]
   - Properties: container
7. **egg** (inside nest)
   - Names: ["jewel-encrusted egg", "egg", "large egg"]
   - Properties: openable, container, valuable
   - Parent: nest

### Room Structure (NOT Objects)

Rooms are referenced in code but not stored in the object table:

1. west_of_house - Starting location
2. north_of_house - North of the house
3. south_of_house - South of the house
4. behind_house - Behind the house (window object here)
5. forest_path - Forest path (tree object here)
6. up_a_tree - Up in the tree (nest object here) - **RENAMED** from `tree` to avoid conflict
7. forest - Dense forest
8. clearing - Forest clearing

## Dictionary Requirements

All object names from the `names:` property must be in the Z-Machine dictionary for the parser to recognize them.

### Complete Object Name List (Must Be In Dictionary)

From mailbox:
- "small mailbox"
- "mailbox"
- "box"

From leaflet:
- "leaflet"
- "paper"

From window:
- "window"
- "small window"

From tree:
- "tree"
- "large tree"

From nest:
- "bird's nest"
- "nest"

From egg:
- "jewel-encrusted egg"
- "egg"
- "large egg"

**Total: 13 distinct object vocabulary terms** (plus grammar verbs and other words)

## Bug History: Missing Tree Object

### The Problem (October 15, 2025)

User reported that "examine tree" produced no output. The forest_path room description mentioned "One particularly large tree with some low branches stands at the edge of the path" but there was no corresponding object.

### Root Cause

1. Room description referenced a tree
2. `handle_climb()` function (line 282) checked `if obj == tree`
3. **No tree object was defined in the game**

### The Fix

Added tree object to forest_path room:
```grue
object tree {
    names: ["tree", "large tree"]
    desc: "The tree has low branches that look climbable."
    takeable: false
}
```

Additionally renamed room `tree` to `up_a_tree` to avoid naming conflict.

### Impact

- Dictionary entries increased from 38 to 40 (added "tree" and "large tree")
- "examine tree" now works correctly
- "climb tree" can properly check for the tree object

## Dictionary Growth Timeline

1. **Before object name fix**: 25 entries (only grammar verbs)
2. **After object name fix**: 38 entries (all object names added)
3. **After tree addition**: 40 entries (tree vocabulary added)

## Naming Conflicts: Objects vs Rooms

**Issue**: Cannot have both an object named `tree` and a room named `tree` in the same scope.

**Resolution**: Room identifiers and object identifiers share the same namespace during compilation, so room `tree` was renamed to `up_a_tree` to avoid conflict with object `tree`.

## Debugging Commands

### Check dictionary contents:
```bash
RUST_LOG=debug cargo run --quiet --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test.z3 2>&1 | grep "Added dictionary entry"
```

### Count dictionary entries:
```bash
RUST_LOG=debug cargo run --quiet --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test.z3 2>&1 | grep "Total dictionary entries"
```

### Verify object name in dictionary:
```bash
echo "examine tree" | RUST_LOG=gruesome::dictionary=debug cargo run --quiet --bin gruesome tests/mini_zork.z3 2>&1 | grep tree
```

## Related Documentation

- See `docs/ARCHITECTURE.md` - "Grue Compiler Grammar System Architecture"
- See `CLAUDE.md` - Object tree iteration and property system bugs
