# Opcode Form Selection Bug - Root Cause Analysis

## Bug Summary

**Issue**: Runtime crash with "Invalid opcode" or "index out of bounds" when executing compiled Z-Machine games.

**Root Cause**: The compiler was selecting instruction forms based solely on operand count, ignoring the Opcode enum variant (Op2 vs OpVar). This caused opcode conflicts where the same raw opcode value (e.g., 0x08) has different meanings in different forms.

## The Conflict

Raw opcode **0x08** is used by TWO different instructions:

1. **2OP:OR** (opcode 8)
   - LONG form: `0x08` (bits: 00 01 000)
   - VAR form: `0xC8` (bits: 11 00 01000)
   - Requires 2 operands, stores result

2. **VAR:push** (opcode 232 = 0xE8)
   - VAR form only: `0xE8` (bits: 11 10 01000)
   - Raw opcode: 8 (within VAR range 224-255)
   - Requires 1 operand, no result stored

Similarly, **0x09**:
- **2OP:AND** vs **VAR:pull**

## The Bug Sequence

1. Code calls: `emit_instruction_typed(Opcode::OpVar(OpVar::Push), &[LargeConstant(1000)], None, None)`
2. Old code extracts raw opcode: `0x08`
3. Sees 1 operand → chooses VAR form (since 1 operand can't use LONG form for 2OP)
4. Emits: `0xC8 0x3F 0x03 0xE8`
   - `0xC8` = VAR form of 2OP:or (WRONG!)
   - Should be: `0xE8` = VAR:push

5. Interpreter decodes `0xC8`:
   - Form: VAR (bits 7-6 = 11)
   - Bit 5 = 0 → 2OP range opcode
   - Opcode: 8 → OR instruction
   - Expects 2 operands but only gets 1 → crash

## The Fix

### Part 1: Respect Opcode Variant in Form Selection

**File**: `src/grue_compiler/codegen_instructions.rs:1389-1436`

Changed `emit_instruction_typed()` to determine form based on enum variant:

```rust
let form = match opcode {
    Opcode::Op0(_) => InstructionForm::Short,  // 0OP form
    Opcode::Op1(_) => InstructionForm::Short,  // 1OP form
    Opcode::Op2(_) => {
        // 2OP can be LONG or VAR form depending on operands
        if operands.len() == 2 {
            let can_use_long = operands.iter().all(|op| match op {
                Operand::LargeConstant(v) => *v <= 255,
                _ => true,
            });
            if can_use_long {
                InstructionForm::Long
            } else {
                InstructionForm::Variable
            }
        } else {
            InstructionForm::Variable
        }
    }
    Opcode::OpVar(_) => InstructionForm::Variable,  // VAR form
};
```

### Part 2: Mark Push/Pull as True VAR Opcodes

**File**: `src/grue_compiler/codegen_instructions.rs:1716-1717`

Added to `is_true_var_opcode()`:

```rust
0x08 => true, // push (raw opcode 8) - MUST be VAR form (0xE8), NOT 2OP:or (0x08/0xC8)
0x09 => true, // pull (raw opcode 9) - MUST be VAR form (0xE9), NOT 2OP:and (0x09/0xC9)
```

This ensures bit 5 is set (0xE0 range) for these opcodes, producing 0xE8/0xE9 instead of 0xC8/0xC9.

### Part 3: Update Store Variable Check

**File**: `src/grue_compiler/codegen_instructions.rs:1622-1650`

Changed `should_not_emit_store_variable()` to receive encoded instruction byte instead of raw opcode:

```rust
fn should_not_emit_store_variable(instruction_byte: u8) -> bool {
    match instruction_byte {
        0xE8 => true, // VAR:push (0xE8) - no store byte
        0xE9 => true, // VAR:pull (0xE9) - no store byte
        // 2OP:OR (0x48/0x68/0xC8) and 2OP:AND (0x49/0x69/0xC9) DO have store bytes
        ...
    }
}
```

Updated all three callers (SHORT, LONG, VAR form emitters) to pass `instruction_byte` instead of raw `opcode`.

## Verification

**Before fix**:
```
0x13BD: C8 3F 03 E8    # Wrong: VAR form of 2OP:or with 1 operand
        └─ 0xC8 = VAR form, bit 5=0 (2OP range), opcode 8 = OR
```

**After fix**:
```
0x13BD: E8 3F 03 E8    # Correct: VAR:push with 1 operand
        └─ 0xE8 = VAR form, bit 5=1 (VAR range), opcode 8 = push
```

## Lessons Learned

1. **Z-Machine opcode encoding is form-dependent**: The same raw opcode number has different meanings in different instruction forms.

2. **Type safety matters**: Using enum variants (Op2 vs OpVar) prevents these conflicts at compile time.

3. **Don't trust raw opcodes alone**: Always consider the instruction form when encoding/decoding.

4. **The interpreter is correct**: It properly distinguishes 0xC8 (VAR form 2OP:or) from 0xE8 (VAR:push). The compiler must emit the right bytes.

## Related Files

- `src/grue_compiler/codegen_instructions.rs` - Instruction emission logic
- `src/grue_compiler/opcodes.rs` - Opcode definitions and metadata
- `COMPILER_ARCHITECTURE.md` - Architecture documentation

## Next Steps

The stack underflow error is now a different issue - likely the GetArrayElement placeholder implementation needs to be completed. But the core architectural opcode form selection bug is resolved.
