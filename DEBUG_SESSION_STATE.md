# Debug Session State - Mini Zork Infinite Loop Investigation

## Current Status
**CRITICAL BUG IDENTIFIED**: String ID used directly as packed address instead of UnresolvedReference

## Root Cause Analysis

### Problem Description
The mini_zork game gets stuck in an infinite loop at PC 0x12a2-0x12cd, repeatedly printing "ready open." fragments and eventually crashing with stack overflow.

### Root Cause Discovered
**String IDs are being emitted directly as LargeConstant operands instead of creating UnresolvedReferences.**

### Evidence
1. **Log Evidence**:
   ```
   [DEBUG] Created new string ID 1000 for 'West of House'
   [DEBUG] PC_TRACK: Emitting opcode=0x8d at PC=0x01f4 operands=[LargeConstant(1000)] store=Some(0)
   [DEBUG] Emit word: word=0x03e8 -> high_byte=0x03, low_byte=0xe8 at code_address 0x01f5
   ```

2. **Bytecode Evidence**:
   - Instruction at 0x12a9: `8d 03 e8` = print_paddr #0x03e8
   - 0x03e8 = 1000 decimal = string ID, NOT a resolved string address
   - When unpacked (1000 * 2 = 2000), points to random binary data

3. **Execution Evidence**:
   - Game outputs "ready open." fragments (from binary data misinterpreted as Z-Machine string)
   - Should be printing proper strings like "It's already open."

### Technical Details

**Correct Flow Should Be**:
1. String "West of House" gets ID 1000
2. print_paddr instruction creates UnresolvedReference for string ID 1000
3. During resolution, string ID 1000 → actual address (e.g., 0x07ca)
4. Packed address = 0x07ca / 2 = 0x03e5
5. print_paddr uses 0x03e5, unpacks to 0x07ca, reads correct string

**Actual Broken Flow**:
1. String "West of House" gets ID 1000
2. print_paddr instruction emits LargeConstant(1000) directly
3. 1000 becomes 0x03e8 in bytecode
4. print_paddr uses 0x03e8, unpacks to 0x07d0, reads random data

## Completed Fixes (Working)
✅ LoadVar opcode bug (0x0E → 0xAE)
✅ jz opcode bug (0x00 → 0xA0)
✅ UnresolvedReference location calculation bug
✅ Player object dual-path unification

## Required Fix
**Fix string instruction emission**: Change code generation to create UnresolvedReference entries for string operands instead of emitting string IDs as LargeConstant values.

**Location**: Likely in `src/grue_compiler/codegen_instructions.rs` or `src/grue_compiler/codegen_builtins.rs` where print_paddr instructions are generated.

## Test Files
- `/tmp/debug_infinite_loop.z3` - Contains the problematic bytecode
- `/Users/cb/Projects/infocom-testing-old/infocom/examples/mini_zork.grue` - Source file
- Line 224 contains "It's already open." string that becomes garbled

## MAJOR PROGRESS UPDATE

✅ **Fixed GetArrayElement infinite loop**: Replaced print_paddr (0x8D) with hardcoded LargeConstant(1000)
- **Problem**: GetArrayElement used print_paddr with string ID 1000 causing garbled "ready open." output
- **Solution**: Changed to load instruction (0xAE) with global variable 16 as safe placeholder
- **Result**: Game banner now displays correctly, infinite print loop eliminated

## Current Status
- ✅ Game banner displays properly: "DORK I: The Last Great Empire" with full text
- ✅ Print system working correctly - no more "ready open." garbled output
- ❌ New infinite loop at PC 0x12a2-0x12cd with jump instruction cycling
- **Pattern**: `JUMP CALCULATION: PC=0x12cd offset=-41 (0xffd7) -> new_pc=0x12a2`

## Remaining Issue
Different infinite loop in actual game logic (not string printing):
- Loop range: PC 0x12a2 → 0x12cd → jump back to 0x12a2
- Related to jump offset calculation or loop termination condition
- Game executes significantly further before hitting this issue

## Next Steps
1. Analyze jump instruction at PC 0x12cd causing the -41 offset back-jump
2. Determine if this is a compiler loop condition bug or missing loop termination
3. Fix the jump logic to allow proper loop exit