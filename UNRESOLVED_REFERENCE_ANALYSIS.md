# UnresolvedReference Usage Pattern Analysis

## PURPOSE
Document all UnresolvedReference usage patterns across the codebase to prepare for unified patching system implementation.

## STRUCT DEFINITION
Location: `src/grue_compiler/codegen.rs:154-161`

```rust
#[derive(Debug, Clone)]
pub struct UnresolvedReference {
    pub reference_type: LegacyReferenceType,
    pub location: usize,               // Byte offset where patch is needed
    pub target_id: IrId,              // IR ID being referenced
    pub is_packed_address: bool,      // Whether address needs to be packed
    pub offset_size: u8,              // Size of offset field (1 or 2 bytes)
    pub location_space: MemorySpace,  // Which memory space location belongs to
}

#[derive(Debug, Clone, PartialEq)]
pub enum LegacyReferenceType {
    Jump,                                    // Unconditional jump to label
    Branch,                                  // Conditional branch to label
    Label(IrId),                             // Reference to label
    FunctionCall,                            // Call to function address
    StringRef,                               // Reference to string address
    StringPackedAddress { string_id: IrId }, // Packed string address for properties
    DictionaryRef { word: String },          // Dictionary entry address
    GlobalsBase,                             // Global variables base address
}
```

## STATE MANAGEMENT
Location: `src/grue_compiler/codegen.rs:180`

```rust
pub struct ReferenceContext {
    pub ir_id_to_address: IndexMap<IrId, usize>,
    pub unresolved_refs: Vec<UnresolvedReference>,  // ← Storage
}
```

## CREATION PATTERNS

### Pattern 1: Manual Creation with placeholder_word()
**Most Common Pattern**: Direct construction followed by push to unresolved_refs

**Example Pattern**:
```rust
// Write placeholder bytes to memory space
let placeholder_bytes = placeholder_word().to_be_bytes();
self.some_space.extend_from_slice(&placeholder_bytes);

// Create UnresolvedReference for later patching
self.reference_context.unresolved_refs.push(UnresolvedReference {
    reference_type: LegacyReferenceType::SomeType,
    location: current_location,          // Where placeholder was written
    target_id: some_ir_id,              // What needs to be resolved
    is_packed_address: false,           // Usually false
    offset_size: 2,                     // Usually 2 bytes
    location_space: MemorySpace::SomeSpace,
});
```

### Pattern 2: Creation Through Helper Functions

#### add_unresolved_reference()
**Location**: `codegen.rs:11708`
```rust
pub fn add_unresolved_reference(
    &mut self,
    reference_type: LegacyReferenceType,
    target_id: IrId,
    is_packed_address: bool,
) -> Result<(), CompilerError>
```

#### add_unresolved_reference_at_location()
**Location**: `codegen.rs:11663`
```rust
pub fn add_unresolved_reference_at_location(
    &mut self,
    reference_type: LegacyReferenceType,
    location: usize,
    target_id: IrId,
    is_packed_address: bool,
    location_space: MemorySpace,
) -> Result<(), CompilerError>
```

## RESOLUTION PATTERNS

### Pattern 1: Central Resolution Loop
**Location**: `src/grue_compiler/codegen.rs:2032-2046`

**Process**:
1. Clone all unresolved_refs to avoid borrow checker issues
2. For each reference:
   - Translate location from memory space to final layout
   - Look up target_id in ir_id_to_address mapping
   - Call resolve_unresolved_reference() to patch memory

**Critical Memory Translation**:
```rust
let adjusted_location = self.translate_space_address_to_final(
    reference.location_space,
    reference.location
)?;
```

### Pattern 2: Single Reference Resolution
**Location**: `src/grue_compiler/codegen.rs:10678-10800+`

**Key Properties**:
- **Direct memory patching**: Writes bytes directly to final_data
- **Multiple reference types**: Handles Jump, Branch, FunctionCall, StringRef, etc.
- **Address calculation**: Different logic for each reference type
- **No bounds checking coordination**: Operates independently of DeferredBranchPatch

## USAGE LOCATIONS ACROSS CODEBASE

### Primary Creation Sites (35+ locations in codegen.rs)

#### Function Call References
**Lines**: 3485, 3624, 3795, 3884, 3979, 11398, 11626
- Function address resolution for call instructions
- Both direct calls and builtin function calls

#### Object Property References
**Lines**: 5381, 5907, 5941, 5991, 6029, 6077
- String property packed addresses
- Dictionary references for exit_directions
- Function references for object methods

#### Control Flow References
**Lines**: 6473, 6576, 6622, 6650, 6688, 6741, 6998, 7080
- Jump instructions for loops and conditionals
- GlobalsBase references for variable access
- Branch target resolution

#### Grammar System References
**Lines**: 7235, 7264, 7372, 7413, 7447, 7486, 7514, 7610
- Dictionary word lookups
- Function calls for grammar handlers
- String references for output

#### Builtin System References
**Lines**: 7934, 7986, 8465, 8745, 9551, 10277, 13400
- Complex builtin function implementations
- String and function address resolution

### Reference Type Distribution

#### Most Common Types (by usage frequency):
1. **FunctionCall**: ~15 occurrences - Function address resolution
2. **DictionaryRef**: ~10 occurrences - Word lookup in dictionary
3. **StringRef/StringPackedAddress**: ~8 occurrences - String content addresses
4. **GlobalsBase**: ~5 occurrences - Global variable base address
5. **Jump/Branch**: ~3 occurrences - Control flow targets

## MEMORY SPACE USAGE

### CodeSpace References
- Function calls in generated code
- Jump/branch instructions
- Control flow within functions

### ObjectSpace References
- Property values (strings, functions, numbers)
- Object method references
- Property table data

### StringSpace References
- Text content for properties
- Game messages and descriptions
- Dictionary word storage

## COLLISION POTENTIAL WITH DEFERREDDBRANCHPATCH

### Evidence of Known Conflict
**Source**: `unresolved_reference_tests.rs:4`
```rust
//! to prevent "inscrutable class of bugs" due to conflicts with DeferredBranchPatch.
```

### Root Cause Analysis
1. **Memory Overlap**: Both systems patch bytes in final_data/code_space
2. **Independent Operation**: No coordination between systems
3. **Shared Address Ranges**:
   - DeferredBranchPatch patches branch_offset_location (1-2 bytes)
   - UnresolvedReference patches reference.location (1-2 bytes)
   - When instructions are adjacent, these ranges can overlap

### Example Collision Scenario
```
GetChild instruction:
  Address 0x17a6: Instruction start
  Address 0x17a7: Branch offset (DeferredBranchPatch target)
  Address 0x17a8: Branch offset continues

Next instruction (GlobalsBase reference):
  Address 0x17a7: Global reference operand (UnresolvedReference target)
  Address 0x17a8: Global reference continues
```

**Result**: Both systems write to 0x17a7-0x17a8, creating corrupted bytecode.

## RESOLUTION ORDER ISSUES

### Current Resolution Sequence
1. **DeferredBranchPatch**: Applied during resolve_deferred_branches()
2. **UnresolvedReference**: Applied during main resolution loop
3. **No synchronization**: Order depends on when each resolution function is called

### Memory Safety Problems
- **Last writer wins**: Overlapping patches overwrite each other
- **No conflict detection**: Neither system checks for overlaps
- **Silent corruption**: Invalid bytecode generated without error

## MEMORY PATCHING MECHANISM

### Direct Memory Writing
**Pattern**: `self.final_data[location..location+size] = bytes`

**Examples**:
```rust
// 2-byte address write
self.final_data[location] = (address >> 8) as u8;
self.final_data[location + 1] = (address & 0xFF) as u8;

// 1-byte offset write
self.final_data[location] = offset as u8;
```

### Key Properties
- **Direct access**: No abstraction or coordination layer
- **Size flexibility**: Supports 1-byte and 2-byte patches
- **Address translation**: Translates from memory space to final layout
- **Type-specific logic**: Different patching for each reference type

## CONCLUSION

UnresolvedReference is:

1. **Manually created** throughout code generation with placeholder_word() pattern
2. **Stored** in ReferenceContext.unresolved_refs Vec
3. **Resolved** by writing bytes directly to final_data
4. **Collision-prone** with DeferredBranchPatch due to independent operation
5. **Memory-unsafe** without coordination with other patching systems
6. **Complex**: Handles 8 different reference types with type-specific logic

The core architectural problem is that UnresolvedReference operates directly on memory locations without any awareness of or coordination with DeferredBranchPatch that may target overlapping byte ranges. Both systems assume exclusive access to their target memory locations.