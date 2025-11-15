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

## CRITICAL DISCOVERY: FUNCTION ADDRESS MISALIGNMENT BUG (November 15, 2025)

### Problem Summary

After implementing PropertyTableAddress fixes for property optimization corruption, testing revealed the real issue: **function addresses are being placed at odd addresses, causing packed address calculation failures**.

### Root Cause Analysis

**The Issue**: Z-Machine packed addresses must be word-aligned (even addresses), but our optimized memory layout places the code base at odd addresses.

**Evidence**:
1. **Pre-optimization**: Works correctly (even code base)
2. **Post-optimization**: Function calls fail with "180 locals" corruption
3. **Address calculation**:
   - Function 9000 at address `0x066f` (odd)
   - Packed address: `0x066f / 2 = 0x0337` (integer division truncation)
   - Interpreter unpacks: `0x0337 × 2 = 0x066e` (points one byte before actual function)
   - Result: Reads `0xB4` (180) instead of `0x07` (7 locals)

### Technical Analysis

**Function Address Calculation**:
```
Function 9000 code offset: 0x0018 (even)
+ Final code base:         0x0657 (odd)
= Final address:           0x066f (odd - INVALID!)
÷ 2 (packed calculation):  0x0337 (truncated)
× 2 (interpreter unpack):  0x066e (wrong address)
```

**What's at the addresses**:
- `0x066e` (where call points): `0xB4` = 180 locals (invalid)
- `0x066f` (where function is):  `0x07` = 7 locals (valid)

### Memory Layout Impact

**The optimization campaign successfully reorganized memory layout but introduced an alignment bug**:

**Pre-optimization**: Even code base → functions at even addresses → packed addresses work
**Post-optimization**: Odd code base → functions at odd addresses → packed address truncation

### Fix Options Analysis

#### Option 1: Fix Code Base Alignment ⭐ **RECOMMENDED**
**Approach**: Ensure `final_code_base` is always word-aligned (even)
```rust
final_code_base = (final_code_base + 1) & !1  // Round up to even
```

**Pros**:
- Fixes root cause (ensures all functions end up at even addresses)
- Minimal code change
- Maintains Z-Machine packed address requirements
- No impact on existing code

**Cons**:
- Adds 1 byte padding when code base is odd
- Need to verify doesn't break memory layout calculations

#### Option 2: Fix Packed Address Calculation
**Approach**: Round up odd addresses before packing
```rust
let packed = (final_addr + 1) / 2  // Always round up for odd addresses
```

**Pros**:
- Direct fix to packed address calculation
- No memory layout changes required

**Cons**:
- Masks underlying alignment issue
- More complex logic in address resolution
- Could hide other alignment problems
- Violates Z-Machine word alignment assumption

#### Option 3: Add Function Address Validation
**Approach**: Verify all function addresses are even before reference resolution

**Pros**:
- Good safety check
- Clear error messages for alignment issues

**Cons**:
- Doesn't fix the underlying cause
- Runtime detection rather than prevention

### Analysis: Padding Insertion Strategy (November 15, 2025)

**DECISION: Implement padding insertion for function address alignment**

After analyzing the architectural documentation and current system design, padding insertion is the optimal solution because:

#### **1. Architectural Compliance**
- Z-Machine packed addresses require word-alignment for division/unpacking operations
- Current system violates this by placing functions at odd addresses
- No existing documentation addresses alignment → indicates design oversight, not intentional choice

#### **2. Minimal System Impact**
- Adds maximum 1 byte padding per misaligned function
- Preserves all memory layout optimizations achieved
- Maintains reference resolution system integrity
- No impact on existing functionality or test results

#### **3. Root Cause Resolution**
- Fixes fundamental Z-Machine specification compliance issue
- Prevents similar alignment problems in future development
- Addresses problem at source rather than masking symptoms

#### **4. Implementation Strategy**
```rust
// Before placing function at address, check alignment
if next_function_address % 2 != 0 {
    // Insert 1 byte padding to align to even boundary
    next_function_address += 1;
    emit_padding_byte()?;
}
```

**Benefits**:
- Simple, clear logic
- Easy validation and testing
- Follows systems programming alignment patterns
- Preserves memory layout optimization gains

### Implementation Plan

#### Phase 1: Investigation ✅ **COMPLETED**
- ✅ Clean build resolved cache issues
- ✅ PropertyTableAddress fix working correctly (placeholders written, references resolved)
- ✅ Identified real issue: function address misalignment from odd code base
- ✅ Proven corruption was introduced by optimization work (not pre-existing)
- ✅ Fixed interpreter to panic on invalid locals (no more silent corruption)

#### Phase 2: Alignment Solution Implementation - **IN PROGRESS**
1. **Add packed address validation**: Panic on odd address division attempts
2. **Implement padding insertion**: Detect odd function addresses and insert padding
3. **Locate alignment insertion point**: Find where functions are placed in memory
4. **Test with baseline**: Verify pre-optimization test cases still work
5. **Test with optimization**: Verify post-optimization corruption is resolved

#### Phase 3: Comprehensive Validation - **SAFETY NET**
1. **Add function address validation**: Verify all functions are at even addresses during compilation
2. **Add packed address validation**: Verify packed addresses unpack correctly
3. **Regression testing**: Ensure no other memory layout issues

### Architecture Requirements

**Z-Machine Compliance**:
- All functions must be at even addresses (word-aligned)
- Packed addresses must unpack to correct locations
- Integer division in packed address calculation assumes even addresses

**Code Generation**:
- Function placement must respect alignment requirements
- Memory layout optimization must preserve address alignment
- Reference resolution must handle address translations correctly

### Success Metrics

**✅ Corruption Fixed When**:
- Function calls work without "180 locals" errors
- Packed addresses unpack to correct function locations
- All functions are placed at even addresses
- Property optimization works without runtime corruption
- Both baseline and optimized compilation produce valid games

**✅ Architecture Preserved When**:
- Memory layout optimizations maintain alignment requirements
- Z-Machine packed address semantics work correctly
- Reference resolution handles memory layout changes
- No regression in existing functionality

### Key Insights

1. **Memory optimization success**: The layout reorganization campaign was technically successful
2. **Alignment oversight**: Code base alignment wasn't considered during optimization
3. **Property fix success**: PropertyTableAddress system works correctly
4. **Real issue identification**: Function address misalignment, not property table corruption
5. **Systematic testing**: Comparison with pre-optimization builds identified exact cause

### Files to Investigate

**Code Base Calculation**:
- `src/grue_compiler/codegen_image.rs` - Memory layout assembly
- `src/grue_compiler/codegen.rs` - Code generation and address mapping
- Memory layout calculation functions

**Address Resolution**:
- `src/grue_compiler/codegen_resolve.rs` - Function address calculation (lines 362-370)
- FunctionCall reference type processing

### Historical Context

This issue represents the intersection of two major system changes:
1. **Memory layout optimization campaign**: Successfully reduced memory gaps and reorganized layout
2. **Z-Machine packed address requirements**: Functions must be word-aligned for correct unpacking

The optimization succeeded in its goals but inadvertently violated a Z-Machine architectural constraint that wasn't explicitly validated.

---

## OPTIMIZATION CAMPAIGN FINAL STATUS (November 15, 2025)

### ✅ **FUNCTION ADDRESS ALIGNMENT BUG: RESOLVED**

**CRITICAL UPDATE**: The function address alignment bug has been **COMPLETELY FIXED**. The alignment padding system is working correctly.

**Evidence of Resolution**:
- ✅ **Compilation**: Alignment padding working (`CODE_ALIGNMENT: Padding 1 byte for V3 function alignment (0x0897 -> 0x0898)`)
- ✅ **Runtime**: Complex gameplay working (object manipulation, property access, scoring)
- ✅ **Validation**: Packed address validation functions prevent future misalignment
- ✅ **Testing**: Both V3 and V4 compilation working with appropriate alignment (2-byte vs 4-byte)

### ❌ **ABBREVIATIONS: CREATED BUT NOT USED**

**Answer to Sparky's Question**: **NO, we are NOT actually using the abbreviations in string encoding.**

**Evidence**:
- ✅ **Abbreviation Creation**: System generates 32 candidates ("You", "the", "can't", etc.)
- ❌ **String Encoding**: No evidence of abbreviation replacement in Phase 1 string encoding
- ❌ **Implementation Gap**: Abbreviations are created but string encoder doesn't use them

**Impact**: We're missing the **main benefit** of abbreviations - file size reduction through string compression.

**Technical Details**:
```
Phase 1: Content analysis and string encoding finished  ← No abbreviation encoding here
📚 Generated 32 abbreviation candidates  ← Created but unused
```

### ✅ **SECTION ORDERING: MOSTLY SPEC-COMPLIANT**

**Answer to Sparky's Question**: **YES, we fixed the section ordering to be more spec-compliant.**

**Current Layout** (Post-Optimization):
```
Header → Globals → Objects → Dictionary(0x0791) → Code(0x0898) → Strings
```

**Memory Base Addresses**:
- Dictionary base: `0x0791` (static memory start)
- Code base: `0x0898` (even-aligned, fixed!)
- Object base: `0x02e0` (in dynamic memory)

**Comparison with Commercial Games**:
- ✅ Objects in dynamic memory ✅
- ✅ Dictionary at static memory boundary ✅
- ✅ Code after dictionary ✅
- 🔄 Strings after code (differs from some commercial games but valid)

### ❌ **DISASSEMBLER STILL FINDS ONLY 1 ROUTINE**

**Answer to Sparky's Question**: **NO, the disassembler still only finds 1 routine instead of ~25 functions.**

**Evidence**:
```bash
grep -c "Routine " /tmp/current_mini_zork_disasm.txt
1  ← Should be ~25 functions from mini_zork.grue
```

**Root Cause**: Our layout still differs enough from commercial games that the disassembler can't properly parse all routines.

### ✅ **FILE SIZE REDUCTION: SIGNIFICANT IMPROVEMENT**

**Answer to Sparky's Question**: **YES, we achieved substantial size reduction.**

**Size Comparison**:
- **Old mini_zork** (pre-optimization): `9.2k` (9,420 bytes)
- **New mini_zork** (current optimized): `8.5k` (8,546 bytes)
- **Total savings**: **874 bytes** (9.3% reduction)

**Gap Reduction Analysis**:
- **Pre-optimization**: Massive 2777-byte gap between dictionary and code
- **Post-optimization**: ~107-byte gap (0x0898 - 0x0791 = 263 bytes for code space calculation)
- **Improvement**: Dramatic gap reduction achieved ✅

### 🔄 **REMAINING OPTIMIZATION OPPORTUNITIES**

#### **1. Activate Abbreviation Encoding**
**Potential**: Additional 10-20% file size reduction
**Status**: Framework complete, needs encoding implementation

#### **2. Property Table Optimization**
**Potential**: 3-7% additional reduction
**Status**: Framework ready, was blocked by alignment bug (now fixed)

#### **3. Disassembler Compatibility**
**Status**: Layout working for runtime, external tools still limited

### **OVERALL ASSESSMENT**

**✅ Major Success**:
- 9.3% file size reduction achieved
- Function alignment bug completely resolved
- Memory layout dramatically improved (87% gap reduction)
- All gameplay functionality preserved

**🔄 Incomplete**:
- Abbreviation system created but not encoding strings
- Disassembler still finds limited routines
- Additional optimization potential available

### **PRIORITIZED WORK ANALYSIS (November 15, 2025)**

Based on Sparky's direction and current findings, the priorities should be:

#### **🎯 PRIORITY 1: LAYOUT AND GAMEFILE COMPLIANCE (CRITICAL)**
**Objective**: Ensure disassembler finds all ~25 routines instead of just 1

**Evidence of Problem**:
- Our gruedasm-txd finds only 1 routine in mini_zork
- Should find ~25 functions from source
- External standard tools still struggle with our layout

**Root Cause Analysis**:
- Layout is "mostly spec-compliant" but still differs from commercial patterns
- Routine detection algorithms expect specific memory patterns
- Our string placement after code may confuse parsing logic

**Implementation Priority**: **CRITICAL - This affects ecosystem compatibility**

#### **🎯 PRIORITY 2: ABBREVIATION ENCODING IMPLEMENTATION (HIGH)**
**Objective**: Activate the created abbreviation system for actual string compression

**Current Gap**:
- 32 high-value abbreviation candidates identified
- String encoding phase ignores abbreviations completely
- Missing 10-20% additional file size reduction

**Implementation Priority**: **HIGH - Major optimization benefit available**

#### **🎯 PRIORITY 3: PROPERTY TABLE OPTIMIZATION (MEDIUM)**
**Objective**: Re-activate property optimization now that alignment bug is fixed

**Status**:
- Framework complete and tested
- Previously blocked by alignment bug (now resolved)
- Potential 3-7% additional file size reduction

**Implementation Priority**: **MEDIUM - Optimization enhancement**

#### **🎯 PRIORITY 4: DISASSEMBLER ENHANCEMENT (LOW)**
**Objective**: Improve our tools to handle non-standard layouts better

**Approach**:
- Enhance gruedasm-txd routine detection
- Add layout-agnostic parsing algorithms
- Better handling of our specific memory patterns

**Implementation Priority**: **LOW - Tool improvement vs core compliance**

### **RECOMMENDED IMPLEMENTATION ORDER**

**Phase A: Layout Compliance (Sparky's Priority)**
1. **Investigate routine detection failure**: Why does disassembler find only 1 routine?
2. **Compare with commercial layout patterns**: Identify specific differences affecting parsing
3. **Implement standard layout compliance**: Adjust memory layout for better tool compatibility
4. **Verify disassembler compatibility**: Ensure all ~25 routines are detected

**Phase B: Abbreviation Activation**
1. **Locate string encoding logic**: Find where abbreviations should be applied
2. **Implement abbreviation replacement**: Add encoding during string generation
3. **Test file size impact**: Measure actual compression achieved
4. **Verify functionality**: Ensure games still work with compressed strings

**Phase C: Property Optimization Re-activation**
1. **Re-enable property table optimization**: Use existing framework
2. **Test with alignment fixes**: Verify no corruption occurs
3. **Measure additional savings**: Quantify property space reduction

**🎯 Next Priority**: **Layout and gamefile compliance for disassembler compatibility** (Per Sparky's direction)

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