# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 15, 2025) - ALL SYSTEMS OPERATIONAL ✅

**PROGRESS**: Fixed Bug #22 (Interpreter Debug Code). All known bugs resolved. Mini Zork fully playable with clean error-free output.

### Bug 21: Object Names Property ✅ FIXED (Oct 15, 2025)
- **Issue**: "examine mailbox" and "examine tree" should work but didn't initially
- **Root Cause**: Rooms were getting property 7 populated when they shouldn't
  - Only objects with explicit `names:` property should have property 7
  - Rooms (IrRoom) have no `names` field in source code
  - Items (IrObject) have `names: Vec<String>` field in source code
- **Solution**: Check `if !object.names.is_empty()` at `codegen_objects.rs:453` ✅ ALREADY IN PLACE
  - IrRoom: Empty `names` field → NO property 7 → Can't be matched by grammar
  - IrObject: Populated `names` field → Property 7 gets dictionary address → Can be matched
- **Architectural Insight**:
  - Both `examine X` and `look at X` call same function: `examine(obj)`
  - Only `look` (no args) calls `look_around()` for room description
  - Design intent: Only objects with explicit `names:` can be examined/looked at
- **Verification**: Comprehensive gameplay test shows all features work:
  - ✅ Navigation (north, south, east, west, up, down)
  - ✅ Looking around rooms
  - ✅ Examining items (mailbox, window, tree, nest, leaflet)
  - ✅ Taking items (leaflet, nest, egg)
  - ✅ Inventory management
  - ✅ Opening containers (mailbox)
  - ✅ Reading items (leaflet)
  - ✅ Dropping items
  - ✅ "take all" command
- **Files**: `src/grue_compiler/codegen_objects.rs:449-469`

## Recent Fixes (October 15, 2025)

### Bug 20: Grammar Dispatch Chain Broken ✅ FIXED (Oct 15, 2025)
- **Issue**: Only first verb worked, subsequent verbs caused infinite loop
- **Fix**: Implemented proper verb handler chaining - each verb jumps to next verb on mismatch
- **Files**: `codegen.rs:5913-5944, 5952-5960, 6170-6194`
- **See**: `CLAUDE_HISTORICAL.md` for complete details

### Bug 19: Missing Return Instructions ✅ FIXED (Oct 15, 2025)
- **Issue**: Functions without explicit returns caused infinite loops
- **Fix**: Added `generate_block_for_function()` helper that adds implicit `rtrue`
- **Files**: `ir.rs:2086-2106, 1392, 1458`
- **See**: `CLAUDE_HISTORICAL.md` for complete details

**Previous milestones**: Room handlers, exit system, grammar system complete. See `CLAUDE_HISTORICAL.md` for full history.

### Bug 22: Interpreter Debug Code Accessing Invalid Objects ✅ FIXED (Oct 15, 2025)
- **Issue**: 90+ "BOUNDS ERROR" messages when running compiled games
- **Symptoms**: "Property table address 0x2707 out of bounds (file size: 7208)" for objects 16-255
- **Root Cause**: `dump_object_tree()` debug function iterated through 1-256 objects without knowing actual count
  - Z-Machine has NO explicit object count field in header or object table
  - Function tried to detect object existence by checking parent/sibling/child bytes
  - For fake objects #16+, those bytes are actually property table data and can be non-zero
  - Function thought they were real objects, called `get_object_name()` with garbage property pointers
- **Memory Layout**:
  - Object table: Property defaults (62 bytes) → Object entries (9 bytes each) → Property tables
  - Object entries occupy offsets 0x3e-0xc4 (135 bytes: 15 objects × 9 bytes)
  - Property tables start at offset 0xc5 (indicated by minimum property table pointer)
  - Bytes at 0xc5+ are property table data, NOT object entries
- **The Proper Fix**: Calculate object count from Z-Machine structure (interpreter.rs:340-373)
  - Property tables come AFTER all object entries (architectural invariant)
  - Each object has property table pointer (last 2 bytes of entry)
  - Find minimum property table address across all potential objects
  - Object count = (min_prop_table_addr - first_obj_addr) / obj_entry_size
  - Result: 15 objects, property tables at 0x0481 ✅
- **Impact**: **Zero BOUNDS ERROR messages** (down from 90+), proper architectural solution
- **Lesson**: Use file structure to calculate boundaries, not error handling as control flow
- **Files**: `src/interpreter.rs:340-373` (proper object count calculation)

---

## CRITICAL: INTERPRETER MODIFICATION POLICY

**FUNCTIONALITY CHANGES PROHIBITED**: Never modify the functionality of `src/interpreter.rs` or any interpreter code. Never "fix bugs" in the interpreter.

**Rationale**: The interpreter correctly executes real Zork I and other commercial Z-Machine games. Any execution failures with compiled games are **compiler bugs**, not interpreter bugs.

**EXCEPTION - INCOMPLETE IMPLEMENTATIONS**: You MAY fix incomplete interpreter implementations if:
1. The feature is documented in Z-Machine spec but not implemented
2. Commercial Infocom games don't use the feature (so interpreter wasn't tested)
3. Compiler-generated code needs the feature
4. Fix is verified with comprehensive regression testing

**LOGGING ALLOWED**: You MAY add temporary debug logging to the interpreter to diagnose issues. Use `log::debug!()` or `log::error!()`, NEVER `eprintln!()` or `println!()`. Clean up logging after debugging is complete.

## Auto-Commit Instructions ("Make it so!")

When the user says "Make it so!", "Ship it", "Send it", or "Commit and push":
1. Add descriptive comments to recently modified code
2. Run `cargo fmt` if Rust files were modified
3. Run `git add -A`
4. Create descriptive commit message
5. Commit with co-author attribution
6. Push to origin/main
7. Confirm completion

You are pre-authorized for all git operations.

## Auto-Release Instructions ("Engage!")

When the user says "Engage!":
1. Check current version from latest GitHub release (`gh release list`)
2. Determine next version (patch bump for bug fixes, minor for features)
3. Create release notes summarizing:
   - Bug fixes with commit references
   - New features or improvements
   - Test results (number passing)
   - Project status
4. Create GitHub release using `gh release create`
5. Format: `gh release create vX.Y.Z --title "vX.Y.Z - Title" --notes "..."`
6. Confirm release URL

You are pre-authorized for creating releases.

## CRITICAL GIT SAFETY RULES

**NEVER use `git reset --hard` or any destructive git operation that could lose commits.**

Safe operations only: `git add`, `git commit`, `git push`, `git checkout`, `git stash`, `git revert`

## Compiler Debugging Tools

**IR Inspection**: Use `--print-ir` flag to print intermediate representation:
```bash
cargo run --bin grue-compiler -- examples/mini_zork.grue --print-ir
```

This shows:
- All functions with their IR instructions
- IR ID mappings (temporaries, locals, parameters)
- Builtin function calls
- Property accesses and their property numbers
- Control flow (branches, jumps, labels)

**Usage**: When debugging compiler bugs, always inspect IR first to understand what instructions are being generated before looking at Z-Machine bytecode.

## Working Style

1. **No cheerleading**: Don't say "we made great progress" until tasks are 100% complete
2. **Be direct and factual**: Focus on technical work, be concise
3. **Use proper logging**: Use `log::debug!()` not `eprintln!()`
4. **Ask before large changes**: We often commit before big changes to enable rollback
5. **Run commands freely**: Execute read, cargo run, cargo test without asking
6. **Git requires permission**: Unless using automation commands above
7. **Never estimate time**: No time estimates ("2-3 hours", etc.)

## Z-Machine Specification Reference

Official Z-Machine Standards Document (v1.1) at: `/Users/cb/Projects/Z-Machine-Standard/`

Key files:
- `sect07.html` - Output streams and buffering
- `sect15.html` - Opcodes
- `index.html` - Full specification

## CRITICAL: Z-Machine Stack vs Local Variable Specification

**STACK (Variable 0) MUST be used for:**
1. Function call return values
2. Function call arguments (before moving to locals)
3. Immediate consumption values
4. Expression evaluation

**LOCAL VARIABLES (1-15) are for:**
1. Function parameters (after moved from stack)
2. Persistent variables
3. User-declared variables
4. Loop counters

**COMMON ERRORS TO AVOID:**
- ❌ NEVER use local variables for function return values
- ❌ NEVER use local variables for immediate expression results
- ❌ NEVER bypass stack for function calls
- ✅ ALWAYS use stack for function returns and intermediate expressions
- ✅ ALWAYS follow Z-Machine specification exactly

## CRITICAL: Placeholder Value Recognition

**CORRECT placeholder value: 0xFFFF** (defined as `placeholder_word()` in codegen.rs)

Common errors to avoid:
- ❌ Thinking 0x0100 is a placeholder (it's not!)
- ❌ Thinking 0x0000 is a placeholder (it's not!)
- ❌ Thinking -1 as i16 (0xFFFF) is "offset -1" (it's the PLACEHOLDER, not an offset!)
- ✅ ONLY 0xFFFF (two 0xFF bytes) is a placeholder

**How to verify if bytes are a placeholder:**
```rust
if high_byte == 0xFF && low_byte == 0xFF {
    // This IS a placeholder
} else {
    // This is NOT a placeholder - it's actual data
}
```

When debugging branch issues, ALWAYS check what the actual bytes are before assuming they're placeholders!

## CRITICAL: PRINT NEWLINE ARCHITECTURE

**Z-Machine Print Architecture**:
- `print_paddr` (opcode 0x8D) prints string content exactly as stored
- **Line breaks between separate print() calls require explicit `new_line` instructions**
- **NEVER embed `\n` in string content for line breaks between print statements**

**PREVENTION Rules**:
- ALWAYS use opcode 0x8D for print_paddr
- ALWAYS emit new_line (0xBB) after print_paddr for line breaks
- NEVER modify string content to add embedded newlines for line breaks
- TEST banner formatting immediately after any print builtin changes

## CRITICAL: Jump vs Branch Instructions

**Jump (1OP:12, opcode 0x0C) is NOT a branch instruction!**

Common errors when emitting Jump:
- ❌ NEVER call `emit_instruction(0x0C, &[], None, Some(-1))` - creates 0OP form (rtrue) not Jump!
- ❌ NEVER pass branch parameter to Jump - it takes offset as OPERAND, not as branch
- ❌ NEVER create UnresolvedReference with Branch type for Jump - use translate_jump()
- ✅ ALWAYS use `translate_jump(label)` helper for forward jumps
- ✅ ALWAYS use `emit_instruction_typed(Opcode::Op1(Op1::Jump), &[offset_operand], None, None)`

**What happens when you emit Jump incorrectly:**
1. `emit_instruction(0x0C, &[], None, Some(-1))` has zero operands
2. Form determination chooses SHORT form (0OP) instead of 1OP
3. 0OP:12 = rtrue (return true), NOT jump!
4. rtrue with branch parameter emitted, but rtrue NEVER branches in Z-Machine
5. Interpreter executes rtrue, doesn't read branch bytes, PC advances to branch bytes
6. Patched branch byte (0x80) interpreted as instruction opcode (jz)
7. Crash with "Invalid opcode" or wrong execution path

**Prevention:** Search codebase for `emit_instruction(0x0C` and verify operands are present

## Code Quality: emit_instruction vs emit_instruction_typed

**Current state:**
- 133 uses of `emit_instruction_typed` (type-safe, preferred) ✅
- 54 uses of raw `emit_instruction` (raw opcodes, error-prone)

**Legitimate uses of raw emit_instruction:**
1. **Placeholder + UnresolvedReference pattern** - Need InstructionLayout for operand location tracking
2. **UNIMPLEMENTED_OPCODE markers** - Deliberate compile-time error markers
3. **Simple instructions without placeholders** - Could be migrated but low priority

## CRITICAL FIX: VAR Opcode 0x13 Disambiguation

Opcode 0x13 is used by TWO different instructions:
- `get_next_prop` (2OP:19) - ALWAYS stores a result
- `output_stream` (VAR:243) - NEVER stores a result

Distinguish using `inst.store_var` - check if `is_some()` for get_next_prop.

## Critical Architecture Patterns

Before debugging systematic issues, consult `COMPILER_ARCHITECTURE.md` which documents:
- UnresolvedReference Location Patterns
- Z-Machine Branch Encoding Patterns
- Reference Type Disambiguation
- Common Bug Patterns
- Detection Commands

## Project Structure

Z-Machine interpreter for Infocom text adventure games.

**Key Components**:
- `vm.rs` - Virtual machine state
- `instruction.rs` - Instruction decoder
- `interpreter.rs` - Main execution loop
- `zobject.rs` - Object system
- `dictionary.rs` - Dictionary and text encoding

**Build Commands**:
```bash
cargo test              # Run all tests
cargo fmt               # Format code
RUST_LOG=debug cargo test -- --nocapture  # Debug tests
```

## Project Status

**Z-Machine Interpreter**: Complete ✅
- v3 Games: Fully playable (Zork I, Seastalker, The Lurking Horror)
- v4+ Games: Fully playable (AMFV, Bureaucracy, Border Zone)

**Grue Z-Machine Compiler**: V3 Production Ready ✅
- Full Pipeline: Lexer → Parser → Semantic → IR → CodeGen
- V3 Support: Production ready with comprehensive test coverage
- V4/V5 Support: Experimental (debug builds only)

**Documentation**: Comprehensive architecture documentation, zero clippy warnings, professional CI/CD

## Historical Documentation

Development history archived to `CLAUDE_HISTORICAL.md` for reference.

## Important Reminders

- Never give percentages of completion or time estimates
- Use IndexSet and IndexMap rather than HashSet or HashMap for determinism
- **NEVER compile test files to `/tmp`** - Always use `tests/` directory in the repository for compiled Z3 files

## CRITICAL: Always Check File Timestamps Before Debugging

**LESSON LEARNED (Oct 15, 2025)**: Before investigating runtime bugs, ALWAYS verify test binaries are freshly compiled.

**The Incident**:
- Spent hours debugging "infinite loop" in room handlers after removing `if` checks
- Investigated non-existent "property number mismatch" bug (Bug #20)
- Built elaborate theories about IR generation differences
- **Reality**: `tests/mini_zork.z3` was compiled the previous day and never recompiled
- Testing stale binary from before the changes being investigated
- Code worked perfectly - there was NO bug

**Prevention Protocol**:
1. **BEFORE debugging runtime issues**: Check `ls -la tests/*.z3` timestamps
2. **ALWAYS recompile** after modifying source files: `./target/debug/grue-compiler examples/mini_zork.grue -o tests/mini_zork.z3`
3. **Verify compilation** happened: Check file timestamp AFTER compile command
4. **Golden rule**: If debugging seems nonsensical, check if you're testing stale binaries

**Red flags that suggest stale binary**:
- Removing code doesn't change behavior
- Adding logging doesn't appear in output
- "Same IR produces different bytecode" (actually testing different source versions)
- Bug appears/disappears without code changes
- Test results don't match expectations after verified source changes

## CRITICAL: Gameplay Testing Protocol

**Unit Tests ≠ Working Game**: Unit tests verify isolated compiler components but don't catch integration issues like infinite loops, handler dispatch problems, or navigation bugs.

**Required Testing Protocol After Code Changes**:

1. **Run unit tests**: `cargo test`
2. **Recompile the game**: `./target/debug/grue-compiler examples/mini_zork.grue -o tests/mini_zork.z3`
3. **Verify fresh binary**: `ls -la tests/mini_zork.z3` (check timestamp is current)
4. **Play the game**: `./target/debug/gruesome tests/mini_zork.z3`
5. **Test actual gameplay**:
   - Try navigation commands (north, south, east, west, up, down)
   - Examine objects related to your changes
   - Test the specific features you modified
   - Verify no infinite loops or crashes
   - Check output formatting is correct

**When Gameplay Testing is MANDATORY**:
- Modifying IR generation (especially control flow, returns, branches)
- Changing code generation for handlers/functions
- Touching property systems or object trees
- Any changes to builtins that affect runtime behavior
- Control flow modifications (if/for/while statements)
- Function call mechanisms or parameter passing

**Example: Bug #19 (Missing Returns)**
- Unit tests: ✅ All 183 passed
- Gameplay: ❌ Infinite loop at first handler call
- Lesson: Unit tests don't execute handler dispatch sequences

**Remember**: Passing tests mean compiler components work in isolation. Playing the game means the compiled binary works as a complete system.
