# Exit System Implementation Plan

## Current Status (October 6, 2025)

### ✅ Completed (Phase 1)
- **Documentation**: Complete architecture in `docs/ARCHITECTURE.md`
- **Property Generation**: Exit tables encoded in room properties
  - Format: `[count][dir1_len][dir1_chars...][type1][data_hi][data_lo]...`
  - Single property per room (conserves property numbers)
  - Type 0 = room destination, Type 1 = blocked with inline message
- **Tests**: 6 comprehensive unit tests in `src/grue_compiler/codegen_tests.rs`
- **Placeholder Builtin**: `get_exit()` returns 0 (allows compilation)

### 🚧 Pending Implementation

#### Phase 2B: Full get_exit Builtin Logic
**Status**: Not started (currently returns placeholder 0)

**Complexity**: HIGH - Requires generating complex Z-Machine assembly

**What It Needs to Do**:
```
get_exit(room_object, direction_string) -> packed_result
  where packed_result = (type << 14) | data
  - type=0, data=room_id for normal exits
  - type=1, data=message_addr for blocked exits
  - result=0 for no exit found
```

**Implementation Requirements**:
1. **Get exit_table property from room**
   - Use Z-Machine `get_prop` instruction
   - Property number: 20 (exit_table)
   - Returns address of property data

2. **Parse table format**
   ```
   [count: 1 byte]
   [dir1_len: 1 byte][dir1_chars: N bytes][type1: 1 byte][data1_hi: 1 byte][data1_lo: 1 byte]
   [dir2_len: 1 byte][dir2_chars: N bytes][type2: 1 byte][data2_hi: 1 byte][data2_lo: 1 byte]
   ...
   ```

3. **Loop through exits**
   - Read count from first byte
   - For each exit (0 to count-1):
     - Read direction length
     - Compare direction bytes with input string
     - If match: extract type and data, pack result, return
     - If no match: advance to next exit entry

4. **String comparison**
   - Byte-by-byte comparison
   - Handle variable-length strings
   - Case-insensitive? (TBD)

5. **Pack and return result**
   - Shift type left 14 bits: `type << 14`
   - OR with data: `(type << 14) | data`
   - Return packed 16-bit value

**Z-Machine Instructions Needed**:
- `get_prop` - Read exit_table property
- `loadb` - Read bytes from property data
- `je` - Compare bytes (equality test)
- `add` - Advance pointer through table
- `jump` - Loop control
- `or` / `shift` - Pack result
- `store` / `load` - Variable management
- `call` - Possibly helper routines

**Estimated Code Size**: 50-100 Z-Machine instructions

**Challenges**:
- Variable-length data parsing in Z-Machine
- String comparison without built-in string ops
- Pointer arithmetic through byte arrays
- Maintaining loop state across iterations
- Handling edge cases (empty strings, no exits, etc.)

#### Phase 3: Accessor Methods
**Status**: Not started

**Complexity**: MEDIUM - Depends on Phase 2B result format

**Required Methods**:
1. **`.none()`** - Test if exit is null
   ```
   value.none() -> bool
   Returns: value == 0
   ```

2. **`.blocked`** - Test if exit is blocked
   ```
   value.blocked -> bool
   Returns: (value >> 14) == 1
   ```

3. **`.destination`** - Get room ID from exit
   ```
   value.destination -> room_id
   Returns: value & 0x3FFF
   ```

4. **`.message`** - Get blocked message
   ```
   value.message -> string
   Returns: string at address (value & 0x3FFF)
   ```

**Implementation Location**: `src/grue_compiler/ir.rs` method handlers

**Depends On**: Phase 2B must be complete to test these properly

---

## Alternative Approaches

### Option A: Keep Current Complex Design
**Pros**:
- Maximum flexibility
- Handles arbitrary number of exits
- Compact memory usage

**Cons**:
- Very complex Z-Machine code generation
- Hard to debug
- Performance concerns (string search on every movement)

### Option B: Pre-compile Exit Dispatch Functions
Instead of runtime table parsing, generate a specialized function for each room:

```rust
// For west_of_house with exits {north: room2, east: blocked("door")}
fn west_of_house_get_exit(direction) {
    if (direction == "north") return room2;
    if (direction == "east") return blocked_exit_marker;
    return 0;
}
```

**Pros**:
- Much simpler codegen (just if-statements)
- Faster at runtime (direct comparisons, no parsing)
- Easier to debug (readable Z-Machine code)

**Cons**:
- More code space (one function per room)
- Still need string comparison logic
- More complex IR structure

### Option C: Use Room Properties Directly
Store each exit as a separate property:

```
room.exit_north = room_id or blocked_marker
room.exit_south = room_id or blocked_marker
```

Then `get_exit(direction)` becomes:
```
property_name = "exit_" + direction
return room.get_property(property_name)
```

**Pros**:
- Simplest runtime logic
- Uses standard property system
- Fast property lookup

**Cons**:
- Limited property numbers (max 63 in v3)
- Can't concatenate strings at runtime in Z-Machine
- Would need fixed direction names only

### Option D: Dictionary-Based Lookup
Store direction strings in dictionary, use dictionary addresses as keys:

```
For each exit:
  dict_word = encode_to_dictionary(direction)
  room.exits_table[dict_word] = exit_data
```

**Pros**:
- Dictionary encoding is already implemented
- Fast word-based lookup instead of string comparison
- Leverages existing Z-Machine infrastructure

**Cons**:
- Still need table lookup logic
- Dictionary space constraints
- Complex address-based indexing

---

## Recommended Path Forward

### Step 1: Reconsider Architecture
The current "parse byte table at runtime" approach is very complex. Before implementing, we should:

1. **Prototype simpler approaches**
   - Try Option B (per-room dispatch functions)
   - Measure code size impact
   - Compare complexity

2. **Benchmark existing games**
   - How do real Infocom games handle exits?
   - Look at Zork I disassembly
   - Learn from proven patterns

3. **Simplify requirements**
   - Do we need arbitrary direction strings?
   - Can we limit to: n/s/e/w/ne/nw/se/sw/u/d/in/out?
   - Fixed set enables simpler encoding

### Step 2: If Keeping Current Design
1. **Write detailed pseudocode** for table parsing
2. **Implement helper functions** for:
   - String comparison
   - Table iteration
   - Result packing
3. **Test incrementally** with simple cases
4. **Add extensive logging** for debugging

### Step 3: Accessor Methods
These are simpler and can be done independently:
- Implement `.none()` first (trivial: test == 0)
- Implement `.blocked` and `.destination` (bit manipulation)
- Defer `.message` until string handling is clear

---

## Decision Points

**Key Questions to Answer**:

1. **Is runtime table parsing worth the complexity?**
   - Could we pre-compile dispatch functions instead?
   - What's the code size tradeoff?

2. **Can we simplify the direction string handling?**
   - Fixed direction set vs arbitrary strings?
   - Dictionary encoding for directions?

3. **What do real Z-Machine games do?**
   - Research existing exit system patterns
   - Learn from Inform or other compilers

4. **Should we implement Phase 3 first?**
   - Get accessor methods working with placeholder data
   - Defer complex lookup logic
   - Make incremental progress

5. **Is there a hybrid approach?**
   - Simple dispatch for common directions (n/s/e/w)
   - Fall back to table parsing for unusual directions
   - Best of both worlds?

---

## Research Findings (October 6, 2025)

### Inform 6 Exit System ✅

**Research Source**: IFWiki - Properties in Inform 6

**Finding**: Inform 6 (the industry-standard Z-Machine compiler) uses **individual properties per direction**:

- `n_to` - north exit
- `s_to` - south exit
- `e_to` - east exit
- `w_to` - west exit
- `ne_to`, `nw_to`, `se_to`, `sw_to` - diagonals
- `u_to` - up
- `d_to` - down
- `in_to`, `out_to` - special directions

**Property Values**:
Each direction property can hold:
1. **Room object ID** - normal exit
2. **Door object ID** - door that must be opened
3. **Routine address** - custom logic (for blocked exits, conditional passages)

**How It Works**:
```inform6
Room Kitchen
  with n_to Hallway,      // Simple room exit
       s_to locked_door,  // Door object
       e_to [;           // Routine for blocked exit
         print "The door is locked.";
         rtrue;
       ];
```

At runtime: `room.n_to` is just a property read - returns room ID, door ID, or routine address.

**Key Insights**:
- ✅ **Uses fixed property numbers** (n_to, s_to, etc. are pre-defined properties 1-48)
- ✅ **No runtime string comparison** - direction parsed in command parser, mapped to property
- ✅ **Simple property reads** - `get_prop` instruction, that's it
- ✅ **Proven pattern** - thousands of games use this successfully

**Implications for Grue**:
This validates **Option C** completely. We should:
1. Define fixed properties: `exit_north`, `exit_south`, `exit_east`, etc.
2. Store exit data directly in these properties
3. Map direction strings to property names at compile time (or in builtin)
4. Use simple `get_prop` at runtime

**Advantages**:
- Matches proven industry standard
- Simple runtime implementation
- Fast (single property read)
- No complex table parsing
- Easy to debug

**Property Number Concerns - RESOLVED**:
- Z-Machine v3 supports 63 properties
- We only need ~12 direction properties (n/s/e/w/ne/nw/se/sw/u/d/in/out)
- Plenty of room for other game properties
- This is why Inform pre-defines them in slots 1-48

---

## Revised Implementation Plan

### NEW Approach: Follow Inform 6 Pattern

#### Phase 2B-Revised: Implement Inform-Style Direction Properties

**Implementation Steps**:

1. **Define direction properties** in PropertyManager:
   ```rust
   - exit_north (property 20)
   - exit_south (property 21)
   - exit_east (property 22)
   - exit_west (property 23)
   - exit_northeast (property 24)
   - exit_northwest (property 25)
   - exit_southeast (property 26)
   - exit_southwest (property 27)
   - exit_up (property 28)
   - exit_down (property 29)
   - exit_in (property 30)
   - exit_out (property 31)
   ```

2. **Update room property generation** (codegen_objects.rs):
   ```rust
   for (direction, exit_target) in &room.exits {
       let prop_name = match direction.as_str() {
           "north" => "exit_north",
           "south" => "exit_south",
           // ... etc
       };
       let prop_num = self.property_numbers.get(prop_name);

       match exit_target {
           IrExitTarget::Room(id) => {
               room_properties.set_word(prop_num, *id as u16);
           }
           IrExitTarget::Blocked(msg) => {
               // Store string address (or special marker + message property)
               room_properties.set_string(prop_num, msg.clone());
           }
       }
   }
   ```

3. **Implement get_exit builtin** (codegen_builtins.rs):
   ```rust
   // Map direction string to property number
   let prop_num = match direction {
       "north" => 20,
       "south" => 21,
       // ... etc
   };

   // Single get_prop instruction
   emit_get_prop(room_object, prop_num, target_var);

   // Return value is:
   // - Room ID for normal exits
   // - String address for blocked exits (type identified by high bit or range)
   // - 0 if property doesn't exist (no exit)
   ```

**Complexity**: LOW - Just property reads and direction mapping

**Code Size**: ~10-20 Z-Machine instructions (property lookup + direction mapping)

**Estimated Time**: 1-2 hours

---

## Next Actions

**Immediate**:
1. ✅ Research complete - Inform 6 pattern identified
2. Remove old exit_table property generation
3. Implement direction property generation
4. Update get_exit builtin to use property reads
5. Test with mini_zork

**Future Enhancements**:
- Support door objects (like Inform 6)
- Support routine addresses for conditional exits
- Add more directions if needed

---

## References

- **Architecture Doc**: `docs/ARCHITECTURE.md` (lines 1728-1882)
- **Current Implementation**:
  - Property generation: `src/grue_compiler/codegen_objects.rs:396-435`
  - Placeholder builtin: `src/grue_compiler/codegen_builtins.rs:811-849`
- **Tests**: `src/grue_compiler/codegen_tests.rs:989-1196`
- **Example Game**: `examples/mini_zork.grue` (handle_go function)
