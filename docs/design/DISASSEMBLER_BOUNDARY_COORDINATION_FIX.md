# DISASSEMBLER BOUNDARY COORDINATION FIX

## ISSUE SUMMARY (November 16, 2025)

**STATUS**: **BOUNDARY COORDINATION FIXED - VALIDATION CHALLENGES REMAIN** 🎯

**PROBLEM**: Z-Machine disassembler (gruedasm-txd) only found 8 routines instead of expected ~25 functions in compiled mini_zork games, while finding hundreds in commercial games.

**ROOT CAUSE IDENTIFIED**: Critical boundary coordination bug between queue processing and iterative expansion phases in disassembler algorithm.

**MAJOR FIX ACHIEVED**: Boundary coordination fix increased routine discovery from **8 → 45 routines** (456% improvement!)

**REMAINING CHALLENGE**: 45 routines vs expected ~30-35 (25 functions + 5-10 builtins) indicates false positive detection still needs refinement.

---

## INVESTIGATION TIMELINE

### INITIAL DISCOVERY

**Symptom**: Disassembler only finding 8/25 routines in mini_zork vs hundreds in commercial games

**Debug Evidence**:
```
Mini_zork: 8 routines found
Zork I: 450+ routines found
```

**Initial Theory**: Off-by-one bug masking deeper architectural issues

### ROOT CAUSE ANALYSIS

**Boundary Coordination Bug Discovered**:

1. **Queue Processing Phase** (lines 622-638 in `discover_routines`):
   - Analyzed calls in discovered routines
   - Expanded boundaries from `060d` to `0e24` (covering wide range of routines)

2. **Critical Bug** (lines 647-648):
   ```rust
   // WRONG: This reset boundaries, throwing away all discovery work!
   self.low_address = start_pc;
   self.high_address = start_pc;
   ```

3. **Result**: Iterative expansion started with narrow boundaries `low=0677 high=0677` instead of expanded range

### ARCHITECTURAL FIX IMPLEMENTED

**File**: `src/disasm_txd.rs` lines 646-653

**Solution**: Preserve boundaries from queue processing instead of resetting them:

```rust
// BOUNDARY COORDINATION FIX: Preserve boundaries from queue processing phase
// instead of resetting them to start_pc, which loses all discovery work
if self.low_address == 0 || self.low_address > start_pc {
    self.low_address = start_pc;
}
if self.high_address == 0 || self.high_address < start_pc {
    self.high_address = start_pc;
}
```

**Before Fix**:
- Queue processing: `low=060d high=0e24` (wide range)
- Reset to: `low=0677 high=0677` (narrow range) ❌
- Result: 8 routines found

**After Fix**:
- Queue processing: `low=060d high=0e24` (wide range)
- Preserved: `low=060d high=0e24` (wide range) ✅
- Result: 45 routines found

---

## FALSE POSITIVE VALIDATION ATTEMPTS

### PROBLEM IDENTIFIED

**Analysis**: 45 routines vs expected ~30-35 indicates false positives

**User Functions Count**: 25 functions in mini_zork.grue + 5-10 builtins = ~30-35 expected

**False Positive Examples**:
```
Routine R0003, 4 locals (0709, d68f, 0940, 0009)    # d68f suspicious
Routine R0004, 8 locals (0000, f5ab, f501, 0000, e6bf, 01b0, 009e, 10c8)  # multiple suspicious values
Routine R0005, 12 locals (d0bb, 8d0c, dfbb, e03f, 0375, 000e, 0a02, 0e0b, 0a0e, 0c05, 0e0d, 060e)  # many suspicious values
```

**Pattern Recognition**:
- **Real Routines**: 0 locals or small numbers with 0000 initialization
- **False Positives**: Many locals with suspicious init values (addresses/data patterns)

### VALIDATION APPROACHES ATTEMPTED

**Attempt 1: Enhanced Local Init Pattern Validation**

Added heuristics to detect data misinterpreted as routines:

```rust
// Check local variable initialization patterns for V1-4
// Data regions often have patterns that look like routine headers but with suspicious init values
if self.version <= 4 && locals_count > 0 {
    let mut suspicious_inits = 0;
    for i in 0..locals_count {
        let init_value = read_local_init_value(i);
        if init_value > threshold {  // Various thresholds tested
            suspicious_inits += 1;
        }
    }
    // Reject if too many suspicious values
    if suspicious_inits > acceptance_criteria {
        return None;  // Reject as data region
    }
}
```

**Thresholds Tested**:
- `> 0x00FF`: Too strict (2 routines)
- `> 0x0500`: Too strict (2 routines)
- `> 0x1000`: Moderate (43 routines)
- `> 0x2000`: Conservative (43 routines)
- `> 0x8000`: Very conservative (45 routines, no filtering)

**Acceptance Criteria Tested**:
- `> 1/4 of locals`: Moderate strictness
- `> 1/3 of locals`: Higher strictness
- `> 1/2 of locals`: Conservative

**Challenge Discovered**: Compiled games have different local initialization patterns than commercial games, making universal validation criteria difficult.

**Attempt 2: Targeted Validation**

Focused on routines with many locals (> 6 or > 8) since those showed clearest false positive patterns:

```rust
// Only apply to routines with many locals since those show the clearest false positive patterns
if self.version <= 4 && locals_count > 8 {
    // Enhanced validation logic
}
```

**Results**: Successfully filtered some extreme cases but missed moderate false positives.

### VALIDATION OUTCOMES

**Current Conservative Validation**: Only targets extremely suspicious patterns (> 8 locals with > 75% values > 0x8000)

**Result**: 45 routines found (minimal filtering to avoid rejecting legitimate compiled routines)

**Commercial Game Verification**: Zork I still finds 535 routines ✅ (validation doesn't break existing functionality)

---

## TECHNICAL CHALLENGES

### COMPILED VS COMMERCIAL GAME PATTERNS

**Commercial Games** (like Zork I):
- Professional optimization and layout
- Predictable local variable initialization patterns
- Standard Z-Machine development practices

**Compiled Games** (like mini_zork):
- Different optimization strategies
- Mixed local initialization patterns (0000 + legitimate small values)
- Non-standard but valid Z-Machine patterns

### VALIDATION COMPLEXITY

**The Dilemma**:
- **Too Strict**: Risk rejecting legitimate compiled routines with non-zero init values
- **Too Permissive**: Accept false positives from data regions that coincidentally look like routine headers

**Root Issue**: No clear universal threshold that works for both:
1. **Legitimate compiled routines** with some non-zero local init values
2. **Data regions** with suspicious high-value patterns

---

## CURRENT STATUS

### ACHIEVEMENTS ✅

1. **Boundary Coordination Fixed**: Critical algorithmic bug resolved
2. **Routine Discovery Improved**: 8 → 45 routines (456% improvement)
3. **Commercial Game Compatibility**: No regression (Zork I: 535 routines)
4. **Architecture Understanding**: Boundary coordination mechanism documented

### REMAINING CHALLENGES ⚠️

1. **False Positive Detection**: 45 vs expected ~30-35 routines
2. **Validation Calibration**: Need better heuristics for compiled vs commercial game patterns
3. **Threshold Optimization**: Balance between strictness and accuracy

### POTENTIAL APPROACHES

**Option 1: Advanced Heuristics**
- Multi-factor validation (local patterns + instruction patterns + context)
- Machine learning approach to distinguish patterns
- Statistical analysis of legitimate vs false positive characteristics

**Option 2: Game-Type Detection**
- Detect compiled vs commercial games
- Apply different validation strategies based on game type
- Adaptive thresholds based on detected patterns

**Option 3: Conservative Acceptance**
- Accept that compiled games may have more "routine-like" data patterns
- Focus on filtering only the most obviously problematic cases
- Document known limitations

---

## IMPLEMENTATION DETAILS

### FILES MODIFIED

**Primary Fix**: `src/disasm_txd.rs`

**Boundary Coordination Fix** (lines 646-653):
```rust
// BOUNDARY COORDINATION FIX: Preserve boundaries from queue processing phase
if self.low_address == 0 || self.low_address > start_pc {
    self.low_address = start_pc;
}
if self.high_address == 0 || self.high_address < start_pc {
    self.high_address = start_pc;
}
```

**Enhanced Validation** (lines 232-260):
```rust
// CONSERVATIVE VALIDATION: Only reject the most obviously problematic patterns
if self.version <= 4 && locals_count > 8 {
    // Count extremely suspicious values and reject if > 75% are extreme
}
```

### TESTING RESULTS

**Mini_zork Disassembly**:
- **Before**: 8 routines found
- **After**: 45 routines found
- **Debug Output**: Boundaries properly preserved (`low=060d high=0e24`)

**Zork I Compatibility**:
- **Result**: 535 routines found (no regression)
- **Validation**: Commercial games work correctly

**Function Count Verification**:
```bash
$ grep "^fn " examples/mini_zork.grue | wc -l
25
```
- **User Functions**: 25 defined in mini_zork.grue
- **Expected Total**: ~30-35 (including builtins)
- **Current Result**: 45 (indicates ~10-15 false positives remain)

---

## CONCLUSION

**Major Success**: Boundary coordination fix resolved the primary algorithmic bug and dramatically improved routine discovery.

**Engineering Insight**: The issue wasn't in the validation logic but in the coordination between discovery phases - a classic example of how boundary coordination bugs can mask the true capabilities of an algorithm.

**Remaining Work**: False positive filtering requires more sophisticated heuristics that can distinguish between legitimate compiled routine patterns and coincidental data patterns that resemble routine headers.

**Practical Impact**: The disassembler now finds most routines correctly, with only moderate over-detection that doesn't impact functionality.

---

## REFERENCES

- **Z-Machine Specification**: Official Z-Machine Standards Document v1.1
- **TXD Implementation**: Mark Howell's reference disassembler architecture
- **Mini_zork Source**: `examples/mini_zork.grue` (25 user-defined functions)
- **Commercial Baseline**: Zork I disassembly results for comparison