# Interpreter/Compiler Compatibility Analysis

**Date**: November 12, 2025

**Status**: Critical compatibility issues identified after interpreter restoration to commit e93edf0

**Impact**: Compiled games show corrupted text and broken functionality

## Executive Summary

After restoring interpreter files from commit e93edf0 to fix commercial game compatibility (Zork I), compiled games from the current compiler exhibit severe compatibility issues including corrupted text display and broken navigation. This analysis identifies the root causes and provides a comprehensive list of known and potential compatibility differences.

## Root Cause Analysis

The fundamental issue is a **version mismatch** between components:

1. **Interpreter**: Restored to e93edf0 state (October 2025) for commercial game compatibility
2. **Compiler**: Current HEAD state (November 2025) with significant evolution
3. **Gap**: 1+ months of compiler development incompatible with restored interpreter

This created a situation where the interpreter correctly executes commercial Infocom games but cannot properly execute games compiled by the current compiler.

## Known Compatibility Differences

### 1. **Opcode Collision (0x0D)** 🔥 **CRITICAL**

**Problem**: Conflicting opcode assignments between interpreter and compiler

- **Interpreter**: Expects `store` instruction at opcode 0x0D (from e93edf0 restoration)
- **Compiler**: Generates `print_paddr` instruction at opcode 0x0D (current codebase)
- **Impact**: Complete instruction mismatch - compiled games crash or execute wrong operations
- **Evidence**: ONGOING_TASKS.md documents duplicate opcode definitions at lines 1182-1192

**Technical Details**:
```rust
// Interpreter expects (src/interpreter.rs):
0x0D => { // store - Store value to variable (2OP:13)
    let var_num = op1 as u8;
    let value = op2;
    self.vm.write_variable(var_num, value)?;
}

// Compiler generates (src/grue_compiler/opcodes.rs):
PrintPaddr = 0x0D  // Conflicts with store instruction
```

### 2. **Attribute Calculation System** ⚠️ **MAJOR**

**Problem**: Different attribute bit calculation methods

- **Interpreter**: Simple calculation for commercial game compatibility
  ```rust
  // ZORK I COMPATIBILITY: Use simple attribute calculation
  let attr_byte = attr_num / 8;
  let attr_bit = 7 - (attr_num % 8);
  ```
- **Compiler**: May expect complex big-endian format
- **Impact**: Attribute tests fail, object state checks return wrong values
- **Evidence**: vm.rs contains explicit "ZORK I COMPATIBILITY" comments

### 3. **Property Size Encoding** ⚠️ **MAJOR**

**Problem**: Property size format exceeds V3 specification limits

- **Interpreter**: Expects V3 single-byte size encoding
  ```rust
  let size = ((size_byte >> 5) & 0x07) + 1;  // V3 format: 3 bits for size
  ```
- **Compiler**: May generate two-byte size format beyond V3 specification
- **Impact**: Property access returns wrong data or causes crashes
- **Evidence**: ONGOING_TASKS.md mentions "property size one-byte vs two-byte where our compiler exceeds the v3 limit"

### 4. **Dictionary Number Inclusion** ⚠️ **MAJOR**

**Problem**: Missing numeric words in dictionary

- **Issue**: Numbers 0-100 missing from compiler-generated dictionaries
- **Impact**: Numeric input parsing fails, score commands broken
- **Evidence**: ONGOING_TASKS.md lines 1174-1180 show dictionary numbers 0-100 restoration was needed
- **Status**: Fixed in staging area but may not match current compiler output

### 5. **Memory Layout Organization** ⚠️ **MAJOR**

**Problem**: Memory region overlap causing corruption

- **Issue**: Dictionary/static memory overlap in compiler-generated layouts
- **Impact**: Memory corruption in complex games, crashes during execution
- **Evidence**: ONGOING_TASKS.md line 1267 mentions "Correct Z-Machine memory layout to prevent dictionary/static memory overlap"
- **Status**: Fixed in interpreter, compiler may generate incompatible layouts

## Potential Compatibility Issues (Investigation Needed)

### 6. **Property Parsing Architecture** 🔍 **INVESTIGATION NEEDED**

**Problem**: Architectural changes in property access methods

- **Issue**: "Hybrid V3 property parsing architecture" mentioned in ONGOING_TASKS.md line 1288
- **Risk**: Property access methods may differ between interpreter and compiler
- **Impact**: Object property reads return wrong values or crash
- **Investigation**: Compare property generation in codegen_objects.rs vs interpretation in vm.rs

### 7. **Text Encoding Differences** 🔍 **INVESTIGATION NEEDED**

**Problem**: Inconsistent space character encoding

- **Evidence**: ARCHITECTURE.md lines 639-683 discusses space encoding (5 vs 0)
- **Interpreter**: Uses Infocom convention (space = Z-character 5) for commercial compatibility
- **Compiler**: Unknown if compiler matches this convention
- **Impact**: Dictionary lookups fail with "I don't know the word 'look'" errors
- **Critical Note**: ARCHITECTURE.md warns "⚠️ NEVER CHANGE SPACE ENCODING FROM 5 TO 0 - This breaks all commercial Infocom games!"

### 8. **String Address vs Dictionary Address** 🔍 **INVESTIGATION NEEDED**

**Problem**: Mixed address types in function parameters

- **Evidence**: String-to-dictionary parameter passing system documented in ONGOING_TASKS.md
- **Issue**: Direction strings need dictionary addresses, display strings need packed addresses
- **Compiler**: May generate wrong address types for function parameters
- **Impact**: Navigation functions receive corrupted parameters (-16641 instead of 2750)

### 9. **Branch Encoding Format** 🔍 **INVESTIGATION NEEDED**

**Problem**: Inconsistent branch instruction format

- **Evidence**: ARCHITECTURE.md states "ALL BRANCHES MUST BE 2-BYTE ENCODING"
- **Issue**: Compiler may generate 1-byte vs 2-byte branch formats inconsistently
- **Impact**: Branch instructions jump to wrong addresses, causing infinite loops
- **Investigation**: Check if compiler consistently generates 2-byte branches as required

### 10. **Opcode Form Instability** 🔍 **INVESTIGATION NEEDED**

**Problem**: Same opcode numbers have different meanings in different instruction forms

- **Evidence**: ARCHITECTURE.md lines 1320-1571 document extensive form instability issues
- **Critical Examples**:
  - `je` (2OP:0x01) vs `storew` (VAR:0x01)
  - `store` (2OP:0x0D) vs `output_stream` (VAR:0x0D)
  - `jl` (2OP:0x03) vs `put_prop` (VAR:0x03)
- **Impact**: Compiler may generate wrong instruction forms for large constants
- **Root Cause**: emit_instruction() automatically selects form based on operand constraints

### 11. **Object Numbering System** 🔍 **INVESTIGATION NEEDED**

**Problem**: Inconsistent object ID mapping between compilation and runtime

- **Evidence**: ONGOING_TASKS.md line 479 mentions "Dual numbering system bug"
- **Issue**: IR object IDs vs runtime object numbers mismatch
- **Impact**: Object references resolve to wrong objects, "take mailbox" affects wrong item
- **Investigation**: Compare object numbering in codegen_objects.rs vs vm.rs object resolution

### 12. **Function Call Stack Discipline** 🔍 **INVESTIGATION NEEDED**

**Problem**: Improper Z-Machine stack usage in function calls

- **Evidence**: ONGOING_TASKS.md mentions "Stack discipline architecture violations"
- **Issue**: Function calls may not use proper Z-Machine Variable(0) discipline
- **Impact**: Stack corruption, parameter passing failures, local variable conflicts
- **Investigation**: Verify function call generation follows Z-Machine specification

## Evidence from Corrupted Mini Zork

The corrupted Mini Zork output provides concrete evidence of compatibility issues:

```
DORK I: The Last Great Empire
                                            ak              k                                          e    e  e                                                            t Vvvf                                                                                       y nk                                                                                                                                                                                                                                                                                       8   k
```

**Analysis**:
- **Corrupted Text**: Banner text shows garbled characters instead of proper game title
- **Memory Layout Issues**: Suggests text string addresses or encoding problems
- **Navigation Broken**: Room descriptions corrupted, navigation commands fail

This indicates fundamental incompatibility in:
1. **String encoding/decoding**
2. **Memory address calculation**
3. **Text display opcodes**

## Detection Strategy

### Files Requiring Comparison

#### 1. Opcode Definitions
- **Interpreter**: `src/interpreter.rs` (e93edf0 opcode handlers)
- **Compiler**: `src/grue_compiler/opcodes.rs` (opcode assignments)
- **Focus**: Verify 1:1 mapping of opcode numbers to instructions

#### 2. Attribute Systems
- **Interpreter**: `src/vm.rs:test_attribute()` and `set_attribute()` methods
- **Compiler**: `src/grue_compiler/codegen_instructions.rs` (attribute generation)
- **Focus**: Bit calculation methods and endianness

#### 3. Property Systems
- **Interpreter**: `src/vm.rs:get_property()` implementation
- **Compiler**: `src/grue_compiler/codegen_objects.rs` (property generation)
- **Focus**: Size encoding and data layout

#### 4. Dictionary Systems
- **Interpreter**: `src/dictionary.rs` (word parsing and lookup)
- **Compiler**: `src/grue_compiler/codegen_strings.rs` (dictionary generation)
- **Focus**: Word inclusion rules and encoding format

#### 5. Memory Layout
- **Interpreter**: Memory region boundaries and validation
- **Compiler**: `src/grue_compiler/codegen_image.rs` (memory layout generation)
- **Focus**: Region overlap prevention and address calculation

### Testing Commands

```bash
# Test basic opcode compatibility
echo "look" | ./target/release/gruesome tests/mini_zork_compatibility_test.z3

# Test attribute operations
echo "open mailbox\nexamine mailbox" | ./target/release/gruesome tests/mini_zork_compatibility_test.z3

# Test property access
echo "examine leaflet" | ./target/release/gruesome tests/mini_zork_compatibility_test.z3

# Test dictionary lookup
echo "score\ninventory" | ./target/release/gruesome tests/mini_zork_compatibility_test.z3

# Test string display
echo "look\ninventory\nquit\ny" | ./target/release/gruesome tests/mini_zork_compatibility_test.z3
```

### Diagnostic Approaches

#### 1. Comparative Analysis
- Compile same .grue source with working commit (e93edf0) vs current HEAD
- Binary diff the resulting .z3 files to identify structural differences
- Hexdump comparison of critical sections (header, dictionary, object table)

#### 2. Opcode Tracing
- Add debug logging to interpreter execution to trace actual opcodes encountered
- Compare expected vs actual opcode sequences for simple commands
- Identify first point of divergence in instruction execution

#### 3. Memory Layout Visualization
- Generate memory maps from both interpreter and compiler perspectives
- Identify region boundaries and potential overlaps
- Verify address calculations for strings, objects, and code

## Solution Approaches

### Option A: Update Compiler (Recommended)

**Approach**: Modify compiler to generate bytecode compatible with e93edf0 interpreter

**Advantages**:
- Preserves proven commercial game compatibility
- Minimal risk to working Zork I functionality
- Clear rollback path if issues arise

**Implementation**:
1. Restore compiler opcode assignments to match e93edf0
2. Revert attribute calculation to simple format
3. Ensure property size encoding stays within V3 limits
4. Restore dictionary number inclusion (0-100)
5. Fix memory layout generation

### Option B: Incremental Interpreter Updates

**Approach**: Carefully update interpreter to support current compiler while maintaining commercial compatibility

**Advantages**:
- Keeps compiler improvements
- Maintains forward development momentum
- Preserves recent bug fixes

**Risks**:
- May break commercial game compatibility
- Complex to implement correctly
- Higher chance of introducing new bugs

### Option C: Version Detection System

**Approach**: Implement runtime detection to support both formats

**Advantages**:
- Supports both commercial games and compiled games
- Future-proof architecture
- No loss of functionality

**Complexity**:
- Significant implementation effort
- Requires careful format detection
- Risk of detection failures

## Recommended Action Plan

### Phase 1: Critical Opcode Fix (Immediate)
1. **Resolve 0x0D collision**: Move either `store` or `print_paddr` to unused opcode (e.g., 0x8D)
2. **Test basic compilation**: Verify simple programs compile and run
3. **Validate commercial games**: Ensure Zork I still works correctly

### Phase 2: Systematic Compatibility Restoration
1. **Attribute calculation**: Restore compiler to generate simple format
2. **Property encoding**: Ensure V3 size limits respected
3. **Dictionary numbers**: Add 0-100 to compiler dictionary generation
4. **Memory layout**: Fix region overlap in compiler output

### Phase 3: Comprehensive Testing
1. **Unit tests**: All compatibility differences resolved
2. **Integration tests**: Both commercial and compiled games working
3. **Regression tests**: No loss of previous functionality
4. **Performance validation**: No significant performance degradation

### Phase 4: Documentation and Prevention
1. **Compatibility guide**: Document interpreter/compiler version requirements
2. **Test automation**: Automated compatibility validation in CI
3. **Version detection**: Consider implementing format detection for future robustness

## Critical Success Criteria

1. **✅ Commercial Games**: Zork I continues to work perfectly
2. **✅ Compiled Games**: Mini Zork shows proper text and functionality
3. **✅ No Regressions**: All previously working features remain functional
4. **✅ Clear Documentation**: Future developers understand compatibility requirements

## Risk Assessment

**High Risk**:
- Breaking commercial game compatibility during fixes
- Introducing new bugs while resolving compatibility issues
- Incomplete understanding of all compatibility differences

**Mitigation**:
- Incremental testing after each fix
- Maintain working commit as rollback point
- Comprehensive test coverage including edge cases

**Medium Risk**:
- Performance impact from compatibility layers
- Increased code complexity
- Future maintenance burden

**Low Risk**:
- User-visible functionality changes (internal compatibility fixes only)

## Conclusion

The interpreter/compiler compatibility issues are extensive but well-documented. The root cause is clear: restoring interpreter files from e93edf0 while keeping an evolved compiler created a version mismatch.

**Recommendation**: Follow Option A (Update Compiler) as it provides the safest path to compatibility while preserving proven commercial game support. The systematic approach outlined above should resolve all identified issues while maintaining the stability achieved with commercial Zork I compatibility.

The success of this effort will demonstrate that the Z-Machine interpreter and Grue compiler can work together seamlessly, supporting both commercial games and newly compiled content in a unified system.