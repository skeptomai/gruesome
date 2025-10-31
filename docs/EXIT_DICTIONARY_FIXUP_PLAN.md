# Exit System Dictionary Address Fixup - Implementation & Testing Plan

## Problem Statement

**Current Bug**: Navigation commands fail because exit property dictionary addresses are incorrect.

**Root Cause**: Exit direction properties store dictionary word addresses calculated during Phase 2 (object generation) using `self.dictionary_addr`, but this address is not finalized until Phase 3 (final assembly).

**Symptom**:
- Compilation: "east" direction stored as 0x062B
- Runtime: "east" dictionary lookup finds 0x07B5
- Result: `get_exit("east")` comparison fails, returns 0 (no exit found)

## Infrastructure Status: ✅ COMPLETE

All infrastructure already exists:
1. ✅ `LegacyReferenceType::DictionaryRef { word: String }` (codegen.rs:165)
2. ✅ Dictionary resolution code (codegen.rs:1388-1412)
3. ✅ `MemorySpace::Objects` support (codegen.rs:52, 1903)
4. ✅ `lookup_word_in_dictionary_with_fixup()` helper (codegen.rs:6195-6235)

**What's Missing**: Exit property generation uses wrong function!

## Implementation Plan

### Step 1: Fix Exit Property Dictionary Lookups ⚙️

**File**: `src/grue_compiler/codegen_objects.rs`
**Location**: Lines 525-543
**Complexity**: LOW (5 lines changed)

**Current Code (BROKEN)**:
```rust
let dict_addr = match self.lookup_word_in_dictionary(direction) {
    Ok(addr) => addr,  // ← Wrong! Uses unfinalized dictionary_addr
    ...
};
direction_addrs.push((dict_addr >> 8) as u8);
direction_addrs.push((dict_addr & 0xFF) as u8);
```

**Fixed Code**:
```rust
// Calculate byte offset in property data where this address will be written
let property_offset = direction_addrs.len();

// Use placeholder that will be patched in Phase 3
let placeholder = placeholder_word();
direction_addrs.push((placeholder >> 8) as u8);
direction_addrs.push((placeholder & 0xFF) as u8);

// Find word position in dictionary for later resolution
let position = self.dictionary_words
    .iter()
    .position(|w| w == &direction.to_lowercase())
    .ok_or_else(|| CompilerError::CodeGenError(format!(
        "Direction '{}' not in dictionary", direction
    )))? as u32;

// Calculate final location in assembled story file
// This will be: object_base + property_table_offset + property_data_offset
// For now, store relative offset and adjust during property finalization
let location_offset = /* track this during property generation */;

// Create unresolved reference
self.reference_context.unresolved_refs.push(UnresolvedReference {
    reference_type: LegacyReferenceType::DictionaryRef {
        word: direction.clone(),
    },
    location: location_offset,  // Will be adjusted when property base is known
    target_id: position,  // Dictionary word position
    is_packed_address: false,
    offset_size: 2,
    location_space: MemorySpace::Objects,
});
```

**Challenge**: We need to track the absolute byte offset where each property will be written.

### Step 2: Track Property Data Offsets 📍

**File**: `src/grue_compiler/codegen_objects.rs`
**Location**: Property generation loop (around line 500-650)
**Complexity**: MEDIUM (offset tracking logic)

**Current**: Property bytes are accumulated in vectors, but final location not tracked
**Needed**: Track cumulative offset from object table base as properties are generated

**Implementation**:
```rust
// At start of property generation for a room
let mut property_data_offset = 0usize;  // Offset from start of this room's property table

// When adding exit_directions bytes
let exit_directions_offset = property_data_offset;
// ... add property header bytes ...
property_data_offset += property_header_size;

for (i, (direction, _)) in room.exits.iter().enumerate() {
    let direction_addr_offset = property_data_offset + (i * 2);  // 2 bytes per entry

    // Create unresolved reference with THIS offset
    self.pending_property_refs.push(PendingPropertyRef {
        room_name: room.name.clone(),
        property_offset: direction_addr_offset,
        reference_type: LegacyReferenceType::DictionaryRef {
            word: direction.clone(),
        },
        target_id: position,
    });
}

property_data_offset += direction_addrs.len();
```

Then after ALL properties are written and object table base is known:
```rust
// Convert pending refs to real UnresolvedReferences
for pending in self.pending_property_refs.drain(..) {
    let room_obj_num = /* lookup room's object number */;
    let property_table_offset = /* calculate from object headers */;
    let final_location = self.object_table_base
        + (room_obj_num * object_entry_size)
        + property_table_offset
        + pending.property_offset;

    self.reference_context.unresolved_refs.push(UnresolvedReference {
        reference_type: pending.reference_type,
        location: final_location,
        target_id: pending.target_id,
        is_packed_address: false,
        offset_size: 2,
        location_space: MemorySpace::Objects,
    });
}
```

### Step 3: Alternative Simpler Approach (RECOMMENDED) 🎯

**Instead of complex offset tracking, write properties AFTER object base is known:**

Currently properties are built into vectors, then written. We already know the structure. So:

1. Build property data into temporary buffers (current behavior)
2. Calculate where each byte will land in final memory
3. Write property bytes to `object_space`
4. Create UnresolvedReferences pointing to correct locations in `object_space`

**Key Insight**: Property data is written sequentially to `object_space`. We can calculate absolute offsets as we write!

**Implementation**:
```rust
// When writing exit_directions property data
let property_data_start = object_space.len();  // Current position

for (direction, _) in &room.exits {
    let dict_ref_location = object_space.len();  // Exact position for this address

    // Write placeholder
    object_space.push(0xFF);
    object_space.push(0xFF);

    // Create unresolved reference
    let position = self.dictionary_words
        .iter()
        .position(|w| w == &direction.to_lowercase())
        .unwrap() as u32;

    self.reference_context.unresolved_refs.push(UnresolvedReference {
        reference_type: LegacyReferenceType::DictionaryRef {
            word: direction.clone(),
        },
        location: dict_ref_location,  // EXACT byte offset in object_space
        target_id: position,
        is_packed_address: false,
        offset_size: 2,
        location_space: MemorySpace::Objects,
    });
}
```

**Why This Works**:
- `object_space.len()` gives exact offset where next byte will be written
- UnresolvedReference resolution adds `self.final_object_base` to this offset (see codegen.rs:1903)
- Result: Correct absolute address in final story file

## Testing Plan

### Test 1: Verify Placeholder Generation ✓

**Objective**: Confirm placeholders are written to exit_directions property

**Method**:
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3 2>&1 | grep "DICT.*placeholder\|exit_directions"
```

**Expected Output**:
```
Writing placeholder 0xFFFF for direction 'east' at object_space offset 0x???
Creating DictionaryRef for 'east' at location 0x???
```

**Success Criteria**:
- Placeholders (0xFFFF) written instead of immediate addresses
- Log confirms UnresolvedReference created

### Test 2: Verify Reference Creation ✓

**Objective**: Confirm UnresolvedReferences are created for each direction

**Method**:
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3 2>&1 | grep "UnresolvedReference.*Dictionary\|DictionaryRef"
```

**Expected Output**:
```
Created UnresolvedReference: DictionaryRef { word: "north" } at location 0x??? (Objects space)
Created UnresolvedReference: DictionaryRef { word: "east" } at location 0x??? (Objects space)
Created UnresolvedReference: DictionaryRef { word: "south" } at location 0x??? (Objects space)
```

**Success Criteria**:
- One reference per exit direction
- `location_space` is `MemorySpace::Objects`
- `target_id` is word position (0-22)

### Test 3: Verify Reference Resolution ✓

**Objective**: Confirm dictionary addresses are patched during Phase 3

**Method**:
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3 2>&1 | grep "DICT_RESOLVE"
```

**Expected Output**:
```
📖 DICT_RESOLVE: Word 'north' position 13 -> dict_base=0x078D + 4 + (13 * 6) = 0x07C5
📖 DICT_RESOLVE: Word 'east' position 6 -> dict_base=0x078D + 4 + (6 * 6) = 0x07AF
📖 DICT_RESOLVE: Word 'south' position 17 -> dict_base=0x078D + 4 + (17 * 6) = 0x07F1
```

**Success Criteria**:
- Final addresses match dictionary entries
- Addresses use finalized `self.dictionary_addr`

### Test 4: Binary Verification ✓

**Objective**: Confirm correct addresses are written to compiled Z3 file

**Method**:
```bash
# Compile
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3

# Find west_of_house property table in compiled file
# Object #2 (west_of_house) property table contains exit_directions property
# Extract and verify dictionary addresses

# Get dictionary base from header
xxd -l 16 tests/mini_zork.z3 | grep -o "078d"  # Should show 078d

# Find exit_directions property for west_of_house (property 20/0x14)
# Calculate expected addresses for north/east/south
# north (position 13): 0x078D + 4 + (13*6) = 0x07C5
# east (position 6):  0x078D + 4 + (6*6) = 0x07AF
# south (position 17): 0x078D + 4 + (17*6) = 0x07F1

xxd tests/mini_zork.z3 | grep -A 2 "07c5\|07af\|07f1"
```

**Expected**: Dictionary addresses in property data match calculated values

**Success Criteria**:
- Property bytes contain correct dictionary addresses
- No 0xFFFF placeholders in final file
- Addresses point to actual dictionary entries

### Test 5: Runtime Verification ✅

**Objective**: Confirm navigation works in-game

**Method**:
```bash
printf "north\n" | cargo run --bin gruesome tests/mini_zork.z3 2>&1 | grep -A 2 "North of House"
printf "east\n" | cargo run --bin gruesome tests/mini_zork.z3 2>&1 | grep "door is boarded"
```

**Expected Output**:
```
North of House
You are facing the north side of a white house...

The door is boarded and you can't remove the boards.
```

**Success Criteria**:
- "north" successfully moves to North of House room
- "east" triggers blocked exit message (not "You can't go that way")
- `get_exit()` returns correct packed values

### Test 6: End-to-End Gameplay Test ✅✅✅

**Objective**: Full navigation test through multiple rooms

**Method**:
```bash
printf "north\nsouth\nwest\neast\nquit\ny\n" | cargo run --bin gruesome tests/mini_zork.z3
```

**Expected**:
- Start: West of House
- north → North of House
- south → West of House (back)
- No "Invalid object" errors
- All directions resolve correctly

**Success Criteria**:
- No "You can't go that way" for valid exits
- Blocked exits show messages, not errors
- Game completes without interpreter errors

## Rollback Plan

If implementation fails:

1. **Revert codegen_objects.rs** to use `lookup_word_in_dictionary()` (current state)
2. **Alternative workaround**: Pre-calculate dictionary addresses during Phase 1, store in `IndexMap<String, u16>`, use those cached values during object generation
3. **Last resort**: Hard-code dictionary base offset (0x078D) and calculate addresses manually

## Implementation Sequence

1. ✅ Document current state (this file)
2. ⚙️ Implement Step 3 (Simpler Approach) - write placeholders + create refs as we write
3. ✓ Run Test 1: Verify placeholders written
4. ✓ Run Test 2: Verify references created
5. ✓ Run Test 3: Verify resolution works
6. ✓ Run Test 4: Binary verification
7. ✅ Run Test 5: Runtime navigation test
8. ✅✅✅ Run Test 6: Full gameplay test
9. 🎉 Update CLAUDE.md with Bug 7 status

## Estimated Effort

- Step 3 Implementation: 30-45 minutes
- Testing: 15 minutes
- Documentation: 10 minutes
- **Total: ~1 hour**

## Key Files

- `src/grue_compiler/codegen_objects.rs` - Object/property generation (FIX HERE)
- `src/grue_compiler/codegen.rs:1388-1412` - Dictionary reference resolution (works)
- `src/grue_compiler/codegen.rs:6195-6235` - Helper function example (reference)
- `examples/mini_zork.grue` - Test game
- `tests/mini_zork.z3` - Compiled output for testing
