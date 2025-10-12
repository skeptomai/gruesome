# Bug: Property 21 (exit_types) Not Serialized to Z3 File

**Date**: October 11, 2025
**Status**: Root cause identified - Property exists in compiler but not in runtime
**Severity**: Critical - Blocks all navigation commands
**Related**: BUG_EXIT_DESTINATION_VARIABLE_CORRUPTION.md

## Problem Statement

Property 21 (exit_types) is created by the compiler but does not appear in the compiled Z3 file, causing `get_prop_addr` to return 0 at runtime.

## Evidence

### Compiler Creates Property 21 ✅

```
[2025-10-12T00:15:43Z DEBUG] 🔍 ROOM_PROPS: Room 'west_of_house' has 7 properties:
[2025-10-12T00:15:43Z DEBUG] 🔍   Property 19: Byte(0)
[2025-10-12T00:15:43Z DEBUG] 🔍   Property 20: Bytes([255, 255, 255, 255, 255, 255])  # exit_directions
[2025-10-12T00:15:43Z DEBUG] 🔍   Property 21: Bytes([0, 0, 1])                        # exit_types ← EXISTS!
[2025-10-12T00:15:43Z DEBUG] 🔍   Property 7: String("You are standing in an open field west of a white house, with a boarded front door.")
[2025-10-12T00:15:43Z DEBUG] 🔍   Property 22: Bytes([0, 2, 0, 3, 255, 255])           # exit_data
[2025-10-12T00:15:43Z DEBUG] 🔍   Property 13: Word(0)
[2025-10-12T00:15:43Z DEBUG] 🔍   Property 14: Byte(0)
```

Property 21 is successfully created with `room_properties.set_bytes(exit_types_prop, exit_types.clone())` in `codegen_objects.rs:498`.

### Runtime Cannot Find Property 21 ❌

```
[2025-10-12T00:12:55Z ERROR] 🔧 EXEC: PC=0x10b7, Instruction {
    opcode: 18,                                  # get_prop_addr
    operands: [Variable(0), SmallConstant(21)],  # Stack (obj 2), property 21
    store_var: Some(236)
}
[2025-10-12T00:12:55Z ERROR] 🔍 WRITE_VAR_236: value=0x0000 (0), PC=0x10b7
```

`get_prop_addr(obj=2, prop=21)` returns **0** instead of a valid property table address.

### Comparison with Property 20

Property 20 (exit_directions) **IS** found at runtime:

```
[2025-10-12T00:12:55Z ERROR] 🔧 EXEC: PC=0x10ae, Instruction {
    opcode: 18,                                  # get_prop_addr
    operands: [Variable(0), SmallConstant(20)],  # Stack (obj 2), property 20
    store_var: Some(235)
}
[2025-10-12T00:12:55Z ERROR] 🔍 get_property_addr: obj=2, prop=20 -> addr=0x03bf, size=6
```

Property 20 is found at address 0x03bf with size 6 bytes ✅

## Root Cause Hypothesis

Property 21 is:
1. Created in compiler's `PropertySet` ✅
2. Listed in debug output ✅
3. **NOT** written to Z3 file during serialization ❌
4. Not found at runtime ❌

Likely causes:
1. **Property ordering/size bug**: Property 21 has size 3 bytes, Property 20 has size 6 bytes, Property 22 has size 6 bytes
2. **Two-byte format issue**: Properties > 4 bytes use two-byte size format (Bug 10 fix), but property 21 is exactly 3 bytes
3. **Skipped during iteration**: Property serialization loop may skip property 21 due to ordering or size validation
4. **V3 property encoding**: Property 21 with 3 bytes may hit edge case in `encode_property_value()`

## Serialization Code Path

Property serialization happens in `src/grue_compiler/codegen.rs`:

1. **Line 4892-4936**: `serialize_properties()` - Main property serialization function
2. **Line 759-852** (codegen_strings.rs): `encode_property_value()` - Encodes individual property values

Key code in `serialize_properties()`:
```rust
// Sort properties in descending order by property number (required by Z-Machine spec)
let mut sorted_props: Vec<_> = properties.properties.iter().collect();
sorted_props.sort_by(|a, b| b.0.cmp(a.0));

for (prop_num, prop_value) in sorted_props {
    let encoded = self.encode_property_value(*prop_num, prop_value, version)?;
    // Write size byte(s) and data...
}
```

## What Happens with Property 21

Given properties: 22, 21, 20, 19, 14, 13, 7

After sorting (descending): 22, 21, 20, 19, 14, 13, 7

Serialization should write:
1. Property 22 (6 bytes): Two-byte format [0x96, 0x06, data...]
2. Property 21 (3 bytes): One-byte format [0x55, data...] (size=3, 32*(3-1)+21=0x55)
3. Property 20 (6 bytes): Two-byte format [0x94, 0x06, data...]
4. ...

**Question**: Is property 21 being skipped during this loop? Or is it being written incorrectly?

## Debugging Commands

### Check Raw Property Table in Z3 File

```bash
# Object 2 property table starts at address (from object tree)
# Find property bytes for properties 20, 21, 22
xxd tests/mini_zork.z3 | grep -A 10 "03bf"
```

### Check Property Serialization Logging

```bash
RUST_LOG=debug ./target/debug/grue-compiler examples/mini_zork.grue -o /tmp/test.z3 2>&1 | \
    grep -E "Property 2[0-2]|encode_property_value"
```

### Compare Compiled vs Source Properties

```bash
# Source properties (compiler internal)
RUST_LOG=debug ./target/debug/grue-compiler examples/mini_zork.grue -o /tmp/test.z3 2>&1 | \
    grep "west_of_house.*Property 2[0-2]"

# Runtime properties (from Z3 file)
echo "look" | RUST_LOG=error ./target/debug/gruesome tests/mini_zork.z3 2>&1 | \
    grep "get_property_addr.*prop=2[0-2]"
```

## Impact Chain

This bug causes the complete corruption chain documented in BUG_EXIT_DESTINATION_VARIABLE_CORRUPTION.md:

1. `get_prop_addr(obj=2, prop=21)` returns **0** (property not found)
2. Code stores 0 in Variable 236
3. Code sets Variable 239 = 0
4. `loadb Variable(236), Variable(239)` loads from address **0+0 = 0**
5. Memory[0] = **3** (Z-Machine version byte)
6. Value 3 is shifted: `3 << 14` = **0xC000**
7. 0xC000 is OR'd with 0x0300 = **0xC300**
8. 0xC300 stored in exit variable
9. `.destination` extracts 0xC300 & 0x3FFF = **768**
10. Navigation fails: "Invalid object number: 768"

## Next Steps

1. **Add serialization logging**: Log every property written during serialization
2. **Verify property table**: Dump raw bytes of property table for object 2
3. **Check encode_property_value**: Ensure 3-byte properties use one-byte format correctly
4. **Check loop iteration**: Ensure no properties are skipped during serialization loop
5. **Verify sort order**: Confirm properties are sorted descending (22, 21, 20, ...)

## Key Files

- `src/grue_compiler/codegen_objects.rs:498` - Property creation (`set_bytes`)
- `src/grue_compiler/codegen.rs:4892-4936` - Property serialization (`serialize_properties`)
- `src/grue_compiler/codegen_strings.rs:759-852` - Property encoding (`encode_property_value`)
- `src/vm.rs:433-459` - Property reading at runtime (`get_property_info`)

## Success Criteria

Property 21 must:
1. Appear in compiler debug output ✅ (already working)
2. Be written to Z3 file property table ❌ (currently broken)
3. Be found by `get_prop_addr` at runtime ❌ (currently broken)
4. Return a valid address (not 0) ❌ (currently broken)
5. Contain correct 3-byte value [0, 0, 1] ✅ (already working in compiler)
