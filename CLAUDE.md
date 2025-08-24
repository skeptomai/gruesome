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
7. **Never estimate time**: Do not provide time estimates ("2-3 hours", "1-2 weeks", etc.) since development happens through AI assistance, not manual coding

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
- **Testing**: 108 compiler tests passing, golden file validation working
- **Built-in Functions**: `print()`, `move()`, `get_location()`
- **Cross-Platform Builds**: All binaries available for macOS and Windows

### Documentation & Quality
- Comprehensive architecture documentation (13KB guide)
- Zero clippy warnings, consistent formatting
- Professional CI/CD with cross-platform releases
- Enhanced testing infrastructure with pre-CI validation
- Clean historical preservation system

## Last Session Summary (v0.9.0 Release - Aug 11, 2025)

### Major Accomplishments
✅ **Complete Testing Infrastructure Setup**
- All 108 Grue compiler tests verified and passing
- Gameplay tests validated (Zork I, AMFV, Trinity)
- Disassembly tests confirmed working
- Golden file validation system operational

✅ **Enhanced CI/CD Infrastructure**
- Updated CI workflows to include Grue compiler builds
- Enhanced pre-CI script with all three binary builds
- Release workflow now includes all tools (gruesome, grue-compiler, gruedasm-txd)
- Cross-platform binary generation for macOS and Windows

✅ **Professional Release v0.9.0**
- Major milestone release with complete compiler implementation
- Fixed clippy warning in semantic analysis
- Successful automated release process
- Comprehensive release notes with feature overview

### Current State
- **Repository**: Up to date with all changes committed and pushed
- **Latest Release**: v0.9.0 (successfully deployed)
- **CI Status**: All workflows passing
- **Code Quality**: Zero warnings, fully formatted
- **Test Coverage**: Complete (interpreter + compiler + integration)

### Next Session Preparation
- All infrastructure is in place for continued development
- Pre-CI script ready for validation before any commits
- Release automation working properly
- Ready for new features or game compatibility work

## Version-Aware Instruction Selection (Future Enhancement)

**Priority**: Medium - Architecture improvement for proper multi-version support

**Current Issue**: The compiler currently hard-codes Version 3 instructions for all Z-Machine versions. While this ensures broad compatibility, it's not optimal for V4+ games.

**Required Implementation**:
1. **Version-Aware Call Instructions**:
   - V3: Use `call` (VAR:224) - the only available call instruction
   - V4+: Use optimal variants: `call_1s`, `call_2s`, `call_1n`, `call_2n` for efficiency
   - Benefit: Smaller bytecode, better performance on V4+ interpreters

2. **Version-Specific Instruction Sets**:
   - Many instructions added in V4 (buffer_mode, erase_window, etc.)
   - V5 added more instructions (read_char, scan_table, etc.)
   - V6 added graphics and sound instructions
   - Some opcodes repurposed between versions

3. **Implementation Strategy**:
   - Add `target_version: u8` parameter to compiler
   - Modify `determine_instruction_form()` with version-aware logic
   - Update `generate_function_call()` for optimal call instruction selection
   - Add version compatibility checks throughout codegen
   - Create instruction availability tables per version

4. **Code Locations**:
   - `src/grue_compiler/codegen.rs`: Add version parameter and selection logic
   - `determine_instruction_form()`: Version-aware form selection
   - `generate_function_call()`: Optimal call instruction selection
   - Header generation: Ensure version field matches instruction usage

**Benefits**:
- Proper Z-Machine specification compliance for all versions
- Optimal bytecode generation (smaller, more efficient V4+ games)
- Better interpreter compatibility across different Z-Machine versions
- Professional-grade multi-version compiler architecture

**Example**:
```rust
// V3: call (VAR:224) - 4+ bytes
call routine_addr -> result

// V4+: call_1s (1OP:136) - 3 bytes  
call_1s routine_addr -> result
```

This enhancement ensures our compiler generates the most appropriate instruction set for each target Z-Machine version.

## Runtime Issues Fix Plan (Aug 12-14, 2025) - ✅ COMPLETED

### Status: All Critical Runtime Issues Resolved ✅

**Completed Runtime Fixes:**
- ✅ Property access implementation (complete)
- ✅ Error handling and recovery (complete)  
- ✅ Complex control flow compilation (complete)
- ✅ **Stack Management Crisis RESOLVED** (Aug 14, 2025)
- ✅ **String Concatenation System** (complete implementation)
- ✅ **Function Call Stack Balance** (rtrue instruction fix)
- ✅ **100% Success Rate Achievement** (27/27 examples working)

**Previous Runtime Issues (NOW RESOLVED):**

### 1. Stack Management Crisis ✅ FIXED
- **Was**: "Stack is empty" errors in complex control flow
- **Solution**: Implemented proper `rtrue` instructions for function call placeholders
- **Result**: Perfect stack balance, no more underflow errors

### 2. String Concatenation ✅ IMPLEMENTED  
- **Was**: Missing support for complex string operations
- **Solution**: Full compile-time string concatenation with `to_string()` support
- **Result**: Complex expressions like `"Level " + to_string(x) + " complete"` working

### 3. Property System Gaps 🟡 → **NEXT PRIORITY**
- **Issue**: Property access uses placeholder implementations
- **Root Cause**: Hardcoded object numbers instead of proper IR→Z-Machine mapping
- **Impact**: Property operations don't access real object data

### 4. Object Resolution Problems 🟡 → **NEXT PRIORITY**  
- **Issue**: Objects not properly mapped to Z-Machine object table
- **Root Cause**: Missing object table generation and ID resolution
- **Impact**: Object manipulation operations fail

### Systematic Fix Plan

**Phase 1: Bytecode Diagnostics & Validation (Priority 1)**
- Create bytecode inspection tools (hexdump, instruction decoder, stack trace)
- Add bytecode validation during generation (validate encoding, operand compatibility)
- Implement generation debugging (log instructions, track stack depth, identify invalid opcodes)

**Phase 2: Stack Management Overhaul (Priority 1)**
- Audit stack operations in codegen (review emit_instruction calls, ensure balanced push/pop)
- Implement stack depth tracking (compilation warnings, safety assertions)
- Fix nested control flow stack handling (if/else balance, function calls, expression evaluation)

**Phase 3: Complete Object/Property System (Priority 2)**
- Implement proper object table generation (Z-Machine format, IR→object mapping, property table)
- Fix property access operations (real object resolution, correct get_prop/put_prop operands)
- Add object manipulation operations (complete move() builtin, container support, attributes)

**Phase 4: Runtime Error Recovery (Priority 3)**
- Enhanced runtime error handling (catch Z-Machine errors, graceful degradation)
- Debugging and profiling tools (performance monitoring, execution trace, breakpoints)

**Execution Order:** Phase 1+2 parallel → Phase 3 → Phase 4

## Zork I-Level Features Battle Plan (Aug 13, 2025)

### Current Grue Capabilities vs Zork I Requirements

**What We Have ✅:**
- Basic text adventure structure (rooms, objects, movement)
- Simple property system (openable, container, takeable)
- Basic built-in functions (print, move, get_location)
- Grammar system with verb patterns
- Container relationships and inventory management
- Simple conditionals and control flow
- Z-Machine V3/V5 bytecode generation

**Critical Gaps for Zork I Complexity 🔴:**

### 1. **Advanced Object System** 
- **Missing**: Complex object relationships, inheritance, class hierarchies
- **Zork I has**: 200+ objects with sophisticated attribute systems
- **Current**: Only basic properties (open/closed, container/non-container)

### 2. **Comprehensive Attribute System**
- **Missing**: Full Z-Machine attribute support (32 attributes per object)
- **Zork I uses**: Attributes for light sources, weapons, treasures, scenery
- **Current**: Hardcoded boolean properties only

### 3. **Advanced Property System** 
- **Missing**: Numbered properties, property inheritance, dynamic property modification
- **Zork I has**: Complex property tables for descriptions, capacity, value
- **Current**: Basic string properties only

### 4. **Sophisticated Parser**
- **Missing**: Multi-word nouns, adjectives, prepositions, disambiguation
- **Zork I needs**: "get lamp from trophy case", "examine rusty knife"
- **Current**: Single-word noun matching only

### 5. **Game State Management**
- **Missing**: Save/restore, scoring system, turn counters, timers
- **Zork I has**: Complex state tracking, multiple endings, score calculation
- **Current**: No persistent state beyond object locations

### 6. **Advanced Text Features**
- **Missing**: Dynamic text generation, string manipulation, formatted output
- **Zork I uses**: Complex description assembly, conditional text
- **Current**: Static string literals only

## Implementation Battle Plan

### **Phase 1: Core Infrastructure (2-3 weeks)**
**Priority: Critical - Foundation for everything else**
- [x] Enhanced Object System (32-attribute support, inheritance) - **COMPLETED**
- [ ] Advanced Property System (numbered properties, dynamic modification)
- [ ] Robust Parser Engine (multi-word nouns, disambiguation)

### **Phase 2: Game Mechanics (2-3 weeks)**
**Priority: High - Essential gameplay features**
- [ ] State Management System (save/restore, scoring, turn counters)
- [ ] Advanced Text System (dynamic generation, conditional text)
- [ ] Environmental Systems (light/darkness, capacity, complex interactions)

### **Phase 3: Advanced Features (2-3 weeks)**
**Priority: Medium - Polish and sophistication**
- [ ] AI and NPCs (movement, dialogue, interaction)
- [ ] Complex Puzzles (multi-step sequences, transformations)
- [ ] Polish and Optimization (performance, memory management)

### **Phase 4: Testing and Validation (1-2 weeks)**
**Priority: Critical - Ensuring production readiness**
- [ ] Comprehensive Testing (full Zork I recreation, stress testing)
- [ ] Cross-platform validation and production hardening

### Success Metrics
- **Capability**: Support all Zork I game mechanics (200+ objects, complex puzzles)
- **Compatibility**: Generate Z-Machine files playable in standard interpreters  
- **Performance**: Handle complex games without runtime errors
- **Completeness**: Successfully compile and run full Zork I recreation

### Milestone Tracking - UPDATED Aug 14, 2025
- **Phase 0 (Runtime Stability)**: ✅ **COMPLETED** (Aug 12-14, 2025)
  - Stack management issues resolved
  - String concatenation implemented  
  - 100% success rate achieved (27/27 examples)
- **Phase 1 Start**: **READY TO BEGIN** (Aug 14, 2025)
- **Phase 1 Target**: Advanced opcodes and object system (Sep 2025)
- **Phase 2 Target**: Game mechanics and parser (Oct 2025)  
- **Phase 3 Target**: Advanced features and polish (Nov 2025)
- **Phase 4 Target**: Full Zork I recreation (Dec 2025)

### Current Implementation Status (Aug 16, 2025)
📍 **Position**: Debugging critical bytecode corruption in address resolution
🎯 **Next Step**: Fix instruction stream corruption in reference patching system
📊 **Success Rate**: Temporary regression - basic compilation corrupted during execution
📋 **See**: Current analysis below for detailed investigation results

## Current Critical Bug Investigation (Aug 16, 2025) - 🔴 IN PROGRESS

### Root Cause Analysis: Address Patching Memory Corruption

**Issue**: "Invalid object number: 989" runtime error in simple test cases
**Actual Problem**: Address resolution phase corrupting instruction bytecode stream

### Technical Investigation Results:

#### 1. Error Manifestation
- **Runtime Error**: "Invalid object number: 989" during execution of `debug_object_error.z3`
- **Object 989**: = 0x03DD (packed string address) being interpreted as object number
- **Symptom**: Address resolution patches corrupting nearby instruction bytes

#### 2. Memory Corruption Discovery
**Original Expected Layout:**
```
0x0732: 0x82 (print_paddr)
0x0733: 0x00 (operand placeholder)  
0x0734: 0x00 (operand placeholder)
0x0735: 0xE4 (VAR sread instruction)
0x0736: 0x0F (operand types)
```

**Actual Patched Layout:**
```
0x0732: 0x82 (print_paddr) ✓
0x0733: 0x03 (patched string address high byte) ✓
0x0734: 0xDD (patched string address low byte) ✓ - WAS CORRUPTION
0x0735: 0xE4 (VAR sread instruction) ✓
0x0736: 0x0F (operand types) ✓
```

#### 3. Address Resolution Trace
**Critical Patch Events:**
- String ID 9002 (prompt "> ") resolved to address 0x07BA
- Packed address: 0x07BA / 2 = 0x03DD  
- Patch location: 0x0733 (2 bytes: 0x03 0xDD)
- **Result**: Correct patching, NOT corruption as initially thought

#### 4. Real Issue: Control Flow Problems
**Discovery**: The patching is working correctly. The issue is **execution flow**:
- Print instruction executes successfully (shows "Simple test - no objects")
- Jump instruction at 0x0741: `0x8C 0xFF 0xF0` (jump with offset -16)
- **Target calculation**: PC=0x0744, offset=-16 → target=0x0732 (correct)
- **Problem**: Something after this is interpreting 0x03DD as an object number

#### 5. Next Investigation Steps
1. **Trace actual PC during execution** - where exactly does the object error occur?
2. **Verify jump target calculation** - is the jump landing in the right place?
3. **Check instruction following patterns** - what instruction is using 989 as object?
4. **Validate main loop generation** - are the generated instructions semantically correct?

### Current Status
- ✅ **Identified**: Address patching is working correctly (not corrupted)
- ✅ **Isolated**: Error occurs after successful print execution  
- ✅ **Located**: Issue is in post-print control flow execution
- 🔄 **Next**: Trace exact PC location when object error occurs
- 🔄 **Fix**: Correct the instruction that's misinterpreting string address as object

### Working Theory
The interpreter is correctly executing the initial sequence but hitting a control flow issue where:
1. A jump or control transfer lands at wrong address, OR
2. An instruction is incorrectly interpreting a string address operand as an object number, OR  
3. The main loop generation has semantic errors in instruction sequencing

**Priority**: High - blocking all basic game execution

## Historical Documentation

Development history and detailed implementation logs have been archived to `CLAUDE_HISTORICAL.md` for reference. This file is not automatically loaded but preserves all technical implementation details from the development process.