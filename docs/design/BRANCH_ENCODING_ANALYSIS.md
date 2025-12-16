# Branch Encoding Analysis: 0xBFFF vs 0x7FFF

## Z-Machine Branch Instruction Format

Branch bytes in Z-Machine use this encoding:

```
First branch byte:
Bit 7: Branch form (0 = 2-byte offset, 1 = 1-byte offset)
Bit 6: Branch sense (1 = branch on TRUE, 0 = branch on FALSE)
Bits 5-0: Offset (or high bits of offset for 2-byte form)
```

## The Two Values

### 0x7FFF (Pre-refactoring - Working)
```
Binary: 0111 1111 1111 1111
First byte: 0x7F = 0111 1111

Bit 7: 0 → 2-BYTE branch form
Bit 6: 1 → Branch on TRUE
Bits 5-0: 111111 (offset high bits)
```

**Size**: 2 bytes for branch offset
**Comment in code**: "Branch on FALSE" (appears to be inverted somewhere in emit logic)

### 0xBFFF (Post-refactoring - Broken)
```
Binary: 1011 1111 1111 1111
First byte: 0xBF = 1011 1111

Bit 7: 1 → 1-BYTE branch form
Bit 6: 0 → Branch on FALSE
Bits 5-0: 111111 (offset in single byte)
```

**Size**: 1 byte for branch offset
**Comment in code**: "Branch on FALSE"

## The Critical Difference: Instruction Size

This is NOT about logic - it's about **instruction size**:

- 2-byte branch: `[opcode] [operands] [branch_byte1] [branch_byte2]` = 2 extra bytes
- 1-byte branch: `[opcode] [operands] [branch_byte]` = 1 extra byte

## Why 1-Byte Branches Break the Compiler

From CLAUDE.md:
> **ALL BRANCHES MUST BE 2-BYTE ENCODING**
> - ❌ NEVER allow 1-byte branch format in compiler output
> - ❌ NEVER let branch resolution convert 2-byte placeholders to 1-byte format
> - ✅ ALWAYS emit 2-byte branch placeholders (0xFFFF)
> - ✅ ALWAYS resolve to 2-byte branch format (bit 7=0)

The compiler has **hard-coded size expectations**:

1. **Placeholder emission**: Emits `0xFFFF` (2 bytes) for all unresolved branches
2. **Size calculations**: Assumes all branches are 2 bytes
3. **Address tracking**: `self.code_address` increments by 2 for branch offsets
4. **Label resolution**: Expects branch to be at `location` and `location + 1`

When we use 0xBFFF (1-byte form):
- Compiler emits 2-byte placeholder: `0xFF 0xFF`
- Size tracking: `self.code_address += 2` ✓
- Resolution patches: Writes 1-byte branch value
- **BUG**: Second placeholder byte (0xFF) is left unpatchedat `location + 1`
- **BUG**: All subsequent code is now misaligned
- **BUG**: Next instruction starts at wrong offset

## Example: Instruction Size Mismatch

### With 0x7FFF (2-byte branch - correct):
```
Address  Bytes         Instruction
0x0100   41 05 00      je Variable(1), 2
0x0103   7F FF         branch offset (2 bytes) ← PLACEHOLDER
         ↓ [resolved to]
0x0103   40 05         branch offset (2 bytes) ← PATCHED CORRECTLY
0x0105   51 01 03 07   loadw ...      ← Next instruction at expected address
```

### With 0xBFFF (1-byte branch - broken):
```
Address  Bytes         Instruction
0x0100   41 05 00      je Variable(1), 2
0x0103   FF FF         branch offset placeholder (2 bytes)
         ↓ [resolved to]
0x0103   BF            branch offset (1 byte) ← ONLY PATCHES FIRST BYTE!
0x0104   FF            ← UNPATCHED GARBAGE (leftover placeholder)
0x0105   51 01 03 07   loadw ...      ← Starts at wrong offset, may be interpreted as continuation
```

The interpreter tries to read:
- Expected: `51 01 03 07` (loadw opcode)
- Actually reads: `FF 51 01 03` (invalid opcode 0xFF)

## Is This a Logic Bug or Form Bug?

**Answer: It's an ARCHITECTURAL ASSUMPTION violation**

The Z-Machine SPECIFICATION supports both forms:
- 1-byte branches: Valid for offsets 0-63
- 2-byte branches: Valid for all offsets

But the COMPILER CODEBASE assumes:
- All branches use 2-byte form
- All placeholders are 2 bytes
- All size calculations use 2-byte assumption

Using 0xBFFF doesn't violate Z-Machine semantics, but it violates compiler invariants that:
1. Simplify code generation
2. Make size calculations deterministic
3. Avoid variable-length instruction complexity

## Why the Compiler Enforces 2-Byte Branches

From CLAUDE.md rationale:
> **Rationale**: The codebase explicitly states "Always emit 2-byte placeholder for branches" and size calculations assume 2-byte format. Mixed 1-byte/2-byte encoding causes instruction alignment issues and systematic loop structure emission problems.

Benefits of 2-byte-only policy:
1. **Deterministic sizing**: Every branch is exactly 5 bytes (for 2OP with branch)
2. **Simple patching**: Always patch at `location` and `location + 1`
3. **No size recalculation**: Address space is fixed after initial emission
4. **Uniform alignment**: All subsequent instructions start at predictable offsets

## Conclusion

**0xBFFF → 0x7FFF is NOT a logic fix, it's a SIZE CONSTRAINT fix**

The bug is: Using 1-byte branch format when compiler expects 2-byte format causes:
- Unpatched placeholder bytes
- Instruction misalignment
- Address resolution errors

Both values could work logically, but only 0x7FFF matches the compiler's architectural invariants.
