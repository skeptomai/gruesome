# Object Mapping Architecture - Anti-Regression Documentation

## Problem Statement
The Grue compiler has experienced recurring issues with object identifier resolution where the same object gets assigned different numbers in different phases, leading to "Property X not found for object Y" runtime errors.

## Root Cause Analysis
1. **Duplicate Object Assignment**: Objects were assigned numbers in both registration pass and generation pass
2. **Inconsistent IR ID Mapping**: The same identifier created different IR IDs in different contexts
3. **Missing IR → Object Mapping**: No systematic way to map IR IDs to Z-Machine object numbers

## Architectural Solution

### 1. Two-Phase Object Number Assignment
```
Registration Pass: room_a -> object #2 (AUTHORITATIVE)
Generation Pass:  room_a -> SKIP (already assigned)
```

### 2. IR Program Object Mapping Transfer
```
IrGenerator.object_numbers -> IrProgram.object_numbers -> CodeGen.ir_id_to_object_number
```

### 3. Resolution Priority Order
```rust
resolve_ir_id_to_operand(ir_id):
1. Check ir_id_to_stack_var (stack results)
2. Check ir_id_to_object_number (object references) <- NEW
3. Check ir_id_to_integer (literals)  
4. Fallback to player object
```

## Implementation Details

### Key Files Modified
- `src/grue_compiler/ir.rs`: Added duplicate assignment prevention
- `src/grue_compiler/codegen.rs`: Added object mapping infrastructure
- Both phases now coordinate through IrProgram structure

### Critical Code Paths
1. **ir.rs:1389-1403**: Duplicate assignment prevention
2. **codegen.rs:2999-3019**: Object mapping setup  
3. **codegen.rs:2974-2981**: Object reference resolution

## Regression Prevention

### Test Cases That Must Always Pass
```bash
# Simple property access
cargo run --bin gruesome test_property_regression.z3

# Complex chained access  
cargo run --bin gruesome debug_ir_compare.z3
```

### Architecture Invariants
1. **Single Assignment**: Each object gets exactly one object number (registration pass)
2. **Consistent Mapping**: IR IDs always resolve to correct object numbers
3. **No Overwrites**: Generation pass never overwrites registration assignments

### Debug Verification Commands
```bash
# Verify no duplicate assignments
RUST_LOG=debug cargo run --bin grue-compiler -- file.grue 2>&1 | grep "already has object number"

# Verify mapping setup
RUST_LOG=debug cargo run --bin grue-compiler -- file.grue 2>&1 | grep "Object mapping setup complete"

# Verify no fallback to player assumptions
RUST_LOG=debug cargo run --bin grue-compiler -- file.grue 2>&1 | grep "assumed to be player object"
```

## Remaining Technical Debt
1. **IR ID Duplication**: Same identifier still creates multiple IR IDs in different contexts
2. **Integer Literal Fallback**: Some object references still resolve via integer literals (fragile)
3. **Semantic Analysis**: Root cause in AST→IR conversion not fully addressed

However, the current fix provides **architectural guarantees** that prevent the original regression even if the IR ID duplication persists.