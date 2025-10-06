# Runtime Debugging Analysis - Grue Compiler Bytecode Issues

## Problem Summary

The Grue compiler successfully compiles source code but generates malformed Z-Machine bytecode that causes runtime crashes with errors like:
```
Error during execution: Failed to decode instruction at fffffa1a: Instruction address 4294965786 out of bounds
```

## Root Cause Identified ✅

**Manual instruction encoding is producing malformed Z-Machine bytecode**

The core issue is that several functions in the codegen bypass the proper Z-Machine instruction framework and manually emit bytes using `emit_byte()` calls. This produces invalid instruction sequences that don't follow Z-Machine encoding standards.

## Evidence of the Problem

### Bytecode Analysis
Using `./target/debug/debug_bytecode mini_zork.z3` revealed:

1. **Invalid opcode patterns**: Repeated 0x8d bytes, unknown opcodes like 0xb6, 0xd7
2. **Malformed instruction sequences**: `8d 08 0a 8d` and similar patterns
3. **Address corruption**: PC getting set to invalid values like `fffffa1a` (4294965786)

### Debug Trace
Runtime execution shows:
- Valid start: PC at 0x0b5e
- Proper property operations initially
- Then corruption leads to invalid address access

## Fixes Applied ✅

### 1. Fixed `generate_get_location_builtin` (lines 1977-1980)
**Before (BROKEN):**
```rust
self.emit_byte(0x83)?; // Manual opcode - WRONG!
self.emit_word(object_id as u16)?; // Manual operand
self.emit_byte(0x00)?; // Manual store variable
```

**After (FIXED):**
```rust
self.emit_instruction(
    0x04, // get_parent opcode (correct)
    &[Operand::LargeConstant(object_id as u16)],
    Some(0), // Store result on stack
    None, // No branch
)?;
```

### 2. Fixed `generate_get_child_builtin` (lines 2071-2074)
**Before (BROKEN):**
```rust
self.emit_byte(0x82)?; // get_child opcode - WRONG ENCODING!
self.emit_word(object_id as u16)?; // Object ID
self.emit_byte(0x00)?; // Store result
```

**After (FIXED):**
```rust
self.emit_instruction(
    0x03, // get_child opcode
    &[Operand::LargeConstant(object_id as u16)],
    Some(0), // Store result on stack
    None, // No branch
)?;
```

### 3. Fixed `generate_get_sibling_builtin` (lines 2095-2098)
**Before (BROKEN):**
```rust
self.emit_byte(0x81)?; // get_sibling opcode - WRONG!
self.emit_word(object_id as u16)?; // Object ID
self.emit_byte(0x00)?; // Store result
```

**After (FIXED):**
```rust
self.emit_instruction(
    0x02, // get_sibling opcode
    &[Operand::LargeConstant(object_id as u16)],
    Some(0), // Store result on stack
    None, // No branch
)?;
```

## Why Manual Encoding Fails

Z-Machine instructions have complex encoding rules:
1. **Instruction format bits**: Encode operand types in the opcode byte
2. **Operand type encoding**: Different bit patterns for constants vs variables
3. **Store variable encoding**: Must follow operand encoding rules
4. **Proper instruction length**: Instructions must be properly terminated

Manual `emit_byte()` calls bypass all of this and produce invalid sequences.

## Current Status

### ✅ Working
- Simple programs: `test_01_basic.grue` compiles and runs correctly
- Compilation phase: All examples compile without errors
- Z-Machine interpreter: Works perfectly with real games (Zork I, AMFV, Trinity)

### 🔴 Still Broken
- Complex programs: `mini_zork.grue` still crashes at runtime
- **Remaining manual emission points**: There are still more functions using manual byte emission that need to be found and fixed

## Remaining Work

### Search Pattern for More Bugs
Look for these patterns in `src/grue_compiler/codegen.rs`:
```rust
self.emit_byte(0x8X)?; // Manual 1OP opcodes
self.emit_byte(0xXX)?; // Followed by
self.emit_word(...)?;  // Manual operands
self.emit_byte(0x00)?; // Manual store variables
```

### Functions Still Needing Investigation
Based on grep results, these areas likely have more manual emission bugs:
- Lines around 2175: `generate_list_objects_builtin` 
- Lines around 2225: Another function with manual get_child calls
- Any other functions that emit object-related instructions manually

### Fix Strategy
For each manual emission sequence:

1. **Identify the intended Z-Machine instruction**
2. **Look up the correct opcode** (not the hex values being manually emitted)
3. **Replace with proper `emit_instruction()` call**
4. **Test compilation and runtime**

### Example Fix Template
```rust
// Replace this pattern:
self.emit_byte(0xXX)?; // Some opcode
self.emit_word(operand)?; // Some operand  
self.emit_byte(0x00)?; // Store variable

// With this pattern:
self.emit_instruction(
    CORRECT_OPCODE, // Look up in Z-Machine spec
    &[Operand::LargeConstant(operand)],
    Some(0), // Store on stack
    None, // No branch
)?;
```

## Testing Progress

### Simple Case (WORKING)
```bash
cargo run --bin grue-compiler -- examples/test_01_basic.grue
RUST_LOG=warn cargo run --bin gruesome test_01_basic.z3
# Result: ✅ "Test 1: Basic print works"
```

### Complex Case (STILL FAILING)
```bash
cargo run --bin grue-compiler -- examples/mini_zork.grue  
RUST_LOG=warn cargo run --bin gruesome mini_zork.z3
# Result: 🔴 "Failed to decode instruction at fffffa1a"
```

## Next Session Priority

**Systematically find and fix all remaining manual instruction emission bugs**

1. Search for manual `emit_byte(0x8X)` patterns
2. Replace each with proper `emit_instruction()` calls
3. Test after each fix
4. Continue until `mini_zork.z3` runs without crashes

Once this is complete, the Grue compiler will have working runtime execution, enabling full end-to-end testing of all the advanced features implemented in Phase 1.

---

**Key Insight**: The problem was never in the advanced features (objects, properties, parser) - those compile correctly. The issue is in basic instruction encoding where manual byte emission bypasses the Z-Machine standard compliance built into the `emit_instruction()` framework.