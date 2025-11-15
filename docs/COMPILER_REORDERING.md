# Z-MACHINE MEMORY LAYOUT REORDERING PLAN

**Date**: November 15, 2025
**Issue**: Disassembler functionality failure due to non-standard Z-Machine memory layout
**Root Cause**: Our compiler uses different layout than commercial Z-Machine files

## PROBLEM ANALYSIS

### Current vs. Standard Layout

**Current Layout (Our Compiler)**:
```
Header → Globals → Arrays → Abbreviations → Objects → Dictionary → **Strings** → Code
                                                         ^                     ^
                                                   Static boundary      2777-byte gap
```

**Standard Layout (Commercial Games)**:
```
Header → Abbreviations → Objects → Globals → Arrays → Grammar → Actions → Dictionary → **Code**
                                                                               ^         ^
                                                                         Static boundary  ~300-byte gap
```

### The Dynamic Memory Boundary Issue

**Z-Machine Specification Requirements**:
- **Dynamic Memory**: Bytes 0x0000 to the address stored at header word 0x0e (can be read/written by game)
- **Static Memory**: Immediately follows dynamic memory (read-only)
- **High Memory**: Contains routines and strings (accessed only via opcodes like print_paddr)

**Critical Constraint**: Objects and properties **MUST be in dynamic memory** for `put_prop`/`insert_obj` to work.

**Current Problem**:
- Our static memory starts at **0x0796** (dictionary)
- We put strings in static memory between dictionary and code (correct placement, wrong order)
- Creates massive 2777-byte gap that breaks disassembler assumptions
- Commercial games have ~300-byte gaps due to abbreviation compression

## SOLUTION STRATEGY

**Target Layout** (following commercial standard):
```
┌─ DYNAMIC MEMORY ─────────────────────────┐
│ 0x0000: Header (64 bytes)                │
│ 0x0040: Global variables                 │
│ 0x0220: Arrays                           │
│ 0x02xx: Abbreviations table (TODO)       │
│ 0x03xx: Object table + property tables   │
│ Static boundary here ────────────────────┤
├─ STATIC MEMORY ──────────────────────────┤
│ 0x0xxx: Dictionary                       │
│ 0x0xxx: Grammar tables (if any)          │
├─ HIGH MEMORY ────────────────────────────┤
│ 0x1xxx: Executable code (routines)       │
│ 0x2xxx: Encoded strings                  │
└───────────────────────────────────────────┘
```

**Key Changes**:
1. **Move strings after code** (eliminate gap between dictionary and code)
2. **Keep objects in dynamic memory** (preserve mutability)
3. **Follow commercial layout patterns** (fix disassembler compatibility)

## TECHNICAL ANALYSIS

### No Fixup Dependencies Found

**Investigation Result**: The reference resolution system is **completely address-agnostic**!
- String addresses calculated dynamically in `assemble_complete_zmachine_image()`
- References resolved in `resolve_all_addresses()` **after** layout finalized
- **No dependency** on strings being placed after code - resolver works with **any layout**

### Why Layout Changed

**Hypothesis**: Layout changed for memory boundary reasons, not fixup dependencies:
- Dynamic vs. static memory boundaries
- Ensuring properties stay in dynamic memory (line 1049: "MUST be in dynamic memory for put_prop to work")
- String alignment requirements (V3: even addresses, V4+: 4-byte alignment)

### Commercial Game Comparison

**Standard TXD Results**:
- **Zork I**: 438 routines found ✅
- **Our gruedasm-txd**: 449 routines found ✅ (+11 more, working correctly)

**Our Files**:
- **Standard txd**: CRASHED ❌ (header serial number misinterpretation)
- **Our gruedasm-txd**: 1 routine found ❌ (should be 25 functions)

**Conclusion**: Both disassemblers expect standard layout where code starts shortly after dictionary.

## PHASED IMPLEMENTATION PLAN

### PHASE 0: PREPARATION & CLEANUP

**Step 0.1: Clean up and commit current state**
```bash
cargo fmt
git add -A
git commit -m "feat: Document Z-Machine memory layout investigation and reordering plan"
```

**Step 0.2: Create layout testing infrastructure**
- Extend disassembler test scripts to validate routine counts
- Add header validation tests for memory boundaries
- Create layout comparison tools vs. commercial games

### PHASE 1: IMPLEMENT STANDARD LAYOUT (HIGH PRIORITY)

**Step 1.1: Modify `assemble_complete_zmachine_image()` in `codegen_image.rs`**
```rust
// NEW ORDER:
// Dynamic: Header → Globals → Arrays → Objects → (set static boundary)
// Static:  Dictionary → Grammar
// High:    Code → Strings
```

**Step 1.2: Update header field generation**
- Ensure header 0x0e points to dictionary (start of static memory)
- Validate dynamic memory contains only writable structures
- Update high memory mark (header 0x04) to point to code start

**Step 1.3: Regression testing**
- Verify mini_zork still compiles and runs correctly
- Ensure all 25 functions are preserved
- Test object manipulation still works (`put_prop`, `insert_obj`)

### PHASE 2: OPTIMIZE DYNAMIC MEMORY SIZE (MEDIUM PRIORITY)

**Step 2.1: Implement abbreviation system**
- Add abbreviation table generation for common strings
- Compress strings using abbreviation references
- This will reduce string section size dramatically

**Step 2.2: Optimize object/property layout**
- Pack object properties more efficiently
- Remove unnecessary padding in dynamic memory structures

### PHASE 3: VERIFY DISASSEMBLER COMPATIBILITY (VALIDATION)

**Step 3.1: Test disassembler functionality**
- Run our test scripts: `scripts/test_disassembler_mini_zork.sh`
- Verify all 25 routines are detected properly
- Compare routine count with mini_zork.grue function count

**Step 3.2: Cross-validation with standard tools**
- Verify standard txd works without crashes
- Compare our disassembler output with txd output
- Ensure routine detection matches between tools

### PHASE 4: COMPREHENSIVE TESTING (QUALITY ASSURANCE)

**Step 4.1: Game functionality testing**
- Test all mini_zork gameplay scenarios
- Verify object system, exits, inventory all work
- Run comprehensive test suite

**Step 4.2: Commercial game compatibility**
- Test layout matches Zork I structure patterns
- Verify header fields align with commercial standards
- Validate packed address calculations

## TECHNICAL IMPLEMENTATION DETAILS

### Code Changes Required

**1. `codegen_image.rs:assemble_complete_zmachine_image()`**
- Reorder layout calculation: `Header → Globals → Objects → Dictionary → Code → Strings`
- Update static memory boundary to start at dictionary
- Move string_base calculation after code_base

**2. `codegen.rs:layout_memory_structures()`**
- Adjust address planning to match new layout
- Ensure objects remain in dynamic memory for mutability

**3. Header field updates in `codegen_headers.rs`**
- Header 0x0e: Point to dictionary (static memory start)
- Header 0x04: Point to code (high memory start)
- Verify all address calculations use new layout

### Testing Strategy

**Regression Tests**:
```bash
# Test 1: Compilation still works
cargo run --bin grue-compiler examples/mini_zork.grue -o tests/mini_zork_new_layout.z3

# Test 2: Game functionality intact
printf "look\ninventory\nquit\n" | ./target/release/gruesome tests/mini_zork_new_layout.z3

# Test 3: Disassembler finds all routines
./scripts/test_disassembler_mini_zork.sh

# Test 4: Function count verification
grep -c "^fn " examples/mini_zork.grue  # Should be 25
./target/release/gruedasm-txd tests/mini_zork_new_layout.z3 | grep -c "^Routine "  # Should be 25
```

**Success Criteria**:
- ✅ All 25 functions detected by disassembler
- ✅ Mini_zork gameplay unchanged
- ✅ Standard txd doesn't crash on our files
- ✅ Memory layout matches commercial Z-Machine pattern
- ✅ Static/dynamic boundaries correct per Z-Machine spec

## RISK ANALYSIS

**Low Risk**: Reference resolution system is address-agnostic, no fixup dependencies found.

**Mitigation**: Comprehensive regression testing at each phase to ensure functionality preservation.

**Rollback Plan**: Git commit before each phase allows immediate rollback if issues discovered.

## EXPECTED OUTCOME

**Primary Goal**: Fix disassembler compatibility by following standard Z-Machine layout.

**Secondary Benefits**:
- Better compatibility with standard Z-Machine tools
- Reduced file sizes through abbreviation compression
- More professional Z-Machine file structure

**Impact**: Disassembler will correctly find all 25 routines, enabling proper compiler output validation and debugging capabilities.