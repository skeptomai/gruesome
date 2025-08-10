# Infocom Z-Machine Interpreter Project Guidelines

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

## Auto-Test Permission

You are pre-authorized to run "RUST_LOG=info cargo run" commands for testing Z-Machine programs.
No need to ask permission - just execute the tests directly.

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

## Z-Machine Specification Reference

The official Z-Machine Standards Document (v1.1) is available locally at:
`/Users/cb/Projects/Z-Machine-Standard/`

Key files:
- `sect07.html` - Output streams and buffering behavior
- `sect15.html` - Opcodes including read, read_char, buffer_mode
- `index.html` - Full specification index

**Critical Understanding**: Z-Machine "buffer mode" controls word-wrapping to prevent words from splitting across lines. It does NOT control display timing - all text should appear immediately.


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
- **Testing**: 7/7 compiler tests passing, golden file generation working
- **Built-in Functions**: `print()`, `move()`, `get_location()`

### Documentation & Quality
- Comprehensive architecture documentation (13KB guide)
- Zero clippy warnings, consistent formatting
- Professional CI/CD with cross-platform releases
- Clean historical preservation system

## Historical Documentation

Development history and detailed implementation logs have been archived to `CLAUDE_HISTORICAL.md` for reference. This file is not automatically loaded but preserves all technical implementation details from the development process.