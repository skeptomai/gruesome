# Z-MACHINE COMPILER COMPLIANCE AND OPTIMIZATION ANALYSIS

**Created**: November 15, 2025
**Status**: Critical compliance issues blocking optimization efforts
**Priority**: CRITICAL - Affects all compiled files and ecosystem compatibility

## OVERVIEW

This document consolidates analysis of two critical compiler issues:
1. **Z-Machine Standard Compliance Violations** - Our files don't work with standard tools
2. **Memory Layout and Optimization Challenges** - Aggressive optimization attempts fail due to underlying compliance issues

The key insight: **Memory optimization failures are symptomatic of deeper Z-Machine specification compliance problems**.

---

## CRITICAL ANALYSIS REVISION

### Discovery Summary

**REVISED CONCLUSION**: After thorough investigation, the disassembler failures are **NOT** due to packed address generation bugs, but rather **memory layout compatibility issues** where standard tools misinterpret our file structure.

### Evidence Analysis

**Timeline Investigation**:
- Tested files compiled BEFORE recent layout changes
- **SAME disassembler failures** with old layout
- Property optimization bugs are **separate, recent issues**
- **Games work perfectly** with our interpreter across all versions

**Header Analysis**:
```
Our file:  0300 0001 1208 1272 (non-standard layout)
Zork I:    0300 0058 4e37 4f05 (commercial standard)
```

**Disassembler Debug Output**:
```
DEBUG load_cache: header.file_size=4275, file_size=8550
```
The disassembler thinks our file should be 4275 bytes (from header) but it's actually 8550 bytes.

### Root Cause Analysis - CORRECTED

#### **Memory Layout Compatibility Issue - NOT Packed Address Bug**

**The Real Problem**: Standard disassemblers expect **specific memory layout patterns** from commercial games. Our non-standard layout causes them to:

1. **Misinterpret header fields** (wrong file size expectations)
2. **Look for data in wrong memory locations**
3. **Treat random data as packed addresses** when parsing unknown regions
4. **Crash when those misinterpreted "addresses" are invalid**

#### **Evidence AGAINST Packed Address Theory**:

1. **Games work perfectly**: All functionality intact, no runtime address errors
2. **Our interpreter works**: Handles all addresses correctly during execution
3. **Timeline mismatch**: Disassembler failures predate recent optimization work
4. **Pattern mismatch**: Static analysis failure vs runtime success indicates parsing issue, not generation bug

### Three Distinct Issues Identified

#### **1. Memory Layout Compatibility** (affects disassemblers, NOT gameplay)
- **Evidence**: Standard tools expect commercial layout patterns
- **Scope**: External static analysis tools only
- **Runtime**: Zero impact - games work perfectly
- **Root cause**: Our layout differs from commercial game structure

#### **2. Property Optimization Runtime Bugs** (affects gameplay, introduced recently)
- **Evidence**: "Invalid Long form opcode 0x00" during property optimization attempts
- **Scope**: Runtime execution failures during optimization
- **Timeline**: Introduced during recent property table optimization work
- **Root cause**: Calculation errors in optimization code boundaries

#### **3. Disassembler Implementation Gaps** (affects our own tools)
- **Evidence**: Our gruedasm-txd finds fewer routines than expected
- **Scope**: Our disassembler doesn't handle our layout optimally
- **Solution**: Enhance our tools rather than change file format

---

## MEMORY LAYOUT INVESTIGATION

### Current vs. Standard Layout Analysis

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

### Technical Analysis - No Fixup Dependencies Found

**Investigation Result**: The reference resolution system is **completely address-agnostic**!
- String addresses calculated dynamically in `assemble_complete_zmachine_image()`
- References resolved in `resolve_all_addresses()` **after** layout finalized
- **No dependency** on strings being placed after code - resolver works with **any layout**

### Commercial Game Comparison

**Standard TXD Results**:
- **Zork I**: 438 routines found ✅
- **Our gruedasm-txd**: 449 routines found ✅ (+11 more, working correctly)

**Our Files**:
- **Standard txd**: CRASHED ❌ (header serial number misinterpretation)
- **Our gruedasm-txd**: 1 routine found ❌ (should be 25 functions)

**Conclusion**: Both disassemblers expect standard layout where code starts shortly after dictionary.

---

## MEMORY OPTIMIZATION ANALYSIS

### Property Table Space Estimation Challenge

During the recent memory optimization campaign, we attempted to replace hardcoded property table allocation (1000 bytes) with precise space estimation. This investigation revealed fundamental challenges that explain why aggressive memory optimization triggers the same compliance issues documented above.

### The Property Table Cross-Reference Problem

**Root Issue**: Property table optimization requires **byte-perfect** calculations across multiple interdependent memory structures:

```
Object Entry → Property Pointer → Property Table → Property Values → String References
```

Each component in this chain must be precisely positioned, or the entire reference chain breaks, causing:
- Invalid property pointers
- Corrupted instruction streams
- "Invalid Long form opcode 0x00" errors
- The same type of address calculation failures seen with packed addresses

### Circular Dependency Challenge

Property table space calculation requires knowing:
1. **Final string table addresses** (unknown during object layout phase)
2. **Property value encodings** (depend on final memory layout)
3. **Cross-object references** (depend on object numbering which depends on space allocation)
4. **Memory boundary calculations** (depend on total property space)

This creates a circular dependency:
```
Property space size → Object layout → Memory boundaries → String addresses → Property values
```

### Why Safety Margins Failed

Even 200-500 byte safety margins caused corruption because:

**1. Alignment Dependencies**: Property data has Z-Machine-specific alignment requirements that shift with any layout change

**2. Reference Resolution**: Object-to-object property references become invalid when memory layout shifts, even slightly

**3. Instruction Boundary Violations**: Code generation assumes specific memory boundaries that change with property table optimization

**4. Packed Address Recalculation**: Property optimization triggers recalculation of the same packed addresses that cause our compliance violations

### Connection to Compliance Issues

The property table optimization failures are **symptomatic of the same underlying problem**:
- **Packed address generation**: Both issues involve addresses that point beyond valid memory
- **Cross-reference integrity**: Both require precise coordination between memory sections
- **Boundary validation**: Both fail due to inadequate file size vs address calculation coordination

### Technical Analysis Results

**Attempted Optimizations**:
- Initial estimation: 1206→551 bytes (54% reduction) - **FAILED** (instruction corruption)
- Conservative margin (200 bytes): 1206→701 bytes (42% reduction) - **FAILED** (instruction corruption)
- High safety margin (500 bytes): 1206→1001 bytes (17% reduction) - **FAILED** (instruction corruption)

**Success Only With**: Hardcoded 1000-byte allocation (0% optimization but 100% stability)

### Implementation Framework Preserved

Despite the optimization failures, we successfully implemented:
- `estimate_property_table_space()` function with sophisticated property space calculation
- Comprehensive documentation of property table structure requirements
- Framework for future optimization once compliance issues are resolved

### Successful Optimizations Achieved

**PHASE 1 COMPLETED: Memory Layout Reordering**
- Implemented standard Z-Machine memory layout (static tables, dynamic memory, strings, code)
- Achieved 87% reduction in dictionary-code gap (2780→368 bytes)
- Updated header field generation for correct memory boundaries
- Comprehensive regression testing confirms functionality preservation

**PHASE 2.1 COMPLETED: Abbreviation System**
- Implemented intelligent string frequency analysis for compression
- Added analyze_strings_for_abbreviations() with sophisticated candidate selection
- Identified 32 high-value abbreviation candidates (e.g., "You can't" ×13, "You" ×28)
- Framework ready for significant file size reduction through string deduplication

### Critical Insight

**Property table optimization is blocked by the same compliance issues affecting our entire compiler**: until we fix the fundamental packed address generation and file boundary calculation problems, any memory layout optimization will trigger the same corruption patterns.

**Resolution Path**: Fix the core compliance issues first, then return to property table optimization with proper address validation and boundary checking.

---

## AREAS REQUIRING INVESTIGATION

### **1. Compiler Address Generation**
Likely sources of invalid packed addresses:
- **String table generation** (`src/grue_compiler/codegen.rs`)
- **Routine table creation** (function address packing)
- **Property default values** (object system addresses)
- **Dictionary entries** (word address references)

### **2. File Size Calculation**
- Are we calculating total file size correctly?
- Do we account for all sections (header, objects, strings, code)?
- Are section boundaries properly calculated?

### **3. Interpreter Compliance**
- Why does our interpreter tolerate invalid addresses?
- Should we add strict compliance checking?
- Are we masking critical bugs with lenient behavior?

---

## PHASED IMPLEMENTATION PLAN

### Target Layout (Following Commercial Standard)
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

### PHASE 1: STANDARD MEMORY LAYOUT COMPATIBILITY ⭐ **CRITICAL PRIORITY**

**Step 1.1: Implement standard Z-Machine memory layout**
- Follow commercial game layout patterns exactly
- Ensure header fields point to expected memory regions
- Match disassembler expectations for tool compatibility

**Step 1.2: Validate layout with external tools**
- Test with standard txd disassembler
- Ensure our gruedasm-txd finds all expected routines
- Verify compatibility with Z-Machine tool ecosystem

**Step 1.3: Document layout compliance**
- Create comprehensive layout validation tests
- Add regression testing for disassembler compatibility
- Establish layout compatibility as build requirement

### PHASE 2: STANDARD MEMORY LAYOUT (HIGH PRIORITY)

**Step 2.1: Modify `assemble_complete_zmachine_image()` in `codegen_image.rs`**
```rust
// NEW ORDER:
// Dynamic: Header → Globals → Arrays → Objects → (set static boundary)
// Static:  Dictionary → Grammar
// High:    Code → Strings
```

**Step 2.2: Update header field generation**
- Ensure header 0x0e points to dictionary (start of static memory)
- Validate dynamic memory contains only writable structures
- Update high memory mark (header 0x04) to point to code start

**Step 2.3: Regression testing**
- Verify mini_zork still compiles and runs correctly
- Ensure all 25 functions are preserved
- Test object manipulation still works (`put_prop`, `insert_obj`)

### PHASE 3: FIX PROPERTY OPTIMIZATION RUNTIME BUGS (MEDIUM PRIORITY)

**Step 3.1: Debug property table space estimation**
- Identify boundary calculation errors in `estimate_property_table_space()`
- Fix memory alignment issues causing instruction corruption
- Add comprehensive validation to property layout optimization

**Step 3.2: Implement safe property optimization**
- Restore property table optimization with proper error checking
- Add runtime validation for memory boundary integrity
- Ensure optimization doesn't affect instruction stream

### PHASE 4: ENHANCE DISASSEMBLER COMPATIBILITY (LOW PRIORITY)

**Step 4.1: Improve our disassembler tools**
- Enhance gruedasm-txd to handle our layout patterns
- Add routine detection improvements for non-standard layouts
- Create layout-agnostic disassembly algorithms

**Step 4.2: Optional: Layout standardization**
- Consider adopting commercial layout patterns for better tool compatibility
- Weigh benefits vs. current working implementation
- Only if significant external tool compatibility benefits identified

---

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

# Test 5: Standard tool compatibility
~/Projects/ztools/txd our_file.z3  # Must complete without errors
```

---

## COMPLIANCE REQUIREMENTS

### **Compiler Fixes Needed**
1. **Validate all packed address calculations**
2. **Ensure unpacked addresses stay within file bounds**
3. **Implement proper file size accounting**
4. **Add compliance verification in build process**

### **Interpreter Fixes Needed**
1. **Add strict Z-Machine specification compliance**
2. **Fail fast on invalid packed addresses**
3. **Remove any non-standard tolerances**
4. **Match behavior of commercial interpreters**

### **Testing Requirements**
1. **All generated files must pass txd disassembly**
2. **Files must work with other Z-Machine interpreters**
3. **Compliance verification in CI/CD pipeline**
4. **Regression tests against Z-Machine specification**

---

## SUCCESS CRITERIA

**✅ Compliance Achieved When**:
- `~/Projects/ztools/txd our_file.z3` completes without errors
- Generated files work in other Z-Machine interpreters
- All packed addresses unpack to valid file locations
- File structure matches Z-Machine specification exactly
- All 25 functions detected by disassembler
- Mini_zork gameplay unchanged
- Memory layout matches commercial Z-Machine pattern
- Static/dynamic boundaries correct per Z-Machine spec

**✅ Optimization Achieved When**:
- Property table space optimization works without instruction corruption
- Abbreviation system provides significant file size reduction
- Memory layout improvements maintain commercial game compatibility
- Both optimization and compliance goals are achieved together

---

## PRIORITY AND SCOPE

**CRITICAL PRIORITY**: This is a fundamental correctness issue that:
- Makes our files incompatible with standard Z-Machine ecosystem
- Indicates serious bugs masked by non-compliant interpreter
- Prevents professional use of our compiler
- Violates core Z-Machine specification requirements
- Blocks all aggressive memory optimization efforts

**SCOPE**: Affects all compiled files, suggesting systemic issue in compiler architecture rather than isolated bug.

**RESOLUTION ORDER**:
1. **Layout compatibility** for tool ecosystem integration
2. **Property optimization debugging** for safe memory improvements
3. **Disassembler enhancement** for better development tools

**KEY INSIGHT**: Our files are functionally correct Z-Machine files that work perfectly at runtime. The "compliance" issue is actually a **layout compatibility** challenge with external static analysis tools, not a fundamental correctness problem.