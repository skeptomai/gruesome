# IR Generator Refactoring Plan
**Date**: December 15, 2025
**Objective**: Split `ir_generator.rs` (3,724 lines) into focused functional modules
**Pattern**: Follow proven grammar extraction success (ONGOING_TASKS.md)

---

## Analysis Summary

### Current Structure
- **Total Lines**: 3,724
- **Single impl block**: `impl IrGenerator` (all methods)
- **Top 3 Giants**:
  - `generate_expression_with_context`: 867 lines (23.3% of file)
  - `generate_statement`: 586 lines (15.7% of file)
  - `generate_builtin_function_call`: 354 lines (9.5% of file)

### Functional Areas Identified

| Category | Methods | Approx Lines | Module Target |
|----------|---------|--------------|---------------|
| **Expression Gen** | `generate_expression*`, `expr_to_ir_value`, `is_array_type` | ~930 | `ir_gen_expressions.rs` |
| **Statement Gen** | `generate_statement`, `generate_block`, iteration helpers | ~730 | `ir_gen_statements.rs` |
| **Builtin Handling** | `generate_builtin_function_call`, `is_builtin_function` | ~470 | `ir_gen_builtins.rs` |
| **Object System** | `generate_object`, `generate_world`, placement, tree ops | ~460 | `ir_gen_objects.rs` |
| **Function Gen** | `generate_function`, dispatch, polymorphism | ~340 | `ir_gen_functions.rs` |
| **Grammar Gen** | `generate_grammar`, pattern matching | ~115 | `ir_gen_grammar.rs` |
| **Room Gen** | `generate_room`, exit handling | ~115 | `ir_gen_rooms.rs` |
| **Core** | `generate`, `generate_item`, `next_id`, utilities | ~560 | `ir_generator.rs` (main) |

**Note**: Line counts include helper methods and are approximate.

---

## Target Architecture

```
src/grue_compiler/
├── ir_generator.rs              # Core struct, main generate(), orchestration (~560 lines)
├── ir_gen_expressions.rs        # Expression compilation (~930 lines)
├── ir_gen_statements.rs         # Statement compilation (~730 lines)
├── ir_gen_builtins.rs          # Builtin function handling (~470 lines)
├── ir_gen_objects.rs           # Object/world generation (~460 lines)
├── ir_gen_functions.rs         # Function compilation & dispatch (~340 lines)
├── ir_gen_grammar.rs           # Grammar pattern generation (~115 lines)
└── ir_gen_rooms.rs             # Room generation (~115 lines)
```

**Design Principles**:
- Each module is a focused functional area
- Core module contains orchestration and shared utilities
- Modules use `impl IrGenerator` blocks (like `codegen_*.rs` pattern)
- Clear dependency flow: Core → Specialized modules

---

## Safety Mechanisms

### 1. **Compilation Verification** (Every Step)
```bash
cargo build
cargo test
```
**Requirement**: Must pass with zero warnings/errors

### 2. **Bytecode Comparison** (After Each Module)
```bash
# Compile test game before extraction
cargo run --bin grue-compiler -- tests/mini_zork.z3 > /tmp/before.z3

# After extraction
cargo run --bin grue-compiler -- tests/mini_zork.z3 > /tmp/after.z3

# Compare bytecode (should be identical or document differences)
cmp /tmp/before.z3 /tmp/after.z3 && echo "IDENTICAL ✅" || xxd /tmp/before.z3 > /tmp/before.hex && xxd /tmp/after.z3 > /tmp/after.hex && diff /tmp/before.hex /tmp/after.hex
```

### 3. **Gameplay Testing** (After Each Module)
```bash
# Run comprehensive test suite
./scripts/test_mini_zork_comprehensive.sh

# Verify critical patterns
./scripts/test_literal_pattern_matching.sh
```

### 4. **Git Discipline**
- Commit after each successful module extraction
- Use descriptive commit messages: `refactor: Extract [module] from ir_generator`
- Enable easy rollback if issues discovered

### 5. **IR Output Verification** (Expressions & Statements)
```bash
# Check IR structure (before/after)
cargo run --bin grue-compiler -- tests/mini_zork.z3 --print-ir > /tmp/before_ir.txt
# ... extract module ...
cargo run --bin grue-compiler -- tests/mini_zork.z3 --print-ir > /tmp/after_ir.txt
diff /tmp/before_ir.txt /tmp/after_ir.txt
```
**Requirement**: IR should be identical (proves semantic preservation)

---

## Extraction Phases

### Phase 1: Commit Current State ✅
**Action**: Commit the existing `ir.rs` / `ir_generator.rs` split
```bash
git add src/grue_compiler/ir.rs src/grue_compiler/ir_generator.rs
git commit -m "refactor: Split ir.rs into type definitions and generator implementation"
```

**Verification**:
- ✅ Builds successfully
- ✅ All tests pass
- ✅ Clean baseline for future work

---

### Phase 2: Extract Grammar Generation
**Why First**: Smallest module (~115 lines), lowest risk, builds confidence

**Steps**:
1. Create `src/grue_compiler/ir_gen_grammar.rs`
2. Extract `generate_grammar()` method
3. Add module declaration in `ir_generator.rs`: `mod ir_gen_grammar;`
4. Keep `impl IrGenerator` pattern

**Safety Checks**:
- ✅ Compilation succeeds
- ✅ Bytecode comparison (identical expected)
- ✅ Grammar test suite passes
- ✅ Git commit

**Files Modified**: `ir_generator.rs`, new `ir_gen_grammar.rs`

---

### Phase 3: Extract Room Generation
**Why Next**: Small module (~115 lines), isolated functionality

**Steps**:
1. Create `src/grue_compiler/ir_gen_rooms.rs`
2. Extract `generate_room()` method
3. Add module declaration

**Safety Checks**:
- ✅ Compilation succeeds
- ✅ Bytecode comparison
- ✅ Room navigation tests
- ✅ Git commit

**Files Modified**: `ir_generator.rs`, new `ir_gen_rooms.rs`

---

### Phase 4: Extract Function Generation
**Why Next**: Medium size (~340 lines), clear boundaries

**Methods to Extract**:
- `generate_function()`
- `mangle_function_name()`
- `detect_specialization()`
- `register_function_overload()`
- `generate_dispatch_functions()`
- `create_dispatch_function()`

**Steps**:
1. Create `src/grue_compiler/ir_gen_functions.rs`
2. Extract all function-related methods
3. Add module declaration

**Safety Checks**:
- ✅ Compilation succeeds
- ✅ Bytecode comparison
- ✅ Function call tests (including polymorphism)
- ✅ Git commit

**Files Modified**: `ir_generator.rs`, new `ir_gen_functions.rs`

---

### Phase 5: Extract Object System
**Why Next**: Medium-large (~460 lines), some interdependencies

**Methods to Extract**:
- `generate_object()`
- `generate_world()`
- `assign_object_numbers_recursively()`
- `assign_object_number_to_object_and_nested()`
- `setup_property_defaults()`
- `register_object_and_nested()`
- `extract_object_hierarchy()`
- `generate_object_placement_instructions()`
- `generate_placement_for_object()`
- `add_player_object()`
- `generate_room_object_placement()`

**Steps**:
1. Create `src/grue_compiler/ir_gen_objects.rs`
2. Extract object/world generation methods
3. Add module declaration

**Safety Checks**:
- ✅ Compilation succeeds
- ✅ Bytecode comparison
- ✅ Object manipulation tests
- ✅ Container tests
- ✅ Git commit

**Files Modified**: `ir_generator.rs`, new `ir_gen_objects.rs`

---

### Phase 6: Extract Builtin Handling
**Why Next**: Medium size (~470 lines), clear functional boundary

**Methods to Extract**:
- `generate_builtin_function_call()` (354 lines - the big one!)
- `is_builtin_function()` (112 lines)

**Steps**:
1. Create `src/grue_compiler/ir_gen_builtins.rs`
2. Extract builtin-related methods
3. Add module declaration

**Safety Checks**:
- ✅ Compilation succeeds
- ✅ Bytecode comparison
- ✅ Builtin function tests (print, move, scoring, etc.)
- ✅ Git commit

**Files Modified**: `ir_generator.rs`, new `ir_gen_builtins.rs`

---

### Phase 7: Extract Statement Generation
**Why Next**: Large (~730 lines), but well-isolated

**Methods to Extract**:
- `generate_statement()` (586 lines - second biggest!)
- `generate_block()` (23 lines)
- `generate_object_tree_iteration()` (24 lines)
- `generate_object_tree_iteration_with_container()` (113 lines)

**Steps**:
1. Create `src/grue_compiler/ir_gen_statements.rs`
2. Extract statement compilation methods
3. Add module declaration

**Safety Checks**:
- ✅ Compilation succeeds
- ✅ Bytecode comparison
- ✅ IR comparison (--print-ir)
- ✅ Control flow tests (if/while/for)
- ✅ Git commit

**Files Modified**: `ir_generator.rs`, new `ir_gen_statements.rs`

---

### Phase 8: Extract Expression Generation (FINAL BIG ONE)
**Why Last**: Largest module (~930 lines), most complex

**Methods to Extract**:
- `generate_expression_with_context()` (867 lines - THE GIANT!)
- `generate_expression()` (10 lines wrapper)
- `expr_to_ir_value()` (37 lines)
- `is_array_type()` (32 lines)

**Steps**:
1. Create `src/grue_compiler/ir_gen_expressions.rs`
2. Extract expression compilation methods
3. Add module declaration

**Safety Checks**:
- ✅ Compilation succeeds
- ✅ Bytecode comparison
- ✅ IR comparison (--print-ir)
- ✅ Expression evaluation tests
- ✅ Comprehensive gameplay testing
- ✅ Git commit

**Files Modified**: `ir_generator.rs`, new `ir_gen_expressions.rs`

---

### Phase 9: Final Verification
**Comprehensive Testing**:
```bash
# Build all targets
cargo build --all-targets

# Run full test suite
cargo test

# Compile all test games
cargo run --bin grue-compiler -- tests/mini_zork.z3
cargo run --bin grue-compiler -- tests/integration/*.grue

# Run comprehensive gameplay tests
./scripts/test_mini_zork_comprehensive.sh
./scripts/test_literal_pattern_matching.sh

# Verify bytecode matches original baseline
cmp /tmp/phase1_baseline.z3 /tmp/final.z3
```

**Documentation**:
- Update this document with actual results
- Note any bytecode differences (and explanations)
- Document any unexpected challenges

**Final Commit**:
```bash
git add docs/IR_GENERATOR_REFACTORING_PLAN.md
git commit -m "docs: Complete IR generator refactoring - all phases successful"
```

---

## Risk Assessment

### Low Risk Phases (2-4)
- Small, isolated modules
- Clear functional boundaries
- Low dependency coupling

### Medium Risk Phases (5-6)
- Larger modules
- Some cross-cutting concerns
- More methods to coordinate

### High Risk Phases (7-8)
- Very large functions
- Complex logic
- Expression generation is deeply interconnected

**Mitigation**:
- Incremental approach with verification at each step
- Strong bytecode/IR verification
- Easy rollback via git
- Can pause between phases if needed

---

## Success Criteria

### Per-Phase Success
- ✅ Zero compilation warnings or errors
- ✅ Bytecode identical or documented differences
- ✅ All tests passing
- ✅ Gameplay verification successful
- ✅ Clean git commit

### Overall Success
- ✅ `ir_generator.rs` reduced to ~560 lines (85% reduction)
- ✅ 7 new focused modules created
- ✅ 100% functionality preserved
- ✅ Zero test regressions
- ✅ Bytecode matches baseline

---

## Rollback Plan

**If Any Phase Fails**:
```bash
# Discard uncommitted changes
git checkout -- .

# Or revert last commit
git revert HEAD

# Or hard reset to before phase
git reset --hard <phase_start_commit>
```

**When to Rollback**:
- Compilation errors that can't be quickly fixed
- Bytecode divergence without clear explanation
- Test failures that indicate logic bugs
- Complexity spiraling out of control

**CRITICAL**: Never proceed to next phase with failing verification

---

## Lessons from Grammar Extraction (Applied Here)

✅ **Extract to module first** - Move code wholesale before refactoring
✅ **Incremental approach** - One module at a time
✅ **Bytecode verification** - Catches regressions immediately
✅ **Gameplay testing** - Real-world validation
✅ **Git discipline** - Commit each step independently
✅ **Start small** - Build confidence with easy modules first
✅ **End with giants** - Tackle largest/riskiest modules last

---

## Timeline Expectations

**Per CLAUDE.md**: No time estimates

**Effort Distribution**:
- Small modules (2-4): Quick wins
- Medium modules (5-6): Steady progress
- Large modules (7-8): Careful, methodical work
- Testing: Ongoing throughout

**Philosophy**: Speed comes from safety, not rushing

---

## Post-Refactoring State

### Final Structure
```
ir_generator.rs          ~560 lines (core, 85% reduction from 3,724)
ir_gen_expressions.rs    ~930 lines (expression compilation)
ir_gen_statements.rs     ~730 lines (statement compilation)
ir_gen_builtins.rs       ~470 lines (builtin handling)
ir_gen_objects.rs        ~460 lines (object system)
ir_gen_functions.rs      ~340 lines (function compilation)
ir_gen_grammar.rs        ~115 lines (grammar generation)
ir_gen_rooms.rs          ~115 lines (room generation)
────────────────────────────────────
TOTAL                   ~3,720 lines (same code, better organized)
```

### Maintainability Wins
- ✅ Easy to find code by functionality
- ✅ Reduced cognitive load per file
- ✅ Clear separation of concerns
- ✅ Follows established codebase patterns
- ✅ Easier to test individual areas
- ✅ Better collaboration potential

---

## Execution Checklist

### Pre-Flight
- [ ] Read this plan completely
- [ ] Understand safety mechanisms
- [ ] Commit current clean state (Phase 1)
- [ ] Create baseline bytecode snapshot

### Per Phase
- [ ] Create new module file
- [ ] Extract methods (copy first, delete from original after verify)
- [ ] Add module declaration
- [ ] Build succeeds
- [ ] Bytecode comparison passes
- [ ] Tests pass
- [ ] Gameplay verification
- [ ] Git commit with clear message
- [ ] Update this checklist

### Post-Completion
- [ ] All 8 phases complete
- [ ] Final verification suite passes
- [ ] Documentation updated
- [ ] ONGOING_TASKS.md updated
- [ ] Celebrate! 🎉

---

## Questions Before Starting

1. Should we create baseline snapshots now?
2. Any specific test scenarios beyond mini_zork?
3. Prefer pause points between phases for review?
4. Any concerns about the phase ordering?

**Ready to begin when you are!**
