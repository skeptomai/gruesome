# Experimental Branch: Extra Jump Pattern Investigation

**Branch**: `experimental/extra-jump-pattern-investigation`
**Created**: December 14, 2025
**Status**: ✅ Investigation complete - Pattern works correctly!

## Purpose

Investigate why the "extra jump pattern" control flow (introduced in refactoring commit 8a2c27d) fails to work correctly, even when using proper 2-byte branch encoding.

## Background

During December 11-12 refactoring, the literal pattern matching control flow was changed from:

### Pre-Refactoring (Working) ✅
```
1. je word, literal → branch-on-FALSE to skip_literal_label
2. [fall through] call handler
3. jump main_loop
4. skip_literal_label: [next pattern]
```

### Post-Refactoring (Broken) ❌
```
1. je word, literal → branch-on-TRUE to execute_literal_label
2. [fall through] jump skip_literal_label
3. execute_literal_label: call handler
4. jump main_loop
5. skip_literal_label: [next pattern]
```

## The Problem

The post-refactoring pattern logically should work:
- Match → branch to execute_literal_label → call handler ✓
- No match → fall through → jump to skip_literal_label ✓

However, **it fails in practice** with test results:
- "look around" → "You can't see any such thing" ❌
- "look at mailbox" → Works correctly ✅
- "look" → Works correctly ✅

## Attempted Branch Encodings

All tested with 2-byte branch form (bit 7=0):

1. **0x4000** - Branch-on-TRUE per bit 6 encoding ❌ Failed
2. **0x7FFF** - Same encoding as word count check ❌ Failed
3. **0x3FFF** - Inverted bit 6 from 0x7FFF ❌ Failed

None worked for "look around" test case.

## Investigation Questions

### 1. Branch Sense Encoding
- Is there semantic inversion in emit_instruction_typed()?
- Do comments match actual encoding behavior?
- Are branch sense bits interpreted differently for different opcodes?

### 2. Label Resolution
- Does execute_literal_label resolve correctly?
- Is skip_literal_label offset calculation affected by extra jump?
- Does reference resolution handle two labels (execute + skip) correctly?

### 3. Code Address Tracking
- Does emit_instruction_typed() update self.code_address correctly for jump?
- Are all addresses after the extra jump offset by 3 bytes as expected?
- Is there fence-post error in label definition timing?

### 4. Compiler Invariants
- Does compiler assume specific control flow patterns?
- Are there hidden dependencies on branch-to-skip vs branch-to-execute?
- Do other parts of codebase make assumptions about pattern structure?

### 5. Runtime Execution
- Add debug logging to trace actual execution flow
- Where does execution actually go when pattern doesn't match?
- Is the jump instruction being executed correctly?

## Debugging Approach

### Phase 1: Bytecode Analysis
```bash
# Compare pre-refactor vs extra-jump-pattern bytecode
git show e688aec:tests/mini_zork_test.z3 > /tmp/pre.z3
# Compile current experimental version
cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/post.z3
# Hex diff the literal pattern sections
xxd /tmp/pre.z3 | grep -A10 -B10 "look" > /tmp/pre_hex.txt
xxd /tmp/post.z3 | grep -A10 -B10 "look" > /tmp/post_hex.txt
diff -u /tmp/pre_hex.txt /tmp/post_hex.txt
```

### Phase 2: Label Resolution Tracing
```rust
// Add debug logging in codegen_grammar.rs
debug!("🏷️ EXECUTE_LABEL: Registering execute_literal_label={} at 0x{:04x}",
       execute_literal_label, self.code_address);
debug!("🏷️ SKIP_LABEL: Registering skip_literal_label={} at 0x{:04x}",
       skip_literal_label, self.code_address);

// Add logging in codegen_resolve.rs
debug!("📍 RESOLVING: Branch to label {} → address 0x{:04x}",
       target_id, resolved_address);
```

### Phase 3: Runtime Execution Tracing
```bash
# Run with interpreter debug logging
RUST_LOG=debug printf "look around\nquit\ny\n" | cargo run --bin gruesome /tmp/post.z3 2>&1 | grep -i "instruction\|branch\|jump"
```

### Phase 4: Control Flow Verification
- Manually trace expected execution path
- Compare with actual bytecode instruction sequence
- Identify where divergence occurs

## Expected Findings

Possible root causes:
1. **Branch sense inversion**: emit logic inverts meaning of branch values
2. **Label timing**: Labels defined before/after wrong instructions
3. **Address calculation**: Jump offset calculated incorrectly
4. **Hidden invariant**: Some compiler assumption about control flow violated
5. **Reference resolution**: Extra label confuses resolution logic

## Success Criteria

✅ Identify exact reason why extra jump pattern fails
✅ Document root cause with evidence
✅ Determine if pattern can be made to work with code changes
✅ OR confirm pattern is fundamentally incompatible with compiler architecture

## Notes

- Pattern is **less efficient** (+3 bytes per literal pattern)
- Pattern is **more complex** (extra label, extra jump)
- Pattern provides **zero benefit** over pre-refactoring approach
- Even if made to work, pre-refactoring pattern is superior

**Recommendation**: Understanding WHY it fails is valuable for avoiding similar issues in future refactoring, even if we never use this pattern.

## FINDINGS (December 14, 2025) ✅

### Investigation Result: Pattern Works Correctly

**Test Results**: All pattern matching commands work perfectly with experimental extra jump pattern:
```
✅ "look around" - Literal pattern executes correctly
✅ "look" - Verb-only pattern executes correctly
✅ "look at mailbox" - Literal+noun pattern executes correctly
```

### Root Cause Analysis

**Original Problem**: The extra jump pattern failed during December 11-12 refactoring

**True Cause**: The failure was NOT due to the control flow pattern itself, but due to **mixed 1-byte/2-byte branch encoding bug**

**Evidence**:
1. Commit 8a2c27d introduced extra jump pattern
2. Tests failed with "look around" returning "You can't see any such thing"
3. Various branch encodings tested (0x4000, 0x7FFF, 0x3FFF) - all failed
4. Pattern was reverted in favor of pre-refactoring approach

**What Was Actually Wrong**:
- Compiler was emitting MIXED branch encoding (some 1-byte, some 2-byte)
- This violated compiler policy: "ALL branches use 2-byte format"
- Branch resolution logic assumed consistent 2-byte encoding
- Systematic misalignment caused pattern matching failures

**Resolution**:
- December 14: Fixed systematic 2-byte encoding enforcement
- December 14: Tested extra jump pattern with proper encoding
- Result: Pattern works perfectly with 0xBFFF (branch-on-TRUE, 2-byte format)

### Architecture Implications

**The Extra Jump Pattern IS Valid**:
```
1. je word, literal → branch-on-TRUE (0xBFFF) to execute_literal_label
2. [fall through] jump skip_literal_label
3. execute_literal_label: call handler
4. jump main_loop
5. skip_literal_label: [next pattern]
```

**When Properly Encoded**:
- 0xBFFF correctly signals "branch when TRUE" to resolution phase
- `patch_branch_offset` sets bit 7=1 (branch on true), bit 6=0 (2-byte)
- Pattern functions identically to pre-refactoring approach

### Efficiency Comparison

**Pre-Refactoring Pattern** (Currently on main):
```
Size: N bytes
Labels: 1 (skip_literal_label)
Jumps: 1 (to main_loop after handler)
```

**Extra Jump Pattern** (Experimental):
```
Size: N+3 bytes (extra jump instruction)
Labels: 2 (execute_literal_label + skip_literal_label)
Jumps: 2 (to skip for no-match, to main_loop after handler)
```

**Analysis**: Extra jump pattern is **less efficient** (+3 bytes per literal pattern) with **no functional benefit**

### Recommendation

**Do NOT adopt extra jump pattern** for the following reasons:

1. **No functional advantage** - Both patterns work identically
2. **Increased code size** - Extra jump adds 3 bytes per pattern
3. **Increased complexity** - Extra label and control flow to maintain
4. **Pre-refactoring pattern simpler** - Fewer moving parts, easier to understand

**Keep pre-refactoring "branch-to-skip" pattern on main** - it is both smaller and simpler.

### Value of Investigation

**What We Learned**:
1. Control flow pattern was NOT fundamentally broken
2. Real cause was systematic branch encoding inconsistency
3. Proper 2-byte encoding is CRITICAL for all branch instructions
4. Pattern matching logic is actually quite flexible

**Prevented Future Issues**:
- Understanding branch encoding semantics prevents similar bugs
- Documented that both branch-to-execute and branch-to-skip patterns work
- Confirmed compiler flexibility for future refactoring needs

## Related Documentation

- `docs/BRANCH_ENCODING_ANALYSIS.md` - Branch format details
- `docs/BRANCH_LOGIC_INVERSION_ANALYSIS.md` - Control flow comparison
- `docs/REFACTORING_BRANCH_INVERSION_BUG.md` - Original bug investigation
- `docs/COMPILER_ARCHITECTURE.md` - Branch placeholder encoding semantics
- `ONGOING_TASKS.md` - Resolution notes
