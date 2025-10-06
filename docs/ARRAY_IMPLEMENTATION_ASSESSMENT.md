# Array Implementation Assessment - REVISED

## CRITICAL Z-MACHINE SPECIFICATION FINDINGS ⚠️

### Dynamic Memory Allocation is Impossible in Z-Machine

From the **Z-Machine Standards Document** (Preface):
> "The design's cardinal principle is that any game is 100% portable to different computers: that is, any legal program exactly determines its behaviour. This portability is largely made possible by a willingness to constrain maximum as well as minimum levels of performance (for instance, **dynamic memory allocation is impossible**)."

From **Section 15.2**:
> "The Z-machine has the same concept of 'table' (as an internal data structure) as Inform. Specifically, **a table is an array of words (in dynamic or static memory) of which the initial entry is the number of subsequent words in the table**. For example, a table with three entries occupies 8 bytes, arranged as the words 3, x, y, z."

### What This Means:
1. **NO runtime malloc/free** - Z-Machine has no memory allocator by design
2. **All arrays must be compile-time allocated** - Fixed addresses determined during compilation
3. **Table format**: `[count_word][element1][element2]...` - First word stores current element count
4. **Access via loadw/storew**:
   - `loadw array index -> result` reads `array[index]`
   - `storew array index value` writes `array[index] = value`
   - Address arithmetic: `array + 2*index` (words are 2 bytes)

### Grue Language Analysis:

Examining `mini_zork.grue` for all array usage:

```grue
// 1. String arrays in object properties - COMPILE-TIME
names: ["mailbox", "box"]

// 2. Array property access - NO CREATION
let exit = player.location.exits[direction];

// 3. Empty array literal - SINGLE RUNTIME ARRAY
let visible_objects = [];
visible_objects.add(obj);
for item in visible_objects { ... }
```

**Finding**: Only ONE dynamic array exists (`visible_objects` in `list_objects()` function). All other arrays are:
- Compile-time constants (property name arrays)
- Property accesses (no allocation)
- Iteration (by reference, no copying)

**Conclusion**: We can allocate ALL arrays statically at compile time. No runtime allocation needed!

---

## CORRECT IMPLEMENTATION: Static Pre-Allocation

### Approach:

**Compile-Time Analysis**:
1. During IR generation, collect all CreateArray instructions
2. Assign each array a unique ID and fixed address
3. Reserve space in dynamic memory section during header generation

**Runtime Behavior**:
1. `CreateArray` → Returns pre-allocated address (not 0/1000 placeholder!)
2. `ArrayAdd` → `storew` to increment count and add element
3. `GetArrayElement` → `loadw` from array address + offset
4. `ArrayLength` → `loadw` from array address (first word)

### Z-Machine Table Format:
```
Address  Content
-------  -------
0x0500:  0x0000    # Initial count = 0
0x0502:  0x0000    # Element 0 (initially zero)
0x0504:  0x0000    # Element 1
0x0506:  0x0000    # Element 2
...
0x0500 + 2 + 2*N  # Last element
```

### Implementation Plan:

#### Phase 1: Collect Arrays at IR Generation
```rust
// In IrGenerator
struct IrGenerator {
    arrays: IndexMap<IrId, ArrayInfo>,  // Track all arrays
}

struct ArrayInfo {
    ir_id: IrId,
    max_size: usize,  // From CreateArray size parameter
    address: Option<u16>,  // Filled during codegen
}

// When generating CreateArray IR:
self.arrays.insert(array_id, ArrayInfo {
    ir_id: array_id,
    max_size: size,
    address: None,
});
```

#### Phase 2: Allocate in Dynamic Memory (codegen_headers.rs)
```rust
// After globals, before objects:
let mut array_address = globals_end;
for (id, info) in &mut ir_generator.arrays {
    info.address = Some(array_address);
    // Reserve: count word + max_size elements (2 bytes each)
    let array_size = 2 + (info.max_size * 2);

    // Initialize to zeros
    for i in 0..array_size {
        self.memory[array_address + i] = 0;
    }

    array_address += array_size;
}
```

#### Phase 3: Implement CreateArray (codegen_instructions.rs)
```rust
IrInstruction::CreateArray { target, size } => {
    // Look up pre-allocated address
    let array_info = self.array_allocations.get(target)?;
    let array_addr = array_info.address;

    // Push array address to stack
    self.emit_instruction_typed(
        Opcode::OpVar(OpVar::Push),
        &[Operand::LargeConstant(array_addr)],
        None,
        None,
    )?;

    self.use_stack_for_result(*target);
}
```

#### Phase 4: Implement ArrayAdd
```rust
IrInstruction::ArrayAdd { array, value } => {
    let array_op = self.resolve_ir_id_to_operand(*array)?;
    let value_op = self.resolve_ir_id_to_operand(*value)?;

    // Load current count from array[0]
    // loadw array 0 -> -(SP)
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Loadw),
        &[array_op.clone(), Operand::SmallConstant(0)],
        Some(0),  // Store to stack
        None,
    )?;

    // Store value at array[count]
    // storew array (SP)+ value
    self.emit_instruction_typed(
        Opcode::OpVar(OpVar::Storew),
        &[array_op.clone(), Operand::Variable(0), value_op],
        None,
        None,
    )?;

    // Increment count
    // loadw array 0 -> -(SP)
    // add (SP)+ 1 -> -(SP)
    // storew array 0 (SP)+
    ...
}
```

#### Phase 5: Implement GetArrayElement
```rust
IrInstruction::GetArrayElement { target, array, index } => {
    let array_op = self.resolve_ir_id_to_operand(*array)?;
    let index_op = self.resolve_ir_id_to_operand(*index)?;

    // Add 1 to index (skip count word)
    // add index 1 -> -(SP)
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Add),
        &[index_op, Operand::SmallConstant(1)],
        Some(0),
        None,
    )?;

    // loadw array (SP)+ -> -(SP)
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Loadw),
        &[array_op, Operand::Variable(0)],
        Some(0),
        None,
    )?;

    self.use_stack_for_result(*target);
}
```

#### Phase 6: Implement ArrayLength
```rust
IrInstruction::ArrayLength { target, array } => {
    let array_op = self.resolve_ir_id_to_operand(*array)?;

    // loadw array 0 -> -(SP)
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Loadw),
        &[array_op, Operand::SmallConstant(0)],
        Some(0),
        None,
    )?;

    self.use_stack_for_result(*target);
}
```

### Memory Layout Example:

```
Dynamic Memory:
0x0040: [Globals - 480 bytes]
0x0220: [Array Table Start]
0x0220:   [visible_objects array]
0x0220:     0x0000  (count = 0)
0x0222:     0x0000  (element[0])
0x0224:     0x0000  (element[1])
...
0x0234:     0x0000  (element[9] - max 10 elements)
0x0236: [Object Table Start]
...
```

### Constraints:
- Maximum 100 arrays per game
- Maximum 256 elements per array
- Total array space: ~5KB (reasonable for V3)
- Arrays are function-scoped (no explicit free needed)

### Advantages:
1. **Spec-compliant** - Uses only static allocation as Z-Machine requires
2. **Simple** - No runtime allocator needed
3. **Fast** - Direct memory access, no overhead
4. **Deterministic** - All addresses known at compile time

### Testing Strategy:
1. Test CreateArray returns correct address
2. Test ArrayAdd increments count and stores elements
3. Test GetArrayElement reads correct values
4. Test ArrayLength returns count
5. Test mini_zork list_objects function

### Estimated Effort:
- Phase 1-2 (Allocation): 2-3 hours
- Phase 3-6 (Operations): 3-4 hours
- Testing: 2 hours
- **Total**: 1 day

---

## Original Assessment (OBSOLETE - Left for Reference)

The original assessment proposed runtime dynamic allocation (Options A-D below). This is **architecturally impossible** in Z-Machine. The correct approach is static pre-allocation only.

[Original content follows for historical reference...]

## Current State

### Implemented IR Instructions:
1. **CreateArray** - Returns 0 (null) placeholder, no actual memory allocated
2. **ArrayAdd** - No-op, doesn't modify anything
3. **ArrayLength** - Returns 0 placeholder
4. **ArrayEmpty** - Not implemented
5. **GetArrayElement** - Uses placeholder value (1000), causes invalid object errors
6. **SetArrayElement** - Not implemented

### Problem:
When executing `let visible_objects = []; visible_objects.add(obj); for item in visible_objects { ... }`:
- CreateArray returns placeholder value (0 or 1000)
- ArrayAdd does nothing
- For-loop iteration calls GetArrayElement with placeholder, reads garbage memory
- Result: "Invalid object number: 1000" error
