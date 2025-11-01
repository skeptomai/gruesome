# Ongoing Tasks

**Last Updated**: October 31, 2025

## Primary Task: Phase Separation Bug Fix

**Priority**: Critical
**Status**: Root cause identified, solution planning in progress

### Problem Summary
The "open mailbox" command incorrectly shows "It's already open" when the mailbox should start closed. Investigation revealed this is caused by a fundamental phase separation bug in the compiler where computed object property expressions are compiled during Step 2c (object generation) instead of being properly deferred to Step 2f (code generation).

### Root Cause
- Objects with computed `desc` properties (e.g., `"The mailbox is " + (mailbox.open ? "open" : "closed") + "."`) trigger immediate expression compilation during object generation
- This causes memory layout conflicts between object space and code space
- TestAttributeBranch instructions get corrupted (test_attr 0x4A becomes je 0x41)
- Results in wrong branch evaluation and incorrect game behavior

### Implementation Plan
**See comprehensive phased plan**: `docs/PHASE_SEPARATION_BUG_ANALYSIS_AND_PLAN.md`

#### **Phase 1: Infrastructure Setup ⚙️** (Next Step)
- Establish computed property function architecture
- Create `IrPropertyValue::ComputedFunction` variant
- Implement property function registry system
- Modify parser to store AST expressions instead of `"[expression]"` placeholders

#### **Phase 2: Deferred Function Generation 🔧**
- Generate computed property functions during Step 2f code generation
- Implement proper memory layout separation
- Create runtime property access via function calls

#### **Phase 3: Memory Layout Validation 🛡️**
- Comprehensive testing of address space isolation
- End-to-end verification that "open mailbox" works correctly
- Regression testing for all computed properties

### Origin Tracking
This investigation originated from debugging the "open mailbox" command in the mini_zork.grue example:
- User reported: "open mailbox" → "It's already open" (should be "Opened")
- Initial focus: Verb dictionary matching and branch polarity
- Discovery: Memory corruption in TestAttributeBranch instructions
- Root cause: Phase separation architectural flaw exposed by TestAttributeBranch implementation

## Secondary Tasks

### Test Coverage Enhancement
- Comprehensive verb dictionary matching tests
- Memory corruption detection tests
- Phase separation validation tests

### Documentation Updates
- Update COMPILER_ARCHITECTURE.md with phase separation requirements
- Document computed property compilation patterns
- Add debugging guides for memory layout issues

## Completed Tasks

### TestAttributeBranch Implementation ✅
- **Commit**: 41e03ff - Implemented missing TestAttributeBranch codegen
- **Impact**: Exposed pre-existing phase separation bug
- **Status**: Implementation correct, revealed architectural issue

### Verb Dictionary Analysis ✅
- **Finding**: Verb matching works correctly after previous fixes
- **Verification**: "open" command properly resolves to handle_open function
- **Status**: No issues found in dictionary lookup system

### Memory Corruption Investigation ✅
- **Discovery**: test_attr instruction (0x4A) corrupted to je (0x41)
- **Location**: Address 0x01a5 in compiled bytecode
- **Cause**: Object space and code space memory conflicts
- **Status**: Root cause identified

## Next Steps

1. **Design Property Function Architecture**: Define how computed properties become function calls
2. **Implement Phase Separation Fix**: Ensure Step 2c only handles static data
3. **Create Verification Tests**: Validate fix across all computed property patterns
4. **Test Original Bug**: Confirm "open mailbox" works correctly after fix

## Historical Context

This work stems from systematic debugging of Z-Machine compiler correctness. The "open mailbox" command served as a critical test case that revealed deeper architectural issues affecting computed property expressions throughout the system.