# CodeGen.rs Comprehensive Analysis & Refactoring Guide

**File**: `src/grue_compiler/codegen.rs`  
**Size**: 12,363 lines, 496KB  
**Functions**: 212 functions in 1 impl block  
**Status**: Monolithic architecture requiring refactoring

## 🎯 Executive Summary

This analysis maps all 212 functions in codegen.rs to enable:
1. **Safe refactoring** without data loss (extract modules incrementally)
2. **V4/V5 bug fixes** by isolating version-specific code 
3. **Improved maintainability** through logical module separation

## 📊 Function Categories Analysis

### **Core Infrastructure (25 functions)**
**Risk Level: HIGH** - Core memory management, don't touch first

- `new()` - Constructor
- `allocate_*_space()` (4 functions) - Memory allocation
- `write_to_*_space()` (5 functions) - Memory writing
- `emit_byte()`, `emit_word()` - Low-level emission
- `generate_complete_game_image()` - Main entry point
- `assemble_complete_zmachine_image()` - Final assembly
- Memory space management (7 functions)

### **Version-Dependent Functions (33 locations) ⚠️**
**Risk Level: MEDIUM** - Extract these for V4/V5 fixes

**Alignment Functions (8 functions):**
- `align_function_address()` - V3: 2-byte, V4/V5: 4-byte boundaries
- String alignment in `allocate_string_address()`
- Function alignment in code generation
- Memory layout alignment

**Packed Address Calculation (6 functions):**
- String address packing: V3 ÷ 2, V4/V5 ÷ 4
- Routine address packing
- Reference resolution with version awareness

**Object Table Generation (4 functions):**
- V3: 9 bytes per object, V4/V5: 14 bytes per object  
- Property defaults: V3=31, V4/V5=63 properties
- Header structure differences

**File Length Calculation:**
- V3: Length in 2-byte words at offset 2
- V4/V5: Length in 4-byte words at offset 26

### **IR Translation Layer (52 functions)**
**Risk Level: MEDIUM** - Contains V4/V5 IR mapping bugs

**IR Instruction Handlers (`translate_*` functions - 18 functions):**
- `translate_load_immediate()` 
- `translate_binary_op()` - **Contains V4/V5 comparison bugs**
- `translate_call()` - Function call generation
- `translate_branch()` - Branch instruction generation
- `translate_*_builtin()` (12 functions) - Built-in function implementations

**IR Analysis & Validation (8 functions):**
- `analyze_instruction_expectations()`
- `validate_ir_input()`
- `count_total_ir_instructions()`
- `log_ir_instruction_breakdown()`

### **Address Resolution System (28 functions)**
**Risk Level: MEDIUM** - Core but well-structured

**Reference Resolution (`resolve_*` functions - 8 functions):**
- `resolve_all_addresses()` - Main resolution entry
- `resolve_unresolved_reference()` - Individual reference fixing
- `resolve_legacy_fixup()` - Legacy system support

**Address Patching (`patch_*` functions - 6 functions):**
- `patch_address()` - Generic address patching
- `patch_jump_offset()` - Jump instruction fixup
- `patch_branch_offset()` - Branch offset calculation

### **Z-Machine Structure Generation (45 functions)**
**Risk Level: LOW** - Well-defined, safe to extract

**Object System (12 functions):**
- `generate_objects_to_space()`
- `generate_object_table()`
- Property table generation (6 functions)
- Object data structure creation

**String & Dictionary (8 functions):**
- `generate_strings_space()`
- `encode_string()` 
- Dictionary table generation (4 functions)

**Header Generation (6 functions):**
- `generate_static_header_fields()`
- `fixup_header_addresses()` - **Version-dependent**
- `calculate_checksum()`

### **Builtin Functions (18 functions)**
**Risk Level: LOW** - Self-contained, safe to extract

**Tier 1 - Movement & Location:**
- `generate_move_builtin()` - Object movement
- `generate_get_location_builtin()` - Parent object lookup
- `generate_get_*_builtin()` (5 functions) - Object relationships

**Tier 2 - Properties & Attributes:**
- `generate_get_prop_builtin()` - Property access
- `generate_set_attr_builtin()` - Attribute manipulation  
- `generate_test_attr_builtin()` - Attribute testing

**Tier 3 - Text & Utility:**
- `generate_print_*_builtin()` (4 functions) - Text output **Recently fixed**
- `generate_random_builtin()` - Random number generation

### **Utility & Helper Functions (38 functions)**
**Risk Level: LOW** - Safe to extract first

**Debugging & Analysis:**
- `log_ir_inventory()` - IR analysis
- `validate_final_assembly()` - Post-generation validation
- Instruction form detection helpers

**Memory Layout:**
- `layout_memory_structures()` - Memory planning
- `calculate_*_size()` functions (8 functions)
- Space allocation helpers

## 🚨 Critical V4/V5 Issues Identified

### **1. String Alignment Bug** 
**Location**: Lines 1082-1086, 5022-5027, 9522-9530
**Issue**: V4/V5 require 4-byte aligned string addresses
**Error**: `"String address must be multiple of 4 for v4/v5"`
**Fix Strategy**: Extract alignment logic into separate module

### **2. Function Alignment Bug**
**Location**: Lines 6034-6039, 6199-6204  
**Issue**: V4/V5 functions must be 4-byte aligned vs V3 2-byte
**Impact**: Function calls fail with misaligned addresses
**Fix Strategy**: Unify alignment system across versions

### **3. Packed Address Division**
**Location**: Lines 520, 547, 1716, 9499, 9523
**Issue**: V3 divides by 2, V4/V5 divides by 4
**Status**: Implementation appears correct, test needed
**Fix Strategy**: Add comprehensive V4/V5 address tests

### **4. IR Mapping Regression**
**Location**: Complex IR translation functions (lines 2000-4000)
**Issue**: "No mapping found for IR ID X" errors
**Impact**: Blocks all complex V4/V5 programs  
**Fix Strategy**: Extract IR mapping to separate module for debugging

### **5. Object Table Size Mismatch**
**Location**: Lines 2017-2020, 5090-5093
**Issue**: V3=9 bytes, V4/V5=14 bytes per object
**Status**: Implementation appears correct
**Fix Strategy**: Verify with actual V4/V5 object generation

## 📋 Safe Refactoring Roadmap

### **Phase 1: Low-Risk Extractions (Weeks 1-2)**
**Goal**: Extract 38 utility functions without touching core logic

1. **Create `utils.rs`** - Extract debugging/analysis functions (15 functions)
2. **Create `builtins.rs`** - Extract 18 builtin functions  
3. **Create `validation.rs`** - Extract validation helpers (5 functions)

**Benefits**: Reduce file size by ~2000 lines, no functional changes

### **Phase 2: Structure Generation (Weeks 3-4)**
**Goal**: Extract Z-Machine structure generators  

1. **Create `objects.rs`** - Extract object/property generation (12 functions)
2. **Create `strings.rs`** - Extract string/dictionary generation (8 functions)  
3. **Create `header.rs`** - Extract header generation (6 functions)

**Benefits**: Clear separation of data structure generation

### **Phase 3: Version-Specific Logic (Weeks 5-6)**
**Goal**: Isolate V4/V5 code for targeted fixes

1. **Create `alignment.rs`** - Extract all alignment logic (8 functions)
2. **Create `addressing.rs`** - Extract packed address calculations (6 functions)
3. **Create `version_support.rs`** - Version branching logic

**Benefits**: Enable focused V4/V5 debugging without breaking V3

### **Phase 4: IR Translation (Weeks 7-8)** 
**Goal**: Extract complex IR mapping system

1. **Create `ir_translation.rs`** - Extract IR handlers (18 functions)
2. **Create `instruction_mapping.rs`** - Extract mapping system
3. **Debug V4/V5 IR mapping regressions** in isolation

**Benefits**: Fix "No mapping found for IR ID" errors safely

## 🎯 V4/V5 Bug Fix Strategy

### **Immediate Actions (Can Start Now)**

1. **Create experimental V4/V5 unit tests**
   ```rust
   #[cfg(test)]
   mod v4_v5_tests {
       // Test string alignment specifically
       // Test function alignment specifically  
       // Test packed address calculation
       // Test object table generation
   }
   ```

2. **Extract alignment module first** (lowest risk, highest V4/V5 impact)
   - Move 8 alignment functions to `alignment.rs`
   - Create comprehensive alignment tests
   - Fix 4-byte alignment bugs in isolation

3. **Add comprehensive V4/V5 logging**
   ```rust
   log::debug!("V4/V5_ALIGNMENT: addr=0x{:04x}, aligned=0x{:04x}", addr, aligned);
   log::debug!("V4/V5_PACKING: byte_addr=0x{:04x} packed=0x{:04x}", addr, addr/4);
   ```

### **Medium-term Fixes (After Phase 1 Refactoring)**

1. **IR mapping system overhaul**
   - Extract to separate module
   - Add comprehensive IR ID tracking
   - Fix "No mapping found" regressions

2. **V4/V5 integration tests**
   - Test actual V4/V5 game compilation
   - Validate against known working V4/V5 games
   - Compare output with other Z-Machine compilers

## ⚡ Quick Wins Available

1. **Extract builtins.rs** - 18 self-contained functions, ~800 lines
2. **Extract utils.rs** - 15 debugging functions, ~600 lines  
3. **Create alignment tests** - Catch V4/V5 bugs immediately
4. **Add V4/V5 debug logging** - Trace execution paths

**Result**: ~3000 lines extracted, cleaner main file, targeted V4/V5 fixes possible

## 🔍 Next Steps Recommendation

**Immediate (This Week)**:
1. Create experimental V4/V5 alignment tests
2. Extract builtins.rs (lowest risk, high value)
3. Add comprehensive V4/V5 debug logging

**Short-term (Next 2 Weeks)**:
1. Extract utils.rs and validation.rs
2. Create alignment.rs module  
3. Fix V4/V5 string alignment in isolation

**Medium-term (Next Month)**:
1. Extract structure generation modules
2. Fix IR mapping regressions
3. Re-enable V4/V5 in CI with passing tests

This analysis provides both the **safe refactoring path** AND the **targeted V4/V5 fix strategy** you requested. The key insight is that V4/V5 bugs are concentrated in ~40 specific functions that can be extracted and fixed in isolation.