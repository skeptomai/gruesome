# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (September 10, 2025) - MASSIVE COMPILER BREAKTHROUGH ✅

**BREAKTHROUGH SUCCESS**: Fixed PC calculation inconsistency causing widespread bytecode corruption!

### ✅ LATEST MAJOR FIX (Session Sep 10, 2025):

**PC Calculation Corruption Fix** - Root cause: Inconsistent PC calculations between two code paths
- Early calculation (line 885): Used correct `init_routine_locals_count`  
- Final calculation (line 1009): Hardcoded 1-byte assumption
- **Impact**: PC pointed to wrong addresses (e.g., 0x0355 instead of 0x0354)
- **Result**: Execution started in middle of instructions, causing "Invalid opcode 0x00" errors

### 📊 MASSIVE PROGRESS - Test Success Rate:
- **Before Session**: 27 passing / 14 failing tests (66% success)
- **After PC Fix**: 31 passing / 6 failing tests (84% success)
- **Net Improvement**: +8 additional tests now passing
- **Impact Factor**: Single architectural fix resolved 57% of remaining failures

### 🎯 REMAINING WORK (6 tests):
1. **Execution Flow Issues** (4 tests) - Runtime jumps to invalid addresses
2. **Instruction Operand Issues** (1 test) - `sread` missing operands  
3. **Memory Bounds Issues** (1 test) - Branch targets outside memory

### ✅ COMPLETED FIXES (Previous Sessions):
1. **Branch System Fixed** - UnresolvedReference system for conditional branches
2. **Object Mapping Fixed** - Unique Z-Machine object numbers
3. **IR ID Mapping Fixed** - Array instruction target registration
4. **PC Calculation Fixed** - Unified calculation logic for all scenarios

**Current Status**: Compiler now generates functionally correct Z-Machine bytecode for 84% of test cases. Remaining issues are edge cases in specific instruction patterns.

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

## CRITICAL: Z-Machine Stack vs Local Variable Usage - Aug 28, 2025

**FUNDAMENTAL PRINCIPLE**: When questioning stack vs local variable usage, refer to the Z-Machine specification - it's almost always in favor of the stack.

### **Stack Usage (Preferred)**:
- **Temporary expression results** (comparisons, arithmetic, property access)
- **Intermediate calculations** 
- **Function call arguments** (pushed before call)
- **Function return values** (returned on stack)
- **Immediate consumption values** (used once then discarded)

### **Local Variable Usage (Limited)**:
- **Function parameters** (persistent throughout function)
- **Loop variables** (persistent across iterations)
- **Explicit variable declarations** in source code
- **Values that need to be stored/retrieved multiple times**

### **Key Z-Machine Specification Points**:
- Variable(0) = stack top
- Instructions without store_var push results to stack  
- Instructions with store_var = None consume from stack
- Stack operations are more efficient than variable storage
- Most Z-Machine instructions are designed for stack-based computation

## CRITICAL: Systematic Debugging Patterns - Learned Aug 27, 2025

**FUNDAMENTAL DEBUGGING PRINCIPLE**: Add comprehensive logging to all shared emission paths and crash early with detailed context to aid debugging.

### **Always Add Logging To:**

1. **Instruction Emission** (`emit_instruction`):
   ```rust
   log::debug!("EMIT: opcode=0x{:02x} operands={:?} store={:?} branch={:?} at address=0x{:04x}", 
               opcode, operands, store_var, branch_offset, self.current_address);
   ```

2. **Memory Allocation** (`current_address` updates):
   ```rust  
   log::debug!("MEMORY: Allocated {} bytes at address 0x{:04x} -> 0x{:04x}", 
               size, old_address, self.current_address);
   ```

3. **IR ID Mappings** (all mapping insertions):
   ```rust
   log::debug!("MAPPING: IR ID {} -> {} mapping type: {:?}", 
               ir_id, target_value, mapping_type);
   ```

4. **Target Registration** (every instruction with target field):
   ```rust
   log::debug!("TARGET: Instruction {:?} creates target IR ID {}", 
               instruction_type, target);
   ```

### **Crash Early Patterns:**

1. **Missing Mappings** - Never use fallbacks, always crash with full context:
   ```rust
   panic!("COMPILER BUG: No mapping found for IR ID {}. Available mappings: {:?}", 
          ir_id, all_mapping_tables);
   ```

2. **Invalid State** - Crash immediately when detecting inconsistencies:
   ```rust
   assert_eq!(expected_address, actual_address, 
              "COMPILER BUG: Address mismatch during instruction generation");
   ```

3. **Unimplemented Paths** - Never return "Ok" from placeholder code:
   ```rust
   panic!("UNIMPLEMENTED: Instruction type {:?} at compilation stage", instruction);
   ```

### **Systematic Investigation Approach:**

1. **Add comprehensive logging FIRST** before attempting fixes
2. **Dump complete IR instruction sequences** to see the full picture
3. **Trace every instruction that creates targets** to find missing mappings
4. **Follow the data flow** from IR generation → mapping → resolution
5. **Remove all fallback/default behaviors** that hide bugs

### **Key Insight - Aug 27, 2025:**

The "IR ID 83 unmapped" bug was found through systematic logging that revealed:
- LoadVar instruction was missing target registration in one code path
- Parameter mapping was failing for complex functions
- Previous debugging attempts failed because they examined instruction types without tracing the actual data flow

**Never** attempt fixes without first adding comprehensive logging to understand the exact execution flow causing the issue.

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

### Grue Z-Machine Compiler: Complete ✅
- **Full Pipeline**: Lexer → Parser → Semantic → IR → CodeGen
- **Features**: String literals, function calls, control flow, object manipulation
- **Testing**: 108 compiler tests passing, golden file validation working
- **Built-in Functions**: `print()`, `move()`, `get_location()`
- **Cross-Platform Builds**: All binaries available for macOS and Windows

### Documentation & Quality
- Comprehensive architecture documentation (13KB guide)
- Zero clippy warnings, consistent formatting
- Professional CI/CD with cross-platform releases
- Enhanced testing infrastructure with pre-CI validation
- Clean historical preservation system


## Current Architecture Status

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

## Historical Documentation

Development history and detailed implementation logs have been archived to `CLAUDE_HISTORICAL.md` for reference. This file is not automatically loaded but preserves all technical implementation details from the development process.