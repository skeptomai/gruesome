# PACKED ADDRESS ROOT CAUSE ANALYSIS (November 13, 2025)

## ISSUE SUMMARY

**TXD crashes on packed address `0x4a52` which unpacks to `0x94a4` (38,052 bytes), exceeding our file size of 9,156 bytes.**

## KEY FINDINGS

### 1. **Multiple Invalid Addresses, Not Just One**
- **TXD reported**: `0x4a52` → unpacks to 38,052 bytes (WAY out of bounds)
- **Our debug logs**: String ID 1018 uses `0x094a` → unpacks to 4,756 bytes (within bounds)
- **These are DIFFERENT addresses** - TXD is encountering other invalid packed addresses

### 2. **String Allocation Analysis**
- **String ID 1018**: "a small mailbox" (object property string)
- **Allocated at**: offset `0x079a` (1946 bytes) within string space (2,412 bytes)
- **Final address**: `0x0afa + 0x079a = 0x1294` (4,756 bytes - VALID)
- **Packed correctly**: `0x1294 / 2 = 0x094a` (VALID)

### 3. **Systematic Pattern Discovery**
From earlier analysis: **hundreds of `94a5` repeated in file starting at 0x790**
```
000007a0: 14a5 94a5 8000 14a5 94a5 8000 14a5 94a5  ................
000007b0: 8000 14a5 94a5 8000 14a5 94a5 8000 14a5  ................
[continues for hundreds of lines]
```

### 4. **Invalid Address Source Located**
- **`0x4a52` found at**: address `0x11ce` in compiled file (code section)
- **Context**: Part of systematic pattern generation, not individual string allocation
- **This suggests**: Bulk data generation with incorrect address calculations

## ROOT CAUSE HYPOTHESIS

**The compiler is generating systematic patterns of invalid addresses during bulk data structure creation**, likely:

1. **Property table initialization** with placeholder values that exceed file bounds
2. **String table padding** or initialization with incorrect address calculations
3. **Routine table generation** with addresses pointing beyond code space

## WHY GAMEPLAY WORKS BUT TXD FAILS

- **Gameplay**: Only accesses specific, valid strings and routines needed for the game
- **TXD**: Systematically scans ALL addresses in the file, including unused bulk data
- **Our tolerance**: Interpreter silently handles out-of-bounds during decode loops
- **TXD strict**: Fails fast when any address calculation exceeds file boundaries

## IMPLICATIONS

1. **Not a string allocation issue**: Individual strings like "a small mailbox" are allocated correctly
2. **Bulk generation problem**: Systematic patterns suggest automated generation of invalid data
3. **Hidden violations**: Hundreds of compliance violations masked by interpreter tolerance
4. **Professional impact**: Files incompatible with standard Z-Machine ecosystem

## NEXT STEPS

1. **Identify bulk data generation source**: Find where hundreds of `94a5` patterns are created
2. **Fix pattern generation logic**: Ensure all generated addresses stay within file bounds
3. **Validate bounds checking**: Add compiler-time validation for all generated addresses
4. **Test with TXD**: Ensure fixed files pass standard disassembler validation