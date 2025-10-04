# Next Session Status - Grammar System FIXED! ✅

## Current State (October 4, 2025)

### ✅ GRAMMAR SYSTEM WORKING!

The grammar matching system is now fully functional. Users can define custom verbs and they will be recognized and executed correctly.

**Test Result:**
```bash
> test
Test handler called
```

## What Was Fixed

### Bug 1: Opcode Form Conflict with Buffer Address Initialization

**Problem:** Using `2OP:add (0x14)` with `LargeConstant` operands forced VAR form, changing the opcode to `VAR:call_vs (0x14)` - a completely different instruction!

**Location:** `src/grue_compiler/codegen.rs` lines 5179-5229 (text/parse buffer initialization)

**Root Cause:** Z-Machine opcode form instability - same opcode number means different instructions in different forms:
- `2OP:0x14` = add
- `VAR:0x14` = call_vs

**Fix:** Replaced `add` instructions with `VAR:storew` to directly write buffer addresses to global variables:
```rust
// OLD (broken):
self.emit_instruction(
    0x14, // add - but becomes call_vs with LargeConstant!
    &[Operand::LargeConstant(text_buffer_addr), Operand::SmallConstant(0)],
    Some(TEXT_BUFFER_GLOBAL),
    None,
)?;

// NEW (working):
let text_buffer_offset = (TEXT_BUFFER_GLOBAL - 16) as u16;
self.emit_instruction(
    0x01, // VAR:storew (always VAR with 3 operands - no conflict!)
    &[
        Operand::LargeConstant(placeholder_word()), // globals_addr
        Operand::SmallConstant(text_buffer_offset as u8),
        Operand::LargeConstant(text_buffer_addr),
    ],
    None,
    None,
)?;
```

**Files Modified:**
- `src/grue_compiler/codegen.rs:5179-5229` - Use storew for buffer initialization
- `src/grue_compiler/codegen.rs:5437` - Fix parse buffer offset (see Bug 2)

### Bug 2: Wrong Parse Buffer Offset

**Problem:** `loadw` used offset 2 instead of offset 1 to read word 1 dictionary address from parse buffer.

**Parse Buffer Structure:**
```
Word offset 0 (bytes 0-1): max words + word count
Word offset 1 (bytes 2-3): word 1 dictionary address  ← CORRECT
Word offset 2 (bytes 4-5): word 1 length + position   ← WAS READING THIS!
```

**Fix:** Changed loadw offset from 2 to 1:
```rust
// OLD (broken):
Operand::SmallConstant(2), // Offset 2 = WRONG - reads length/position

// NEW (working):
Operand::SmallConstant(1), // Offset 1 = word 1 dict addr (bytes 2-3)
```

**Location:** `src/grue_compiler/codegen.rs:5437`

### Related Fixes

**Verb Dictionary Address Storage** (already working, same pattern):
- `src/grue_compiler/codegen.rs:5443-5474` - Uses storew to write verb dict addr to Variable 17

**Debug Logging Removed:**
- `src/opcodes_memory.rs:107` - Removed storew debug logging
- `src/vm.rs:246-249` - Removed write_word debug logging
- `src/vm.rs:330-337` - Removed Variable 2/17 read logging

## Architecture Insights

### Z-Machine Opcode Form Instability

This session revealed a fundamental architectural issue documented in `docs/ARCHITECTURE.md` (lines 1126-1351):

**The Problem:** Z-Machine opcodes are NOT stable across instruction forms. The same opcode number means different instructions depending on form:

| Opcode | 2OP (Long Form) | VAR (Variable Form) |
|--------|----------------|---------------------|
| 0x01 | je (jump if equal) | storew (store word) |
| 0x0D | store (store value) | output_stream (select stream) |
| 0x14 | add (addition) | call_vs (call routine) |

**Why It Matters:** Using `LargeConstant` operands (values > 255) forces emit_instruction to switch from Long to Variable form, silently changing the opcode meaning!

**The Workaround:** Use `VAR:storew` to write directly to global variable memory instead of using 2OP instructions with large constants.

**Validation Framework:** A compile-time validation framework was implemented (lines 1619-1633 of `codegen_instructions.rs`) but is currently DISABLED because it would break placeholder resolution. See ARCHITECTURE.md for future solution approaches.

## Test Coverage

### ✅ All Tests Passing

```bash
cargo test
# 170+ tests passing
```

### ✅ Grammar Test Working

```bash
printf 'test\nquit\nyes\n' | ./target/debug/gruesome /tmp/test_grammar_working.z3
# Output: "Test handler called"
```

### ✅ Existing Games Still Work

- Zork I: Fully playable
- AMFV: Fully playable
- All integration tests passing

## Files Modified This Session

1. **src/grue_compiler/codegen.rs**
   - Lines 5179-5229: Replace add with storew for buffer initialization
   - Line 5437: Fix parse buffer offset from 2 to 1
   - Lines 5443-5474: Verb dict addr storew (already existed, same pattern)

2. **src/opcodes_memory.rs**
   - Line 107: Removed debug logging

3. **src/vm.rs**
   - Lines 246-249: Removed write_word debug logging
   - Lines 330-337: Removed Variable read debug logging

4. **docs/ARCHITECTURE.md**
   - Lines 1126-1351: Comprehensive opcode form instability documentation (added in previous session)

5. **src/grue_compiler/codegen_instructions.rs**
   - Lines 1619-1633: Validation framework (disabled, from previous session)

6. **src/grue_compiler/error.rs**
   - Lines 38-42, 113-125, 168-176: OpcodeFormConflict error (from previous session)

## Next Steps

### High Priority

1. **Test with Real Grammar** - Create more complex grammar tests with:
   - Multiple verbs
   - Verbs with nouns (2-word commands)
   - Synonym handling
   - Prepositions

2. **Status Line Score Bug** - The score displays as 1826 (0x0722) which is the verb dictionary address
   - This is Variable 17 being displayed instead of Variable 16
   - Likely a separate bug in score variable initialization
   - Low priority since it's cosmetic

### Medium Priority

3. **Opcode Form Conflict Solution** - Consider implementing proper fix:
   - Option A: Add `PlaceholderConstant` operand type with deferred validation
   - Option B: Accept limitation and document workarounds
   - See ARCHITECTURE.md lines 1280-1351 for detailed analysis

4. **Additional Grammar Features**
   - Object names in grammar rules
   - Direction handling
   - Multiple word verbs ("pick up", "turn on")

### Low Priority

5. **Validation Framework** - Enable compile-time opcode form validation once placeholder solution is implemented

6. **Performance** - Profile compiled games vs commercial Z-Machine files

## Known Issues

### Score Display Shows Wrong Value

**Symptom:** Status line shows "Score: 1826" instead of actual score

**Cause:** Variable 17 (verb dict addr = 0x0722 = 1826 decimal) is being displayed instead of Variable 16 (actual score)

**Impact:** Cosmetic only - doesn't affect gameplay

**Priority:** Low - fix after more important features

### Validation Framework Disabled

**Status:** Opcode form conflict validation is implemented but disabled

**Reason:** Would prevent using placeholders with form-sensitive opcodes

**Solution Path:** See ARCHITECTURE.md "Option 2: Fix emit_instruction Architecture"

## Git State

**Current Branch:** main

**Local Commit:** f1d05a3 "fix: Grammar system now working - fixed opcode form conflicts"

**Status:** Committed locally, but branches have diverged from origin/main
- Local has 36 commits not on remote
- Remote has 48 commits not on local
- Push was rejected (non-fast-forward)

**Action Needed:** Next session should decide whether to:
1. Pull and merge remote changes, then push
2. Force push if local work is definitive
3. Create a new branch for this work

**All Changes Committed:** Yes - everything is saved in local git

## Success Metrics

- ✅ Grammar matching working
- ✅ All 170+ tests passing
- ✅ Zork I still playable
- ✅ AMFV still playable
- ✅ No regression in existing functionality
- ✅ Clean code (debug logging removed)
- ✅ Documented architecture insights

## Architecture Documentation

Comprehensive documentation of the opcode form instability issue is now in:
- `docs/ARCHITECTURE.md` lines 1126-1351
- Includes problem description, examples, solution approaches, and lessons learned

This documentation will help future development avoid similar issues and provides a roadmap for proper architectural fixes.
