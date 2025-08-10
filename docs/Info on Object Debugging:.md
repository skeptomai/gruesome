# Info on Object Debugging

## Overview

The Z-Machine interpreter includes comprehensive object debugging capabilities to help understand the structure and relationships of game objects during development and testing.

## Object Table Structure

### Version-Specific Support
- **v3**: 255 objects, 32 attributes, 31 properties, 9-byte entries
- **v4+**: 65535 objects, 48 attributes, 63 properties, 14-byte entries

### Object Entry Format
Each object contains:
- **Attributes**: Bit flags for object properties (light, container, etc.)
- **Parent**: Object number of containing object/room
- **Sibling**: Next object at same hierarchy level
- **Child**: First child object (if container)
- **Description**: Short text name of object
- **Property Table**: Variable-length properties with data

## Debug Methods

### ObjectTable Debug Functions

#### `debug_dump_object(obj_num)`
Dumps detailed information about a specific object (1-based numbering):

```rust
if let Some(obj_table) = game.get_object_table() {
    obj_table.debug_dump_object(1);  // Dump object #1
}
```

**Example Output:**
```
[DEBUG gruesome::zobject] === Object #1 ===
[DEBUG gruesome::zobject]   Attributes: [14, 28]
[DEBUG gruesome::zobject]   Set attributes: [4, 5, 6, 11, 12, 13]
[DEBUG gruesome::zobject]   Parent: 247
[DEBUG gruesome::zobject]   Sibling: 2
[DEBUG gruesome::zobject]   Child: 0
[DEBUG gruesome::zobject]   Description: "pair of hands"
[DEBUG gruesome::zobject]   Properties address: 0x0bb8
[DEBUG gruesome::zobject]   Properties:
[DEBUG gruesome::zobject]     Property 18: [70, 220, 66, 194, 66, 180] (len=6)
[DEBUG gruesome::zobject]     Property 16: [130] (len=1)
```

#### `debug_dump_all()`
Dumps information for all objects in the table:

```rust
if let Some(obj_table) = game.get_object_table() {
    obj_table.debug_dump_all();
}
```

## Object Information Fields

### Attributes
- **Raw attributes**: Array of attribute numbers that are set
- **Set attributes**: Human-readable list of active attribute flags
- Common attributes:
  - Attribute 4-6: Light sources
  - Attribute 11-13: Container properties
  - Attribute 15: "On" state for lights

### Relationships  
- **Parent**: Containing object/room (0 = no parent)
- **Sibling**: Next object at same level in hierarchy
- **Child**: First contained object (0 = no children)

### Properties
- **Properties address**: Memory location of property table
- **Property entries**: Number, data bytes, and length
- Property data shown as byte arrays with interpretive comments

## Usage in Development

### Debugging Game State
Use object debugging to understand:
- Object hierarchy and containment
- Which objects have which attributes set  
- Property values and their meanings
- Relationship chains (parent → child → sibling)

### Common Debugging Patterns

```rust
// Check specific object
game.get_object_table()?.debug_dump_object(180); // West of House

// Find objects with light attributes
for obj in 1..50 {
    for light_attr in [4, 5, 6, 15] {
        if game.test_attribute(obj, light_attr)? {
            println!("Object {} has light attribute {}", obj, light_attr);
        }
    }
}

// Examine object relationships
let player_location = game.read_variable(16)?; // G16 = player location
game.get_object_table()?.debug_dump_object(player_location);
```

### Integration with Logging

The object debugging uses the Rust `log` crate with `DEBUG` level output:

```bash
# Enable debug logging to see object dumps
RUST_LOG=debug cargo run --bin gruesome game.z3

# Filter to only object debugging
RUST_LOG=debug cargo run --bin gruesome game.z3 2>&1 | grep zobject
```

## Technical Implementation

### Version-Aware Decoding
The object debugging automatically adapts to the Z-Machine version:
- v3: 32 attributes, properties 1-31, 9-byte object entries
- v4+: 48 attributes, properties 1-63, 14-byte object entries

### Memory Safety
All object debugging includes bounds checking:
- Object numbers validated against object table limits
- Property addresses verified before dereferencing
- Attribute numbers checked against version-specific limits

### Performance Considerations
Object debugging is designed for development use:
- Only enabled in debug builds by default
- Uses lazy evaluation for expensive operations
- Caches frequently accessed object table metadata

## Integration with Grue Compiler

The object debugging capabilities are particularly useful when testing compiled Grue programs:

```rust
// In compiled Grue code, objects get assigned specific IDs
// Use debugging to verify object relationships are correct
game.get_object_table()?.debug_dump_all(); // See all compiled objects

// Check that room/object hierarchy matches Grue source
game.get_object_table()?.debug_dump_object(room_id);
```

This helps ensure that the Grue → Z-Machine compilation process correctly generates object tables and maintains proper object relationships.