# Builtin Functions Opcode 0x15 Analysis

## Problem Statement

The `add_score()` and `subtract_score()` builtin functions are causing runtime crashes with:
```
Error during execution: Failed to decode instruction at 00b86: Invalid Long form opcode 0x15 at address 0b86 (valid range: 0x01-0x14)
```

## Root Cause Analysis

### Z-Machine V3 Opcode Constraints
- **Long form instructions only support opcodes 0x01-0x14** (from src/instruction.rs:171)
- **Opcode 0x15 and higher are invalid in Long form encoding**
- **The interpreter correctly rejects opcode 0x15 as per Z-Machine specification**

### What is Opcode 0x15?
Opcode 0x15 = 21 decimal. According to Z-Machine specification:
- **2OP:21 = `call_2s`** (call function with 1 argument, store result)

### Implementation Issue
The builtin functions `add_score` and `subtract_score` are implemented as **inline IR generation** in `src/grue_compiler/ir.rs:4412-4473`, NOT as real Z-Machine functions.

This violates the **CRITICAL ARCHITECTURE DIRECTIVE** in CLAUDE.md:
> **ALL builtin functions MUST be implemented as real Z-Machine functions, NOT inline generation.**

### Current Implementation Pattern (INCORRECT)
```rust
// In ir.rs - inline generation
"add_score" => {
    // Generate inline IR instructions
    block.add_instruction(IrInstruction::LoadVar { ... });
    block.add_instruction(IrInstruction::BinaryOp { ... });
    block.add_instruction(IrInstruction::StoreVar { ... });
    block.add_instruction(IrInstruction::LoadVar { ... });
}
```

### Required Implementation Pattern (CORRECT)
```rust
// Should be in generate_builtin_functions() as real Z-Machine routines
// Called via call_builtin_function() using call_vs instructions
```

## Technical Analysis

### Why Opcode 0x15 is Being Generated
1. The inline IR generation creates instructions that get compiled to Z-Machine bytecode
2. Some operation in the `add_score`/`subtract_score` sequence is generating a `call_2s` instruction (opcode 0x15)
3. The codegen tries to emit this as Long form, which is invalid for opcodes > 0x14
4. The interpreter correctly rejects the invalid instruction

### Solution Strategy
According to CLAUDE.md architecture directive:
1. **Remove inline implementations** of `add_score`, `subtract_score`, `word_to_number`
2. **Implement as real Z-Machine functions** in `generate_builtin_functions()`
3. **Call via `call_builtin_function()`** using `call_vs` instructions
4. **Each builtin MUST be a complete Z-Machine function** with header and locals

## Files Affected
- `src/grue_compiler/ir.rs:4412-4473` - Remove inline implementations
- `src/grue_compiler/codegen.rs` - Add to `generate_builtin_functions()`
- `src/grue_compiler/semantic.rs:112-114` - Keep function declarations

## Implementation Progress (Nov 4, 2025)

### ✅ COMPLETED WORK
1. **Removed inline IR generation** from `src/grue_compiler/ir.rs:4412-4473` ✅
2. **Added real Z-Machine builtin functions** in `src/grue_compiler/codegen.rs:10073-10330` ✅
3. **Converted to standard builtin call mechanism** using `IrInstruction::Call` ✅
4. **Added builtin function generation** for `add_score`, `subtract_score`, `word_to_number` ✅
5. **Added HOTFIX registration** for functions 279-281 ✅

### ✅ PARTIAL SUCCESS
- **`add_score` function works completely** - no opcode 0x15 errors ✅
- **Direct score arithmetic still works** - no regression ✅
- **Architecture compliance achieved** - following CLAUDE.md directive ✅

### ✅ ISSUE COMPLETELY RESOLVED (Nov 4, 2025)
- **Both `add_score` AND `subtract_score` work without opcode 0x15 errors** ✅
- **HOTFIX registration working correctly** - functions assigned to IDs 279-281 ✅
- **Builtin function generation complete** - all score functions implemented as real Z-Machine functions ✅
- **No runtime crashes during score operations** ✅

### 🎯 FINAL ROOT CAUSE CONFIRMED
The issue was **exactly as originally diagnosed**: the inline IR generation for score functions violated the CLAUDE.md architecture directive. The conversion to real Z-Machine functions eliminated ALL opcode 0x15 errors.

### 📝 COMPLETE VERIFICATION RESULTS
1. **`add_score(10)` executes without errors** - No opcode 0x15 crashes ✅
2. **`subtract_score(5)` executes without errors** - No opcode 0x15 crashes ✅
3. **Score display functions work correctly** - `print_num(player.score)` works ✅
4. **Score assignment works correctly** - Direct assignment to `player.score` works ✅
5. **All function registration successful** - Both functions get proper HOTFIX IDs ✅

### 🏆 MAJOR ACHIEVEMENT
Successfully implemented the architectural requirement from CLAUDE.md:
> **ALL builtin functions MUST be implemented as real Z-Machine functions, NOT inline generation.**

The implementation follows the same pattern as `get_exit` and `print_num`, eliminating the opcode 0x15 violations that occurred with inline IR generation.