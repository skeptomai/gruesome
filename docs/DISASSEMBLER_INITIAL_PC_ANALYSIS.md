# Disassembler Initial PC Analysis

## Core Issue: Interpreter vs Disassembler Initial PC Assumptions

### Discovery Date
November 2025 - Investigation into why disassembler finds only 8/25 routines in mini_zork

### Root Cause
**Architectural mismatch between interpreter and disassembler handling of initial_pc**

### Execution Models

#### Interpreter (works with both bare code and routine headers)
```rust
// interpreter.rs:354
let pc = self.vm.pc
// interpreter.rs:377
Instruction::decode(&self.vm.game.memory, pc as usize, ...)
```
- **Just executes whatever instruction is at initial_pc**
- No assumptions about routine headers
- Works with bare code or routine headers

#### Disassembler (assumes initial_pc is routine header)
```rust
// TXD iterative expansion assumes start_pc is valid routine
self.low_address = start_pc;
self.high_address = start_pc;
// Then tries to validate as routine header
```
- **Assumes initial_pc points to routine header**
- Tries to validate vars count ≤ 15
- Fails when initial_pc points to bare code

### Test Cases

#### Mini_zork (bare code at initial_pc)
- **initial_pc**: `0677` points to bare instructions (no routine header)
- **Interpreter**: ✅ Executes instructions starting at 0677 directly
- **Disassembler**: ❌ Tries to treat 0677 as routine header → `vars=158` → `FAIL_VARS`

#### Commercial Zork I (routine header at initial_pc)
- **initial_pc**: `4f04` points to valid routine header
- **Interpreter**: ✅ Executes instructions (routine header contains valid instructions)
- **Disassembler**: ✅ 4f04 passes routine header validation

### Boundary Reset Problem

Queue processing correctly discovers routine boundaries:
- **Mini_zork**: Finds routines from `060e` to `0e24` via call/branch following
- **Commercial**: Finds routines and expands boundaries

But iterative expansion resets boundaries to start_pc:
```rust
// This throws away queue processing discoveries
self.low_address = start_pc;  // 0677 in mini_zork (invalid)
self.high_address = start_pc; // 4f04 in Zork I (valid)
```

### Solutions

1. **Preserve queue processing boundaries** instead of resetting to start_pc
2. **Don't assume start_pc is routine header** for iterative expansion
3. **Validate start_pc before using** for boundary reset

### Impact

- **Commercial games**: Work by accident (start_pc happens to be valid routine)
- **Compiled games**: Fail because start_pc points to bare code
- **Interpreter**: Always works regardless of initial_pc type

### Design Decision Validation

The compiler's choice to emit bare code at initial_pc is **perfectly valid per Z-Machine specification**:

**Z-Machine Standards Document v1.0 - Section 5.5:**
> "In all other Versions, the word at **$06** contains the byte address of the first instruction to execute. The Z-machine starts in an environment with no local variables from which, again, a return is illegal."

**Section 5.4 (Version 6 only):**
> "In Version 6, there is a 'main' routine (whose packed address is stored in the word at **$06** in the header) called when the game starts up."

**Specification Analysis:**
- **V1-V5**: ✅ **Bare code allowed** - initial_pc = "first instruction to execute"
- **V6**: ❌ **Routine required** - initial_pc = "main routine packed address"
- **Mini_zork (V3)**: ✅ **Compliant** - bare code is spec-legal
- **Commercial V3**: ✅ **Also compliant** - routine headers are also legal (just not required)

**Validation Results:**
- **Z-Machine spec**: ✅ **Bare code explicitly allowed for V1-V5**
- **Interpreter compatibility**: ✅ Works correctly (executes instructions regardless)
- **Disassembler compatibility**: ❌ Breaks due to **false assumption** that start_pc must be routine header

**Root Cause Confirmed**: **Disassembler architectural bug** - assumes routine headers required when Z-Machine spec explicitly allows bare code for V1-V5.

### Next Steps

1. ~~Check Z-Machine specification for initial_pc requirements~~ ✅ **COMPLETED**
2. Implement boundary preservation fix to handle both bare code and routine headers
3. Test with both commercial and compiled games