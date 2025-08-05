# TXD Orphan Detection Analysis

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

## Implementation Plan

To fix without regressing V3:

1. **Add orphan detection to preliminary scan**
   - Check pcindex after validation in backward scan
   - Don't add routine if pcindex > 0

2. **Ensure proper pcindex tracking**
   - Currently we set pcindex = 0 but never increment it
   - Need to implement the actual orphan fragment detection

3. **Test carefully**
   - V3 must maintain 440+ routines
   - V4 should remove false positives like caf8, cafc, 33c04
   - Watch for any regression in routine discovery

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