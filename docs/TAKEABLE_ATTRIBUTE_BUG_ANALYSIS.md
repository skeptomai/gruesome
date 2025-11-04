# Takeable Attribute Bug Analysis

## Issue Description
Players can execute `take tree` and put immovable objects like trees into their inventory, even though these objects are correctly marked as `takeable: false`.

## Root Cause Analysis

### Object Definition (Correct)
In `examples/mini_zork.grue` line 82, the tree is properly defined:
```grue
object tree {
    names: ["tree", "large tree"]
    desc: "The tree has low branches that look climbable."
    takeable: false  // ✅ CORRECTLY MARKED AS NON-TAKEABLE
}
```

### Bug Location: handle_take() Function
**Lines 308-328** in `examples/mini_zork.grue`:

```grue
fn handle_take(obj) {
    clear_quit_state();
    if obj.location == player {
        println("You already have that.");
        return;
    }

    if !player_can_see(obj) {
        println("You can't see any such thing.");
        return;
    }

    // Check if object is inside a closed container
    if obj.location.container && !obj.location.open {
        println("The " + obj.location.name + " is closed.");
        return;
    }

    move(obj, player);              // ❌ BUG: Never checks takeable attribute!
    println("Taken.");
}
```

**Missing Check:** The function never validates `obj.takeable` before moving the object.

### Correct Implementation Reference
The `take_all()` function (lines 476-491) **correctly implements** the check:

```grue
fn take_all() {
    let objects = player.location.contents();
    let taken = 0;

    for obj in objects {
        if obj != player && obj.takeable != false {  // ✅ CORRECT CHECK
            move(obj, player);
            println(obj.name + ": Taken.");
            taken = taken + 1;
        }
    }

    if taken == 0 {
        println("There's nothing here to take.");
    }
}
```

## Fix Required
Add takeable attribute check to `handle_take()` function:

```grue
fn handle_take(obj) {
    clear_quit_state();
    if obj.location == player {
        println("You already have that.");
        return;
    }

    if !player_can_see(obj) {
        println("You can't see any such thing.");
        return;
    }

    // NEW: Check if object is takeable
    if obj.takeable == false {
        println("You can't take that.");
        return;
    }

    // Check if object is inside a closed container
    if obj.location.container && !obj.location.open {
        println("The " + obj.location.name + " is closed.");
        return;
    }

    move(obj, player);
    println("Taken.");
}
```

## Test Case
- **Before Fix:** `take tree` → "Taken." (incorrect)
- **After Fix:** `take tree` → "You can't take that." (correct)

## Impact
This affects all objects marked with `takeable: false`, including:
- Trees, buildings, and other immovable scenery
- Objects that should be non-portable for gameplay reasons
- Any object explicitly designed to remain in place

## File to Modify
`examples/mini_zork.grue` - Add takeable check to `handle_take()` function around line 320.