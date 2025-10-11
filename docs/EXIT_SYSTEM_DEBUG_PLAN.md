# Exit System Debugging Plan

## Current Status (as of commit 39ff1f6)

### What Works ✅
- get_exit loop correctly iterates with proper indices (0, 1, 2)
- exit_directions property (20) exists and contains correct dictionary addresses
- exit_types property (21) exists and contains correct type bytes [0, 0, 1]
- Object 2 (west_of_house) IS being written to object table at 0x0327
- Property table pointer (0x03b0) is correct
- All 201 tests pass

### What's Broken ❌
- Property 22 (exit_data) is MISSING from property table
- Navigation fails because get_prop_addr returns 0 for missing property
- Without exit_data, get_exit cannot determine room IDs or message addresses

## Bug Analysis: Missing Property 22

### Evidence from hex dump at 0x03b0 (west_of_house property table)
```
0x03b0: 0096 06 [text: 00 16 00 17 05 6e] = short name "West of House"
0x03b9: 55 [data: 00 00 01] = property 21 (exit_types), 3 bytes
0x03bd: 94 06 [data: 07 df 07 f7 07 b5] = property 20 (exit_directions), 6 bytes
0x03c5: 13 [data: 00] = property 19 (1 byte)
```

Property 22 should appear BEFORE property 21 (properties are in descending order), but it's completely absent.

### Code Location
Exit properties set at `codegen_objects.rs:455-458`:
```rust
room_properties.set_bytes(exit_directions_prop, direction_addrs);
room_properties.set_bytes(exit_types_prop, exit_types);
room_properties.set_bytes(exit_data_prop, exit_data);
```

## Debugging Strategy

### Phase 1: Trace Property Collection (30 min)

**Goal:** Verify exit_data Vec is correctly populated and stored in IrProperties

1. **Check what's in `exit_data` Vec before set_bytes:**
   - Add debug logging at `codegen_objects.rs:458`
   - Log: `exit_data` length, contents as hex
   - Expected for west_of_house: `[room_id_h, room_id_l, room_id_h, room_id_l, 0xFF, 0xFF]`
   - Should be 6 bytes: north (2 bytes), south (2 bytes), east blocked (2 bytes placeholder)

2. **Check IrProperties after set_bytes:**
   - Add debug logging after line 458
   - Call `room_properties.properties.get(&exit_data_prop)`
   - Verify: Property 22 exists in the HashMap
   - Check: Value type is `PropertyValue::Bytes` with correct length

3. **Check property serialization entry point:**
   - Find where `create_property_table_from_ir` is called (codegen.rs:4691)
   - Add logging to show which properties are being serialized
   - Check if property 22 is present in the input ObjectData

### Phase 2: Trace Property Serialization (30 min)

**Goal:** Find where property 22 gets lost during serialization

4. **Find property writing code:**
   - Search for `create_property_table_from_ir` implementation
   - Look for property iteration/sorting logic (properties must be in descending order)
   - Check for filters that might skip properties

5. **Check property 22 encoding:**
   - Property 22 with 6 bytes should use two-byte format: `96 06 [6 bytes]`
   - Check if `encode_property_value` handles 6-byte properties correctly
   - Verify: size > 4 triggers correct two-byte encoding (see Bug 10 fix)
   - Check if `PropertyValue::Bytes` with length=6 is handled

6. **Check for empty/placeholder filtering:**
   - Properties with all 0xFF might be filtered out as "empty"
   - Check if `IrProperties.set_bytes()` skips "empty" arrays
   - Check if serialization code skips properties with placeholder values
   - Look for validation logic that rejects specific byte patterns

### Phase 3: Root Cause Identification (20 min)

**Goal:** Determine exact cause of missing property

7. **Hypothesis 1: set_bytes filters 0xFF values**
   - Check `IrProperties::set_bytes()` implementation in `ir.rs`
   - Look for validation that might reject placeholder values
   - Test: Does set_bytes accept Vec with 0xFF bytes?

8. **Hypothesis 2: Serialization skips properties with placeholders**
   - UnresolvedReferences might be processed incorrectly
   - Property 22 has placeholders that need StringRef resolution
   - Check: Are properties written before or after UnresolvedReference resolution?
   - Check: Does property serialization skip properties with pending UnresolvedRefs?

9. **Hypothesis 3: Property ordering/numbering issue**
   - Property 22 might collide with existing property number
   - Check `property_numbers` HashMap to verify 22 is correctly mapped
   - Check: Are properties correctly sorted in descending order during serialization?
   - Verify: Property 22 appears before property 21 in the output

### Phase 4: Fix Implementation (45 min)

**Goal:** Implement fix and verify no regressions

10. **Once root cause found, implement fix:**
    - If filtering issue: Modify set_bytes to accept placeholders
    - If serialization issue: Ensure properties written before resolution OR write placeholders correctly
    - If ordering issue: Fix property number assignment or sorting logic

11. **Test the fix:**
    - Recompile mini_zork: `RUST_LOG=warn cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3`
    - Check hex dump shows property 22: `xxd -s 0x03b0 -l 80 tests/mini_zork.z3`
    - Test navigation: `echo "east" | timeout 2 ./target/debug/gruesome tests/mini_zork.z3`
    - Expected: Should show blocked message "The door is boarded..." instead of "You can't go that way"

12. **Verify no regressions:**
    - Run full test suite: `cargo test --lib`
    - Check other rooms with exits still work
    - Test normal exits: `echo -e "north\nlook" | timeout 2 ./target/debug/gruesome tests/mini_zork.z3`
    - Verify north/south directions work correctly

## Key Files to Examine

### 1. src/grue_compiler/codegen_objects.rs (lines 400-470)
- Exit property generation
- set_bytes calls for exit_directions, exit_types, exit_data
- Placeholder creation for blocked exits (0xFF, 0xFF)

### 2. src/grue_compiler/ir.rs (IrProperties implementation)
- `set_bytes()` method
- Property storage logic
- Validation or filtering that might skip properties

### 3. src/grue_compiler/codegen.rs (lines 4690-4750)
- `create_property_table_from_ir` implementation
- Property iteration and serialization
- Property ordering (descending by property number)

### 4. src/grue_compiler/codegen_strings.rs (lines 759-852)
- `encode_property_value` function
- Two-byte format handling for sizes > 4
- PropertyValue::Bytes encoding

## Debug Commands

```bash
# Compile with debug logging
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3 2>&1 | grep -A5 "exit_data\|Property 22"

# Compile with warn logging (less verbose)
RUST_LOG=warn cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3

# Check hex dump of west_of_house property table
xxd -s 0x03b0 -l 80 tests/mini_zork.z3

# Check object table for object 2
xxd -s 0x0327 -l 16 tests/mini_zork.z3

# Test navigation (should show blocked message after fix)
echo -e "look\neast" | timeout 2 ./target/debug/gruesome tests/mini_zork.z3 2>&1 | tail -10

# Test with error logging (shows LOADW operations)
echo -e "look\neast" | RUST_LOG=error timeout 2 ./target/debug/gruesome tests/mini_zork.z3 2>&1 | grep "LOADW\|LOADB"

# Run tests
cargo test --lib

# Run specific navigation test
cargo test --test go_handler_test test_go_handler_dispatch
```

## Expected Timeline

- **Phase 1-2 (Investigation):** 1 hour
- **Phase 3 (Root Cause):** 20 minutes
- **Phase 4 (Fix + Test):** 45 minutes
- **Total:** ~2 hours

## Success Criteria

1. ✅ Property 22 appears in hex dump between properties 23 and 21
2. ✅ Property 22 contains 6 bytes of exit data (2 bytes per exit)
3. ✅ Navigation command "east" shows blocked message "The door is boarded..."
4. ✅ All 201 tests pass
5. ✅ No regressions in other navigation tests (north, south work correctly)
6. ✅ Other rooms with exits continue to function properly

## Related Bugs

### Bug 9: exit_data String Addresses (FIXED)
- Exit data for blocked exits needed packed string addresses
- Fixed by writing placeholders (0xFF, 0xFF) and creating StringRef UnresolvedReferences
- See `codegen_objects.rs:434-440` and commit history

### Bug 10: V3 Two-Byte Property Format (FIXED)
- Properties > 4 bytes need two-byte format in V3
- Fixed in both compiler (writing) and interpreter (reading)
- See `ARCHITECTURE.md` and commit 018212b

### Bug 18: inc_chk Off-by-One (FIXED - commit 39ff1f6)
- Loop was using inc_chk (0x05) which increments before checking
- Fixed by using jl (0x02) with inverted branch logic
- Array indices now correct: 0, 1, 2

## Architecture Notes

### Exit System Design
The exit system uses three parallel arrays stored as separate properties:

- **Property 20 (exit_directions):** Packed array of dictionary addresses (2 bytes each)
- **Property 21 (exit_types):** Array of type codes (1 byte each): 0=room, 1=blocked
- **Property 22 (exit_data):** Packed array of data values (2 bytes each): room_id OR message_addr

At runtime, `get_exit(direction)` returns: `(type << 14) | data`

This encoding allows:
- Normal exits: type=0, data=room_object_number → return value = 0x0000-0x3FFF
- Blocked exits: type=1, data=packed_message_addr → return value = 0x4000-0x7FFF

The calling code checks bit 14 to determine exit type and extracts data from lower 14 bits.

### Property Table Format (V3)
Properties must be in **descending order** by property number:

```
[short_name_size] [short_name_text]
[prop_header] [prop_data]  ← highest property number
[prop_header] [prop_data]
...
[prop_header] [prop_data]  ← lowest property number
[0x00]                     ← terminator
```

For sizes > 4 bytes, use two-byte header format:
```
[0x80 | prop_num] [size_byte] [data...]
```

Example for property 22 with 6 bytes:
```
0x96 0x06 [6 bytes of data]
```
Where: 0x96 = 0x80 | 0x16 (property 22 = 0x16 in hex)
