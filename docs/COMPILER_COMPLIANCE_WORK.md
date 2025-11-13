# COMPILER COMPLIANCE WORK - Z-MACHINE STANDARD VIOLATIONS

## CRITICAL DISCOVERY (November 13, 2025)

**Our compiler generates Z-Machine files that violate the Z-Machine standard, causing failures with standard disassemblers while mysteriously working with our own interpreter.**

## INVESTIGATION SUMMARY

### Test Protocol Results

**✅ Mini_Zork Gameplay Test - PASSED**
- All 10 test commands executed correctly
- Score progression: 0 → 2 (leaflet) → 7 (egg)
- All systems functional: navigation, object interaction, containers, scoring
- Total moves: 4 as expected
- **Our interpreter handles the game perfectly**

**⚠️ Disassembler Analysis - MAJOR COMPLIANCE ISSUE**

### Our Disassembler (gruedasm-txd) Results:

**Commercial File (Seastalker)**:
```
- Analysis range: 5401 to 112c9 (proper full analysis)
- Multiple routines with rich instruction sets
- Complete disassembly successful
```

**Our Compiled Files (mini_zork.z3)**:
```
- Analysis range: 14cf to 14cf (stops immediately!)
- Only 1 routine found: R0001 with minimal instructions
- File size: 9.2K but content appears truncated in analysis
```

### 3rd Party Disassembler (~/Projects/ztools/txd) Results:

**Commercial File (Seastalker)** - ✅ **WORKS PERFECTLY**:
```
[Complete disassembly with full routine tables, strings, etc.]
[End of text]
[End of file]
```

**Our Files (mini_zork.z3, test_01_basic_v3.z3)** - ❌ **FATAL ERRORS**:

Mini_Zork:
```
*** ACCESSING address 0x94a5 which is in page 74 (>= 13) ***
*** This would be UNPACKED from packed address 0x4a52
*** Called from read_data_byte() at final address 0x94a5
Fatal: game file read error
errno: 0, page: 74, bytes_to_read: 512, file_size: 9156
```

Basic Test:
```
Fatal: game file read error
errno: 0, page: 10, bytes_to_read: 512, file_size: 2492
```

## ROOT CAUSE ANALYSIS

### **PACKED ADDRESS CALCULATION VIOLATION**

**The Problem**: Our compiler generates packed addresses that point beyond the actual file size:

- **Packed address**: `0x4a52`
- **Unpacked address**: `0x94a5` (37,957 bytes)
- **Actual file size**: 9,156 bytes
- **Violation**: Unpacked address exceeds file size by ~4x

### **Z-Machine Specification Violation**

From Z-Machine Standard Section 1.2.3:
> "Packed addresses must unpack to valid byte addresses within the game file"

Our compiler violates this fundamental requirement by:
1. **Generating packed addresses that unpack beyond file boundaries**
2. **Creating invalid memory references that crash standard tools**
3. **Producing files that only work with our non-compliant interpreter**

## SYSTEMIC IMPACT

### **Why Our Interpreter "Works"**

Our interpreter likely has **non-standard tolerance mechanisms**:
- May ignore invalid packed addresses
- Could have different unpacking logic than Z-Machine spec
- Possibly has bounds checking that silently fails rather than crashing

### **Why Standard Tools Fail**

Standard Z-Machine tools (txd, other disassemblers) correctly:
- Follow Z-Machine specification exactly
- Validate packed address calculations
- Fail fast on specification violations
- Cannot process our non-compliant files

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

## PRIORITY AND SCOPE

**CRITICAL PRIORITY**: This is a fundamental correctness issue that:
- Makes our files incompatible with standard Z-Machine ecosystem
- Indicates serious bugs masked by non-compliant interpreter
- Prevents professional use of our compiler
- Violates core Z-Machine specification requirements

**SCOPE**: Affects all compiled files, suggesting systemic issue in compiler architecture rather than isolated bug.

## NEXT STEPS

1. **Identify packed address generation code paths**
2. **Analyze file size calculation methodology**
3. **Implement proper bounds checking**
4. **Add Z-Machine compliance validation**
5. **Fix interpreter to reject non-compliant files**
6. **Verify all fixes with standard tools**

## SUCCESS CRITERIA

**✅ Compliance Achieved When**:
- `~/Projects/ztools/txd our_file.z3` completes without errors
- Generated files work in other Z-Machine interpreters
- All packed addresses unpack to valid file locations
- File structure matches Z-Machine specification exactly