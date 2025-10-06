# Phase 1 Implementation Plan: Core Object-Property System

## Current Status: Architecture Complete ✅
- **Function compilation framework**: Working (pre-registration, Call instructions, routine headers)
- **Single-path byte emission**: Fixed (all writes through emit_byte())
- **Call instruction generation**: Working (0xe0 opcodes with placeholders)
- **Root cause identified**: Missing IR instruction implementations in function bodies

## Phase 1: Object Loading System (Priority: CRITICAL)

**Target**: Enable basic object references like `player`, `west_of_house` to load into Z-Machine global variables

**Implementation Location**: `src/grue_compiler/codegen.rs` around line 2240 (LoadImmediate handler)

**Current Issue**:
```rust
LoadImmediate { target: 62, value: Integer(1) }  // Maps player → object #1
// Currently: ⚠️ UNEXPECTED: IR instruction generated no bytecode
// Needed: Z-Machine global variable assignment
```

**Implementation Requirements**:

1. **Object Name → Object Number Mapping**:
   ```rust
   // Global object registry (compile-time)
   let mut object_numbers = HashMap::new();
   object_numbers.insert("player", 1);
   object_numbers.insert("west_of_house", 2);
   object_numbers.insert("north_of_house", 3);
   // ... etc for all game objects
   ```

2. **LoadImmediate for Objects** (currently line ~2240):
   ```rust
   IrInstruction::LoadImmediate { target, value } => {
       match value {
           IrValue::Integer(obj_num) if self.is_object_reference(*obj_num) => {
               // Store object number in Z-Machine global variable
               // Use storew instruction to global variable table
               let global_var = self.allocate_global_variable(target);
               self.emit_store_global(global_var, *obj_num as u16)?;
               log::debug!("✅ OBJECT_LOAD: IR ID {} -> object #{} in global {}", target, obj_num, global_var);
           }
           // ... existing string/integer handling
       }
   }
   ```

3. **Z-Machine Global Variable Management**:
   - Global variables 16-255 available for object storage
   - Use `storew` instruction (VAR:225, hex 0xE1) to write to globals
   - Track IR ID → global variable mappings for later resolution

**Success Criteria**:
- `LoadImmediate { target: 62, value: Integer(1) }` generates Z-Machine `storew` instruction
- No more "UNEXPECTED: IR instruction generated no bytecode" for object loading
- Object references stored in Z-Machine global variable table

## Phase 2: Property Access System (Priority: CRITICAL)

**Target**: Enable property access like `player.location.desc`

**Implementation Location**: `src/grue_compiler/codegen.rs` line 3630 (GetProperty)

**Current Issue**:
```rust
GetProperty { target: 63, object: 62, property: "location" }
// Currently: ⚠️ UNIMPLEMENTED: GetProperty - skipping
// Needed: Z-Machine get_prop instruction
```

**Implementation Requirements**:

1. **Property Name → Property Number Mapping**:
   ```rust
   // Standard Z-Machine properties
   let mut property_numbers = HashMap::new();
   property_numbers.insert("location", 1);
   property_numbers.insert("desc", 2);
   property_numbers.insert("name", 3);
   // ... etc
   ```

2. **GetProperty Implementation** (line ~3630):
   ```rust
   IrInstruction::GetProperty { target, object, property } => {
       let obj_operand = self.resolve_ir_id_to_operand(*object)?;
       let prop_num = self.get_property_number(property)?;
       
       // Generate Z-Machine get_prop instruction (2OP:17, hex 0x11)
       let target_var = self.get_or_allocate_variable(*target)?;
       self.emit_instruction(
           0x11, // get_prop opcode
           vec![obj_operand, Operand::SmallConstant(prop_num)],
           Some(target_var),
           None,
       )?;
       log::debug!("✅ GET_PROPERTY: Object {} property '{}' -> target {}", object, property, target);
   }
   ```

3. **Z-Machine get_prop Instruction**:
   - Format: `get_prop object_number property_number -> result`
   - Opcode: 0x11 (2OP:17)
   - Returns property value or default if property doesn't exist

## Phase 3: Property Assignment System (Priority: CRITICAL)

**Target**: Enable assignments like `player.location = west_of_house`

**Implementation Location**: `src/grue_compiler/codegen.rs` around line 3641 (SetProperty)

**Implementation Requirements**:

1. **SetProperty Implementation**:
   ```rust
   IrInstruction::SetProperty { object, property, value } => {
       let obj_operand = self.resolve_ir_id_to_operand(*object)?;
       let value_operand = self.resolve_ir_id_to_operand(*value)?;
       let prop_num = self.get_property_number(property)?;
       
       // Generate Z-Machine put_prop instruction (VAR:227, hex 0xE3)
       self.emit_instruction(
           0xE3, // put_prop opcode  
           vec![obj_operand, Operand::SmallConstant(prop_num), value_operand],
           None, // put_prop doesn't return a value
           None,
       )?;
       log::debug!("✅ SET_PROPERTY: Object {} property '{}' = {}", object, property, value);
   }
   ```

## Estimated Implementation Time: 4-6 hours
1. **Phase 1** (Object Loading): 2 hours - Core global variable system
2. **Phase 2** (Property Access): 2 hours - get_prop instruction implementation  
3. **Phase 3** (Property Assignment): 2 hours - put_prop instruction implementation

## Success Metrics:
- ✅ `player.location = west_of_house` compiles without warnings
- ✅ `player.location.desc` generates proper get_prop bytecode
- ✅ mini_zork.grue compiles with functional init block
- ✅ Basic room navigation and object interaction works

**Next Session Focus**: Implement Phase 1 (LoadImmediate for object references) to eliminate the "UNEXPECTED: IR instruction generated no bytecode" warnings and enable basic object manipulation.

## Implementation Strategy

**Step 1**: Add object number mapping system and global variable allocation
**Step 2**: Implement LoadImmediate for Integer values that represent objects
**Step 3**: Test with mini_zork to verify object loading works
**Step 4**: Move to GetProperty implementation
**Step 5**: Implement SetProperty for assignments
**Step 6**: Full mini_zork compilation and execution testing

## Key Files to Modify:
- `src/grue_compiler/codegen.rs` - Main implementation
- `src/grue_compiler/ir.rs` - May need object type definitions
- Testing with `examples/mini_zork.grue` throughout

## Debugging Strategy:
- Add comprehensive logging for each phase
- Test individual IR instructions in isolation
- Verify Z-Machine bytecode generation at each step
- Use mini_zork as integration test case