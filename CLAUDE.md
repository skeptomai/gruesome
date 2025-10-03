# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 2, 2025) - OPCODE CLEANUP COMPLETE ✅

**SYSTEMATIC FIX**: All call instruction opcodes standardized to use raw opcode 0x00 instead of mixed encoded bytes (0xE0, 0x20).

### ✅ CALL OPCODE STANDARDIZATION (Session Oct 2, 2025):

**Architecture Clarified**: `emit_instruction()` expects raw opcodes (0x00-0x1F) OR fully encoded bytes (0x80+), NOT the range 0x20-0x7F.

**Changes Made**:
- Line 2591: `0xE0` → `0x00` for call_vs
- Line 2677: `0xE0` → `0x00` for call_vs
- Line 4170: `0x20` → `0x00` for call_vs (main loop call)
- Line 4997: `0xE0` → `0x00` for call_vs (user main call)
- Lines 6378-6379: Removed `args.len()` match, now always uses `0x00`
- Line 6295: Marked unused `generate_call()` with `#[allow(dead_code)]`

**Rationale**: call_vs (VAR:224) has raw opcode 0x00. The `emit_instruction` function handles VAR form encoding internally (0xE0 = 0xC0 | 0x20 | 0x00). Using encoded bytes was architecturally incorrect.

**Verification**:
- ✅ All 170 tests passing (108 compiler, 53 unit, 4 integration, 5 disasm)
- ✅ Commercial games (Zork I, AMFV) confirmed working
- ✅ No regressions introduced
- ✅ Consistent opcode usage throughout compiler

## 📊 GRAMMAR SYSTEM ARCHITECTURE REVIEW (October 2, 2025)

**STATUS**: Comprehensive architecture review complete. System is fully implemented with proper control flow and placeholder resolution.

### ✅ ARCHITECTURE DOCUMENTATION (Session Oct 2, 2025):

#### **Complete Control Flow** (Lines 5048-5230):
1. **Main Loop** (ID 9000):
   - Prints prompt "> "
   - Reads user input via SREAD into buffers (Globals 109, 110)
   - Calls `generate_command_processing()`
   - Jumps back to start (infinite loop)

2. **Command Processing** (Line 5192):
   - Calls `generate_grammar_pattern_matching()` for all verb handlers
   - Falls through to "unknown command" message if no match
   - Returns to main loop via jump at line 5159

3. **Grammar Pattern Matching** (Lines 5241-5673):
   - For each verb: `generate_verb_matching()`
   - Extracts parse buffer tokens (word 1 = verb, word 2 = noun)
   - Compares verb with dictionary address
   - Branches to verb-only or verb+noun handlers
   - All jumps properly registered with UnresolvedReference system

#### **Verb Matching Logic** (Lines 5263-5673):
**Phase 1: Parse Buffer Access**
- Global 110 (G6e) = parse buffer address
- Load word count from buffer[1]
- Load word 1 dictionary address from buffer[2-3]

**Phase 2: Verb Dictionary Comparison**
- `lookup_word_in_dictionary()` calculates expected verb address
- Compare parsed verb with expected verb
- Branch if no match (try next verb)
- Branch if word_count < 2 (verb-only case)

**Phase 3.1: Verb+Noun Pattern**
- Load word 2 dictionary address from buffer[6-7]
- Call `generate_object_lookup_from_noun()` → Variable(3) = object ID
- Call handler with object parameter
- Jump to end of function

**Phase 3.2: Verb-Only Pattern**
- Call default handler with no parameters OR
- Call noun handler with object ID 0
- Fall through to end label

#### **Object Lookup System** (Lines 5721-5887):
**CRITICAL ARCHITECTURAL FIX**: Dynamic loop-based dictionary→object mapping implemented (Sept 30, 2025).

**Algorithm**:
1. Initialize result Variable(3) = 0 (not found)
2. Loop through objects 1-68 using Variable(4) as counter
3. For each object: `get_prop(object, 7)` → Variable(5) = name dictionary address
4. Compare Variable(5) with Variable(2) (noun dictionary address)
5. If match: Store object ID in Variable(3), exit loop
6. If no match after all objects: Variable(3) remains 0

**Status**: Fully implemented with proper UnresolvedReference system for all branches/jumps.

#### **Handler Invocation**:
- **Verb+Noun**: `call_vs(handler, object_id)` → stack
- **Verb-Only**: `call_vs(handler)` → stack
- Handlers return to grammar pattern matching
- Pattern matching returns to command processing
- Command processing falls through to main loop jump

#### **Control Flow Summary**:
```
Main Loop (9000)
  ↓
Print Prompt
  ↓
SREAD (user input)
  ↓
Command Processing
  ↓
Grammar Pattern Matching (for each verb)
  ↓
Verb Matching Logic
  ├─ No match: try next verb
  ├─ Verb+Noun match: Object Lookup → Handler(object) → return
  └─ Verb-Only match: Handler() → return
  ↓
Unknown Command Message
  ↓
Jump to Main Loop Start (infinite loop)
```

#### **UnresolvedReference Usage**:
✅ All placeholders properly resolved:
- Function calls: `LegacyReferenceType::FunctionCall` with `is_packed_address=true`
- Conditional branches: `LegacyReferenceType::Branch` with target labels
- Unconditional jumps: `LegacyReferenceType::Jump` with target labels
- String references: `LegacyReferenceType::StringRef` with packed addresses

#### **Critical Patterns Followed**:
✅ Layout-based operand tracking (no hardcoded offsets)
✅ Unique label IDs via hashing (no conflicts)
✅ Proper label registration before/after emission
✅ Branch placeholders = 0x7FFF
✅ Jump placeholders = 0xFFFF (via `placeholder_word()`)

### ⚠️ KNOWN ISSUE: Grammar System Runtime Error (Pre-existing)

**Error**: "Failed to decode instruction at 01017: Invalid Long form opcode 0x00 at address 1017"

**Status**: Pre-existing issue in golden file `tests/golden_files/mini_zork_v3.z3`, not caused by recent changes.

**Root Cause Investigation**:
- Bytes at PC 0x1014: `1b 9e 10` (set_colour instruction - V5+ only, inappropriate for V3)
- Interpreter mis-decodes at PC 0x1014, then PC doesn't advance properly
- Eventually tries to decode byte 0x00 at PC 0x1017, which fails validation
- Same bytes in both new compilation and golden file - confirms pre-existing

**Why set_colour in V3?**:
- set_colour (opcode 0x1B) is defined for V5+ only per Z-Machine spec
- Appears in compiled V3 mini_zork at PC 0x1014
- Not explicitly emitted by compiler (SET_COLOUR constant defined but unused)
- Likely part of data/lookup table being incorrectly executed
- OR control flow issue causing execution to jump into wrong memory region

**Current Assessment**: Architecture is complete and correct. Runtime error likely indicates placeholder resolution issue or incorrect memory layout calculation causing PC to execute data as code.

**Next Steps**: Debug actual runtime execution to find where PC 0x1014 gets reached incorrectly.

## 🚨 CRITICAL: PRINT NEWLINE ARCHITECTURE - DO NOT BREAK AGAIN

**THIS HAS BEEN BROKEN AND FIXED MULTIPLE TIMES - REMEMBER THE CORRECT PATTERN**

### ✅ CORRECT Implementation (Working as of Sep 13, 2025):

**Z-Machine Print Architecture**:
- `print_paddr` (opcode 0x8D) prints string content exactly as stored
- **Line breaks between separate print() calls require explicit `new_line` instructions**
- **NEVER embed `\n` in string content for line breaks between print statements**

**Working Implementation** in `src/grue_compiler/codegen_builtins.rs`:
```rust
// Generate print_paddr instruction
self.emit_instruction(0x8D, &[Operand::LargeConstant(0x0000)], None, None)?;
// Add unresolved string reference
self.reference_context.unresolved_refs.push(reference);
// Emit new_line instruction AFTER print_paddr for line breaks
self.emit_instruction(0xBB, &[], None, None)?;  // new_line opcode (0OP:11)
```

**Working Output**:
```
DORK I: The Last Great Empire
Copyright (c) 2025 Grue Games. All rights reserved.
ZORK is a registered trademark of Infocom, Inc.
DORK is .... not
Revision 1 / Serial number 8675309
```

### ❌ WRONG Approaches (These BREAK the formatting):

1. **WRONG**: Using opcode 0x0D instead of 0x8D
   - 0x0D = `get_next_prop`, not `print_paddr`
   - 0x8D = `print_paddr` (1OP:141)

2. **WRONG**: Embedding newlines in string content
   - Adding `format!("{}\n", string_value)` to strings
   - This makes strings longer and breaks address calculations

3. **WRONG**: Using only embedded newlines without separate new_line instructions
   - Z-Machine doesn't automatically add line breaks between print statements
   - Each `print()` call needs explicit `new_line` instruction for line breaks

### 🔒 PREVENTION Rules:
- **ALWAYS** use opcode 0x8D for print_paddr
- **ALWAYS** emit new_line (0xBB) after print_paddr for line breaks
- **NEVER** modify string content to add embedded newlines for line breaks
- **TEST** banner formatting immediately after any print builtin changes

## Auto-Commit Instructions ("Make it so!")

When the user says any of the following:
- "Make it so!"
- "Ship it"
- "Send it"
- "Commit and push"
- "comment, commit, push"

You should automatically:
1. Add a descriptive comment to any recently modified code (if not already commented)
2. Run `cargo fmt` if any Rust files were modified
3. Run `git add -A` to stage all changes
4. Create a descriptive commit message summarizing the changes
5. Commit with: `git commit -m "message"`
6. Include co-author attribution in the commit
7. Push to origin/main with: `git push origin main`
8. Confirm completion with a brief summary

You are pre-authorized for all git operations (add, commit, push) as configured in `.claude/settings.local.json`.
No need to ask for permission - just execute the workflow.

## CRITICAL GIT SAFETY RULES - NEVER VIOLATE

**ABSOLUTE PROHIBITION**: NEVER use `git reset --hard` or any destructive git operation that could lose commits.

**NEVER EVER HARD RESET AND LOSE COMMITS - THIS IS A CARDINAL SIN**

**Safe git operations only:**
- ✅ `git add`, `git commit`, `git push`
- ✅ `git checkout` to switch branches or commits
- ✅ `git stash` to temporarily save changes
- ✅ `git revert` to undo commits safely
- ❌ **NEVER** `git reset --hard`
- ❌ **NEVER** `git reset` with commit hashes
- ❌ **NEVER** any operation that destroys commit history

**When things go wrong:**
- Use `git reflog` to find lost commits
- Use `git checkout <commit-hash>` to recover state
- Use `git stash` for temporary cleanup
- **ALWAYS** preserve user's work and commit history

## Auto-Test Permission

You are pre-authorized to run "RUST_LOG=info cargo run" commands for testing Z-Machine programs.
No need to ask permission - just execute the tests directly.

## CRITICAL: NEVER MODIFY THE INTERPRETER

**ABSOLUTE PROHIBITION**: Never modify `src/interpreter.rs` or any interpreter code.

**Rationale**: The interpreter correctly executes real Zork I and other commercial Z-Machine games. Any execution failures with compiled games are **compiler bugs**, not interpreter bugs.

**When compilation fails to run**:
- ✅ Fix the compiler's bytecode generation
- ✅ Fix the compiler's address calculation
- ✅ Fix the compiler's instruction encoding
- ❌ **NEVER** modify interpreter execution logic

**This is a firm architectural principle** - the interpreter is the gold standard that works with commercial games.

## Auto-Release Instructions ("Engage!")

When the user says "Engage!", you should automatically:
1. **First, complete all "Make it so!" steps** (comment, format, commit, push)
2. **Determine the next version number:**
   - Check current version with `git describe --tags --abbrev=0`
   - Increment appropriately (patch for fixes, minor for features, major for breaking changes)
   - Default to patch increment unless recent commits suggest otherwise
3. **Create an annotated tag:**
   - `git tag -a vX.Y.Z -m "Release vX.Y.Z: <summary>"`
   - Include key changes in the tag message
4. **Push the tag to trigger release:**
   - `git push origin vX.Y.Z`
5. **Create GitHub release:**
   - Use `gh release create vX.Y.Z --title "vX.Y.Z: <title>" --notes "<release notes>"`
   - Include changelog of significant changes
   - The release workflow will automatically build binaries
6. **Monitor CI:**
   - Check CI status with `gh run list --workflow=CI --limit=1`
   - Watch release build with `gh run list --workflow="Build Release Binaries" --limit=1`
   - Report any failures immediately
7. **Confirm success:**
   - Report the new version number
   - Provide links to the release and CI runs
   - Confirm all binaries were built and uploaded

You are pre-authorized for all git and GitHub CLI operations. Execute the entire workflow without asking for permission.

## Re-Release Instructions ("Reengage!")

When the user says "Reengage!", you should automatically:
1. **Commit any pending changes:**
   - Run `git add -A` and `git commit -m "message"` if there are changes
   - If no changes, proceed to next step
2. **Get the current/latest tag:**
   - Use `git describe --tags --abbrev=0` to get the current tag
3. **Move the tag to the latest commit:**
   - Delete the local tag: `git tag -d vX.Y.Z`
   - Delete the remote tag: `git push origin --delete vX.Y.Z`
   - Recreate tag at current commit: `git tag -a vX.Y.Z -m "Re-release vX.Y.Z: <reason>"`
4. **Push the updated tag:**
   - Force push the tag: `git push origin vX.Y.Z --force`
5. **Delete and recreate the GitHub release:**
   - Delete existing release: `gh release delete vX.Y.Z --yes`
   - Recreate with same version: `gh release create vX.Y.Z --title "vX.Y.Z: <title>" --notes "<updated notes>"`
6. **Monitor the new CI/release builds:**
   - Check CI status
   - Confirm release binaries are rebuilt
7. **Report completion:**
   - Confirm tag has been moved
   - Provide links to new CI runs
   - Verify binaries are updated

This is useful when you need to fix something in a release without incrementing the version number.
You are pre-authorized for all operations. Execute without asking for permission.

## Working Style and Behavioral Expectations

1. **No cheerleading or sycophancy**: Don't say "we made great progress" or similar until tasks are 100% complete. Nothing in between counts.
2. **Be direct and factual**: Focus on the technical work. Be concise. Skip the pleasantries.
3. **Use proper debug logging**: In Rust code, use `log::debug!()` not `eprintln!()` for debug output
4. **Ask before large changes**: For substantial code edits, ask first. We often commit before big changes to enable rollback.
5. **Run commands freely**: Execute read, cargo run, cargo test, make, etc. without asking permission
6. **Git requires permission**: Unless using the automation commands above, ask before git commits
7. **Never estimate time**: Do not provide time estimates ("2-3 hours", "1-2 weeks", etc.) since AI development work rate doesn't match traditional estimates

## Z-Machine Specification Reference

The official Z-Machine Standards Document (v1.1) is available locally at:
`/Users/cb/Projects/Z-Machine-Standard/`

Key files:
- `sect07.html` - Output streams and buffering behavior
- `sect15.html` - Opcodes including read, read_char, buffer_mode
- `index.html` - Full specification index

**Critical Understanding**: Z-Machine "buffer mode" controls word-wrapping to prevent words from splitting across lines. It does NOT control display timing - all text should appear immediately.

## CRITICAL: Disassembler Address Offset (TXD Compatibility Issue)

**IMPORTANT DEBUGGING INSIGHT**: TXD and other disassemblers subtract 1 byte from the header's initial PC for alignment purposes. This is NOT a compiler bug.

- **TXD Code**: `initial_pc = start_pc - 1` (line 138 in disasm_txd.rs)
- **Effect**: If header says PC=0x035c, TXD reports PC=0x035b
- **This is NORMAL**: Disassemblers do this for their own alignment calculations
- **Don't Fix**: The compiler's PC calculation is correct, don't adjust it to match disassemblers
- **Remember**: When comparing disassembler output to interpreter execution, account for this 1-byte difference

This has been a recurring source of debugging confusion. The compiler-generated PC addresses are correct for the Z-Machine specification.

## CRITICAL: Z-Machine Stack vs Local Variable Specification Compliance

**CRITICAL ARCHITECTURAL PATTERN**: The Z-Machine specification mandates specific usage of stack vs local variables. Violating this causes runtime errors and is a recurring compiler bug.

### **STACK (Variable 0) MUST be used for:**

1. **Function call return values** - Z-Machine functions MUST return on stack
   - All `call*` instructions store results to stack by default
   - `call routine -> (result)` means result goes to stack (variable 0)
   - From Z-Machine spec sect15: `call`, `call_1s`, `call_2s`, `call_vs`, `call_vs2` all store to stack

2. **Function call arguments** - Parameters are passed via stack before being moved to locals
   - Arguments pushed to stack, then moved to local variables 1-15 by routine header
   - From Z-Machine spec sect06.4.4: "arguments are written into the local variables (argument 1 into local 1 and so on)"

3. **Immediate consumption** - Values used once by the next instruction
   - Temporary results in expression evaluation
   - Intermediate values that don't need persistence

4. **Expression evaluation** - Complex expressions generate stack operations
   - Binary operations: operands → stack → operation → result to stack
   - Ternary conditionals: condition evaluation uses stack for intermediate values

### **LOCAL VARIABLES (1-15) are for:**

1. **Function parameters** - After being moved from stack to local slots by routine setup
2. **Persistent variables** - Values that live across multiple instructions
3. **User-declared variables** - Variables declared in the source code
4. **Loop counters and control variables** - Values that need to persist across control flow

### **COMMON COMPILER ERRORS TO AVOID:**

❌ **NEVER use local variables for function return values**
❌ **NEVER use local variables for immediate expression results**
❌ **NEVER try to "fix" stack usage by converting to local variables**
❌ **NEVER bypass stack for function calls thinking it's "cleaner"**

✅ **ALWAYS use stack for function returns, intermediate expressions, immediate consumption**
✅ **ALWAYS use local variables only for persistent, named variables**
✅ **ALWAYS follow the Z-Machine specification exactly for variable usage**

### **WHY THIS MATTERS:**
- Stack management is NOT broken in the interpreter - it works correctly with Zork I
- Stack "overflow" errors are usually compiler bugs misusing local variables for stack operations
- The stack is designed for rapid push/pop operations that don't need persistence
- Local variables are for named, persistent storage that survives across instructions

**From Z-Machine spec sect06.3**: "Writing to the stack pointer (variable number $00) pushes a value onto the stack; reading from it pulls a value off."

**From Z-Machine spec sect06.4**: "All routines return a value" and "Routine calls preserve local variables and the stack (except when the return value is stored in a local variable or onto the top of the stack)."

**CRITICAL**: This pattern has caused repeated bugs. When implementing binary operations, function calls, or control flow, ALWAYS check: "Should this use stack or local variable according to the Z-Machine specification?"

## Grammar System Implementation Plan (Sep 28, 2025)

### Execution Plan for Grammar System Development

#### Phase 1: Fix Property System Foundation (CURRENT PRIORITY)
**Blocker**: Property placeholders return 65534 instead of valid object references
- **Root Cause**: Property data generation uses placeholder values instead of resolved references
- **Impact**: `obj.name` property access fails, blocking object name resolution
- **Required For**: Grammar system needs working property access for object identification

**Implementation Steps**:
1. **Debug property data generation**: Trace why properties store 65534 placeholders
2. **Fix property value resolution**: Ensure property data contains valid addresses/values
3. **Verify property access**: Test `obj.name` returns proper string content
4. **Validate object system**: Confirm all object properties work correctly

#### Phase 2: Basic Grammar Pattern Matching
**Scope**: Handle simple verb+noun patterns
- **Target**: Implement basic grammar pattern recognition engine
- **Example**: `verb "examine" { noun => examine($noun) }`
- **Requirement**: Working property system from Phase 1

#### Phase 3: Object Name Resolution
**Scope**: Match input words to visible objects
- **Target**: Convert "mailbox" → actual mailbox object reference
- **Challenge**: Handle aliases ("small mailbox" → same object)
- **Requirement**: Property access for object name matching

#### Phase 4: Multi-word Noun Phrases
**Scope**: Handle complex object references
- **Target**: "jewel-encrusted egg" → parse as single object reference
- **Algorithm**: Longest-match-first with disambiguation prompts
- **Challenge**: 3 dictionary words → 1 object resolution

#### Phase 5: Advanced Grammar Patterns
**Scope**: Prepositions, multiple objects, context validation
- **Target**: `verb "put" { noun + "in" + noun => handle_put_in($1, $3) }`
- **Integration**: Complete natural language processing pipeline

### Property System Fix Plan (IMMEDIATE PRIORITY)

**Current Problem**:
```rust
// Property values stored as 65534 placeholders instead of object references
obj.name → returns address but displays nothing
test_obj.property → 65534 instead of valid value
```

**Investigation Strategy**:
1. **Trace property generation**: Find where 65534 placeholders originate
2. **Check UnresolvedReference system**: Verify property references get resolved
3. **Debug object table layout**: Ensure property data structure is correct
4. **Fix placeholder resolution**: Replace 65534 with actual property values

**Key Files to Examine**:
- `src/grue_compiler/codegen.rs` - Object and property generation
- `src/grue_compiler/codegen_instructions.rs` - Property instruction emission
- Property access code paths in instruction generation

**Success Criteria**:
- `obj.name` returns readable string content
- Property access works without crashes
- Object system supports grammar pattern matching

## CRITICAL FIX: VAR Opcode 0x13 Disambiguation

**PROBLEM**: Opcode 0x13 is used by TWO different instructions that share the same hex value:
- `get_next_prop` (2OP:19, hex 0x13) - **ALWAYS stores a result**
- `output_stream` (VAR:243, hex 0x13) - **NEVER stores a result**

**SOLUTION**: In the VAR section execute_var() method, distinguish using `inst.store_var`:

```rust
0x13 => {
    // This opcode can be either get_next_prop or output_stream in VAR form
    // get_next_prop stores a result, output_stream does not
    if inst.store_var.is_some() {
        // This is get_next_prop (VAR form of 2OP:19)
        let obj_num = operands[0];
        let prop_num = if operands.len() >= 2 { operands[1] as u8 } else { 0u8 };
        debug!("VAR get_next_prop: obj={:04x}, prop={}", obj_num, prop_num);
        let next_prop = self.vm.get_next_property(obj_num, prop_num)? as u16;
        if let Some(store_var) = inst.store_var {
            self.vm.write_variable(store_var, next_prop)?;
        }
        return Ok(ExecutionResult::Continue);
    }

    // output_stream (no result storage)
    // ... rest of output_stream implementation
}
```

**WHY THIS MATTERS**:
- AMFV calls VAR get_next_prop with 1 operand during startup
- Without this fix, AMFV crashes with "Unhandled object opcode: 13 with operand count VAR"
- Using heuristics (operand count, value ranges) breaks status line display
- Only the specification-compliant approach (checking store_var) works correctly

**NEVER use heuristics** - Always check the Z-Machine specification at `/Users/cb/Projects/Z-Machine-Standard/sect14.html` and `sect15.html` for the definitive opcode behavior.

## Debugging Guidelines

For all debugging statements in this project, use the Rust `log` crate with `debug!` and `info!` macros instead of `println!`. This provides better control over debug output and follows Rust best practices.

### Usage:
- Use `debug!()` for detailed debugging information
- Use `info!()` for important runtime information
- Use `warn!()` for warnings
- Use `error!()` for errors

## Critical Architecture Patterns

**IMPORTANT**: Before debugging systematic issues, consult `COMPILER_ARCHITECTURE.md` which documents:

- **UnresolvedReference Location Patterns** - Critical timing of location recording vs placeholder emission
- **Z-Machine Branch Encoding Patterns** - Proper byte-level branch instruction formatting
- **Reference Type Disambiguation** - Jump vs Branch vs other reference types
- **Common Bug Patterns** - Systematic issues that have caused major failures
- **Detection Commands** - Specific grep/xxd commands to identify problematic patterns

This file prevents regression of major architectural bugs that took significant time to debug.

## Project Structure

This is a Z-Machine interpreter implementation for playing Infocom text adventure games, starting with Zork I.

### Key Components:
- `vm.rs` - Virtual machine state (PC, stack, variables)
- `instruction.rs` - Instruction decoder for all Z-Machine instruction formats
- `interpreter.rs` - Main execution loop
- `disassembler.rs` - Debugging disassembler
- `opcode_tables.rs` - Opcode mappings per Z-Machine spec v1.1
- `zobject.rs` - Object system with debug dumping
- `game.rs` - Game file loading and parsing
- `header.rs` - Z-Machine file header parsing
- `dictionary.rs` - Game dictionary and text encoding

### Testing:
- Unit tests for each component
- Integration tests that load actual Zork I data file
- Test game file located at: `resources/test/zork1/DATA/ZORK1.DAT`

### Build and Test Commands:
```bash
# Run all tests
cargo test

# Run tests with debug output
RUST_LOG=debug cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture

# Format source code files
rustfmt src/*.rs

# Or format all Rust files in the project
cargo fmt
```

## NULL Call Handling

The interpreter correctly handles calls to address 0x0000 according to the Z-Machine specification:
- In `do_call()` function in interpreter.rs, there's a special check: `if packed_addr == 0`
- When calling address 0, it returns 0 (false) without executing any code

## Project Status Summary

### Z-Machine Interpreter: Complete ✅
- **v3 Games**: Fully playable (Zork I, Seastalker, The Lurking Horror)
- **v4+ Games**: Fully playable (AMFV, Bureaucracy, Border Zone)
- All opcodes, timers, sound effects, and display features implemented
- Version-aware architecture with proper fallback handling

### Grue Z-Machine Compiler: V3 Production Ready, V4/V5 Experimental 🔄
- **Full Pipeline**: Lexer → Parser → Semantic → IR → CodeGen ✅
- **V3 Support**: Production ready with comprehensive test coverage ✅
- **V4/V5 Support**: Experimental - disabled in release builds due to known issues ⚠️
- **Release Policy**: V4/V5 compilation blocked in release builds, available in debug only
- **Test Organization**: V3 tests in CI, V4/V5 tests isolated in `tests/experimental/`
- **Binary Behavior**: Release binaries reject V4/V5, debug builds allow with warnings
- **Current Status**: V3 compilation stable and production-ready, V4/V5 development ongoing

### Documentation & Quality
- Comprehensive architecture documentation (13KB guide)
- Zero clippy warnings, consistent formatting
- Professional CI/CD with cross-platform releases
- Enhanced testing infrastructure with pre-CI validation
- Clean historical preservation system

## Historical Documentation

Development history and detailed implementation logs have been archived to `CLAUDE_HISTORICAL.md` for reference. This file is not automatically loaded but preserves all technical implementation details from the development process.

- never give me percentages of completion and never give me time estimates to complete tasks
- don't give percentages complete or percentage estimates for completion
- we want to maintain determinism build over build. Use IndexSet and IndexMap rather than HashSet or HashMap. The Index versions have deterministic enumeration
