# Object Containment Strategy Analysis

## Problem Statement

**Root Issue**: Objects defined inside rooms (e.g., `mailbox` inside `west_of_house`) are not automatically placed in their containing rooms during compilation.

**Current State**:
- Objects are detected and registered correctly ✅
- No InsertObj instructions are generated for room containment ❌
- Both main branch and our branch have this issue

**Required Solution**: Generate `InsertObj` instructions to establish object tree relationships during init block generation.

**Challenge**: When init block is generated, room processing has already completed. Room AST data is no longer accessible.

## Option 1: Store Mapping During Room Generation

### Implementation Strategy:
```rust
// Add to IrGenerator struct:
room_objects: IndexMap<String, Vec<RoomObjectInfo>>,

struct RoomObjectInfo {
    name: String,
    nested_objects: Vec<RoomObjectInfo>, // For leaflet inside mailbox
}

// During generate_room():
let mut room_object_info = Vec::new();
for obj in &room.objects {
    room_object_info.push(extract_object_info(obj)); // Recursive for nested
}
self.room_objects.insert(room.identifier.clone(), room_object_info);

// During init generation:
for (room_name, objects) in &self.room_objects {
    let room_ir_id = *self.symbol_ids.get(room_name).unwrap();
    generate_placement_for_objects(objects, room_ir_id, block);
}
```

### Detailed Implementation:
1. **Room Generation Phase**: Record object hierarchy in mapping
2. **Init Generation Phase**: Convert mapping to InsertObj instructions
3. **Nested Objects**: Store full hierarchy in RoomObjectInfo structure
4. **Symbol Resolution**: Look up IR IDs during init when all symbols are finalized

### Pros:
- **Clear separation of concerns**: Room generation just records, init generation just places
- **Easy to debug**: Can inspect `room_objects` mapping at any time
- **Flexible timing**: Can decide exactly when in init block to inject placements
- **Explicit data flow**: Obvious where mapping comes from and where it's used
- **Easy error handling**: Symbol lookup failures during init give clear context
- **Maintainable**: Placement logic centralized in one location
- **Testable**: Can verify mapping contents independently of instruction generation

### Cons:
- **Extra state management**: Need to maintain mapping in IrGenerator struct
- **Redundant work**: Store object names, then look them up again later
- **Memory overhead**: Storing potentially large object trees
- **Complex nested handling**: Need to represent object hierarchy in mapping
- **String→IR ID lookup twice**: Once during room gen, once during init gen
- **Additional data structure**: Need to design RoomObjectInfo for nested objects

## Option 2: Generate Placement During Room Processing

### Implementation Strategy:
```rust
// Add to IrGenerator struct:
pending_placement_instructions: Vec<IrInstruction>,

// During generate_room():
for obj in &room.objects {
    self.register_object_and_nested(obj)?;

    // Generate immediately while we have full context
    let obj_ir_id = *self.symbol_ids.get(&obj.identifier).unwrap();
    self.pending_placement_instructions.push(IrInstruction::InsertObj {
        object: obj_ir_id,
        destination: room_id,
    });

    self.generate_nested_placements(obj, obj_ir_id)?; // Recursive
}

// During init generation:
block.instructions.extend(self.pending_placement_instructions.drain(..));
```

### Detailed Implementation:
1. **Room Generation Phase**: Generate InsertObj instructions immediately
2. **Pending Storage**: Store instructions in temporary vector
3. **Init Integration**: Inject stored instructions into init block
4. **Nested Handling**: Recursive AST traversal during room generation

### Pros:
- **Efficient**: Generate instructions once with full context, no redundant lookups
- **Natural nested handling**: Recursive AST traversal handles object hierarchy cleanly
- **Immediate IR generation**: Convert AST→IR while AST is available
- **Less memory**: Instructions smaller than storing full object hierarchy
- **Compiler design principle**: Generate IR as soon as you have the info needed
- **Single symbol lookup**: IR IDs resolved once during room generation
- **No intermediate data structures**: Direct AST→IR conversion

### Cons:
- **Mixed concerns**: Room generation now also handles placement logic
- **Temporal coupling**: Instructions generated early but injected later
- **Harder debugging**: Placement logic split across two phases
- **Potential instruction loss**: If something corrupts pending instructions
- **Less flexible**: Harder to change when/how placement instructions are injected
- **Implicit ordering**: Instruction generation order depends on room processing order

## Key Decision Points

### **Nested Object Complexity:**
- **Option 1**: Need to design data structure for `mailbox contains leaflet`
  - Requires recursive RoomObjectInfo traversal
  - More complex data modeling
- **Option 2**: Natural recursion during AST traversal handles this cleanly
  - Leverages existing AST structure
  - Simpler recursive implementation

### **Error Recovery:**
- **Option 1**: Symbol lookup errors during init are easy to diagnose
  - Clear context: "room X, object Y not found"
  - Can continue processing other rooms/objects
- **Option 2**: Symbol lookup errors during room gen are immediately catchable
  - Fail fast at point of generation
  - All symbols should exist by room generation time

### **Implementation Risk:**
- **Option 1**: More straightforward, harder to get wrong
  - Explicit phases with clear boundaries
  - Easy to verify intermediate state
- **Option 2**: More efficient but more moving parts
  - Instructions generated far from where they're used
  - Potential for subtle bugs

### **Future Maintenance:**
- **Option 1**: Easy to understand and modify placement logic
  - All placement code in one location
  - Clear data flow and dependencies
- **Option 2**: Placement logic embedded in room generation
  - Requires understanding room generation to modify placement
  - Split concerns across multiple functions

### **Performance Considerations:**
- **Option 1**:
  - Two passes: record mapping, then generate instructions
  - Extra memory for mapping storage
  - String lookups during both phases
- **Option 2**:
  - Single pass: generate instructions immediately
  - Minimal memory overhead
  - IR generation happens with full AST context

### **Debugging Experience:**
- **Option 1**:
  - Can inspect room_objects mapping
  - Clear separation between mapping and instruction generation
  - Easy to add logging at each phase
- **Option 2**:
  - Harder to inspect pending instructions
  - Placement logic mixed with room generation logging
  - Instructions generated far from where they're injected

## Example: Mailbox in West of House

### AST Structure:
```
room west_of_house {
    object mailbox {
        contains {
            object leaflet {
            }
        }
    }
}
```

### Option 1 Data Flow:
```
1. generate_room(west_of_house):
   - room_objects["west_of_house"] = [
       RoomObjectInfo {
         name: "mailbox",
         nested: [RoomObjectInfo { name: "leaflet", nested: [] }]
       }
     ]

2. generate_init():
   - room_ir_id = symbol_ids["west_of_house"]
   - mailbox_ir_id = symbol_ids["mailbox"]
   - leaflet_ir_id = symbol_ids["leaflet"]
   - InsertObj { object: mailbox_ir_id, destination: room_ir_id }
   - InsertObj { object: leaflet_ir_id, destination: mailbox_ir_id }
```

### Option 2 Data Flow:
```
1. generate_room(west_of_house):
   - room_ir_id = current room ID
   - mailbox_ir_id = symbol_ids["mailbox"]
   - leaflet_ir_id = symbol_ids["leaflet"]
   - pending_placement_instructions.push(InsertObj { mailbox, room })
   - pending_placement_instructions.push(InsertObj { leaflet, mailbox })

2. generate_init():
   - block.instructions.extend(pending_placement_instructions)
```

## Assessment Summary

### **Technical Elegance**: Option 2 Wins
- Follows compiler principle: "generate IR immediately when context is available"
- More efficient: single pass, minimal memory overhead
- Natural AST→IR conversion flow

### **Maintainability**: Option 1 Wins
- Easier to understand and debug
- Clear separation of concerns
- Centralized placement logic
- Explicit data dependencies

### **Risk Profile**: Option 1 Safer
- More straightforward implementation
- Harder to introduce subtle bugs
- Better error recovery and debugging

### **Project Context Considerations**:
- We're working carefully to avoid unexpected conditions
- Maintainability is prioritized over micro-optimizations
- Clear, debuggable code is preferred

## Recommendation Factors

### **Choose Option 1 If:**
- Maintainability is the top priority
- We want clear, debuggable code
- We prefer explicit data flow
- We want to minimize implementation risk

### **Choose Option 2 If:**
- Technical elegance is prioritized
- Performance optimization is important
- We want to follow strict compiler design principles
- We're confident in handling the complexity

## Implementation Timeline

### **Option 1 Timeline:**
1. Phase 1a: Add room_objects mapping field
2. Phase 1b: Implement object hierarchy recording
3. Phase 1c: Implement init block instruction injection
4. Phase 1d: Test and debug

### **Option 2 Timeline:**
1. Phase 1a: Add pending_placement_instructions field
2. Phase 1b: Implement instruction generation during room processing
3. Phase 1c: Implement init block injection
4. Phase 1d: Test and debug

**Estimated complexity**: Similar for both options
**Risk level**: Option 1 (Lower), Option 2 (Higher)