# Leaflet Property Access Bug Investigation

## Bug Description

The `obj.desc` property access returns the object name instead of the actual description content.

**Expected Behavior:**
- `examine leaflet` should print the full description: "WELCOME TO ZORK!..."
- `read leaflet` should print the full description: "WELCOME TO ZORK!..."

**Actual Behavior:**
- Both commands print just "leaflet" (the object name)

## Evidence

### Source Code Definition
In `examples/mini_zork.grue` lines 17-21:
```grue
object leaflet {
    names: ["leaflet", "paper"]
    desc: "\"WELCOME TO ZORK!\n\nZORK is a game of adventure, danger, and low cunning. In it you will explore some of the most amazing territory ever seen by mortals. No computer should be without one!\""
    takeable: true
}
```

### Function Usage
Both `examine()` (line 268) and `handle_read()` (line 417) use:
```grue
println(obj.desc);
```

### Test Results
```
> examine leaflet
leaflet
> read leaflet
leaflet
```

Both print "leaflet" instead of the description content.

## Root Cause Analysis

**BUG IDENTIFIED**: Property access mechanism correctly generates Z-Machine `get_prop` instructions, but the property values may not be properly stored or the object tables may have incorrect property addresses.

**Key Evidence:**
1. `obj.desc` correctly generates `GetPropertyByNumber` IR instruction (line 3501 in ir.rs)
2. Code generation correctly emits `get_prop` Z-Machine instruction (line 607 in codegen_instructions.rs)
3. Result is correctly marked with `ir_id_from_property` flag (line 602)
4. `print()` correctly uses `print_paddr` for property values (lines 80, 168, 340, 414 in codegen_builtins.rs)

**The bug is NOT in property access logic - it's in property storage or object table generation.**

The `get_prop` instruction is retrieving the wrong value from the object tables, returning what appears to be object name data instead of the actual `desc` property content.

## EXACT BUG IDENTIFIED

**PROOF FROM OBJECT GENERATION DEBUG OUTPUT:**
```
STRING_PROPERTY: Property 7 string='leaflet' -> ID 1022
PROP_WRITE: Writing property 7 for object 'leaflet': ... string_id=Some(1022)
```

**BUG CONFIRMED:** Property #7 (`desc`) for the leaflet object is incorrectly storing the object name `'leaflet'` instead of the actual description content `"WELCOME TO ZORK!..."`.

The property access mechanism is working correctly - the bug is in property value assignment during object table generation.

## Investigation Focus

The bug is in how the leaflet object's `desc` property is stored in the object tables during code generation, NOT in property access.

## REAL BUG IDENTIFIED

**ACTUAL ROOT CAUSE:** The `desc` property content is not being processed from the AST. Debug output shows:
```
📦 Collected object description: '[expression]' -> ID 1019
```

The actual description content `"WELCOME TO ZORK!..."` is being stored as placeholder text `'[expression]'` instead of the real content.

**Progress Made:**
- ✅ Fixed property mapping: `'desc'` now correctly maps to property #7
- ❌ **NEW BUG:** AST property values are not being processed correctly

**The issue is in AST → IR conversion, not property access or storage.**

## Expected Fix Impact

Fixing property storage will resolve:
- Leaflet description display bug
- All object description display throughout the game
- Any other property storage bugs (container contents descriptions, etc.)