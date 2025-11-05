# New Array Implementation Plan - Based on Zork I Analysis

## 🚨 Critical Discovery: Static Arrays ARE Essential, Dynamic Arrays are Anti-Patterns

After analyzing Zork I disassembly, we found **492 array operations** including sophisticated patterns like:
```assembly
STOREW          L05,L03,(SP)+        ; array[variable_index] = value
INC             L02                  ; increment loop counter
JE              L02,L01 [FALSE] L0001; bounds checking
```

**REVISED ANALYSIS (Nov 5, 2025)**: After examining mini_zork.grue, we found:

### ✅ **Required Array Types:**
1. **Static string arrays**: `names: ["mailbox", "box"]` (compile-time)
2. **Static property arrays**: `exits: { north: room }` (compile-time)
3. **Array indexing**: `array[index]` for reading values
4. **Collection methods**: `obj.contents()` returns array-like data

### ❌ **Dynamic Arrays are Anti-Patterns:**
- **NO evidence** of `let visible_objects = []` in mini_zork.grue
- **NO evidence** of `array.add(item)` runtime growth
- **NO evidence** of runtime dynamic allocation anywhere
- Z-Machine has **no malloc()** - all memory must be compile-time allocated

**Conclusion**: Arrays are FUNDAMENTAL, but only STATIC arrays. Dynamic arrays `[]` were a design mistake and should be **prohibited** in proper Z-Machine games.

## Architecture: Clean Implementation with `codegen_arrays.rs`

### Core Design Principles

1. **Z-Machine Native**: Use `loadw`/`loadb` opcodes for reading as Infocom did
2. **Static Allocation Only**: All arrays pre-allocated at compile time with known sizes
3. **Table Format**: Follow Z-Machine standard: `[element1][element2]...` (no count needed for static)
4. **Modular**: Implement in separate `codegen_arrays.rs` for clean separation
5. **Read-Only**: Arrays are immutable after compilation (no runtime modification)

### Phase 1: Core Architecture (`codegen_arrays.rs`)

```rust
pub struct ArrayCodeGen {
    // Track all arrays discovered during IR generation
    allocated_arrays: HashMap<IrId, ArrayInfo>,
    // Memory allocation tracking
    next_array_address: u16,
}

pub struct ArrayInfo {
    ir_id: IrId,
    base_address: u16,           // Where array lives in memory
    max_elements: usize,         // Maximum capacity
    element_type: ElementType,   // Byte vs Word arrays
    is_dynamic: bool,           // Empty [] vs [1,2,3]
}

pub enum ElementType {
    Byte,  // Use loadb/storeb
    Word,  // Use loadw/storew
}

impl ArrayCodeGen {
    // Core operations matching Zork I patterns
    pub fn allocate_static_array(&mut self, elements: &[IrValue]) -> IrId;
    pub fn allocate_dynamic_array(&mut self, max_size: usize) -> IrId;
    pub fn generate_array_read(&mut self, array_id: IrId, index_temp: IrId, target: IrId);
    pub fn generate_array_write(&mut self, array_id: IrId, index_temp: IrId, value_temp: IrId);
    pub fn generate_array_length(&mut self, array_id: IrId, target: IrId);
    pub fn generate_array_add(&mut self, array_id: IrId, value_temp: IrId);
}
```

### Phase 2: IR Instruction Restoration (Static Arrays Only)

**Restore in `src/grue_compiler/ir.rs`:**
```rust
pub enum IrInstruction {
    // Static array creation - ONLY for compile-time known arrays
    CreateArray {
        target: IrId,
        elements: Vec<IrValue>,     // Always populated: [1,2,3] or ["a","b","c"]
        // No max_size - size is elements.len()
        // No empty [] - that was the anti-pattern
    },

    // Array read access - for property arrays and static data
    GetArrayElement { target: IrId, array: IrId, index: IrId },

    // NO dynamic operations:
    // - No SetArrayElement (arrays are read-only after compilation)
    // - No ArrayAdd (no runtime growth)
    // - No ArrayLength (compiler knows length at compile time)
    // - No ArrayEmpty (compiler knows if elements.is_empty())
}
```

### Phase 3: Memory Layout (Z-Machine Standard)

**Array Table Format:**
```
Address    Content               Purpose
---------  ------------------    ----------------------
0x0500:    0x0002               Current element count
0x0502:    0x0045               Element[0] = 69 (object ID)
0x0504:    0x002A               Element[1] = 42 (object ID)
0x0506:    0x0000               Element[2] = 0 (unused)
0x0508:    0x0000               Element[3] = 0 (unused)
...        ...                  ...up to max_size
```

**Memory Allocation Strategy:**
```rust
// During header generation (after globals, before objects)
fn allocate_array_memory(&mut self) -> Result<(), CompilerError> {
    let mut current_addr = self.globals_end_address;

    for (id, info) in &mut self.arrays {
        info.base_address = current_addr;

        // Reserve space: count word + max_elements
        let array_size = 2 + (info.max_elements * 2); // 2 bytes per word

        // Initialize array memory to zeros
        for i in 0..array_size {
            self.memory[current_addr + i] = 0;
        }

        current_addr += array_size;
    }

    self.object_table_start = current_addr;
}
```

### Phase 4: Opcode Generation (Matching Zork I)

**Array Access (Read):**
```rust
fn generate_array_read(&mut self, array_id: IrId, index_temp: IrId, target: IrId) {
    let array_info = self.arrays.get(&array_id)?;
    let array_addr = array_info.base_address;

    // Add 1 to index (skip count word at array[0])
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Add),
        &[index_operand, Operand::SmallConstant(1)],
        Some(temp_var),
        None
    );

    // loadw array_addr (temp_var) -> target
    // This generates the same pattern we saw in Zork I
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Loadw),
        &[Operand::LargeConstant(array_addr), Operand::Variable(temp_var)],
        Some(target_var),
        None
    );
}
```

**Array Assignment (Write):**
```rust
fn generate_array_write(&mut self, array_id: IrId, index_temp: IrId, value_temp: IrId) {
    let array_info = self.arrays.get(&array_id)?;
    let array_addr = array_info.base_address;

    // Add 1 to index (skip count word)
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Add),
        &[index_operand, Operand::SmallConstant(1)],
        Some(temp_var),
        None
    );

    // storew array_addr (temp_var) value
    // This matches Zork I: STOREW L05,L03,(SP)+
    self.emit_instruction_typed(
        Opcode::OpVar(OpVar::Storew),
        &[
            Operand::LargeConstant(array_addr),
            Operand::Variable(temp_var),
            value_operand
        ],
        None,
        None
    );
}
```

### Phase 5: Integration Points

**ZMachineCodeGen modifications needed:**
```rust
impl ZMachineCodeGen {
    // Array subsystem
    pub array_codegen: ArrayCodeGen,

    // Expose methods to ArrayCodeGen
    pub fn emit_for_arrays(&mut self, opcode: Opcode, operands: &[Operand]) -> InstructionLayout;
    pub fn allocate_temp_for_arrays(&mut self) -> IrId;
    pub fn resolve_operand_for_arrays(&self, id: IrId) -> Result<Operand, CompilerError>;
}
```

### Phase 6: Parser & Semantic Support

**Array syntax (existing):**
```grue
let arr = [1, 2, 3];        // Static array
let dynamic = [];           // Dynamic array (empty)
let value = arr[index];     // Array access
arr[index] = value;         // Array assignment
arr.add(item);             // Push operation
let len = arr.length;      // Get length
```

### Phase 7: Testing Strategy

**Test against Zork I patterns:**
1. **Static arrays** - Verify `[1,2,3]` generates proper `loadw` access
2. **Dynamic arrays** - Test empty `[]` with `.add()` operations
3. **Variable indexing** - Ensure runtime indices work like `LOADW L05,(SP)+`
4. **Bounds checking** - Test index validation
5. **Integration** - Run mini_zork with proper array support

**Test files:**
- `examples/test_arrays_basic.grue` - Basic operations
- `examples/test_arrays_zork_patterns.grue` - Patterns from Zork I analysis
- `tests/array_zork_compatibility.rs` - Ensure opcodes match Infocom patterns

### Phase 8: Implementation Order

1. ✅ **Create `codegen_arrays.rs`** with core structures
2. ✅ **Restore IR instructions** (CreateArray, GetArrayElement, etc.)
3. ✅ **Implement memory allocation** in header generation
4. ✅ **Implement static arrays** `[1,2,3]`
5. ✅ **Implement dynamic arrays** `[]` with `.add()`
6. ✅ **Implement array access** `array[index]`
7. ✅ **Add integration hooks** to ZMachineCodeGen
8. ✅ **Test basic operations**
9. ✅ **Test mini_zork compatibility**
10. ✅ **Validate against Zork I disassembly patterns**

### Key Advantages

1. **Spec Compliant** - Uses Z-Machine native opcodes like Infocom
2. **Performance** - Direct memory access, no overhead
3. **Compatibility** - Generates same patterns as commercial games
4. **Maintainable** - Clean separation in `codegen_arrays.rs`
5. **Testable** - Can verify against known working games

### Constraints & Limitations

- **Static allocation only** - All arrays pre-allocated (Z-Machine limitation)
- **Fixed maximum size** - Dynamic arrays need max capacity specified
- **Memory overhead** - Each array reserves full capacity
- **Word arrays only** - Focus on 16-bit elements (can add byte arrays later)

### Estimated Implementation Time

- **Phase 1-3** (Architecture): 4-6 hours
- **Phase 4-5** (Opcode generation): 6-8 hours
- **Phase 6-7** (Integration & testing): 4-6 hours
- **Phase 8** (Validation): 2-4 hours
- **Total**: 2-3 days

This approach will restore proper array functionality that matches how Infocom implemented arrays in their games, ensuring our compiler generates compatible and efficient Z-Machine code.