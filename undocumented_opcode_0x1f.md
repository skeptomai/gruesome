# Undocumented Opcode 0x1F Investigation

## Summary
2OP opcode 0x1F is an undocumented instruction that appears in Infocom games but is not part of the Z-Machine standard.

## Evidence from Zork I

At PC 08cb0, we see:
```
08cb0: 7f 02 a0 52
```

Decoded:
- 7f = 01111111 = Long form, 2OP, opcode 0x1F
- First operand: 0x02 (small constant 2)
- Second operand: 0xa0 (variable 160 = local 0)
- Store to: 0x52 (global 82 - the LIT variable)

## Context
This instruction appears to operate on constant 2 and a local variable, storing the result.

## Speculation
Given:
1. It takes two operands
2. It stores a result
3. The comment in the code suggests it might be a shift operation
4. The first operand is often a small constant (like 2)

This could be:
- A logical shift left/right by N bits
- Some other bitwise operation
- An arithmetic operation not covered by standard opcodes

## Current Implementation
We treat it as a NOP (no operation) because storing 0 was causing bugs. Without reverse engineering the original interpreter or analyzing more game behavior, we cannot determine its exact function.

## Recommendation
Keep it as NOP unless we find games that depend on its correct implementation. The fact that Zork I works with it as NOP suggests it's not critical for this game.