# Phase Separation Bug Analysis - IMPLEMENTATION ARCHIVED

**REVERT DECISION (November 1, 2025)**: This computed property implementation has been archived in favor of a simpler conditional print approach. The work is preserved in the `computed-property-implementation-archive` branch and documented in `docs/COMPUTED_PROPERTY_IMPLEMENTATION_ARCHIVE.md`.

**Status**: Phase 1 & 2 completed successfully, Phase 3 partially implemented. Reverted due to complexity vs. benefit analysis.

---

**Date**: October 31, 2025
**Bug ID**: Phase Separation in Computed Object Properties
**Severity**: Critical - Memory corruption in compiled bytecode
**Root Cause**: TestAttributeBranch implementation exposed pre-existing architectural flaw

## Executive Summary

The implementation of TestAttributeBranch in commit 41e03ff exposed a fundamental phase separation bug where computed object property expressions are compiled during Step 2c (object generation) instead of being properly deferred to Step 2f (code generation). This causes memory layout conflicts and bytecode corruption.

## Problem Statement

When objects contain computed property expressions (like ternary conditionals in `desc` properties), the compiler incorrectly processes these expressions during object table generation rather than deferring them to the code generation phase. This violates the compiler's phase separation architecture and causes memory corruption.

## Technical Details

### Triggering Condition

The bug is triggered by any object with computed property expressions, specifically:

```grue
object mailbox {
    desc: "The small mailbox is " + (mailbox.open ? "open" : "closed") + "."
    //                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    //                              This ternary expression triggers the bug
}
```

### What Happens (Incorrect Behavior)

1. **Step 2c - Object Generation**:
   - Object `mailbox` is processed
   - Property `desc` contains ternary conditional `(mailbox.open ? "open" : "closed")`
   - Compiler immediately compiles this expression, including TestAttributeBranch for `mailbox.open`
   - TestAttributeBranch instruction written to address 0x01a5

2. **Memory Conflict**:
   - Object space simultaneously writes property data to overlapping address ranges (0x01c9, etc.)
   - Code space writes TestAttributeBranch instruction to 0x01a5
   - Memory corruption: test_attr instruction (0x4A) gets overwritten by object data, becomes je (0x41)

3. **Runtime Failure**:
   - Interpreter executes corrupted instruction
   - Wrong branch behavior: `if obj.open` evaluates incorrectly
   - "open mailbox" shows "It's already open" when mailbox should start closed

### What Should Happen (Correct Behavior)

1. **Step 2c - Object Generation**:
   - Object `mailbox` is processed
   - Property `desc` marked as computed property
   - Create property function `mailbox_desc_computed()`
   - Property table contains function address, NOT immediate expression compilation

2. **Step 2f - Code Generation**:
   - Function `mailbox_desc_computed()` compiled with proper memory layout
   - TestAttributeBranch instruction written to code space without conflicts
   - All computed expressions properly deferred

## Evidence Trail

### Debug Output Analysis

```
[DEBUG] 📦 Collected object description: '[expression]' -> ID 1019
[DEBUG] STRING_PACKED_RESOLVE: String ID 1019 offset=0x05b2 + base=0x087e = final=0x0e30 → packed=0x0718
```

This shows `[expression]` being treated as a static string rather than a computed property function call.

### Memory Corruption Evidence

**Expected instruction at 0x01a5**: `4A 20 00 40 XX` (test_attr with 2-byte branch)
**Actual instruction at 0x01a5**: `41 20 00 80 08` (je instruction - corrupted)

### Git History Analysis

- **Before commit 41e03ff**: TestAttributeBranch was silently skipped (not implemented)
- **Commit 41e03ff**: Implemented TestAttributeBranch properly
- **Result**: Previously masked phase separation bug became visible

## Affected Components

### Primary Impact
- Object property compilation (`src/grue_compiler/codegen_objects.rs`)
- Code generation phase management (`src/grue_compiler/codegen.rs`)
- Memory layout management

### Secondary Impact
- Any object with computed property expressions
- TestAttributeBranch and other attribute-related instructions
- Branch evaluation and conditional logic

## Root Cause Analysis

### Why This Bug Exists

The compiler architecture assumes clean phase separation:
- **Step 2c**: Generate static object tables and property data
- **Step 2f**: Generate executable code and functions

However, computed property expressions violate this separation by requiring code generation during object table creation.

### Why It Wasn't Detected Earlier

1. TestAttributeBranch was not implemented (silently skipped)
2. Computed expressions would fail compilation but not crash
3. No instruction corruption occurred without TestAttributeBranch implementation
4. The underlying phase separation flaw remained hidden

## Solution Requirements

### Architectural Fix Required

**All computed object property expressions must be converted to property function calls that are properly deferred to Step 2f.**

Instead of:
```
Property 2: "[expression]" -> string ID 1019 -> static string
```

Should be:
```
Property 2: computed_property_mailbox_desc() -> function call deferred to Step 2f
```

### Implementation Strategy

1. **Identify Computed Properties**: During object analysis, detect expressions that require computation
2. **Generate Property Functions**: Create individual functions for each computed property
3. **Defer Compilation**: Mark property functions for Step 2f compilation
4. **Update Property Tables**: Store function addresses instead of expression results

## Test Cases

### Verification Test 1: Memory Layout
Ensure TestAttributeBranch instructions maintain correct opcodes after compilation.

### Verification Test 2: Expression Evaluation
Verify computed properties evaluate correctly at runtime.

### Verification Test 3: Phase Separation
Confirm no code generation occurs during Step 2c object processing.

## Related Issues

- **Original Bug Report**: "open mailbox" shows "It's already open" when mailbox should start closed
- **Memory Corruption**: test_attr (0x4A) corrupted to je (0x41)
- **Branch Polarity**: Conditional expressions evaluate incorrectly

---

# IMPLEMENTATION PLAN

## Phased Implementation Strategy

### Phase 1: Infrastructure Setup ⚙️
**Goal**: Establish computed property function architecture
**Duration**: Foundation implementation
**Risk**: Low - No existing functionality affected

#### Phase 1a: Define Computed Property Function System
- [ ] **New `IrPropertyValue::ComputedFunction` variant**
  - Add to `IrPropertyValue` enum in `src/grue_compiler/ir.rs:324`
  - Store function name and AST expression for deferred compilation
  - Include source object ID for proper context generation

- [ ] **Property Function Registry**
  - Create `ComputedPropertyManager` in `src/grue_compiler/ir.rs`
  - Track computed property functions: `HashMap<String, (ObjectId, Expression)>`
  - Generate unique function names: `{object_name}_{property_name}_computed`

- [ ] **Parser Enhancement**
  - Modify `parse_expression_as_string()` in `src/grue_compiler/parser.rs:325`
  - Instead of `"[expression]"` placeholder, store AST expression for deferred processing
  - Return `ComputedProperty { expr: Expression, needs_compilation: true }`

**Verification Test 1a**: Parsed objects with computed properties store expressions, not `"[expression]"` strings.

#### Phase 1b: Object Generation Phase Separation
- [ ] **Modify `generate_object()` in `src/grue_compiler/ir.rs:1604`**
  - When processing `obj.description` containing computed expression
  - Register computed property function instead of immediate compilation
  - Store function reference in `IrPropertyValue::ComputedFunction`

- [ ] **Update `IrProperties::set_computed()`**
  - New method to handle computed property registration
  - Store function ID instead of immediate string compilation
  - Defer all expression evaluation to Step 2f

**Verification Test 1b**: Step 2c object generation creates no code instructions.

### Phase 2: Deferred Function Generation 🔧
**Goal**: Generate computed property functions during Step 2f
**Duration**: Core implementation
**Risk**: Medium - Affects code generation pipeline

#### Phase 2a: Function Template Generation
- [ ] **Computed Property Function Headers**
  - Generate function signatures during Step 2f in `src/grue_compiler/codegen.rs:799`
  - Each computed property becomes a real Z-Machine function
  - Proper local variable allocation and return instructions

- [ ] **Expression Compilation Context**
  - Compile stored AST expressions within function context
  - Handle object references (e.g., `mailbox.open`) with proper object resolution
  - Generate TestAttributeBranch and other instructions safely in code space

- [ ] **Function Address Resolution**
  - Update property tables to point to compiled function addresses
  - Replace `IrPropertyValue::ComputedFunction` with `IrPropertyValue::FunctionAddress`
  - Ensure all UnresolvedReference patterns work correctly

**Verification Test 2a**: Computed property functions appear in compiled Z-Machine code space with correct headers.

#### Phase 2b: Property Access Runtime Integration
- [ ] **Property Access Instruction Generation**
  - When Z-Machine needs property value, generate `call_vs` to computed function
  - Handle return value storage and consumption correctly
  - Maintain Z-Machine stack discipline for function calls

- [ ] **Builtin Function Integration**
  - Add computed property functions to builtin function registry
  - Enable `call_builtin_function()` to invoke computed properties
  - Ensure proper parameter passing for object context

**Verification Test 2b**: Runtime property access correctly calls computed functions and returns expected values.

### Phase 3: Memory Layout Validation 🛡️
**Goal**: Ensure no memory conflicts between object and code spaces
**Duration**: Validation and testing
**Risk**: Low - Validation phase

#### Phase 3a: Memory Layout Verification
- [ ] **Address Space Isolation Testing**
  - Verify object space writes never overlap with code space
  - Test Step 2c object generation produces no code instructions
  - Confirm Step 2f code generation operates on isolated memory

- [ ] **Instruction Integrity Testing**
  - Verify TestAttributeBranch instructions maintain correct opcodes (0x4A)
  - Test branch encoding remains 2-byte format as required
  - Confirm no instruction corruption occurs

**Verification Test 3a**: Memory layout analysis shows clean separation between object tables and code space.

#### Phase 3b: End-to-End Integration Testing
- [ ] **Original Bug Verification**
  - Test `"open mailbox"` command shows `"Opened"` instead of `"It's already open"`
  - Verify mailbox description correctly shows `"closed"` initially and `"open"` after opening
  - Confirm all computed property expressions evaluate correctly

- [ ] **Regression Testing**
  - Test all objects with computed properties (mailbox, window, tree, nest, egg)
  - Verify static properties continue working correctly
  - Ensure commercial Z-Machine game compatibility maintained

**Verification Test 3b**: Complete game functionality test with all computed properties working correctly.

## Implementation Order

### Critical Path
1. **Phase 1a** → **Phase 1b**: Foundation must be solid before proceeding
2. **Phase 2a** → **Phase 2b**: Function generation before runtime integration
3. **Phase 3a** → **Phase 3b**: Validation before final testing

### Parallel Development Opportunities
- Phase 1a and Phase 2a planning can proceed simultaneously
- Test case development can proceed in parallel with implementation
- Documentation updates can proceed throughout

## Risk Mitigation

### High-Risk Areas
- **Memory Layout Changes**: Thorough testing of address space separation
- **Function Call Integration**: Z-Machine stack discipline must be maintained
- **Expression Context**: Object resolution within computed functions

### Fallback Strategy
- Each phase can be reverted independently
- Original `"[expression]"` placeholder system remains functional until Phase 3b
- Git commits at each phase boundary for safe rollback

## Success Metrics

### Phase 1 Success
- [ ] Parsed computed properties store AST expressions
- [ ] Object generation produces no code instructions
- [ ] All existing functionality preserved

### Phase 2 Success
- [ ] Computed property functions generated in code space
- [ ] Property access calls computed functions correctly
- [ ] Memory spaces remain isolated

### Phase 3 Success
- [ ] "open mailbox" command works correctly
- [ ] All computed property expressions evaluate correctly
- [ ] Zero instruction corruption or memory conflicts

---

# CURRENT ANALYSIS AND REVISED PLAN (November 1, 2025)

## Critical Discovery: Phase 2 Implementation is Broken

**Investigation Results**:
- Previous Phase 2 implementation exists but is non-functional
- Debug output shows: "🔄 Phase 2: No computed property functions to generate"
- Computed properties are being stored as placeholder strings `__COMPUTED_mailbox_desc` instead of being registered as ComputedFunction
- The computed_property_manager has no functions registered

**Root Cause Identified**:
The Phase 1 (IR Generation) is broken - computed property expressions are never being registered with the computed_property_manager during object processing.

## Revised Implementation Plan

### Phase 1: Fix IR Generation - Computed Property Registration ⚙️
**IMMEDIATE PRIORITY**: The computed property registration is failing during IR generation

#### Phase 1a: Diagnose Registration Failure
- [ ] **Find where computed properties should be registered**
  - Locate `generate_object()` processing of `desc` properties
  - Identify why ternary expressions become `__COMPUTED_mailbox_desc` strings instead of ComputedFunction entries
  - Fix the registration logic to actually store expressions in computed_property_manager

#### Phase 1b: Fix Registration Logic
- [ ] **Ensure computed expressions get registered properly**
  - When processing `desc: "text" + (object.open ? "open" : "closed") + "text"`
  - Store the ternary expression as ComputedFunction variant
  - Register function with computed_property_manager
  - Generate unique function names

**Verification**: Check that debug output shows computed functions being registered instead of "No computed property functions to generate"

### Phase 2: Fix Function Generation Pipeline 🔧
**AFTER Phase 1 Complete**: Make sure registered computed properties become Z-Machine functions

#### Phase 2a: Verify Function Generation Works
- [ ] **Test that registered functions get compiled**
  - Verify compute_property_manager.get_functions() returns registered functions
  - Ensure functions get generated during Step 2f
  - Check function headers and bytecode generation

#### Phase 2b: Fix Function Generation Issues
- [ ] **Address any compilation errors in function generation**
  - Fix expression compilation within function context
  - Handle object references correctly
  - Generate proper return instructions

### Phase 3: Fix GetProperty Function Call Logic 🛡️
**FINAL STEP**: Make GetProperty detect function addresses and call them

#### Phase 3a: Implement Function Address Detection
- [ ] **Complete the GetProperty fix**
  - Add logic to test if property value >= 10000 (function address)
  - If true, emit call_vs instruction to call the function
  - If false, use property value directly
  - Handle return value storage correctly

#### Phase 3b: End-to-End Testing
- [ ] **Verify complete pipeline works**
  - Test "examine mailbox" returns "closed" initially
  - Test "open mailbox" works without crashing
  - Test "examine mailbox" returns "open" after opening
  - Verify leaflet visibility changes

## Current Status Assessment

**What's Actually Working**:
- Game starts without crashing ✓
- Movement commands work ✓
- Basic property access works ✓

**What's Broken**:
- Computed property registration (Phase 1) ❌
- Computed function generation (Phase 2) ❌
- Computed function calling (Phase 3) ❌

**Critical Path**: Fix Phase 1 first, then Phase 2, then Phase 3. No point fixing GetProperty when there are no computed functions to call.

## References

- TestAttributeBranch implementation: commit 41e03ff0affaf7cd82f036feae02c2cd9259ffcb
- Compiler architecture documentation: `COMPILER_ARCHITECTURE.md`
- Z-Machine specification: Branch encoding patterns
- Current codebase structure: Steps 2a-2g in `src/grue_compiler/codegen.rs`