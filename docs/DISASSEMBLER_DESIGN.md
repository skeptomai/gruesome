# Z-Machine Disassembler Design Plan

## Overview

This document outlines the design for a comprehensive Z-Machine disassembler based on analysis of the reference `txd.c` implementation. The goal is to create a Rust disassembler that matches TXD's output exactly and follows the same proven discovery algorithms.

## Key Insights from TXD Analysis

### 1. Two-Phase Discovery Process
TXD uses a sophisticated multi-stage approach:
- **Phase 1**: Analyze and validate potential routine starts by checking routine headers (variable count 0-15)
- **Phase 2**: Iteratively expand boundaries by following routine calls and discovering new routines
- Each phase uses different validation strategies for robustness

### 2. Routine Discovery Strategy
- Start from initial PC and scan through memory for valid routine headers
- Routines must start on even addresses (for v3) and have valid variable counts (0-15) 
- Follow CALL instruction operands to discover additional routines
- Expand low/high boundaries iteratively until no new routines found

### 3. Validation Process
- Check routine header (variable count must be 0-15)
- Attempt to decode entire routine to find proper return instruction
- Track discovered routine addresses to avoid duplicates

### 4. Debug Output Pattern
TXD provides extensive debug output showing:
- `TXD_VALIDATE`: Routine validation attempts
- `TXD_ADD_ROUTINE`: When routines are added to discovery list
- `TXD_PHASE2_START`: Beginning of iterative boundary expansion
- `TXD_ITERATION`: Each iteration of boundary expansion
- `TXD_SCAN`: Individual routine scanning attempts
- `TXD_OPERAND`: Call target analysis and boundary expansion

## Top-Level Implementation Tasks

### 1. Create Multi-Stage Routine Discovery System
- Implement boundary-driven discovery similar to txd.c
- Add debug logging to match TXD's debug output format exactly
- Track low/high address boundaries and expand iteratively
- Handle the complex scanning logic for discovering routines

### 2. Implement Routine Validation
- Validate routine headers (variable count 0-15)
- Decode routines completely to verify they end with return instructions  
- Handle edge cases and invalid routines gracefully
- Implement the same validation heuristics as TXD

### 3. Add Call Target Analysis
- Parse CALL, CALL_1S, CALL_2S, CALL_VS, etc. operands to find routine addresses
- Expand discovery boundaries when new routine addresses found
- Handle packed address calculation for different Z-machine versions
- Track operand analysis for boundary expansion decisions

### 4. Output Format Matching
- Match TXD's output format exactly for comparison
- Include routine headers, local variable counts, and instruction formatting
- Add routine numbering and label generation
- Handle text strings and data sections properly

### 5. Testing and Validation
- Compare output against TXD on Zork 1 and other test files
- Ensure all routines discovered by TXD are also found by Rust implementation
- Validate instruction decoding accuracy
- Use iterative development with frequent TXD comparisons

### 6. Performance and Memory Management
- Efficient storage of discovered routines and labels
- Handle large game files without excessive memory usage
- Optimize boundary expansion algorithm
- Manage cross-references and routine relationships

## Implementation Strategy

### Phase 1: Core Discovery Engine
1. Create `RoutineDiscovery` struct to manage the discovery process
2. Implement boundary tracking (low_address, high_address)
3. Add routine validation logic matching TXD's approach
4. Create debug logging infrastructure

### Phase 2: Call Analysis
1. Implement operand parsing for all CALL variants
2. Add packed address unpacking for different Z-machine versions
3. Implement boundary expansion logic
4. Add iterative discovery loop

### Phase 3: Output Generation  
1. Create output formatter matching TXD's format
2. Implement routine numbering and label generation
3. Add instruction formatting with proper operand display
4. Handle text strings and data sections

### Phase 4: Testing and Refinement
1. Compare against TXD output on multiple game files
2. Fix discrepancies and edge cases
3. Optimize performance
4. Add comprehensive test suite

## Success Criteria

- **Exact Output Matching**: Disassembler output matches TXD for all test cases
- **Complete Discovery**: All routines found by TXD are also discovered by Rust implementation
- **Robust Validation**: Handles edge cases and invalid data gracefully
- **Performance**: Efficiently processes large game files
- **Maintainability**: Clean, well-documented code following Rust best practices

## Technical Notes

### Z-Machine Specifics
- Routines start on even addresses in v3 (packed address * 2)
- Routine headers contain variable count (0-15) followed by initial values for v1-4
- Return instructions: `rtrue`, `rfalse`, `ret`, `ret_popped`
- CALL variants have different operand patterns requiring careful parsing

### Development Guidelines
- Use `log::debug!` for all debug output (not `eprintln!`)
- Follow the existing codebase patterns and conventions
- Make iterative commits to allow rollback if needed
- Test frequently against TXD reference implementation
- Document all algorithmic decisions and edge cases

## Key Findings: False Positives Investigation (August 2025)

### Problem Statement
Our TXD-style disassembler finds 35 false positive routines compared to TXD for V4 games (AMFV). These false positives were introduced in commit 7741710 when implementing the preliminary low routine scan and backward scan.

### Root Cause Analysis

#### 1. Insufficient Validation
Our implementation accepts addresses as valid routines if they:
- Have a valid locals count (0-15)
- Decode to valid instructions
- Eventually reach a return instruction

However, this is insufficient. Data regions can coincidentally match these criteria.

#### 2. Missing Orphan Code Detection
TXD implements a sophisticated "orphan code fragment" detection mechanism:
- After validating a routine, TXD performs a second decode pass
- If this second decode returns END_OF_ROUTINE, it's marked as an orphan fragment
- During the first pass, any routine that creates orphan fragments (pcindex > 0) is rejected
- Orphan fragments are code reachable by falling through, not by being called

Our implementation lacks this crucial validation step.

#### 3. Specific False Positives Found
1. **caf8, cafc** - Addresses before the first valid routine (caf4)
2. **33c04-34498 cluster** - 22 consecutive addresses in what appears to be a data table
3. **Various scattered addresses** - Likely data that coincidentally decodes as valid instructions

### Example: Address 33c04
- Locals count: 0 (valid)
- First instruction: 0x00 (all zeros)
- Decoded as Long form opcode 0x00, which is NOT a valid Z-Machine instruction
- Yet our decoder accepted it because we don't validate opcode validity

### TXD's Validation Process (Detailed)
```c
// From txd.c lines 562-587
if (decode_code() == END_OF_ROUTINE) {
    // First validation passed
    return (END_OF_ROUTINE);
}
// Reset and try second decode
decode.pc = old_pc;
if ((status = decode_code()) != END_OF_ROUTINE) {
    decode.pc = old_pc;  // Not a routine
} else {
    pctable[pcindex++] = old_pc;  // Orphan fragment found
}
```

### Why Our Opcode Validation Failed
We attempted to fix false positives by validating opcodes (rejecting Long form 0x00), but this created new problems:
- Rejected some valid TXD routines
- Still accepted other false positives
- The real issue isn't invalid opcodes, but orphan code detection

### Recommendations
1. **Implement proper orphan code detection**: Track pcindex and reject routines that create orphan fragments
2. **Strict return validation**: Ensure pc > high_pc for valid returns (already implemented)
3. **Consider data vs code context**: TXD's two-pass approach helps distinguish actual routines from data

### Current Status
- V3 games: Achieve strict superset (448 vs TXD's 440 for Zork I) ✓
- V4+ games: Find 624 routines vs TXD's 982 for AMFV
  - Missing 393 routines that TXD finds
  - Have 35 false positives
- Core functionality works well, but lacks TXD's sophisticated validation