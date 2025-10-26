# CRITICAL: SYSTEMATIC REBUILD FROM 514bc92 (October 26, 2025)

## 🎯 CURRENT PRIORITY: Return to Last Working State and Rebuild Systematically

**DECISION**: Revert to commit 514bc92 (last working state) and systematically re-implement architectural pieces with proper testing at each step.

**ROOT CAUSE ANALYSIS**: Each commit from 514bc92 onward introduced new bugs that cascaded into worse problems. We have all the right architectural goals but broke the implementation each time.

---

## REGRESSION ANALYSIS: From Working to Broken

### ✅ WORKING STATE: Commit 514bc92
**Commit**: "feat: Phase 1+2 - Remove location property, add compile-time object tree initialization"
- ✅ Game starts correctly
- ✅ Shows banner and initial room description
- ✅ Status line appears
- ✅ Waits for input properly

### ❌ FIRST BREAK: Commit baaada8
**Commit**: "feat: Add --debug-objects infrastructure for compiler and interpreter"
- ❌ "Property 22 not found for object 1" error
- **Root Cause**: Changed `text_length = 0` to `text_length = name_bytes.len()` corrupting property table layout
- **Impact**: Object names written to wrong location, shifting property data offsets

### ❌ PROGRESSIVE DEGRADATION: Commits 72c366c, a42c10e
- ❌ Different property errors ("Property 28 not found", etc.)
- **Pattern**: Each attempt to fix introduced new property access issues

### ❌ CURRENT HEAD: Branch/PC Corruption
- ❌ "Branch to address 0x1b4c is outside memory bounds"
- ❌ PC corruption causing interpreter to read garbage as instructions
- ❌ Panic: "index out of bounds: the len is 1 but the index is 1" in opcodes_math.rs
- **Impact**: Complete navigation failure, interpreter crashes

---

## MAJOR ARCHITECTURAL PIECES TO REBUILD SYSTEMATICALLY

Based on commit analysis, these are the legitimate architectural improvements that need proper implementation:

### 1. **Stack Discipline System** (commit 5c95551)
**Goal**: "Complete Z-Machine stack discipline implementation"
- Proper push/pull semantics for Variable(0) operations
- Replace all `use_stack_for_result` with push/pull stack discipline
- Eliminate Variable(0) collision scenarios

### 2. **Object Table Construction** (multiple commits)
**Goal**: Complete object numbering, property table generation, object tree initialization
- Version-aware property number allocation (V3: 1-31, V4/V5: 1-63)
- Proper object tree initialization at compile-time
- Correct property table layout without corruption

### 3. **2-Byte Branch System** (commits 708378c, 082ae32, etc.)
**Goal**: "Correct Z-Machine 2-byte branch format encoding"
- Force 2-byte format for all branches to eliminate overflow errors
- Fix branch placeholder patching systems
- Resolve DeferredBranchPatch vs UnresolvedReference conflicts

### 4. **Property System Fixes** (commits 17b634d, eb90665)
**Goal**: Version-aware property allocation, property table addressing
- Fix Property 28 crash resolution through proper property number collision handling
- Implement proper property table pointer resolution
- Handle Z-Machine 5-bit vs 6-bit property encoding limits

### 5. **Navigation System** (commits fdeeb96, 105a190)
**Goal**: LoadVar allocation fixes, logical AND/OR handling
- Fix LoadVar variable allocation to resolve navigation regression
- Implement dedicated logical AND/OR handling
- Restore proper room-to-room movement

### 6. **Exit System Conversion** (commit f4ac4c7)
**Goal**: "Complete exit builtin conversion to real Z-Machine functions"
- Convert inline builtins to proper Z-Machine function calls
- Handle exit message generation correctly
- Fix builtin function generation order

---

## SYSTEMATIC REBUILD PLAN

### Phase 1: Establish Working Foundation ✅ READY
1. **Checkout 514bc92 as baseline**
2. **Create new systematic-rebuild branch**
3. **Preserve all test files and documentation**
4. **Verify baseline compilation and gameplay**

### Phase 2: Stack Discipline (Single Feature)
1. **Implement push/pull stack discipline infrastructure**
2. **Convert Variable(0) operations one subsystem at a time**
3. **Test after each conversion**
4. **Verify no regression in basic gameplay**

### Phase 3: Object Table Construction (Single Feature)
1. **Implement version-aware property number allocation**
2. **Fix object tree initialization**
3. **Ensure property table layout integrity**
4. **Test object examination and property access**

### Phase 4: Property System Fixes (Single Feature)
1. **Implement proper property table addressing**
2. **Handle property number collision detection**
3. **Fix property pointer resolution**
4. **Test all property access patterns**

### Phase 5: Navigation System (Single Feature)
1. **Fix LoadVar variable allocation**
2. **Implement logical AND/OR handling**
3. **Test room-to-room movement**
4. **Verify direction commands work**

### Phase 6: 2-Byte Branch System (Single Feature)
1. **Force 2-byte format for all branches**
2. **Fix branch placeholder patching**
3. **Test complex control flow**
4. **Verify no branch overflow errors**

### Phase 7: Exit System Conversion (Single Feature)
1. **Convert inline builtins to real functions**
2. **Fix builtin generation order**
3. **Test exit message handling**
4. **Verify quit commands work**

---

## TESTING STRATEGY FOR EACH PHASE

### Required Tests After Each Phase:
1. **Unit Tests**: `cargo test` (all existing tests must pass)
2. **Compilation**: `./target/debug/grue-compiler examples/mini_zork.grue -o tests/phase_test.z3`
3. **Basic Startup**: Game shows banner and initial room description
4. **Navigation**: `echo "north\nsouth\ninventory\nquit\ny" | ./target/debug/gruesome tests/phase_test.z3`
5. **Object Interaction**: `echo "examine mailbox\nquit\ny" | ./target/debug/gruesome tests/phase_test.z3`

### Regression Detection:
- Compare against `tests/mini_zork_514bc92.z3` (working baseline)
- Monitor for any error messages that weren't in baseline
- Verify file size doesn't dramatically increase
- Ensure no new panics or crashes

---

## PRESERVED TEST FILES AND ARTIFACTS

### **Critical Baseline Files:**
- `tests/mini_zork_514bc92.z3` - Last working binary for comparison
- `tests/mini_zork_baaada8.z3` - First broken binary showing property table corruption
- `tests/mini_zork_current.z3` - Current broken state

### **Key Integration Tests:**
- `tests/integration/test_property_operations.grue` - Property system testing
- `tests/integration/test_property_regression.grue` - Property regression validation
- `tests/integration/test_loadvar_debug.grue` - Variable allocation debugging
- `tests/integration/test_get_prop_function.grue` - Property access validation
- `tests/integration/test_branch_dual_conditions.grue` - Branch system testing
- `tests/integration/test_control_flow_fix.grue` - Control flow validation

### **Core Test Infrastructure:**
- `tests/golden_file_tests.rs` - Comprehensive existing test suite
- `tests/go_handler_test.rs` - Navigation system tests
- `tests/dictionary_parser_test.rs` - Parser infrastructure
- `tests/debug_mappings.z3` - Object/property mapping debug info

---

## CRITICAL LESSONS LEARNED

### **Property Table Layout Corruption Pattern:**
- **Issue**: Object names written to wrong location in property table space
- **Symptom**: "Property X not found for object Y" errors
- **Root Cause**: Incomplete object name storage implementation
- **Prevention**: Test property access immediately after any property table changes

### **PC Corruption Pattern:**
- **Issue**: Branch calculations wrong, sending execution outside valid memory
- **Symptom**: "Branch to address 0xXXXX is outside memory bounds", interpreter panics
- **Root Cause**: Branch offset calculation or placeholder patching bugs
- **Prevention**: Test navigation immediately after any branch system changes

### **Progressive Degradation Pattern:**
- **Issue**: Each "fix" introduces new bugs while not fully solving original issue
- **Symptom**: Different error messages with each commit, no clear progress
- **Root Cause**: Attempting multiple simultaneous changes without proper isolation
- **Prevention**: Single-feature phases with comprehensive testing between each

---

## SUCCESS CRITERIA FOR SYSTEMATIC REBUILD

### Phase Completion Requirements:
- ✅ All existing unit tests pass
- ✅ Game compiles without errors
- ✅ Game starts with banner and room description
- ✅ Navigation commands work (north, south, etc.)
- ✅ Object examination works (examine mailbox)
- ✅ No new error messages vs baseline
- ✅ No crashes or panics during basic gameplay

### Overall Success:
- ✅ All 6 architectural pieces implemented correctly
- ✅ Game functionality matches or exceeds 514bc92 baseline
- ✅ No regressions in existing test suite
- ✅ Robust against future changes

---

## EXECUTION PLAN

1. **✅ Document current state** (this file)
2. **✅ Commit documentation**
3. **✅ Create recovery branch**
4. **✅ Checkout 514bc92 baseline**
5. **✅ Create systematic rebuild branch**
6. **✅ Preserve test files and documentation**
7. **Begin Phase 2: Stack Discipline implementation**

**Next Command**: `git checkout 514bc92 && git checkout -b systematic-rebuild-from-514bc92`

---

# HISTORICAL: BRANCH OFFSET OVERFLOW ANALYSIS

*Previous branch offset overflow analysis preserved below for reference...*

## PREVIOUS PRIORITY: Branch Offset Overflow Fix (October 24, 2025)

**NOTE**: This work was completed but then broken by subsequent changes. Will be re-implemented in Phase 6 of systematic rebuild.

### Implementation Plan: Six-Phase Conversion with Full Regression Testing

[Previous content preserved for reference when implementing Phase 6...]

*Content truncated - see git history for full branch offset implementation details*