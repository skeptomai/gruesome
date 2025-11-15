# Layout Reordering Baseline Summary

**Generated**: Sat Nov 15 07:09:44 PST 2025
**Purpose**: Establish baseline measurements before layout reordering
**File**: mini_zork_baseline.z3

## Current State Analysis

| Metric | Current Value | Expected After Reordering |
|--------|---------------|---------------------------|
| Source Functions | 25 | 25 (unchanged) |
| Detected Routines | 1 | 25 (fixed) |
| File Size | 8550 bytes | ~8550 bytes (similar) |
| Game Functional | true | true (preserved) |
| Dictionary-Code Gap | 368 bytes | ~300 bytes (standard) |

## Issues Identified

❌ **Disassembler Issue**: Only 1 routines detected, should be 25
   - **Cause**: Non-standard memory layout with large dictionary-code gap
   - **Fix**: Reorder layout to match commercial Z-Machine standard
✅ **Game Functionality**: Working correctly with our interpreter

## Reordering Plan Validation

**Current Layout**: Header → Globals → Objects → Dictionary → Strings → Code
**Target Layout**: Header → Globals → Objects → Dictionary → Code → Strings

**Success Criteria for Reordering**:
- ✅ Source functions: 25 (must be preserved)
- ❌ Detected routines: 25 (must reach this target)
- ✅ Game functionality: Must remain working
- ✅ File size: Should remain similar (~8550 bytes)
- ✅ Dictionary-code gap: Should reduce to ~300 bytes
