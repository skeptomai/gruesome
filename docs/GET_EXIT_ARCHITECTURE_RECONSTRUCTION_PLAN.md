# GET_EXIT ARCHITECTURE RECONSTRUCTION PLAN

## Current Status: IR Mapping Fixed, Architecture Debt Remains

**Progress Report (Oct 27, 2025)**:
✅ **FIXED**: IR ID 11 → Z-Machine variable mapping works correctly
✅ **FIXED**: handle_go function now executes (moves counter 0→2)
❌ **REMAINING**: get_exit uses HOTFIX registration, returns wrong results

## Root Problem: HOTFIX vs Standard Registration Pipeline

### Current HOTFIX Implementation

**Where**: `src/grue_compiler/codegen.rs:2993-2998` (translate_call)

**Pattern**: Reactive registration during function call translation
```rust
// HOTFIX: Reactive registration when get_exit is called
if function_name == "get_exit" {
    if !self.builtin_functions.contains_key("get_exit") {
        self.create_builtin_get_exit()?; // Create function on-demand
    }
    // Route to actual Z-Machine function...
}
```

**Problems**:
1. **Late registration**: Only when function ID 277 is called
2. **Inconsistent**: Other builtins register during compiler initialization
3. **Recursive pattern**: `translate_call()` → `create_builtin_get_exit()` → more call translation
4. **Incomplete mapping**: Only registers name mapping, doesn't create actual Z-Machine function in standard pipeline
5. **Wrong address**: Function created at 0x0000, but calls go to different addresses (0x08c0)

### Standard Builtin Registration Pipeline

**How other builtins work**:
1. **semantic.rs**: `register_builtin_functions()` registers function in symbol table
2. **codegen.rs**: Function creation phase calls `create_builtin_*()` functions
3. **codegen.rs**: `translate_call()` looks up pre-registered functions

**Example**: `print` builtin
- **Registration**: Done during `register_builtin_functions()`
- **Creation**: Done during proactive builtin generation phase
- **Translation**: Simple lookup from pre-populated tables

## Detailed Reconstruction Plan

### Phase 1: Analyze Current Standard Pipeline (1-2 hours)

**Objective**: Understand how properly integrated builtins work

**Tasks**:
1. **Map print builtin lifecycle**:
   - Find registration in `semantic.rs:register_builtin_functions()`
   - Find creation in `codegen.rs` builtin generation phase
   - Find translation in `codegen.rs:translate_call()`
   - Document the data structures used at each phase

2. **Identify integration points**:
   - Where does `register_builtin_functions()` get called?
   - When does builtin creation phase happen vs user function creation?
   - How are builtin IR IDs allocated vs user function IR IDs?

3. **Document data structure flow**:
   - `SemanticAnalyzer` function registry → `CodeGen` function mappings
   - Function ID allocation strategy for builtins vs user functions
   - UnresolvedReference creation patterns for builtin calls

### Phase 2: Remove HOTFIX and Implement Standard Registration (2-3 hours)

#### Step 2A: Add get_exit to semantic registration
**File**: `src/grue_compiler/semantic.rs`
**Location**: `register_builtin_functions()` method

**Action**: Add get_exit alongside existing builtins like print, value_is_none, etc.
```rust
self.register_function(
    "get_exit".to_string(),
    vec![
        ("room".to_string(), TypeInfo::Object),
        ("direction".to_string(), TypeInfo::String),
    ],
    TypeInfo::Object, // Returns object ID or 0
)?;
```

#### Step 2B: Add get_exit to codegen builtin creation phase
**File**: `src/grue_compiler/codegen.rs`
**Location**: Builtin generation phase (where other `create_builtin_*` calls happen)

**Action**: Add call to `create_builtin_get_exit()` during proactive builtin generation:
```rust
// In generate_builtin_functions() or equivalent
if self.semantic_context.builtin_functions.contains_key("get_exit") {
    self.create_builtin_get_exit()?;
}
```

#### Step 2C: Remove reactive registration HOTFIX
**File**: `src/grue_compiler/codegen.rs`
**Location**: `translate_call()` around lines 2993-2998

**Action**: Delete the entire HOTFIX block:
```rust
// DELETE THIS ENTIRE BLOCK:
if function_name == "get_exit" {
    if !self.builtin_functions.contains_key("get_exit") {
        self.create_builtin_get_exit()?;
    }
    // ... HOTFIX routing code
}
```

#### Step 2D: Verify standard call translation works
**Expected Result**: After removal, get_exit calls should work via standard builtin lookup mechanism

### Phase 3: Fix Function Address Resolution (1-2 hours)

**Current Issue**: get_exit created at 0x0000 but calls go to 0x08c0

**Investigation Tasks**:
1. **Trace address allocation**: Why is create_builtin_get_exit getting 0x0000?
2. **Compare with working builtins**: Where do print/value_is_none get allocated?
3. **Check UnresolvedReference patching**: Are get_exit calls creating proper FunctionRef references?

**Likely Fixes**:
- **Fix 1**: Ensure get_exit creation happens after proper code_address initialization
- **Fix 2**: Verify UnresolvedReference creation for get_exit calls matches other builtins
- **Fix 3**: Check packed address calculation and call_vs operand generation

### Phase 4: Verification and Testing (1 hour)

**Test Cases**:
1. **Before Movement**: Verify room descriptions still work in start_room
2. **After Movement**: Verify room descriptions work in end_room
3. **Multiple Movements**: Test back-and-forth navigation
4. **Exit System**: Test blocked exits, multiple exits per room

**Expected Results**:
```bash
> test
Testing room properties:
Room desc: A simple test room.
Room name: Test Room

> north
You moved.

> test
Testing room properties:
Room desc: The destination room.
Room name: End Room
```

## Risk Assessment and Mitigation

### Low Risk: Standard Integration Pattern
- **Approach**: Follow exact pattern used by existing builtins (print, value_is_none)
- **Verification**: All existing tests should continue to pass
- **Rollback**: Can revert to HOTFIX if integration fails

### Medium Risk: Address Resolution Changes
- **Issue**: Changing when/where get_exit is created might affect address calculation
- **Mitigation**: Compare builtin creation timeline before/after changes
- **Testing**: Verify packed addresses are calculated correctly

### High Risk: Breaking Existing Navigation
- **Issue**: Removing HOTFIX might break current partial navigation functionality
- **Mitigation**: Implement in incremental steps, test after each change
- **Safety**: Commit working IR mapping fix before starting (DONE ✅)

## Success Criteria

### Technical Requirements
- ✅ get_exit registered during standard `register_builtin_functions()` phase
- ✅ get_exit created during proactive builtin generation (not reactive)
- ✅ get_exit calls resolved via standard lookup mechanism (no HOTFIX)
- ✅ All function addresses resolve correctly (no 0x0000 addresses)

### Functional Requirements
- ✅ Property access works correctly both before AND after movement
- ✅ `player.location` resolves to correct object after navigation
- ✅ Room descriptions display properly in all rooms
- ✅ No regressions in existing functionality

### Architecture Quality
- ✅ Zero technical debt: No special-case HOTFIX code for get_exit
- ✅ Consistency: get_exit follows same patterns as all other builtins
- ✅ Maintainability: Adding future builtins follows standard documented process

## Implementation Strategy

### Recommended Order
1. **Start with Phase 1**: Understand standard pipeline thoroughly
2. **Implement Phase 2A-2B**: Add proper registration before removing HOTFIX
3. **Test intermediate state**: Verify both registration methods work
4. **Implement Phase 2C**: Remove HOTFIX only after standard method confirmed working
5. **Address Phase 3**: Fix any address resolution issues discovered during testing

### Verification Commands
```bash
# Test compilation
cargo run --bin grue-compiler -- examples/simple_exit_test.grue -o tests/standard_get_exit.z3

# Test navigation
echo "north" | RUST_LOG=debug ./target/debug/gruesome tests/standard_get_exit.z3

# Test room descriptions after movement
echo -e "north\ntest" | ./target/debug/gruesome tests/standard_get_exit.z3

# Verify no regressions
cargo test
```

## Expected Timeline

**Total Effort**: 4-6 hours for complete reconstruction

- **Phase 1** (Analysis): 1-2 hours
- **Phase 2** (Standard Integration): 2-3 hours
- **Phase 3** (Address Resolution): 1-2 hours
- **Phase 4** (Testing & Verification): 1 hour

**Outcome**: Complete room description system working after player movement, zero architectural debt, all navigation functionality restored.