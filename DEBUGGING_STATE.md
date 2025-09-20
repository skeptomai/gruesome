# CRITICAL DEBUGGING STATE - DO NOT LOSE THIS CONTEXT

## CURRENT SITUATION (September 20, 2025)

**WE ARE STUCK IN EXACTLY THE SAME PLACE AFTER HOURS OF WORK**

- Same crash: "Invalid Long form opcode 0x00" at address 0x012a
- Same game state: Banner displays correctly, crashes during function execution
- Same root issue: Unresolved placeholders in compiled bytecode

## FAILED APPROACHES - DO NOT REPEAT

1. **Architectural "Fix" in emit_jz_branch()**:
   - Changed emit_jz_branch() to not pass branch_offset to emit_instruction()
   - Manually emit placeholders and create UnresolvedReference
   - Result: ZERO PROGRESS - same crash at same location
   - This was NOT the real issue

2. **Reverting 0x0000 hack**:
   - Restored placeholder_word() emission in codegen_instructions.rs
   - Result: ZERO PROGRESS - same crash at same location
   - This was NOT helpful

3. **"Double emission" theory**:
   - Theory that emit_instruction() and low-level functions both emit placeholders
   - Spent hours debugging this theoretical architectural issue
   - Result: ZERO PROGRESS - theory was WRONG

## WHAT WE ACTUALLY KNOW

1. **The crash is at 0x012a with opcode 0x00 (all zeros)**
2. **This means unresolved 0xFFFF placeholders that never get patched**
3. **The issue is NOT in emit_jz_branch() or architectural double-emission**
4. **The game banner works fine - print system is NOT the issue**
5. **The crash occurs when calling function at 0x1372 (list_objects function)**

## WHAT WE NEED TO DO NEXT

**DO NOT waste time on:**
- More "architectural fixes"
- Theories about double emission
- Changes to emit_jz_branch()
- Reverting/re-reverting previous changes

**ACTUALLY INVESTIGATE:**
- Why are there still 21 unresolved 0xFFFF placeholders?
- Which specific instruction types are creating untracked placeholders?
- What is the ACTUAL emission code path that bypasses UnresolvedReference tracking?

## USER FRUSTRATION LEVEL: MAXIMUM

The user has repeatedly pointed out we are making ZERO progress despite extensive code changes. We keep "fixing" things that don't actually improve the situation.

**STOP MAKING CHANGES THAT DON'T SOLVE THE ACTUAL PROBLEM**

The real issue is somewhere else entirely and we need to find the ACTUAL source of untracked placeholders, not continue with theoretical architectural changes.