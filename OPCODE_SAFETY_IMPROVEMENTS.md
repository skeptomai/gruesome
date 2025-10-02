# Opcode Safety Improvements

## Problem Summary

We had a bug where `call_vs` was emitted with opcode `0x20` instead of `0x00`. This caused the instruction to be encoded as VAR form with 2OP count (`0xC0`) instead of VAR form with VAR count (`0xE0`), breaking execution.

The root cause: Inconsistent opcode values throughout the codebase - some places using `0x20` for call_vs, others using `0x00`.

## Proposed Solutions

### 1. **Create Opcode Constants**

Define constants for all opcodes in one place:

```rust
// In opcode_tables.rs or a new opcodes.rs file
pub mod opcodes {
    // VAR opcodes (pass the raw opcode number, not the encoded byte)
    pub const CALL_VS: u8 = 0x00;    // VAR:224
    pub const STOREW: u8 = 0x01;     // VAR:225
    pub const STOREB: u8 = 0x02;     // VAR:226
    pub const PUT_PROP: u8 = 0x03;   // VAR:227
    pub const SREAD: u8 = 0x04;      // VAR:228
    pub const PRINT_CHAR: u8 = 0x05; // VAR:229
    pub const PRINT_NUM: u8 = 0x06;  // VAR:230
    pub const RANDOM: u8 = 0x07;     // VAR:231

    // 1OP opcodes
    pub const JZ: u8 = 0x00;         // 1OP:128
    pub const GET_SIBLING: u8 = 0x01; // 1OP:129
    pub const GET_CHILD: u8 = 0x02;   // 1OP:130
    pub const GET_PARENT: u8 = 0x03;  // 1OP:131
    pub const CALL_1S: u8 = 0x08;     // 1OP:136
    pub const PRINT_PADDR: u8 = 0x0D; // 1OP:141
    pub const LOAD: u8 = 0x0E;        // 1OP:142

    // 2OP opcodes
    pub const JE: u8 = 0x01;         // 2OP:1
    pub const JL: u8 = 0x02;         // 2OP:2
    pub const JG: u8 = 0x03;         // 2OP:3
    pub const ADD: u8 = 0x14;        // 2OP:20
    pub const SUB: u8 = 0x15;        // 2OP:21
    pub const LOADW: u8 = 0x0F;      // 2OP:15
    pub const LOADB: u8 = 0x10;      // 2OP:16

    // 0OP opcodes
    pub const RTRUE: u8 = 0x00;      // 0OP:176
    pub const RFALSE: u8 = 0x01;     // 0OP:177
    pub const PRINT: u8 = 0x02;      // 0OP:178
    pub const PRINT_RET: u8 = 0x03;  // 0OP:179
    pub const NEW_LINE: u8 = 0x0B;   // 0OP:187
    pub const QUIT: u8 = 0x0A;       // 0OP:186
}
```

Then use these everywhere:

```rust
self.emit_instruction(
    opcodes::CALL_VS,  // Instead of 0x00 or 0x20
    &[Operand::LargeConstant(placeholder_word())],
    Some(0x00),
    None,
)?;
```

### 2. **Add Validation in emit_instruction**

Add assertions to catch misuse:

```rust
pub fn emit_instruction(
    &mut self,
    opcode: u8,
    operands: &[Operand],
    store_var: Option<u8>,
    branch_offset: Option<i16>,
) -> Result<InstructionLayout, CompilerError> {
    // Validate that opcode is in expected range
    // Valid raw opcodes are 0x00-0x1F
    // Encoded instruction bytes start at 0xC0 for VAR form
    if opcode > 0x1F && opcode < 0xC0 {
        return Err(CompilerError::CodeGenError(format!(
            "Invalid opcode 0x{:02x} - opcodes should be raw numbers 0x00-0x1F, not encoded instruction bytes. \
            Did you mean to use one of the opcode constants from the opcodes module?",
            opcode
        )));
    }

    // Also warn if we see suspicious values that might be mistakes
    if opcode == 0x20 {
        log::warn!(
            "Opcode 0x20 detected - this is not a valid raw opcode. \
            If you meant call_vs, use opcodes::CALL_VS (0x00) instead."
        );
    }

    // ... rest of function
}
```

### 3. **Add Comments Documenting the Convention**

At the top of emit_instruction and emit_variable_form_with_layout:

```rust
/// Emits a Z-Machine instruction.
///
/// **CRITICAL CONVENTION**: The `opcode` parameter must be the RAW opcode number (0x00-0x1F),
/// NOT the encoded instruction byte. This function will determine the proper
/// instruction form (Long, Short, Variable, Extended) and encode it correctly.
///
/// **Examples of CORRECT usage**:
/// - call_vs (VAR:224) → pass `0x00` (or `opcodes::CALL_VS`), NOT `0xE0` or `0x20`
/// - put_prop (VAR:227) → pass `0x03` (or `opcodes::PUT_PROP`), NOT `0xE3`
/// - jl (2OP:2) → pass `0x02` (or `opcodes::JL`)
/// - print_paddr (1OP:141) → pass `0x0D` (or `opcodes::PRINT_PADDR`), NOT `0x8D`
///
/// **Why this matters**:
/// The function uses `is_true_var_opcode(opcode)` to determine if bit 5 should be set
/// in the instruction byte. If you pass the encoded byte (e.g., 0x20 instead of 0x00),
/// the function won't recognize it as a VAR opcode and will encode it incorrectly.
///
/// **Parameters**:
/// - `opcode`: Raw opcode number (0x00-0x1F)
/// - `operands`: Operand values
/// - `store_var`: Variable number to store result (None if instruction doesn't store)
/// - `branch_offset`: Branch offset (None if instruction doesn't branch)
pub fn emit_instruction(
    &mut self,
    opcode: u8,
    operands: &[Operand],
    store_var: Option<u8>,
    branch_offset: Option<i16>,
) -> Result<InstructionLayout, CompilerError> {
    // ... implementation
}
```

### 4. **Add Unit Tests**

```rust
#[cfg(test)]
mod opcode_encoding_tests {
    use super::*;

    #[test]
    fn test_call_vs_encoding() {
        let mut codegen = CodeGen::new();
        codegen.emit_instruction(0x00, &[Operand::LargeConstant(0x1234)], Some(0), None).unwrap();

        // Should emit 0xE0 (VAR form with VAR count), not 0xC0 (VAR form with 2OP count)
        assert_eq!(
            codegen.code_space[0], 0xE0,
            "call_vs (opcode 0x00) should emit 0xE0 instruction byte for VAR form with VAR count"
        );
    }

    #[test]
    fn test_put_prop_encoding() {
        let mut codegen = CodeGen::new();
        codegen.emit_instruction(
            0x03,
            &[
                Operand::Variable(1),
                Operand::SmallConstant(13),
                Operand::LargeConstant(2)
            ],
            None,
            None
        ).unwrap();

        // Should emit 0xE3 for put_prop
        assert_eq!(codegen.code_space[0], 0xE3, "put_prop should emit 0xE3");
    }

    #[test]
    fn test_print_paddr_encoding() {
        let mut codegen = CodeGen::new();
        codegen.emit_instruction(
            0x0D,
            &[Operand::LargeConstant(0x0399)],
            None,
            None
        ).unwrap();

        // Should emit 0x8D (SHORT form, 1OP, opcode 0x0D)
        assert_eq!(codegen.code_space[0], 0x8D, "print_paddr should emit 0x8D");
    }

    #[test]
    #[should_panic(expected = "Invalid opcode")]
    fn test_rejects_encoded_opcode_bytes() {
        let mut codegen = CodeGen::new();
        // Should reject 0x20 as it's not a valid raw opcode
        codegen.emit_instruction(0x20, &[Operand::LargeConstant(0x1234)], Some(0), None).unwrap();
    }
}
```

### 5. **Add CI/Pre-commit Checks**

Add to `.github/workflows/ci.yml` or a pre-commit hook:

```bash
#!/bin/bash
# Check for dangerous opcode patterns

echo "Checking for potentially incorrect opcode values..."

# Find emit_instruction calls with suspicious opcode values (0x20-0xBF)
# These are likely encoded bytes instead of raw opcodes
suspicious=$(grep -rn "emit_instruction.*0x[2-9][0-9A-Fa-f]," src/grue_compiler/ --include="*.rs" || true)

if [ -n "$suspicious" ]; then
    echo "⚠️  WARNING: Found emit_instruction calls with suspicious opcode values:"
    echo "$suspicious"
    echo ""
    echo "Opcodes should be raw numbers (0x00-0x1F), not encoded instruction bytes."
    echo "Example: Use 0x00 for call_vs, not 0xE0 or 0x20"
    exit 1
fi

# Check for hardcoded opcode values instead of constants
hardcoded=$(grep -rn "emit_instruction.*0x[0-1][0-9A-Fa-f]," src/grue_compiler/ --include="*.rs" | grep -v "opcodes::" || true)

if [ -n "$hardcoded" ]; then
    echo "ℹ️  INFO: Found hardcoded opcode values (consider using opcode constants):"
    echo "$hardcoded"
fi

echo "✅ Opcode validation passed"
```

### 6. **Audit Existing Codebase**

Run this command to find all places that might need fixing:

```bash
# Find all emit_instruction calls
grep -rn "emit_instruction" src/grue_compiler/ --include="*.rs" | grep "0x" | less

# Find specific problematic patterns
grep -rn "0x20.*call" src/grue_compiler/ --include="*.rs"
grep -rn "0xE0.*call" src/grue_compiler/ --include="*.rs"
grep -rn "0x8D.*print" src/grue_compiler/ --include="*.rs"
```

## Implementation Priority

1. **Immediate**: Add validation in `emit_instruction` (Solution #2) - prevents future bugs
2. **Short-term**: Add documentation comments (Solution #3) - helps developers understand the convention
3. **Medium-term**: Create opcode constants (Solution #1) - makes code more readable and maintainable
4. **Long-term**: Add comprehensive tests (Solution #4) and CI checks (Solution #5) - ensures quality

## Related Files

- `src/grue_compiler/codegen_instructions.rs` - Contains `emit_instruction` and `emit_variable_form_with_layout`
- `src/grue_compiler/codegen.rs` - Contains many calls to `emit_instruction`
- `src/grue_compiler/codegen_builtins.rs` - Contains builtin function implementations that emit instructions
- `src/opcode_tables.rs` - Contains opcode classifications and helpers
