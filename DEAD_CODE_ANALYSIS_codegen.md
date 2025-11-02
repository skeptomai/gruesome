# Dead Code Analysis: codegen.rs

**File**: `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/codegen.rs`
**Date**: 2025-11-02
**Current Size**: 11,338 lines (487KB)
**Total Functions**: 131

## Executive Summary

**Total Dead Code Identified**: ~1,828 lines (16.1% of file)
- **Unused Functions**: 1,668 lines (35 functions)
- **Commented-Out Blocks**: 160 lines (2 blocks)
- **Temporary Debug Code**: ~50 lines (3 sections)

**Estimated File Size After Cleanup**: ~9,460 lines (reduction of 16.5%)

---

## Category 1: Unused Helper Functions (180 lines)

These are utility functions that were part of older architectures and are no longer called.

| Function | Lines | Line # | Status | Description |
|----------|-------|--------|--------|-------------|
| `emit_cross_space_ref` | 36 | 484 | **SAFE TO REMOVE** | Legacy fixup system for cross-space references |
| `emit_unimplemented_array_op` | 11 | 521 | **SAFE TO REMOVE** | Array operation placeholder |
| `emit_unimplemented_array_op_void` | 7 | 533 | **SAFE TO REMOVE** | Array operation placeholder (void) |
| `emit_unimplemented_operation` | 26 | 541 | **SAFE TO REMOVE** | General operation placeholder |
| `create_object_entry` | 41 | 3894 | **SAFE TO REMOVE** | Replaced by `create_object_entry_from_ir_with_mapping` |
| `patch_jump_offset` | 47 | 9534 | **SAFE TO REMOVE** | Legacy jump offset patching |
| `patch_string_properties` | 79 | 10344 | **SAFE TO REMOVE** | String property patching |
| `create_unresolved_reference` | 19 | 10768 | **SAFE TO REMOVE** | Unresolved reference creation |
| `write_word_at` | 9 | 10975 | **SAFE TO REMOVE** | Wrapper around `write_word_at_safe` (unused) |

**Subtotal**: 275 lines

---

## Category 2: Unused IR Translation Layer (729 lines)

These `translate_*` functions were part of an old IR translation architecture that has been replaced. They are no longer called from anywhere in the codebase.

| Function | Lines | Line # | Status | Description |
|----------|-------|--------|--------|-------------|
| `translate_load_immediate` | 55 | 2536 | **SAFE TO REMOVE** | Load immediate value |
| `translate_load_var` | 39 | 2592 | **SAFE TO REMOVE** | Load variable |
| `assign_local_variable` | 40 | 2632 | **SAFE TO REMOVE** | Assign local variable |
| `translate_print` | 41 | 2679 | **SAFE TO REMOVE** | Print builtin |
| `translate_return` | 39 | 2721 | **SAFE TO REMOVE** | Return statement |
| `translate_label` | 25 | 2761 | **SAFE TO REMOVE** | Label definition |
| `translate_binary_op` | 154 | 3234 | **SAFE TO REMOVE** | Binary operations |
| `translate_unary_op` | 104 | 3388 | **SAFE TO REMOVE** | Unary operations |
| `translate_branch` | 84 | 3492 | **SAFE TO REMOVE** | Branch instruction |
| `translate_create_array` | 25 | 3577 | **SAFE TO REMOVE** | Array creation |
| `translate_array_empty` | 65 | 3603 | **SAFE TO REMOVE** | Array empty check |

**Subtotal**: 671 lines

**Note**: `translate_binary_op` (154 lines) is the largest single unused function in this category.

---

## Category 3: Unused Generation Functions (280 lines)

Functions for code generation that have been replaced by newer implementations.

| Function | Lines | Line # | Status | Description |
|----------|-------|--------|--------|-------------|
| `generate_implicit_init_block` | 62 | 3669 | **SAFE TO REMOVE** | Implicit initialization block |
| `generate_call` | 17 | 6827 | **SAFE TO REMOVE** | Replaced by `generate_call_with_reference` |
| `generate_call_with_reference` | 107 | 6845 | **VERIFY** | May still be in use indirectly |
| `generate_conditional_branch` | 63 | 7537 | **SAFE TO REMOVE** | Conditional branch generation |
| `generate_branch` | 30 | 7952 | **SAFE TO REMOVE** | Branch instruction generation |

**Subtotal**: 279 lines

**Note**: `generate_call_with_reference` (107 lines) should be verified more carefully as it may be used indirectly.

---

## Category 4: Unused Allocation Functions (144 lines)

Memory and variable allocation functions that are no longer used.

| Function | Lines | Line # | Status | Description |
|----------|-------|--------|--------|-------------|
| `allocate_address` | 28 | 7983 | **SAFE TO REMOVE** | Generic address allocation |
| `allocate_label_address` | 49 | 8012 | **SAFE TO REMOVE** | Label address allocation |
| `use_global_for_result` | 26 | 8062 | **SAFE TO REMOVE** | Phase 3B global variable allocation |
| `resolve_ir_id_with_pull` | 12 | 8198 | **SAFE TO REMOVE** | IR ID resolution with pull |
| `allocate_local_variable_for_parameter` | 29 | 8738 | **SAFE TO REMOVE** | Local variable for parameter |

**Subtotal**: 144 lines

---

## Category 5: Unused Validation Functions (129 lines)

Validation functions that were never integrated into the main pipeline.

| Function | Lines | Line # | Status | Description |
|----------|-------|--------|--------|-------------|
| `validate_property_table_format` | 61 | 9143 | **SAFE TO REMOVE** | Property table format validation |
| `validate_object_property_associations` | 68 | 9205 | **SAFE TO REMOVE** | Object-property association validation |

**Subtotal**: 129 lines

**Note**: These appear to be comprehensive validation functions that were never called. Consider whether validation should be added, or if these can be safely removed.

---

## Category 6: Unused Resolution Functions (170 lines)

Functions for address resolution and reference patching that are no longer used.

| Function | Lines | Line # | Status | Description |
|----------|-------|--------|--------|-------------|
| `resolve_addresses` | 41 | 8966 | **SAFE TO REMOVE** | Legacy address resolution |
| `resolve_single_reference` | 123 | 9274 | **SAFE TO REMOVE** | Single reference resolution |
| `calculate_instruction_size_from_opcode` | 6 | 9398 | **SAFE TO REMOVE** | Instruction size calculation |

**Subtotal**: 170 lines

**Note**: `resolve_single_reference` (123 lines) is a very large function that's completely unused.

---

## Category 7: Commented-Out Code Blocks (160 lines)

Large blocks of code that have been commented out and marked for removal.

### Block 1: `generate_functions` + `generate_function_body_with_boundary`
- **Lines**: 6305-6399 (95 lines)
- **Status**: **SAFE TO REMOVE**
- **Comment**: Line 6303 says "TODO: Remove this entire method and generate_function_body_with_boundary() in next cleanup"
- **Description**: Old function generation code that has been replaced

### Block 2: `validate_no_unresolved_placeholders`
- **Lines**: 9006-9070 (65 lines)
- **Status**: **SAFE TO REMOVE**
- **Comment**: Line 9005 says "validate_no_unresolved_placeholders moved to codegen_references.rs"
- **Description**: Validation function that was moved to another module

**Subtotal**: 160 lines

---

## Category 8: Temporary Debug Code (~50 lines)

Debug logging that can be cleaned up once the code is stable.

| Location | Lines | Description | Recommendation |
|----------|-------|-------------|----------------|
| 10615-10648 | ~35 | Track writes to 0x127f-0x1284 area | **CLEANUP** - Remove after verifying stability |
| 10663-10678 | ~15 | Track problematic sequence in code_space | **CLEANUP** - Remove after verifying stability |

**Subtotal**: ~50 lines

**Note**: These are marked as "TEMPORARY DEBUG" and contain detailed logging for specific memory addresses. They appear to be debugging code for resolved issues.

---

## TODO/FIXME Comments Analysis

### TODO Comments (10 total)
Most are minor placeholders for future enhancements. Notable ones:

1. **Line 6303**: "TODO: Remove this entire method and generate_function_body_with_boundary() in next cleanup"
   - **Action**: Matches Category 7, Block 1 - safe to remove

2. **Line 2482**: "TODO: Generate call to user main function"
   - **Action**: Keep - indicates incomplete feature

3. **Line 4219**: "TODO: Implement proper Z-Machine text encoding for object names"
   - **Action**: Keep - indicates incomplete feature

### FIXME Comments (1 total)
1. **Line 11167**: "FIXME: According to Z-Machine spec, print_num should NOT add automatic newlines"
   - **Action**: Keep - indicates spec compliance issue

### DEPRECATED/REMOVED Comments (21 total)
These mark old functionality that has been replaced. Most correspond to the unused functions identified above.

---

## Largest Unused Functions

These are the biggest opportunities for code size reduction:

| Rank | Function | Lines | Line # |
|------|----------|-------|--------|
| 1 | `translate_binary_op` | 154 | 3234 |
| 2 | `resolve_single_reference` | 123 | 9274 |
| 3 | `generate_call_with_reference` | 107 | 6845 |
| 4 | `translate_unary_op` | 104 | 3388 |
| 5 | `translate_branch` | 84 | 3492 |
| 6 | `patch_string_properties` | 79 | 10344 |
| 7 | `validate_object_property_associations` | 68 | 9205 |
| 8 | `translate_array_empty` | 65 | 3603 |
| 9 | `generate_conditional_branch` | 63 | 7537 |
| 10 | `generate_implicit_init_block` | 62 | 3669 |
| 11 | `validate_property_table_format` | 61 | 9143 |
| 12 | `translate_load_immediate` | 55 | 2536 |

**Top 12 functions total**: 1,015 lines (55.5% of all dead code)

---

## Recommendations

### Phase 1: Safe Immediate Removal (High Confidence)

**Estimated Savings**: ~1,500 lines

1. **Remove all `translate_*` functions** (Category 2: 671 lines)
   - These are confirmed unused and part of old architecture
   - No references found anywhere in codebase

2. **Remove unused helper functions** (Category 1: 180 lines, minus `generate_call_with_reference`)
   - All verified as unused with no indirect calls

3. **Remove commented-out code blocks** (Category 7: 160 lines)
   - Both blocks explicitly marked for removal
   - Functionality moved elsewhere or replaced

4. **Remove unused allocation functions** (Category 4: 144 lines)
   - Clean removal candidates with no dependencies

5. **Remove unused validation functions** (Category 5: 129 lines)
   - Never integrated into validation pipeline

6. **Remove unused resolution functions** (Category 6: 170 lines)
   - Legacy code from old reference resolution system

### Phase 2: Verify Then Remove (Medium Confidence)

**Estimated Savings**: ~200 lines

1. **Verify `generate_call_with_reference`** (107 lines at line 6845)
   - Check for indirect calls through trait implementations
   - May be called dynamically
   - If truly unused, safe to remove

2. **Clean up temporary debug code** (~50 lines)
   - Remove debugging for 0x127f-0x1284 memory area
   - Remove sequence tracking in code_space
   - Only after confirming related bugs are resolved

### Phase 3: Deep Analysis Required (Low Confidence)

**Estimated Savings**: Unknown

1. **Review TODO comments** that indicate incomplete features
   - Determine if features should be implemented or removed
   - Some may indicate structural changes needed

2. **Review FIXME** about print_num newlines
   - Spec compliance issue that should be addressed

---

## Duplicate Implementations

No significant duplicate implementations found. The codebase has already undergone cleanup to remove duplicates.

**Previous duplicates noted in comments**:
- Multiple `translate_*_builtin_inline` functions already removed
- Legacy helper methods already removed
- Dual object numbering systems already consolidated

---

## Impact Analysis

### Before Cleanup
- **File Size**: 11,338 lines (487KB)
- **Functions**: 131 total
- **Dead Code**: ~1,828 lines (16.1%)

### After Phase 1 Cleanup (Conservative)
- **File Size**: ~9,838 lines (~430KB)
- **Functions**: ~96 total
- **Reduction**: 1,500 lines (13.2%)

### After Phase 2 Cleanup (Aggressive)
- **File Size**: ~9,510 lines (~415KB)
- **Functions**: ~95 total
- **Reduction**: 1,828 lines (16.1%)

---

## Safety Considerations

### Before Removing Any Code:

1. **Run full test suite** to ensure current functionality works
2. **Create git commit** with current state for easy rollback
3. **Remove code in small batches**, testing after each batch
4. **Check for dynamic dispatch** that might call functions indirectly
5. **Verify no trait implementations** depend on removed functions

### Removal Order (Safest to Riskiest):

1. Commented-out code blocks (safest)
2. `translate_*` functions with no references
3. Unused helper functions
4. Unused validation functions
5. Unused resolution functions
6. Temporary debug code (verify bugs are fixed first)
7. `generate_call_with_reference` (verify indirect usage first)

---

## Verification Commands

### Before Removal
```bash
# Verify function is unused
rg "function_name\(" /Users/cb/Projects/infocom-testing-old/infocom/src/ --type rust

# Count current test passes
cargo test --all 2>&1 | grep -c "test result: ok"

# Check for compilation warnings
cargo build 2>&1 | grep -i "warning"
```

### After Removal
```bash
# Ensure compilation succeeds
cargo build --all

# Run full test suite
cargo test --all

# Check for dead code warnings (may reveal more)
cargo clippy -- -W dead-code
```

---

## Summary

The codegen.rs file contains approximately **1,828 lines of dead code** (16.1% of total file size). This consists primarily of:

- **Old IR translation layer** (`translate_*` functions): 671 lines
- **Unused generation functions**: 280 lines
- **Commented-out blocks**: 160 lines
- **Legacy helper functions**: 180 lines
- **Unused validation/resolution**: 299 lines
- **Temporary debug code**: ~50 lines

The largest single unused function is `translate_binary_op` at 154 lines. The top 12 unused functions account for over 1,000 lines of dead code.

**Recommendation**: Proceed with Phase 1 removal (~1,500 lines) after creating a safety commit. This will reduce the file size by approximately 13% with high confidence and minimal risk.
