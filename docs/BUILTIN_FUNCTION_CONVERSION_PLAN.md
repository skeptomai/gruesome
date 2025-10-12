# Exit Builtin Function Conversion Plan

## Problem Statement

Exit system builtins are currently **inlined** at each call site, causing:
1. Stack/variable confusion (dual storage: stack AND variable)
2. Code bloat (full code duplicated at every call)
3. Stack underflow at PC 0x114f
4. "Invalid object number 7936" errors

## Solution: Convert Inlined Builtins to Real Functions

Convert 5 builtins from inline code generation to proper Z-Machine functions:
1. `value_is_none(value) -> bool`
2. `exit_is_blocked(exit_value) -> bool`
3. `exit_get_data(exit_value) -> u16`
4. `exit_get_message(exit_value) -> u16`
5. `get_exit(room, direction) -> u16`

## Implementation Plan

### Phase 1: Create Builtin Function Infrastructure

**Goal**: Add infrastructure to generate builtin functions at compiler initialization

**Files**: `src/grue_compiler/codegen.rs`, `src/grue_compiler/codegen_builtins.rs`

**Steps**:
1. Add `builtin_functions: IndexMap<String, IrId>` to `CodeGenerator` struct
2. Add `generate_builtin_functions()` method called during initialization
3. Store builtin function addresses for later call_vs generation

**Key Decision**: Where to generate builtins?
- **Option A**: Generate during `CodeGenerator::new()` before user code
  - Pro: Clean separation, builtins always available
  - Con: Modifies initialization sequence
- **Option B**: Generate at start of `generate()` method
  - Pro: Minimal change to initialization
  - Con: Mixed with user code generation

**Recommendation**: Option B - generate at start of `generate()` method

### Phase 2: Implement Individual Builtin Functions

Convert each builtin from inline code to function. For each:

**Template Pattern**:
```rust
fn create_builtin_<name>(&mut self) -> Result<IrId, CompilerError> {
    // 1. Allocate function address
    let func_addr = self.code_address;
    let func_id = self.next_ir_id();

    // 2. Align function (V3 = even address)
    if self.code_address % 2 != 0 {
        self.emit_byte(0xB4)?; // padding
    }

    // 3. Emit function header (local variable count)
    self.emit_byte(num_locals)?;
    // V3: emit default values for each local (word)
    for _ in 0..num_locals {
        self.emit_word(0)?;
    }

    // 4. Generate function body
    //    - Use local variables (1-15) not globals
    //    - Arguments are in locals 1..N (already set by caller)
    //    - Compute result in local variable
    //    - Return via: ret <result_local>

    // 5. Record function address
    self.builtin_functions.insert(name.to_string(), func_id);
    self.function_addresses.insert(func_id, func_addr);

    Ok(func_id)
}
```

#### 2.1: `value_is_none(value) -> bool`

**Function Signature**: 1 argument, returns 0 or 1

**Local Variables**:
- Local 1: value (argument)
- Local 2: result (temp)

**Logic**:
```
1. je local_1, 0 ?branch_to_true
2. store local_2, 0
3. jump end_label
4. true_label:
5. store local_2, 1
6. end_label:
7. ret local_2
```

**Size Estimate**: ~15 bytes

#### 2.2: `exit_is_blocked(exit_value) -> bool`

**Function Signature**: 1 argument, returns 0 or 1

**Local Variables**:
- Local 1: exit_value (argument)
- Local 2: result (temp)

**Logic**:
```
1. jl local_1, 0x4000 ?branch_on_false_to_true
2. store local_2, 0
3. jump end_label
4. true_label:
5. store local_2, 1
6. end_label:
7. ret local_2
```

**Size Estimate**: ~18 bytes

#### 2.3: `exit_get_data(exit_value) -> u16`

**Function Signature**: 1 argument, returns u16

**Local Variables**:
- Local 1: exit_value (argument)
- Local 2: result (temp)

**Logic**:
```
1. and local_1, 0x3FFF -> local_2
2. ret local_2
```

**Size Estimate**: ~8 bytes

#### 2.4: `exit_get_message(exit_value) -> u16`

**Function Signature**: 1 argument, returns u16

**Local Variables**:
- Local 1: exit_value (argument)
- Local 2: result (temp)

**Logic**:
```
1. and local_1, 0x3FFF -> local_2
2. ret local_2
```

**Size Estimate**: ~8 bytes

**Note**: Identical to exit_get_data - could be same function, but keep separate for clarity

#### 2.5: `get_exit(room, direction) -> u16`

**Function Signature**: 2 arguments, returns u16

**Local Variables**:
- Local 1: room (argument)
- Local 2: direction (argument)
- Local 3: directions_addr (temp)
- Local 4: types_addr (temp)
- Local 5: data_addr (temp)
- Local 6: index (loop counter)
- Local 7: num_exits (temp)
- Local 8: type_byte (temp)
- Local 9: result (temp)

**Logic**: (This is complex - 100+ bytes currently)
```
1. get_property_addr room, prop_exit_directions -> local_3
2. je local_3, 0 ?branch_to_not_found
3. get_property_addr room, prop_exit_types -> local_4
4. ... (loop through exits, compare direction, build result)
5. not_found:
6. store local_9, 0
7. ret local_9
```

**Size Estimate**: ~100 bytes (same as current inline, but generated once)

### Phase 3: Update Call Sites

**Goal**: Replace inline builtin generation with `call_vs` to builtin function

**Files**: `src/grue_compiler/codegen.rs` (handler dispatch)

**Current Pattern**:
```rust
IrHandler::BuiltinCall(name, args, target) => {
    match name.as_str() {
        "value_is_none" => self.generate_value_is_none_builtin(args, *target)?,
        // ... inline generation
    }
}
```

**New Pattern**:
```rust
IrHandler::BuiltinCall(name, args, target) => {
    // Look up builtin function address
    let func_id = self.builtin_functions.get(name)
        .ok_or_else(|| CompilerError::CodeGenError(format!("Unknown builtin: {}", name)))?;
    let func_addr = self.function_addresses.get(func_id)
        .ok_or_else(|| CompilerError::CodeGenError(format!("Builtin not generated: {}", name)))?;

    // Generate call_vs
    // 1. Push arguments to stack (in order)
    for &arg_id in args {
        let operand = self.resolve_ir_id_to_operand(arg_id)?;
        // Push to stack via: or <operand>, 0 -> stack
        self.emit_instruction_typed(
            Opcode::Op2(Op2::Or),
            &[operand, Operand::SmallConstant(0)],
            Some(0), // Store to stack
            None,
        )?;
    }

    // 2. Emit call_vs
    let packed_addr = func_addr / 2; // V3 packed address
    self.emit_instruction_typed(
        Opcode::Var(VarOp::CallVs),
        &[Operand::LargeConstant(packed_addr as u16)],
        target.map(|_| 0), // Store return value to stack if target exists
        None,
    )?;

    // 3. If target exists, pop from stack to target variable
    if let Some(target_ir_id) = target {
        let result_var = self.allocate_global_for_ir_id(target_ir_id);
        self.ir_id_to_stack_var.insert(target_ir_id, result_var);
        // Pop from stack and store: store <stack>, result_var
        self.emit_instruction_typed(
            Opcode::Op1(Op1::Store),
            &[Operand::Variable(0)], // Read from stack
            Some(result_var),
            None,
        )?;
    }
}
```

**IMPORTANT**: Z-Machine call_vs calling convention:
- Arguments pushed to stack in order (arg1, arg2, ...)
- Caller emits call_vs with function address as first operand
- Callee reads arguments from its local variables (locals 1, 2, 3, ...)
- Callee returns via `ret` instruction with return value
- Return value goes to caller's store destination (or stack if store_var=0)

### Phase 4: Remove Old Inline Generation

**Goal**: Delete the old inline generation methods

**Files**: `src/grue_compiler/codegen_builtins.rs`

**Actions**:
1. Delete `generate_value_is_none_builtin()`
2. Delete `generate_exit_is_blocked_builtin()`
3. Delete `generate_exit_get_data_builtin()`
4. Delete `generate_exit_get_message_builtin()`
5. Delete `generate_get_exit_builtin()`

**Keep**: The new `create_builtin_<name>()` functions

### Phase 5: Testing

**Goal**: Verify all builtins work correctly as functions

**Tests**:
1. Run mini_zork test: `east` command
   - Should print blocked message, not crash
2. Run full test suite: `cargo test`
   - All 183 tests should pass
3. Check code size: compiled mini_zork.z3 should be SMALLER
   - Currently: each get_exit call = ~100 bytes inline
   - After: one get_exit function + multiple 8-byte call_vs

## Benefits

1. **Clarity**: Functions have clear calling conventions
2. **Size**: Each builtin generated once, not at every call
3. **Correctness**: Stack/variable confusion eliminated
4. **Maintainability**: Easier to debug and modify

## Risks

1. **Call overhead**: Each call_vs has overhead (push args, call, ret)
   - Mitigation: Overhead is small (< 10 bytes), savings from avoiding duplication much larger
2. **Stack depth**: Each function call increases stack depth
   - Mitigation: Builtins are shallow (no recursive calls)
3. **Calling convention bugs**: Must get call_vs semantics exactly right
   - Mitigation: Follow Z-Machine spec precisely, test thoroughly

## Estimated Code Size Impact

**Current** (inlined):
- value_is_none: 15 bytes × N calls
- exit_is_blocked: 18 bytes × N calls
- exit_get_data: 8 bytes × N calls
- exit_get_message: 8 bytes × N calls
- get_exit: 100 bytes × N calls

**After** (functions):
- value_is_none: 15 bytes (once) + 8 bytes × N calls
- exit_is_blocked: 18 bytes (once) + 8 bytes × N calls
- exit_get_data: 8 bytes (once) + 8 bytes × N calls
- exit_get_message: 8 bytes (once) + 8 bytes × N calls
- get_exit: 100 bytes (once) + 8 bytes × N calls

**For mini_zork** (rough estimate: 5 calls to each):
- Current: (15+18+8+8+100) × 5 = 745 bytes
- After: (15+18+8+8+100) + (8 × 5 × 5) = 149 + 200 = 349 bytes
- **Savings: ~400 bytes** (53% reduction)

## Implementation Order

1. Phase 1: Infrastructure
2. Phase 2.3: exit_get_data (easiest - single AND instruction)
3. Phase 2.4: exit_get_message (identical to exit_get_data)
4. Phase 2.1: value_is_none (simple boolean with branch)
5. Phase 2.2: exit_is_blocked (simple boolean with comparison)
6. Phase 2.5: get_exit (most complex - loop with multiple properties)
7. Phase 3: Update call sites
8. Phase 4: Delete old code
9. Phase 5: Testing

## Open Questions

1. Should builtins go before or after user functions?
   - **Decision**: After initialization, before user functions (cleaner)

2. How to handle builtin function IDs in IR?
   - **Decision**: Use separate `builtin_functions` map, not in main IR

3. Should we cache packed addresses or calculate on demand?
   - **Decision**: Store unpacked in function_addresses, pack at call site

4. What about print builtin and other existing builtins?
   - **Decision**: Start with exit system only, evaluate others later
