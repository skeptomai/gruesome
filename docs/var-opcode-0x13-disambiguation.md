# VAR Opcode 0x13 Disambiguation

## Overview

This document explains a critical implementation detail in the Z-Machine interpreter: how to properly handle the VAR:0x13 opcode collision between `get_next_prop` and `output_stream`.

## The Problem

In the Z-Machine specification, the same opcode number `0x13` is used for **two completely different instructions** when they're in **Variable form**:

1. **`get_next_prop`** - Originally a 2OP instruction (2OP:0x13)
2. **`output_stream`** - A VAR instruction (VAR:0x13) 

## How This Opcode Collision Occurs

The Z-Machine has different opcode tables:
- **2OP opcodes:** 0x00-0x1F (32 opcodes)
- **VAR opcodes:** 0x00-0xFF (256 opcodes)

The collision happens because **some 2OP instructions can also be encoded in Variable form** for flexibility (like when you need more than 2 operands, or different operand types).

So we have:
- **2OP:0x13** = `get_next_prop` (normal 2OP form)
- **VAR:0x13 with 2OP semantics** = `get_next_prop` (same instruction, Variable encoding)
- **VAR:0x13 with VAR semantics** = `output_stream` (completely different instruction)

## The Z-Machine Specification Solution

The Z-Machine specification resolves this collision by looking at the **instruction's behavior**:

- **`get_next_prop`** always **stores a result** → has `store_var`
- **`output_stream`** never **stores a result** → no `store_var`

From the Z-Machine Standards Document v1.1:
- **Section 15.4 (get_next_prop):** "get_next_prop object property -> (result)"
- **Section 15.10 (output_stream):** "output_stream number" (no result storage)

## Implementation in the Interpreter

### Before Fix (Broken)
```rust
// BROKEN: Both would be routed to object module
VAR:0x13 with store_var    → get_next_prop ✓
VAR:0x13 without store_var → get_next_prop ✗ (should be output_stream)
```

This caused Trinity to fail with:
```
Error: VAR:0x13 without store_var should be output_stream, not object operation
```

### After Fix (Correct)
```rust
// FIXED: Correct routing based on store_var
VAR:0x13 with store_var    → Object module (get_next_prop) ✓  
VAR:0x13 without store_var → Main interpreter (output_stream) ✓
```

### Code Implementation

In `src/opcodes_object.rs`:
```rust
/// Check if a VAR:0x13 opcode should be routed to the object module
/// This handles the get_next_prop vs output_stream disambiguation
pub fn is_var_13_object_opcode(inst: &crate::instruction::Instruction) -> bool {
    inst.opcode == 0x13 
        && inst.operand_count == crate::instruction::OperandCount::VAR
        && inst.store_var.is_some()
}
```

In `src/interpreter.rs`:
```rust
// Check if this is an object operation and route to object module
} else if Interpreter::is_object_opcode(inst.opcode, &inst.operand_count) 
          || Interpreter::is_var_13_object_opcode(inst) {
    self.execute_object_op(inst, &operands)
```

## Why This Matters

This disambiguation is critical for Z-Machine compatibility:

1. **Game Compatibility:** Many Z-Machine games use both instructions, and incorrect routing breaks gameplay
2. **Specification Compliance:** The Z-Machine spec explicitly defines this disambiguation method
3. **Modular Architecture:** Proper routing ensures object operations stay in the object module while I/O operations stay in the main interpreter

## Games Affected

This fix was essential for:
- **Trinity** - Uses output_stream extensively for display formatting
- **AMFV** - Uses both get_next_prop and output_stream  
- Any v4+ game that uses stream redirection or property iteration

## Historical Context

This issue was discovered during the object operations extraction refactoring when Trinity began failing at startup. The error revealed that VAR:0x13 without store_var was being incorrectly routed to the object module instead of being handled as output_stream in the main interpreter.

The fix maintains the critical insight documented in CLAUDE.md: we use Z-Machine specification compliance rather than heuristics to distinguish opcodes, specifically checking for `inst.store_var.is_some()` as defined in the official specification.

## References

- Z-Machine Standards Document v1.1, Section 15.4 (get_next_prop)
- Z-Machine Standards Document v1.1, Section 15.10 (output_stream)
- Project file: `src/opcodes_object.rs` - Implementation
- Project file: `CLAUDE.md` - Critical VAR opcode 0x13 fix documentation