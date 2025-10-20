# Property 28 Debug Plan

## Problem Statement

**Issue**: Game crashes with "Property 28 not found for object 1" at runtime despite compilation logs showing Property 28 is created for the player object.

**Evidence**:
- Compilation shows: `🔍 PROP_WRITE: Writing property 28 for object 'yourself': size_byte=0x3c, data=0x0000 (0)`
- Runtime error: `Property 28 not found for object 1`
- Property 28 is `quit_pending` property for player object

## Root Cause Hypothesis

Based on our previous investigation that found the same pattern with room Property 7 corruption, this is likely the same underlying property table corruption bug affecting a different property.

**Key Similarity**: Property is created correctly during compilation but missing at runtime.

## Investigation Strategy

### Phase 1: Verify Property Creation ✅
**Status**: CONFIRMED - Compilation logs show Property 28 is created with correct encoding.

### Phase 2: Identify Runtime Property Absence 🔍
**Goal**: Verify that Property 28 is actually missing from object 1's property table at runtime.

**Method**: Use interpreter debug logging to dump object 1's property table and confirm Property 28 absence.

### Phase 3: Compare with Working Properties 🔧
**Goal**: Compare Property 28 with properties that DO work to identify corruption pattern.

**Analysis Points**:
- Does object 1 have ANY properties at runtime?
- Which properties survive and which are lost?
- Is this a complete table corruption or selective property loss?

### Phase 4: Memory Layout Investigation 🔬
**Goal**: Apply the same systematic approach that revealed the room Property 7 fix.

**Pattern from Previous Success**:
- Property creation works ✅
- Property table corruption happens post-creation ❌
- Need to identify corruption source

## Expected Investigation Timeline

1. **Phase 2**: 10 minutes - Runtime property table dump
2. **Phase 3**: 15 minutes - Pattern analysis
3. **Phase 4**: 30 minutes - Systematic corruption source identification

## Success Criteria

Fix is successful when:
1. Object 1 (player) has Property 28 at runtime
2. Game loads without "Property 28 not found" crash
3. Room descriptions continue to work (no regression)

## Files to Investigate

Primary targets:
- Object table generation in `codegen_objects.rs`
- Property table structure verification
- Post-processing steps that might corrupt properties

## Key Questions

1. **What makes Property 28 different** from properties that work?
2. **Is this player-specific** or could it affect other objects?
3. **When exactly does the corruption happen** in the compilation pipeline?

## Current Investigation Status

**READY FOR PHASE 2**: Begin runtime property table analysis to confirm Property 28 absence and identify corruption scope.