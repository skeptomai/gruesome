# Exit System Implementation Plan

## Status: COMPLETE ✅

**Update Oct 7, 2025**: The exit system was already fully implemented in `codegen_objects.rs:generate_object_tables()` (line 414). This document was created based on incorrect analysis that missed the existing implementation.

**What Was Fixed**:
1. Added `room_to_object_id: IndexMap<IrId, u16>` field to `ZMachineCodeGen` (codegen.rs:235)
2. Added `setup_room_to_object_mapping()` method to populate the mapping (codegen.rs:7402)
3. Fixed exit data generation to use object numbers instead of IR IDs (codegen_objects.rs:552-562)

**Where Exit System Lives**:
- Full implementation: `src/grue_compiler/codegen_objects.rs:414-700`
- Accessor methods: `src/grue_compiler/codegen_builtins.rs:809-1002`
- Grammar handlers: `src/grue_compiler/codegen.rs:5954-5994`

## Overview

The exit system implementation includes:

## Current State - What Exists ✅

### Phase 0: AST/IR Representation ✅
- **AST**: `RoomDecl.exits: HashMap<String, ExitTarget>` (`ast.rs:90`)
- **AST**: `ExitTarget::Room(String)` and `ExitTarget::Blocked(String)` (`ast.rs:112-115`)
- **IR**: `IrRoom.exits: IndexMap<String, IrExitTarget>` (`ir.rs:175`)
- **IR**: `IrExitTarget::Room(IrId)` and `IrExitTarget::Blocked(String)` (`ir.rs:462-465`)
- IR generation converts AST exits to IR exits (`ir.rs:1731-1742`)

### Phases 1-3: Accessor Methods ✅
**Location**: `src/grue_compiler/codegen_builtins.rs:809-1002`

- `exit.none()` - checks if exit value == 0 (method call)
- `exit.blocked` - checks if bit 14 set (value >= 0x4000) (property access)
- `exit.destination` - extracts lower 14 bits for room ID (property access)
- `exit.message` - extracts lower 14 bits for message address (property access)

**Exit Value Encoding**: `(type << 14) | data`
- Type 0 (bit 14 clear): Normal exit, data = room object ID
- Type 1 (bit 14 set): Blocked exit, data = message string address

### Grammar System ✅
**Location**: `src/grue_compiler/codegen.rs:5954-5994`

- Direction commands (north, south, etc.) correctly call `handle_go(direction_string)`
- Grammar handlers pass string arguments as dictionary addresses
- **Bug fixed Oct 6, 2025**: Arguments were being ignored (underscore prefix)

## Critical Gaps - What's Missing ❌

### GAP 1: Property Registration ❌
**Problem**: Properties `exit_directions`, `exit_types`, `exit_data` are never registered in the `property_numbers` HashMap.

**Impact**: `get_exit` builtin uses fallback values (20, 21, 22) which may conflict with other properties.

**Location**: Property registration happens in `codegen.rs` but these three properties are absent.

### GAP 2: Room Object Generation ❌
**Problem**: `generate_objects_to_space()` only creates a minimal stub object table (1 dummy object).

**Current**: `src/grue_compiler/codegen_objects.rs:52` - only generates minimal compliance structure
**Needed**: Real rooms from `ir.rooms` must be translated to Z-Machine objects

**Impact**: No room objects exist in the Z-Machine, so all room references fail.

### GAP 3: Exit Property Data Generation ❌
**Problem**: Exit properties are never created on room objects.

**Required Properties** (for each room):
1. **`exit_directions`** (property 20): Array of dictionary addresses
   - Format: `[count, dict_addr1, dict_addr2, ...]`
   - Example: `[0x0003, 0x0655, 0x0678, 0x069A]` for "north", "south", "east"

2. **`exit_types`** (property 21): Array of exit types
   - Format: `[count, type1, type2, ...]`
   - Values: 0 = normal exit, 1 = blocked exit
   - Example: `[0x0003, 0x0000, 0x0001, 0x0000]`

3. **`exit_data`** (property 22): Array of destinations or messages
   - Format: `[count, data1, data2, ...]`
   - Values: room object ID (if normal) or message address (if blocked)
   - Example: `[0x0003, 0x0002, 0x1A3C, 0x0005]`

**How `get_exit` uses these**:
1. Get address of `exit_directions` property on room
2. Loop through directions comparing with input direction
3. When match found at index N:
   - Check `exit_types[N]` to see if blocked
   - Get `exit_data[N]` as destination/message
4. Encode result: `(type << 14) | data`

### GAP 4: Room-to-Object ID Mapping ❌
**Problem**: IR has room IDs (arbitrary), but Z-Machine needs object numbers (1, 2, 3, ...).

**Needed**: Mapping from `IrId` (room IR ID) → `u16` (Z-Machine object number)

**Impact**:
- Exit destinations must reference valid object numbers
- `move(player, room)` expects object number, not IR ID

### GAP 5: Direction Strings in Dictionary ❌
**Problem**: Exit direction strings must be in dictionary for runtime comparison.

**Already Solved**: Direction words are added as grammar verbs, so they're in dictionary.
**Verification Needed**: Ensure all direction strings used in exits are in dictionary.

## Implementation Plan

### PHASE 1: Property Registration
**File**: `src/grue_compiler/codegen.rs` (in property setup)

**Action**: Add to `property_numbers` HashMap:
```rust
property_numbers.insert("exit_directions".to_string(), 20);
property_numbers.insert("exit_types".to_string(), 21);
property_numbers.insert("exit_data".to_string(), 22);
```

**Verification**: Check that `get_exit` builtin finds these properties.

### PHASE 2: Room-to-Object ID Mapping
**File**: `src/grue_compiler/codegen.rs` or helper module

**Action**: Create mapping during codegen setup:
```rust
// In ZMachineCodeGen struct, add field:
room_to_object_id: HashMap<IrId, u16>

// During initialization:
let mut room_to_object_id = HashMap::new();
for (index, room) in ir.rooms.iter().enumerate() {
    // Object numbers start at 1 (0 is invalid)
    room_to_object_id.insert(room.id, (index + 1) as u16);
}
self.room_to_object_id = room_to_object_id;
```

**Verification**: Print mapping during compilation, verify sequential IDs.

### PHASE 3: Full Object Table Generation
**File**: `src/grue_compiler/codegen_objects.rs`

**Action**: Replace `generate_objects_to_space()` stub with full implementation.

#### Step 3.1: Calculate Sizes
```rust
let default_props_size = 62; // 31 properties * 2 bytes (V3)
let num_rooms = ir.rooms.len();
let obj_entries_size = num_rooms * 9; // V3: 9 bytes per object

// Calculate property table sizes
let mut prop_tables_size = 0;
for room in &ir.rooms {
    prop_tables_size += calculate_room_property_size(room);
}

let total_size = default_props_size + obj_entries_size + prop_tables_size;
```

#### Step 3.2: Write Default Property Table
```rust
let mut offset = 0;
for _ in 0..31 { // V3 has 31 default properties
    write_word(offset, 0x0000)?; // Default value = 0
    offset += 2;
}
```

#### Step 3.3: Write Object Entries
For each room in sequential order:
```rust
// V3 Object Entry (9 bytes):
write_to_object_space(offset, 0)?; offset += 1; // Attributes byte 0
write_to_object_space(offset, 0)?; offset += 1; // Attributes byte 1
write_to_object_space(offset, 0)?; offset += 1; // Attributes byte 2
write_to_object_space(offset, 0)?; offset += 1; // Attributes byte 3
write_to_object_space(offset, 0)?; offset += 1; // Parent (0 = no parent)
write_to_object_space(offset, next_sibling)?; offset += 1; // Sibling (next room or 0)
write_to_object_space(offset, 0)?; offset += 1; // Child (0 = no children)

// Property table pointer
let prop_table_addr = calculate_property_table_address(room_index);
write_word(offset, prop_table_addr)?;
offset += 2;
```

#### Step 3.4: Write Property Tables
```rust
for room in &ir.rooms {
    // Short name (text-length byte + encoded text)
    write_short_name(room.short_name)?;

    // Exit properties (see Phase 4)
    write_exit_properties(room)?;

    // Other properties (description, etc.)
    // ...

    // Terminator
    write_to_object_space(offset, 0)?;
    offset += 1;
}
```

### PHASE 4: Exit Property Data Generation
**File**: `src/grue_compiler/codegen_objects.rs` (property generation)

**Core Algorithm**:
```rust
fn write_exit_properties(&mut self, room: &IrRoom) -> Result<(), CompilerError> {
    if room.exits.is_empty() {
        return Ok(()); // No exits = no properties
    }

    let exit_count = room.exits.len();

    // Collect parallel arrays
    let mut directions = Vec::new();
    let mut types = Vec::new();
    let mut data = Vec::new();

    for (dir_name, target) in &room.exits {
        // Look up direction word in dictionary
        let dict_addr = self.lookup_word_in_dictionary(dir_name)?;
        directions.push(dict_addr);

        match target {
            IrExitTarget::Room(room_id) => {
                types.push(0); // Type 0 = normal exit
                let obj_num = self.room_to_object_id[room_id];
                data.push(obj_num);
            }
            IrExitTarget::Blocked(message) => {
                types.push(1); // Type 1 = blocked exit
                let msg_addr = self.allocate_string(message)?;
                data.push(msg_addr);
            }
        }
    }

    // Emit property 20: exit_directions
    self.emit_array_property(20, exit_count, &directions)?;

    // Emit property 21: exit_types
    self.emit_array_property(21, exit_count, &types)?;

    // Emit property 22: exit_data
    self.emit_array_property(22, exit_count, &data)?;

    Ok(())
}

fn emit_array_property(&mut self, prop_num: u8, count: usize, values: &[u16]) -> Result<(), CompilerError> {
    // Size/Number byte: property number in lower 5 bits, size-1 in upper 3 bits
    // Size = (count + 1) * 2 bytes (1 word for count + N words for data)
    let size_bytes = (count + 1) * 2;
    let size_minus_one = size_bytes - 1;
    let size_num_byte = (size_minus_one as u8) << 5 | prop_num;

    self.write_to_object_space(self.object_address, size_num_byte)?;
    self.object_address += 1;

    // Write count
    self.write_word(self.object_address, count as u16)?;
    self.object_address += 2;

    // Write data
    for &value in values {
        self.write_word(self.object_address, value)?;
        self.object_address += 2;
    }

    Ok(())
}
```

**Property Format Details**:

Z-Machine V3 property format:
```
Byte 0:     Size/Number byte: SSSNNNNNN
            SSS = size - 1 (in bytes)
            NNNNNN = property number (0-31)
Bytes 1-N:  Property data
```

For array of 3 words (6 bytes):
```
0x94  = 10010100 binary
        ^^^ = 4 (size - 1 = 5, so size = 6 bytes)
          ^^^^^ = 20 (property number)
```

### PHASE 5: Integration & Testing

#### Test 1: Property Registration
```bash
env RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test.z3 2>&1 | grep "exit_directions\|exit_types\|exit_data"
```
Expected: Should see properties registered.

#### Test 2: Object Table Structure
```bash
env RUST_LOG=error ./target/debug/gruedasm-txd /tmp/test.z3 2>&1 | grep -A10 "Object.*forest_path"
```
Expected: Should see object entry with property table pointer.

#### Test 3: Navigation
```bash
echo -e "east\nlook\nquit\nyes" | env DISPLAY_MODE=terminal ./target/debug/gruesome /tmp/test.z3
```
Expected: Should move to forest room, not "You can't go that way."

#### Test 4: Blocked Exit
Define a blocked exit in mini_zork.grue:
```grue
exits: {
    north: blocked("The door is locked.")
}
```
Test: `north` should print the message, not "You can't go that way."

## Key Data Structures

### Z-Machine Object Entry (V3)
```
Offset 0-3:   Attributes (32 bits)
Offset 4:     Parent object number (0 = none)
Offset 5:     Sibling object number (0 = none)
Offset 6:     Child object number (0 = none)
Offset 7-8:   Property table address (word pointer)
Total: 9 bytes
```

### Z-Machine Property Table
```
Byte 0:       Text-length (of short name)
Bytes 1-N:    Encoded short name (Z-characters)
Bytes N+1...: Properties (in descending order by number)
Last byte:    0x00 (terminator)
```

### Exit Property Arrays
All three properties use same format:
```
Word 0:       Count (number of exits)
Word 1:       First element
Word 2:       Second element
...
Word N:       Nth element
```

## Architecture Notes

### Why Parallel Arrays?
The exit system uses three parallel arrays instead of a single packed structure for efficiency:
- Can iterate over directions without unpacking full exit data
- Dictionary word comparison is fast (just compare addresses)
- Type/data lookup is O(1) once index is found

### Property Numbers
Standard properties: 1-13 (location, name, etc.)
Custom properties: 14+
Exit properties: 20-22 (chosen to avoid conflicts)

### Exit Encoding in Return Value
```
Bits 15-14: Exit type
  00 = normal exit (room ID in bits 13-0)
  01 = blocked exit (message address in bits 13-0)
  10-11 = reserved

Bits 13-0: Data (room ID or message address)
```

Max room ID: 16383 (14 bits)
Max message address: 16383 (within 64KB address space)

## Common Pitfalls

1. **Forgetting count word**: Exit arrays must start with count, not data
2. **Wrong property size**: Must be `(count + 1) * 2` bytes
3. **Object numbering**: Objects start at 1, not 0
4. **Dictionary order**: Direction words must be in sorted dictionary
5. **Property order**: Properties must be in descending order by number

## Files to Modify

### Primary Changes
- `src/grue_compiler/codegen_objects.rs` - Complete rewrite of object generation
- `src/grue_compiler/codegen.rs` - Add property registration, room-to-object mapping

### Supporting Changes
- `src/grue_compiler/codegen.rs` - Add `room_to_object_id` field to struct
- `src/grue_compiler/codegen_builtins.rs` - Verify `get_exit` works correctly (should be fine)

### No Changes Needed
- `src/grue_compiler/ir.rs` - Exit representation already complete
- `src/grue_compiler/ast.rs` - Exit AST already complete
- `src/grue_compiler/parser.rs` - Exit parsing already complete

## Success Criteria

Exit system is complete when:
1. ✅ All direction commands recognized (grammar fixed)
2. ✅ Accessor methods work (`.none()`, `.blocked`, `.destination`, `.message`)
3. ❌ Room objects generated with correct property tables
4. ❌ Exit properties contain correct parallel array data
5. ❌ Navigation works: `north`, `south`, `east`, `west` move between rooms
6. ❌ Blocked exits print messages instead of moving
7. ❌ `move()` builtin works with exit destinations

Current status: 2/7 complete (Grammar + Accessors)
