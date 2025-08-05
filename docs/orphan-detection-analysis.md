# TXD Orphan Detection Analysis

## Session Restart Context (August 2025)

This document captures the deep analysis of TXD's orphan detection mechanism and our implementation status. If restarting work on this feature, this provides complete context.

### Current Status (Final Resolution)
- **Root Cause Found**: False positives were due to accepting invalid Long form opcode 0x00
- **Solution**: Added proper opcode validation in instruction decoder
- **V3 Games**: 451 routines (slight increase from 449, TXD finds 440) ✓
- **V4 Games**: 1010 routines (TXD finds 982) - difference appears to be valid routines
- **False Positives**: Fixed! Invalid addresses like 33c04 (all zeros) now properly rejected

### The Real Problem
The false positives weren't about orphan fragments or fallthrough detection. They were simply invalid instructions that our decoder was incorrectly accepting. Long form opcode 0x00 is not valid according to the Z-Machine specification (Long form opcodes start at 0x01).

### What's Implemented
1. **Orphan detection flag**: `enable_orphan_detection()` - opt-in feature
2. **pctable tracking**: Stores orphan fragment addresses
3. **pcindex increment**: Properly increments when orphans detected
4. **Fallthrough detection**: `is_reachable_by_fallthrough()` method
5. **Post-discovery filtering**: Removes orphans from final routine list

### The Problem
Our current fallthrough detection is too simple. It marks ANY routine reachable by fallthrough as an orphan, but many valid routines can be reached this way. We need more sophisticated criteria to distinguish true orphan fragments from valid routines.

### Key Implementation Files
- `src/disasm_txd.rs`: Main disassembler with orphan detection foundation
- `src/bin/test_orphan_v4.rs`: Tests orphan detection effectiveness
- `src/bin/test_v3_regression.rs`: Ensures no V3 regression
- `src/bin/analyze_false_positives.rs`: Analyzes suspect addresses
- `src/bin/check_fallthrough.rs`: Detects orphan fragments

## Summary of Investigation

After deep analysis of TXD's orphan detection mechanism, I've uncovered how it works and why our implementation isn't catching the false positives.

## How TXD's Orphan Detection Works

### 1. Two-Pass Structure
- **First Pass** (`first_pass = 1`): Discovery phase - finds routines and builds orphan table
- **Second Pass** (`first_pass = 0`): Output phase - skips addresses in orphan table

### 2. Orphan Detection Mechanism (decode_routine lines 44-58)
```c
decode.pc = old_pc;
if ((status = decode_code()) != END_OF_ROUTINE) {
    decode.pc = old_pc;  // Not valid code
} else {
    pctable[pcindex++] = old_pc;  // Mark as orphan fragment
}
```

**Key Insight**: An orphan fragment is code that:
- Can decode successfully to END_OF_ROUTINE
- But doesn't have a valid routine header (or failed triple validation)
- Is tracked in `pctable[]` array with `pcindex` counter

### 3. pcindex Usage
- Reset to 0 before each validation attempt
- Incremented when orphan fragments are found
- During validation: `if (decode_routine() != END_OF_ROUTINE || pcindex)` - reject if pcindex > 0

## Implementation Progress Update

### What's Working
- Fallthrough detection correctly identifies addresses reachable by falling through
- pcindex is now properly incremented when orphan fragments are detected
- False positives caf8, cafc, 33c04 are correctly removed
- V3 games are unaffected (449 routines maintained)

### Current Issue
- Being too aggressive: removing 474 addresses instead of just 35
- From 624 to 406 routines (should be ~589 after removing 35 false positives)
- Need more sophisticated heuristics to distinguish true orphans from valid routines

### Key Insight
The simple "is reachable by fallthrough" test is insufficient. Many valid routines can be reached by fallthrough but are still legitimate entry points. Need additional criteria like:
- Check if the "falling through" instruction is actually executed
- Consider if the routine is called from elsewhere
- Look at the pattern of instructions before the routine

## Why Our Implementation Doesn't Catch False Positives

### Problem 1: Orphan Detection Not Called
- Our false positives (caf8, cafc, 33c04) are found during **preliminary scan**
- They're added directly via `add_routine()` in the backward scan
- The orphan detection logic in `txd_triple_validation()` is never called for these addresses

### Problem 2: Missing pcindex Check in Preliminary Scan
TXD's preliminary scan (lines 413-419):
```c
flag = 1;
pcindex = 0;
pc = ROUND_CODE(decode.pc);
if (decode_routine() != END_OF_ROUTINE || pcindex)
    flag = 0;
```

Our implementation doesn't check pcindex during the preliminary scan.

### Problem 3: Incomplete Implementation
- We added orphan detection to `txd_triple_validation()`
- But preliminary scan bypasses this by calling `add_routine()` directly
- Need to add orphan checking to the preliminary scan path

## Current State

### V3 Games (Zork I)
- Finding 449 routines (TXD finds 440)
- No regression with orphan detection enabled
- Working well as a strict superset

### V4 Games (AMFV)
- Finding 624 routines (TXD finds 982)
- 35 false positives including caf8, cafc, 33c04
- Orphan detection implemented but not triggering for these cases

## Resolution Summary

The false positive issue has been resolved through proper opcode validation rather than complex orphan detection:

1. **Root Cause**: Instruction decoder was accepting invalid Long form opcode 0x00
2. **Fix**: Added validation to reject Long form 0x00 (per Z-Machine spec)
3. **Results**: 
   - False positives like 33c04 (data regions) now properly rejected
   - No regression in V3 games
   - V4 games show we find more routines than TXD, but these appear to be valid

The complex orphan detection mechanism was not needed for this issue. The simpler solution of proper opcode validation was sufficient.

## Lessons Learned

### 1. Start with Spec Compliance
Before implementing complex heuristics or mimicking undocumented behavior, ensure basic specification compliance. The Z-Machine spec clearly states Long form opcodes start at 0x01, which we were violating.

### 2. Validate Root Causes
Our initial analysis correctly identified the false positives (33c04, caf8, cafc) but misdiagnosed the cause. We assumed it was about orphan fragments when it was simply invalid opcodes.

### 3. Simple Solutions First
We spent significant effort implementing:
- Orphan fragment detection
- Fallthrough analysis
- pcindex tracking
- Two-pass filtering

When the actual fix was a 3-line validation check in the instruction decoder.

### 4. Test Invalid Data
The false positive at 33c04 was particularly instructive - it's a region of all zeros. Our decoder was happily "decoding" zeros as Long form opcode 0x00, which doesn't exist.

### 5. Remaining Differences May Be Valid
After fixing the false positives, we find MORE routines than TXD (1010 vs 982). This suggests our scanner is more thorough, not that we have more false positives.

## Analysis of Extra Routines (August 2025)

After further investigation into why we find 28 more routines than TXD for AMFV (after fixing alternate entry points):

### Key Findings

1. **Alternate Entry Points**: Only 1 found (0cafc inside 0caf4's header), not 7 as initially thought
2. **Nested Routine Pattern**: TXD includes nested routines when it also finds the parent
   - When TXD finds both parent and nested: Both are included (e.g., d198/d184, d6f4/d6e8)
   - When TXD finds neither: Both are excluded (e.g., e6f8/e6e8, 25564/25550)
3. **TXD's Consistency**: It doesn't simply reject all nested routines - it's more nuanced

### Nested Routine Analysis

From our 6 "nested in code body" cases:
- **4 are in TXD's list**: d198, d6f4, e96c, 2b3b4 (all have their parents too)
- **2 are NOT in TXD's list**: e6f8, 25564 (neither have their parents)

This suggests TXD has additional validation that rejects both parent and nested routine together when they don't meet certain criteria.

### Current Status After Fixes

1. **Initial count**: 1010 routines (before alternate entry fix)
2. **After rejecting alternate entries**: 1009 routines (removed 0cafc)
3. **TXD finds**: 982 routines
4. **Remaining difference**: 27 routines

### Updated Pattern Categories

The extra routines we find likely fall into:
- **Uncalled routines** that TXD rejects (including parent/nested pairs like e6e8/e6f8)
- **Invalid opcodes** that we still accept somewhere
- **Code fragments** without proper termination
- **Very short routines** that TXD considers suspicious

### Key Insight: Alternate Entry Points vs Nested Routines

- **Alternate Entry Point**: Starts inside another routine's header/locals area (like 0cafc at offset +8 into 9-byte header)
- **Nested Routine**: Starts inside another routine's code body (like d198 at offset +20, well past 7-byte header)

Only alternate entry points should be rejected as they represent scanner confusion about local variable storage. Nested routines in code bodies may be legitimate (as evidenced by TXD including many of them).

### Conclusion

Our implementation now correctly rejects alternate entry points in header areas. The remaining 26-routine difference with TXD appears to be due to other validation criteria, particularly around uncalled routine pairs. TXD's approach of accepting/rejecting parent-nested pairs together suggests it may be using call graph analysis or other reachability checks.

## Risk Assessment

**Low Risk**:
- Orphan detection is behind a flag (opt-in)
- V3 games don't seem to have orphan fragments
- Changes are isolated to validation logic

**Medium Risk**:
- Might remove some edge-case valid routines
- Need careful testing with multiple games

**Mitigation**:
- Keep orphan detection optional
- Test with both V3 and V4 games
- Compare results with/without orphan detection