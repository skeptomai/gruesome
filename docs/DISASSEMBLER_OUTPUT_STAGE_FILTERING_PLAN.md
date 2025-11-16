# DISASSEMBLER OUTPUT-STAGE FILTERING IMPLEMENTATION PLAN

**Status**: Ready for Implementation
**Date**: November 16, 2025
**Objective**: Move all filtering to final disassembly output stage to solve discovery vs. filtering conflict

## **Problem Statement**

Current disassembler filtering architecture has a fundamental design flaw:

- **Discovery Phase**: Multiple paths add routines to `self.routines` HashMap
- **Current Filtering**: Applied during discovery via `add_routine()` and `txd_triple_validation()`
- **Critical Issue**: Aggressive filtering prevents legitimate routine discovery during boundary expansion
- **Result**: Boundary expansion stops prematurely (e.g., `low=060d high=0677` instead of full range)

### **Evidence of the Problem**

- **Without filtering**: 45 routines found (boundary expansion works)
- **With aggressive header filtering**: 2 routines found (discovery broken)
- **Multiple discovery paths**: 9+ different code paths call `add_routine()` directly, bypassing validation

## **Current Architecture Analysis**

### **Discovery Phase Issues**
Multiple paths add routines to `self.routines` HashMap:
- `add_routine()` calls from 9+ different discovery paths (lines 860, 898, 959, 1011, 1771, 1852, 2241, 2380, 2588)
- Some go through `txd_triple_validation()`, others bypass it entirely
- Current filtering happens during discovery, breaking boundary expansion when legitimate routines get rejected

### **Output Phase Location**
Lines 2120-2123 in `src/disasm_txd.rs`:
```rust
let mut sorted_routines: Vec<_> = self.routines.iter().collect();
sorted_routines.sort_by_key(|(addr, _)| *addr);
```

## **Solution: Output-Stage Filtering Architecture**

### **Core Principle**: Clean Separation of Concerns
- **Discovery Phase**: Unimpeded routine discovery to maximize boundary expansion
- **Output Phase**: Comprehensive false positive filtering before final disassembly

## **Implementation Plan**

### **Phase 1: Remove Discovery-Time Filtering**

#### **1.1 Remove Header-Level Filtering from `add_routine()`**
**Location**: `src/disasm_txd.rs` line 471
```rust
// REMOVE THIS BLOCK:
if self.is_header_false_positive(rounded_addr, locals_count) {
    debug!("🔴 ROUTINE_REJECT: {:04x} FAILED header-based false positive detection", rounded_addr);
    return;
}
```

#### **1.2 Remove Post-Validation Filtering from `txd_triple_validation()`**
**Location**: `src/disasm_txd.rs` line 1115
```rust
// REMOVE THIS BLOCK:
if self.is_likely_false_positive(rounded_addr, locals_count, routine_end_pc) {
    debug!("🔴 POST_VALIDATION: {:04x} FAILED post-decode false positive detection", rounded_addr);
    (false, routine_end_pc)
}
```

**Change to always return success for valid routines:**
```rust
(true, routine_end_pc)  // Let all validated routines pass
```

#### **1.3 Preserve Filtering Logic for Reuse**
- Keep `is_header_false_positive()` and `is_likely_false_positive()` functions
- They will be called during output filtering instead

### **Phase 2: Implement Output-Stage Filtering**

#### **2.1 Create `filter_false_positives()` Method**
**Location**: Add new method to `TxdDisassembler` impl
```rust
/// Apply comprehensive false positive filtering to discovered routines
/// Returns filtered HashMap containing only legitimate routines
fn filter_false_positives(&self) -> IndexMap<u32, RoutineInfo> {
    let mut filtered_routines = IndexMap::new();
    let mut header_fp_count = 0;
    let mut post_fp_count = 0;

    debug!("OUTPUT_FILTERING: Starting with {} discovered routines", self.routines.len());

    for (&addr, routine_info) in &self.routines {
        let mut is_false_positive = false;

        // Phase 1: Header-level filtering
        if self.is_header_false_positive(addr, routine_info.locals_count) {
            debug!("❌ OUTPUT_FP_HEADER: {:04x} REJECTED by header filtering", addr);
            header_fp_count += 1;
            is_false_positive = true;
        }

        // Phase 2: Post-validation filtering (only if passed header filtering)
        if !is_false_positive {
            // Calculate routine end address for post-validation
            let routine_end_pc = addr + 1 + (routine_info.locals_count as u32 * 2) +
                                 routine_info.instructions.len() as u32;

            if self.is_likely_false_positive(addr, routine_info.locals_count, routine_end_pc) {
                debug!("❌ OUTPUT_FP_POST: {:04x} REJECTED by post-validation filtering", addr);
                post_fp_count += 1;
                is_false_positive = true;
            }
        }

        if !is_false_positive {
            debug!("✅ OUTPUT_FP_PASS: {:04x} ACCEPTED as legitimate routine", addr);
            filtered_routines.insert(addr, routine_info.clone());
        }
    }

    debug!("OUTPUT_FILTERING: Removed {} header FP + {} post FP = {} total false positives",
           header_fp_count, post_fp_count, header_fp_count + post_fp_count);
    debug!("OUTPUT_FILTERING: Final legitimate routines: {}", filtered_routines.len());

    filtered_routines
}
```

#### **2.2 Insert Filtering Before Output Generation**
**Location**: `src/disasm_txd.rs` around line 2120
```rust
// REPLACE THIS:
let mut sorted_routines: Vec<_> = self.routines.iter().collect();

// WITH THIS:
// Apply final filtering to remove false positives
let filtered_routines = self.filter_false_positives();
let mut sorted_routines: Vec<_> = filtered_routines.iter().collect();
```

### **Phase 3: Enhanced Filtering Logic**

#### **3.1 Two-Pass Filtering Approach**
- **Pass 1**: Header-level filtering with aggressive threshold (20-30%)
- **Pass 2**: Post-validation filtering (routine length, instruction patterns, etc.)

#### **3.2 Comprehensive Validation**
- Apply filtering to ALL routines regardless of discovery path
- Log detailed reasons for each rejection
- Provide filtering statistics in debug output

#### **3.3 Tunable Filtering Parameters**
```rust
// Configurable thresholds for easy tuning
const HEADER_FP_EXTREME_THRESHOLD: u32 = 25; // % of locals with extreme values
const POST_FP_MIN_INSTRUCTION_BYTES: u32 = 4; // Minimum legitimate routine size
const POST_FP_MAX_ROUTINE_LENGTH: u32 = 1000; // Maximum reasonable routine size
```

## **Expected Results**

### **Discovery Phase Results**
- **Routine Discovery**: Find full ~45 routines (boundary expansion works correctly)
- **Boundary Expansion**: Properly expand from initial `060d-0677` to full code range
- **No Discovery Interference**: All legitimate routines discovered regardless of filtering

### **Filtering Phase Results**
- **Header-Level Filtering**: Remove ~5-10 obvious false positives with extreme local patterns
- **Post-Validation Filtering**: Remove ~5-10 additional false positives with suspicious characteristics
- **Final Output**: ~25-30 legitimate routines in final disassembly

### **Architecture Benefits**
- **Clean Separation**: Discovery and filtering phases completely independent
- **Maintainability**: Easy to tune filtering without affecting discovery
- **Debugging**: Clear separation makes issues easier to diagnose
- **Performance**: No filtering overhead during boundary expansion critical path

## **Implementation Steps**

### **Step 1: Discovery Phase Cleanup**
1. Remove `is_header_false_positive()` call from `add_routine()`
2. Remove `is_likely_false_positive()` call from `txd_triple_validation()`
3. Test discovery phase: Should find ~45 routines

### **Step 2: Output Stage Implementation**
1. Implement `filter_false_positives()` method
2. Insert filtering call before output generation
3. Test filtering: Should reduce to ~25-30 routines

### **Step 3: Threshold Tuning**
1. Test on mini_zork compiled game
2. Verify legitimate routines are preserved
3. Adjust thresholds to optimize false positive detection

### **Step 4: Validation**
1. Compare results with commercial game disassembly
2. Verify no legitimate routines lost
3. Confirm false positive reduction effectiveness

## **Testing Protocol**

### **Discovery Phase Testing**
```bash
RUST_LOG=debug ./target/release/gruedasm-txd tests/mini_zork_fixed_validation.z3 2>&1 | grep "TXD_DISCOVERY_COMPLETE"
# Expected: "TXD_DISCOVERY_COMPLETE: 45 routines found"
```

### **Filtering Phase Testing**
```bash
RUST_LOG=debug ./target/release/gruedasm-txd tests/mini_zork_fixed_validation.z3 2>&1 | grep "OUTPUT_FILTERING"
# Expected: Detailed filtering statistics and ~25-30 final routines
```

### **Final Output Testing**
```bash
./target/release/gruedasm-txd tests/mini_zork_fixed_validation.z3 | grep "Routine R" | wc -l
# Expected: ~25-30 legitimate routines
```

## **Rollback Plan**

If output-stage filtering proves problematic:

1. **Immediate Rollback**: Restore current filtering approach with conservative thresholds
2. **Alternative Approach**: Implement selective filtering (apply only to specific discovery paths)
3. **Fallback Option**: Disable filtering entirely and accept some false positives

## **Success Criteria**

- ✅ Discovery finds ~45 total routines (boundary expansion working)
- ✅ Filtering removes ~15-20 false positives
- ✅ Final output contains ~25-30 legitimate routines
- ✅ No regression in legitimate routine detection
- ✅ Clean architectural separation maintained

## **Next Steps**

1. **Implement Phase 1**: Remove discovery-time filtering
2. **Test Discovery**: Verify 45 routines found
3. **Implement Phase 2**: Add output-stage filtering
4. **Test Filtering**: Verify ~25-30 final routines
5. **Document Results**: Update ONGOING_TASKS.md with outcomes