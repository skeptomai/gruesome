# CURRENT PRIORITY: Integrate Missing Infrastructure from Main (October 25, 2025)

## 🎯 ACTIVE TASK: Safe Integration of Main Branch Infrastructure

**STATUS**: Working from good commit 97ae503 that has all critical runtime fixes. Need to carefully integrate missing infrastructure from main (226bb79) without breaking functionality.

**CONTEXT**: After git workflow issues, we discovered that:
- **Good commit (97ae503)**: Has critical runtime fixes, game works perfectly, but missing some infrastructure exports
- **Main (226bb79)**: Has complete infrastructure but broken runtime (Invalid opcode errors, navigation failures)

**GOAL**: Integrate missing infrastructure from main into good commit without losing the critical runtime fixes.

---

## SAFE INTEGRATION PLAN: Incremental Infrastructure Addition

### Phase 1: Create Safety Nets ✅ COMPLETED
**Goal**: Establish multiple safety branches and verify starting point works

#### Tasks Completed:
1. **✅ Created permanent backup**: `good-commit-backup` branch at 97ae503
2. **✅ Created working branch**: `integrate-main-features` from good commit
3. **✅ Verified baseline works**: Game compiles and runs correctly
4. **✅ Added this planning document**: ONGOING_TASKS.md safely added

### Phase 2: Add Missing Critical Exports ⭐ NEXT
**Goal**: Add the minimum exports needed for compilation without breaking runtime

#### Missing Critical Infrastructure from Main:
1. **`z_words()` function** - Required by codegen_headers.rs, codegen_strings.rs
2. **`DeferredBranchPatch` struct** - Required by codegen_instructions.rs, patch_collision_detector.rs
3. **`TwoPassState` struct** - Two-pass compilation infrastructure
4. **Public `function_addresses`** - Required external access
5. **`initial_property_addr` field** - Critical Bug #23 address translation fix

#### Implementation Strategy:
```rust
// Add to codegen.rs - EXPORTS ONLY, no behavior changes initially:

/// Convert byte addresses to Z-Machine word addresses
pub const fn z_words(byte_count: usize) -> usize {
    byte_count / 2
}

/// Deferred branch patch struct (infrastructure only)
#[derive(Debug, Clone)]
pub struct DeferredBranchPatch {
    pub instruction_address: usize,
    pub branch_offset_location: usize,
    pub target_label_id: IrId,
    pub branch_on_true: bool,
    pub offset_size: u8,
}

// Add fields but keep disabled initially
```

#### Test Sequence for Each Addition:
```bash
# After each export addition:
cargo check                    # Must compile
cargo test                     # Unit tests must pass
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/integration_test.z3
echo -e "north\nquit\ny" | timeout 5s ./target/debug/gruesome tests/integration_test.z3
```

### Phase 3: Add Infrastructure Fields (Disabled)
**Goal**: Add struct fields and infrastructure but keep all new systems disabled

1. **Add `two_pass_state` field** with `enabled: false`
2. **Add `initial_property_addr` field**
3. **Add `push_pull_ir_ids` tracking**
4. **Test extensively**: Compilation + runtime behavior unchanged

### Phase 4: Integration Testing and Validation
**Goal**: Prove integration is safe and complete

1. **Compile multiple test games**
2. **Compare bytecode with baseline**
3. **Verify all external dependencies resolve**
4. **Performance testing**

### Phase 5: Optional - Enable New Features Selectively
**Goal**: Carefully enable beneficial features from main (only if needed)

1. **Two-pass compilation** (if branch issues appear)
2. **Enhanced stack discipline** (if stack issues appear)
3. **Test each enablement separately**

---

## SAFETY PRINCIPLES

### Critical Safety Rules:
1. **Never modify working runtime code** - only add exports and disabled infrastructure
2. **Test compilation + runtime after every single change**
3. **Multiple backup branches** with clear naming
4. **Immediate rollback if anything breaks**
5. **Work incrementally** - one export at a time

### Emergency Rollback Plan:
```bash
# If anything breaks:
git checkout good-commit-backup
git branch integration-attempt-failed integrate-main-features  # Preserve work
git checkout -b integrate-main-features-v2 good-commit-backup  # Start over
```

### Success Criteria:
- ✅ All external imports resolve (cargo check passes)
- ✅ Game compiles without errors
- ✅ Game plays identically to baseline (navigation, commands work)
- ✅ No new runtime errors or crashes
- ✅ All unit tests continue to pass

---

## CURRENT STATUS

**✅ Phase 1 Complete**: Safety branches created, baseline verified working
**⭐ Ready for Phase 2**: Add missing exports one at a time

**Next Command**:
```bash
# Start with the most critical export (z_words function)
code src/grue_compiler/codegen.rs +43
```

**Current Branch**: `integrate-main-features` (working from good commit 97ae503)
**Backup Branch**: `good-commit-backup` (permanent safety copy)

---

## WHAT THIS PLAN ACHIEVES

1. **Preserves all working functionality** from the good commit
2. **Adds missing infrastructure** needed for compilation
3. **Maintains architectural improvements** from main branch development
4. **Provides clear rollback path** if anything goes wrong
5. **Enables future development** with complete infrastructure

**Key Insight**: We're building UP from a working foundation rather than trying to FIX a broken foundation. This is much safer.

---

## HISTORICAL CONTEXT

**Previous Task**: Branch Offset Overflow Fix (moved to `BRANCH_OFFSET_OVERFLOW_FIX.md`)
- Had comprehensive plan for 2-byte branch conversion
- Plan is preserved and can be resumed after infrastructure integration
- All test infrastructure from previous plan is valuable for future work

**Git Incident**: Lost work during cherry-pick attempt, learned importance of safety branches
- Now have permanent backup of critical working commit
- Working branch allows safe experimentation
- Multiple safety nets prevent loss of work