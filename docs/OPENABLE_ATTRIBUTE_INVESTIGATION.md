# Z-Machine Object Attribute Initialization Bug Investigation

## Problem Statement

The mailbox object responds with "You can't open that." instead of opening properly, despite being declared with `openable: true` in the source code.

## Root Cause Analysis

### Source Code Declaration (Correct)
```grue
object mailbox {
    names: ["a small mailbox", "mailbox", "box"]
    desc: "The small mailbox is " + (mailbox.open ? "open" : "closed") + "."
    openable: true  // ← Correctly declared
    container: true
    // ...
}
```

### Generated IR Analysis
```
function handle_open (id=3):
  // ...
  TestAttribute { target: 109, object: 95, attribute_num: 2 }  // ← Testing openable (attribute #2)
  t110 = Not t109
  branch t110 ? L105 : L106
L105:
  t112 = String("You can't open that.")  // ← This path is taken
```

**Key Finding**: The `openable` property is correctly compiled as `TestAttribute` with `attribute_num: 2`.

### Z-Machine Object Table Analysis

Object 3 (mailbox) attribute bytes in compiled bytecode:
```
Object 3 at 0x0330:
  Attributes: 00 00 00 00
  Attribute 0 (bit 31): clear
  Attribute 1 (bit 30): clear
  Attribute 2 (bit 29): clear  ← openable should be SET but is CLEAR
  Attribute 3 (bit 28): clear  ← open (initially closed, correct)
```

### Missing Initialization Code

**Problem**: The compiler is **NOT generating** the required `SetAttribute` instruction to initialize the mailbox's openable attribute during startup.

**Expected IR (missing)**:
```
SetAttribute { object: 3, attribute_num: 2, value: true }  // ← This instruction is MISSING
```

**Actual IR**: No `SetAttribute` operations found for mailbox object attribute #2.

## Architecture Analysis

### Attribute vs Property Distinction
- **Attributes**: Boolean flags (32 bits per object, numbered 0-31)
  - `openable` → Attribute #2
  - `open` → Attribute #3
  - `container` → Attribute #1
- **Properties**: Variable values with property numbers
  - `desc` → Property #7
  - `names` → Property table

### Compilation Flow Issue
1. ✅ Parser correctly identifies `openable: true`
2. ✅ Semantic analysis maps `openable` to attribute #2
3. ✅ Code generation correctly emits `TestAttribute` for reading
4. ❌ **Code generation FAILS to emit `SetAttribute` for initialization**

## RESOLUTION

**ISSUE RESOLVED** ✅ - The openable attribute setting was working correctly throughout the entire pipeline.

### Root Cause Analysis Error

Initial investigation focused on object 3 based on legacy documentation, but the actual mailbox is object 10 in the current compilation. The attributes were correctly set at:

- **Object Space Offset**: 0x008F (object 10 attributes)
- **Final File Offset**: 0x036F (object_base 0x02e0 + 0x008F)
- **Actual Bytes**: `00 00 00 06` (Container=1 + Openable=2 = 0x06)

### Verification

```bash
xxd -s 0x036F -l 4 tests/debug_openable.z3
# Output: 0000036f: 0000 0006
```

The Z-Machine object table correctly contains the openable attribute. The remaining "open mailbox" failures are due to systematic stack imbalance issues, not attribute problems.

## Investigation Status (ARCHIVED)

**CRITICAL FINDING**: Duplicate `StandardAttribute` enum definitions with conflicting attribute numbers!

### Conflicting Attribute Assignments

**IR Version** (`/src/grue_compiler/ir.rs`):
```rust
pub enum StandardAttribute {
    Openable = 2,     // ← IR sets attribute #2
    Open = 3,
    // ...
}
```

**Object System Version** (`/src/grue_compiler/object_system.rs`):
```rust
pub enum StandardAttribute {
    Openable = 4,     // ← Object system expects attribute #4
    Open = 5,
    // ...
}
```

### Impact Analysis
1. ✅ **IR Processing**: Sets `openable: true` to attribute #2 during object creation
2. ✅ **Compilation**: `TestAttribute { attribute_num: 2 }` correctly tests attribute #2
3. ❌ **MISMATCH**: Two conflicting enum definitions could cause inconsistency

**ROOT CAUSE**: The two `StandardAttribute` enums have different numbering schemes, potentially causing initialization to use one numbering while testing uses another.

## Next Steps

**CRITICAL FIX REQUIRED**: The IR attribute setting code exists but the attributes aren't reaching the object table.

### Code Flow Analysis
1. ✅ **AST**: `openable: true` parsed correctly as `PropertyValue::Boolean(true)`
2. ✅ **IR Generation**: Code exists to set attribute via `attributes.set(StandardAttribute::Openable as u8, true)`
3. ✅ **Object Table Generation**: Code exists to write `object.attributes.flags` to Z-Machine object table
4. ❌ **BROKEN LINK**: `object.attributes.flags` is zero instead of having bit 29 set

### Investigation Required
- Debug why `IrAttributes.set()` isn't working or
- Debug why the IR attributes aren't being copied to `ObjectData.attributes`
- Add logging to trace the attribute values through the entire pipeline

### Immediate Action
Add debug logging to `ir.rs` attribute setting code to see if `StandardAttribute::Openable as u8` is actually being set during IR generation.

## Files Modified During Investigation

- `tests/stack_balance_test2.z3` - Test compilation with stack fixes applied
- This analysis document

## Technical Context

This investigation was conducted after successfully fixing systematic Z-Machine stack imbalance issues. The openable attribute bug is a separate, unrelated issue in the object initialization code generation phase of the compiler.