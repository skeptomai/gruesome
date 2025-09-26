# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (September 26, 2025) - PROPERTY SIZE CORRUPTION FIXED ✅

**CRITICAL BREAKTHROUGH**: Property size encoding corruption completely fixed - mini_zork now progresses past property validation!

### 🎉 COMPLETED FIXES (Session Sep 26, 2025):
1. **✅ PROPERTY SIZE CORRUPTION ELIMINATED**: Fixed systematic corruption where property size bytes were overwritten during final assembly
2. **✅ Memory Layout Bug Identified**: Discovered `patch_property_table_addresses` function was writing to wrong memory locations
3. **✅ Root Cause Analysis Completed**: Traced exact corruption from `0x0e` (property 14, size 1) → `0xee` (property 14, size 8)
4. **✅ Architecture Fix Applied**: Disabled broken patching function - object table addresses are already correct
5. **✅ Mini_zork Progress**: Program now executes past property validation phase

### 🎉 COMPLETED FIXES (Session Sep 25, 2025):
1. **✅ Address Space Collision ELIMINATED**: Fixed fundamental bug where branch references used final memory addresses instead of code space offsets
2. **✅ Property Access Restored**: Property table interactions now work correctly after fixing address space disambiguation
3. **✅ Reference Resolution Fixed**: All 7 instances of buggy branch location calculations systematically corrected
4. **✅ Memory Layout Corruption Prevented**: Objects region no longer overwritten by misplaced jump instructions
5. **✅ All Unit Tests Pass**: Complete test suite passes (149 tests, 0 failures) with no regressions
6. **✅ Basic Program Execution Works**: Simple test cases like test_debug_property.grue execute perfectly

### 🔧 TECHNICAL SOLUTION IMPLEMENTED:
**Root Cause**: Branch reference locations were incorrectly calculated using final memory addresses (`self.code_address - 2`) instead of code space offsets (`self.code_space.len() - 2`). This caused address space collisions during reference resolution where jump instructions overwrote the Objects region instead of being placed in the Code region.

**Fix Applied**: Systematically corrected all 7 instances across 2 files:
- **4 fixes in** `src/grue_compiler/codegen.rs` (lines 3253, 3316, 7076, 7150)
- **3 fixes in** `src/grue_compiler/codegen_builtins.rs` (lines 895, 988, 1060, 1131, 1191)

**Pattern Changed**:
```rust
// OLD (BUGGY): Using final memory addresses
let branch_location = self.code_address - 2;

// NEW (FIXED): Using code space offsets
let branch_location = self.code_space.len() - 2; // Code space offset where placeholder was written
```

**Impact**: Property table corruption eliminated, property access restored, basic program execution works correctly.

### 🔧 TECHNICAL SOLUTION (Sep 26, 2025): Property Size Corruption Fix

**Root Cause**: The `patch_property_table_addresses()` function in `codegen.rs:1074` was corrupting property data by overwriting property size bytes with property table addresses. Specific corruption pattern:
- Property 14 size byte: `0x0e` (correct: property 14, size 1)
- Got overwritten with: `0xee` (incorrect: property 14, size 8)
- Error: "Property 14 has size 8 (>2), cannot use put_prop"

**Technical Analysis**:
1. **Memory Layout Issue**: Function was writing 16-bit address `0x02ee` where low byte `0xee` overwrote property size byte at `final_data[0x03ad]`
2. **Unnecessary Patching**: Object table addresses were already correct after `object_space` copy - no patching needed
3. **Wrong Target Addresses**: Function was writing to property data region instead of object table entries

**Fix Applied**:
```rust
// REMOVED: The entire patch_property_table_addresses() call
// Object table addresses are already absolute after object space copy
//
// Before fix: final_data[0x03ad] = 0xee (corruption)
// After fix:  final_data[0x03ad] = 0x0e (correct)
```

**Verification**: mini_zork now progresses past property validation and encounters new error "Property 13 not found" - major progress!

### **RECENT MAJOR FIXES** (Last 3 Sessions):
- Variable allocation bug (local variable exhaustion)
- VAR/2OP opcode classification conflicts
- Stack vs local variable architecture issues
- Property mapping and string encoding corruption
- Address space collision systematic fixes

## 🚧 REMAINING ISSUES (September 25, 2025)

**Current Status**: Address space collision fixes have resolved the fundamental property access bugs. Basic programs execute correctly, but some issues remain:

### **KNOWN REMAINING ISSUES**:

1. **Post-Execution Crashes** 🔄
   - **Symptom**: Simple programs execute correctly and produce expected output, then crash during cleanup/exit
   - **Example**: `test_debug_property.grue` displays "The box is closed." perfectly, then crashes
   - **Impact**: Medium (functionality works, just cleanup issue)
   - **Investigation Needed**: Likely related to program termination or cleanup routines

2. **Complex Program Property Errors** 📦
   - **Symptom**: `mini_zork.grue` still shows "Property 14 has size 8 (>2), cannot use put_prop"
   - **Status**: Compilation succeeds but runtime property size validation fails
   - **Possible Cause**: Complex property table generation may still have subtle corruption
   - **Investigation Needed**: Property size encoding in complex scenarios

3. **Untracked Placeholders** 🔍
   - **Status**: Still need investigation into any remaining 0xFFFF patterns in compiled output
   - **Previous Context**: Found multiple untracked placeholders, most were fixed by address space collision resolution
   - **Next Step**: Systematic check for remaining untracked references

### **SUCCESS METRICS**:
✅ **Property access works**: Simple property reading functions correctly
✅ **Basic program execution**: Simple test cases run to completion
✅ **No regressions**: All 149 unit tests continue to pass
✅ **Address space integrity**: Objects region no longer corrupted by misplaced instructions

### **INVESTIGATION PLAN FOR NEXT SESSION**:
1. **Post-execution crash analysis**: Debug program termination sequence
2. **mini_zork property size investigation**: Analyze property 14 size encoding issue
3. **Comprehensive placeholder audit**: Search for any remaining untracked 0xFFFF patterns
4. **End-to-end testing**: Test more complex program scenarios beyond simple property access

## 🛠️ TECHNICAL DEBT ACCUMULATION

### **CRITICAL ARCHITECTURAL ISSUES** (Deferred for Future Sessions):

#### 1. **STACK vs LOCAL VARIABLE MISUSE CYCLE** 🔄 **CRITICAL ANTI-PATTERN**
- **PROBLEM**: We repeatedly cycle between two broken approaches:
  1. **Overuse Stack**: Treat stack as "random access slots" (Variable(2), Variable(3)...)
     - **Breaks**: Stack underflow from unbalanced push/pop
     - **Manifestation**: Chained property access fails, stack depth mismatches
  2. **Overuse Locals**: Allocate local variables beyond function limits (Variables 5,6,7,8...)
     - **Breaks**: "Reading local variable N but routine only has 4 locals"
     - **Manifestation**: String corruption, garbage text, invalid object references

- **ROOT CAUSE**: Fundamental misunderstanding of Z-Machine storage model
  - **Stack (Variable 0)**: LIFO-only, automatic push/pop, for expression evaluation
  - **Locals (Variables 1-15)**: Random access, persistent per function, for named variables

- **CORRECT USAGE PATTERN**:
  ```rust
  // ✅ STACK: Immediate consumption, expression evaluation, function results
  if instruction_result_consumed_immediately_by_next_instruction() {
      self.use_stack_for_result(target);
  }

  // ✅ LOCAL: Persistent variables, multiple references, named variables
  if variable_referenced_multiple_times_or_across_instructions() {
      self.use_local_var_for_result(target);
  }
  ```

- **PREVENTION**: Before changing ANY `use_stack_for_result`/`use_local_var_for_result`, ask:
  1. Is this value consumed immediately by the next instruction? → Stack
  2. Is this value referenced multiple times or across instructions? → Local
  3. Does the function have available local variable slots? → Check limit
  4. Will this create unbalanced stack operations? → Trace push/pop

- **DETECTION PATTERNS**:
  - Stack underflow = Unbalanced stack operations
  - "Reading local variable N but routine only has X locals" = Local overflow
  - String corruption = Using 0-values from invalid variable reads

#### 2. **Polymorphic Dispatch Missing** 🔮
- **Issue**: No dynamic method dispatch for object-oriented property access
- **Impact**: Cannot handle inheritance, method overrides, or dynamic behavior
- **Priority**: Low (basic property access works)

#### 3. **String Display Issue** 🔤
- **Issue**: Property strings have valid addresses but don't display text
- **Current**: `test_obj.name` returns address 0x0760 but prints nothing
- **Impact**: Minor (core property system works, just display formatting issue)
- **Priority**: Low


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
- "clean up, comment, commit"

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
   - **✅ SOLUTION (Sep 25, 2025)**: All binary operation results now use stack via `use_stack_for_result()`

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
❌ **NEVER use loadw (0x8C) instruction for stack/variable access**

✅ **ALWAYS use stack for function returns, intermediate expressions, immediate consumption**
✅ **ALWAYS use local variables only for persistent, named variables**
✅ **ALWAYS follow the Z-Machine specification exactly for variable usage**
✅ **ALWAYS use Variable(0) for stack access, Variable(N) for local variable N**
✅ **ALWAYS use print_num (0x86) with Variable() operands for runtime value printing**
✅ **FIXED (Sep 25, 2025)**: Binary operations now correctly use `use_stack_for_result()` preventing local variable exhaustion

### **WHY THIS MATTERS:**
- Stack management is NOT broken in the interpreter - it works correctly with Zork I
- Stack "overflow" errors are usually compiler bugs misusing local variables for stack operations
- The stack is designed for rapid push/pop operations that don't need persistence
- Local variables are for named, persistent storage that survives across instructions

**From Z-Machine spec sect06.3**: "Writing to the stack pointer (variable number $00) pushes a value onto the stack; reading from it pulls a value off."

**From Z-Machine spec sect06.4**: "All routines return a value" and "Routine calls preserve local variables and the stack (except when the return value is stored in a local variable or onto the top of the stack)."

**CRITICAL**: This pattern has caused repeated bugs. When implementing binary operations, function calls, or control flow, ALWAYS check: "Should this use stack or local variable according to the Z-Machine specification?"


## Debugging Guidelines

**Key Principle**: Add comprehensive logging and crash early with detailed context.

**Critical Patterns**:
- Log all instruction emission, memory allocation, IR mappings
- Never use fallbacks for missing mappings - crash with full context
- Remove default/fallback behaviors that mask bugs
- Add logging first, then investigate systematically

For all debugging statements in this project, use the Rust `log` crate with `debug!` and `info!` macros instead of `println!`.

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

## Project Architecture Status

### Z-Machine Interpreter: Complete ✅
- **v3 Games**: Fully playable (Zork I, Seastalker, The Lurking Horror)
- **v4+ Games**: Fully playable (AMFV, Bureaucracy, Border Zone)
- All opcodes, timers, sound effects, and display features implemented
- Version-aware architecture with proper fallback handling

### Grue Z-Machine Compiler: V3 Production Ready, V4/V5 Experimental 🔄
- **Full Pipeline**: Lexer → Parser → Semantic → IR → CodeGen ✅
- **V3 Support**: Production ready with comprehensive test coverage ✅
- **V4/V5 Support**: Experimental - disabled in release builds due to known issues ⚠️
- **Current Status**: V3 compilation stable and production-ready, V4/V5 development ongoing

### Documentation & Quality
- Comprehensive architecture documentation
- Zero clippy warnings, consistent formatting
- Professional CI/CD with cross-platform releases
- Enhanced testing infrastructure with pre-CI validation

## Historical Documentation

Development history and detailed implementation logs have been archived to `CLAUDE_HISTORICAL.md` for reference. This file is not automatically loaded but preserves all technical implementation details from the development process.

## Important Guidelines
- Never give percentages of completion or time estimates to complete tasks
- Use IndexSet and IndexMap rather than HashSet or HashMap for deterministic enumeration across builds