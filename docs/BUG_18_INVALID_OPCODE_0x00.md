# Bug #18: Invalid Long Form Opcode 0x00 at Address 0x1231

## Status: INVESTIGATING

## Symptom
After fixing Bug #17, "east" command in mini_zork.z3 triggers new error:
```
Error during execution: Failed to decode instruction at 01231: Invalid Long form opcode 0x00 at address 1231
```

## Evidence

### Execution Context
- Command: `east`
- Last successful PC: 0x1118 (JE instruction comparing 0x0001 vs 0x0000)
- Error PC: 0x1231
- Score display shows "Score: 1973 Moves: 0" (unusual value)

### Memory Analysis at 0x1231
```
00001230: 1700 008d 04d4 bb8c 0006 8d04 dcbb b0b4
```

- **0x1230**: `0x17` = test_attr instruction (LONG form, opcode 7)
  - Binary: 00010111
  - Top 2 bits (00) = LONG form
  - Bits 5-4 (01) = small constant, variable operand types
  - Bits 3-0 (0111) = opcode 7 (test_attr)
- **0x1231**: `0x00` = first operand (small constant 0)
- **0x1232**: `0x8d` = second operand (variable 141)
- **0x1233**: `0x04` = branch byte

## Root Cause Analysis

The error "Invalid Long form opcode 0x00 at address 1231" means the interpreter thinks 0x1231 is the START of an instruction, not a parameter byte. This indicates **PC advancement error** - some instruction before 0x1231 has incorrect size calculation, causing PC to land in the middle of the test_attr instruction.

This is the SAME class of bug as #17:
- Bug #17: inc_chk encoded wrong form → interpreter decoded as different instruction → wrong size → PC misalignment
- Bug #18: UNKNOWN instruction encoded wrong form/size → PC misalignment → lands at 0x1231 instead of 0x1230

## Key Observations

1. **Not at 0x1230**: The error is at 0x1231, which is 1 byte off from the actual instruction start
2. **Gap in execution**: Last logged PC was 0x1118, error at 0x1231 (gap of 0x119 bytes = 281 bytes)
3. **Unusual score**: Score shows 1973 instead of 0, suggesting memory corruption or wrong value being displayed

## Investigation Strategy

**Priority**: Use instrumentation, avoid reverse engineering binary.

### Tools Available
1. **Compiler debug output**: Shows what instructions were emitted and their sizes
2. **Interpreter PC logging**: Can trace every instruction execution with PC, opcode, form, size
3. **Disassembler**: Can show expected instruction sequence

### Next Steps (Complexity-Based)

#### Round 1: Simple Instrumentation (TRIVIAL)
1. Enable PC logging for all instructions between 0x1118 and 0x1231
2. Find which instruction's PC + size doesn't match next PC
3. Identify the misencoded instruction

#### Round 2: Targeted Analysis (SIMPLE)
1. Check compiler output for instructions in range 0x1118-0x1231
2. Look for other context-dependent opcodes (like 0x05 from Bug #17)
3. Verify size calculations for all VAR form instructions in that range

#### Round 3: Form Determination Audit (MODERATE)
1. Review determine_instruction_form() for other missing context checks
2. Check if any opcodes besides 0x05 and 0x0D need context-dependent handling
3. Cross-reference with Z-Machine spec for all ambiguous opcodes

## Similar Bugs
- Bug #16: Store instruction form (opcode 0x0D)
- Bug #17: inc_chk vs print_char (opcode 0x05)

Both were caused by missing context checks in determine_instruction_form().

## Hypothesis
There's likely another opcode that needs context-dependent form determination, causing wrong encoding → wrong decoding → wrong size → PC misalignment → error at 0x1231.

## Prevention
After fixing, audit ALL opcodes in determine_instruction_form() to ensure context-dependent opcodes are handled correctly.
