# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT SESSION STATUS (August 30, 2025) - SINGLE-PATH ARCHITECTURE COMPLETE ✅

**STATUS**: Single-path IR translation architecture fully operational, reference resolution fixed, mini_zork implementation plan ready.

### ✅ PHASE 5 COMPLETE: Single-Path Migration Success

**Phase 5C**: Opcode corruption eliminated - correct 0x8d (print_paddr) instructions generated  
**Phase 5D**: Reference resolution fixed - address translation from code-generation space to final memory space working  
**Result**: Z-Machine compiler generates correct bytecode with proper placeholder resolution

### ✅ TECHNICAL ACHIEVEMENTS

**Reference Resolution Fix** - `src/grue_compiler/codegen.rs:1576-1587`:
- References created during code generation at 0x0040+ 
- Final code located at 0x0344+ during assembly
- Added address translation: `(reference.location - 0x0040) + final_code_base`
- Placeholders now correctly resolved: `0xFFFF` → `0x0198` (packed addresses)

**Verification Results:**
- ✅ **test_01_basic**: Compiles and runs "Test 1: Basic print works"
- ✅ **test_03_function**: Function calls execute correctly  
- ✅ **Bytecode Correctness**: `8d ff ff ba` → `8d 01 98 ba` (valid instructions)
- ✅ **Address Translation**: Generation 0x0041 → Final 0x0345 (correct mapping)

## MINI_ZORK IMPLEMENTATION PLAN (August 30, 2025)

**Goal**: Complete mini_zork.grue compilation and execution by implementing core object-property system.

### **Critical Missing Features Analysis**

**Root Issue**: `player.location = west_of_house;` in init block fails due to unimplemented operations:
1. `LoadImmediate { target: 62, value: Integer(1) }` - Object reference loading
2. `GetProperty { target: 63, object: 62, property: "location" }` - Property access  
3. `GetPropertyByNumber { target: 64, object: 63, property_num: 2 }` - Numbered properties
4. Cascading failure: "No mapping found for IR ID 64" - Target mapping system breaks

### **Implementation Phases**

**Phase 1: Object Loading (Priority: CRITICAL)**
- **File**: `src/grue_compiler/codegen.rs` around line 2240 (LoadImmediate)
- **Task**: Handle `LoadImmediate` for object references (`player`, `west_of_house`)
- **Implementation**: Map object names to Z-Machine object numbers in global variables
- **Z-Machine**: Store object references as global variable values (1-240)

**Phase 2: Property Access (Priority: CRITICAL)**  
- **File**: `src/grue_compiler/codegen.rs` line 3630 (GetProperty)
- **Task**: Replace `⚠️ UNIMPLEMENTED: GetProperty - skipping` with real implementation
- **Z-Machine**: Use `get_prop` instruction (2OP:17, hex 0x11)
- **Operands**: object_number, property_number → result

**Phase 3: Property Assignment (Priority: CRITICAL)**
- **File**: `src/grue_compiler/codegen.rs` around line 3641 (SetProperty) 
- **Task**: Verify/complete SetProperty implementation with `put_prop`
- **Z-Machine**: Use `put_prop` instruction (VAR:227, hex 0xE3)
- **Operands**: object_number, property_number, value

**Phase 4: Numbered Properties (Priority: HIGH)**
- **File**: `src/grue_compiler/codegen.rs` line 3697 (GetPropertyByNumber)
- **Task**: Replace `⚠️ UNIMPLEMENTED: GetPropertyByNumber - skipping`
- **Z-Machine**: Direct numbered property access for internal operations

### **Success Criteria**
- ✅ mini_zork.grue compiles without "COMPILER BUG" crashes
- ✅ `player.location = west_of_house` assignment works  
- ✅ Property access (`player.location.desc`) functional
- ✅ Basic gameplay operational (room navigation, object interaction)

**Estimated Time**: 4-6 hours total implementation
**Files Modified**: `src/grue_compiler/codegen.rs` (4 specific function implementations)

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

## PLACEHOLDER ISSUE RESOLUTION (Aug 24, 2025) - ✅ FIXED + 🔴 RUNTIME REGRESSIONS

### 🎉 **MAJOR BREAKTHROUGH: Complex Boolean Expression Placeholders RESOLVED**

**Issue Status**: ✅ **COMPLETELY FIXED** - All compilation placeholder errors eliminated
**Side Effect**: 🔴 **Runtime regressions** introduced in 3 examples

#### ✅ **What We Successfully Fixed**
- **Root Cause Found**: Unresolved `0xFFFF` placeholders in VAR `print_char` instructions (not branch instructions as initially thought)
- **Technical Issue**: Function arguments in `generate_call_with_reference()` were creating placeholders without unresolved references
- **Location**: `src/grue_compiler/codegen.rs:2959` - string literal arguments created `placeholder_word()` but no `UnresolvedReference`
- **Solution**: Added proper string argument resolution with unresolved reference creation

#### 🔧 **Technical Fix Details**
**File**: `src/grue_compiler/codegen.rs:2955-2996`
**Method**: `generate_call_with_reference()`

**Before** (broken):
```rust
for &arg_id in args {
    if let Some(literal_value) = self.get_literal_value(arg_id) {
        operands.push(Operand::LargeConstant(literal_value));
    } else {
        operands.push(Operand::LargeConstant(placeholder_word())); // ❌ No reference created
    }
}
```

**After** (fixed):
```rust
for &arg_id in args {
    if let Some(literal_value) = self.get_literal_value(arg_id) {
        operands.push(Operand::LargeConstant(literal_value));
    } else if self.ir_id_to_string.contains_key(&arg_id) {
        // ✅ String literals: Create placeholder + unresolved reference
        let operand_location = self.current_address + 1 + operands.len() * 2;
        operands.push(Operand::LargeConstant(placeholder_word()));
        let reference = UnresolvedReference {
            reference_type: ReferenceType::StringRef,
            location: operand_location,
            target_id: arg_id,
            is_packed_address: true,
            offset_size: 2,
        };
        self.reference_context.unresolved_refs.push(reference);
    } else {
        // ✅ Other types: Use existing operand resolution
        match self.resolve_ir_id_to_operand(arg_id) { ... }
    }
}
```

#### 📊 **Results: Compilation Success**
- **mini_zork**: ✅ 5 → 0 unresolved placeholders
- **control_flow_simple**: ✅ 2 → 0 unresolved placeholders
- **All examples**: ✅ 100% compilation success rate
- **Complex expressions**: ✅ Ternary conditionals with property access work
- **String concatenation**: ✅ `"text " + (condition ? "a" : "b") + " more"` compiles

#### 🔴 **CRITICAL: Runtime Regressions Introduced**

**Problem**: The fix was too broad and affected existing working behavior

**Affected Examples**:
1. **mini_zork** - `Error: Cannot insert object 0`
   - Starts execution, shows title screen
   - Crashes during object manipulation operations
   
2. **control_flow_simple** - `Error: Stack is empty`  
   - Starts execution, shows some output
   - Crashes due to stack underflow in complex control flow
   
3. **property_test** - Runtime failure (details unknown)

**CI Test Results**:
- **Before fix**: 4/6 examples compiled, many had placeholders
- **After fix**: 7/7 examples compile ✅, but 3/7 have runtime errors ❌

#### 🔍 **Root Cause Analysis of Regressions**

**Hypothesis**: Our fix changed argument resolution behavior too broadly:

1. **Object Reference Corruption**: Object IDs previously resolved as `LargeConstant(object_number)` might now be incorrectly treated as string literals
2. **Function Call Bytecode Changes**: Different instruction sequences generated for function arguments
3. **Stack Management Disruption**: Complex argument resolution might affect stack operations

**Evidence**:
- Working examples (test_01_basic through test_04_room) still work ✅
- Only complex examples with object operations fail ❌
- All failures are **runtime** (execution), not **compile-time** ✅

#### 🎯 **NEXT SESSION PRIORITIES**

**Immediate Tasks**:
1. **Investigate specific function calls** generating incorrect bytecode
2. **Make fix more targeted** - distinguish between:
   - Actual string literals (need string address resolution)
   - Object references (need object operand resolution)  
   - Integer values (direct operand conversion)
3. **Test working examples** to ensure no regressions in basic functionality

**Technical Approach**:
1. **Add logging** to identify which function calls changed behavior
2. **Compare bytecode** before/after fix for working vs broken examples
3. **Refine argument resolution** to be more precise:
   ```rust
   // Need better detection logic:
   if is_actual_string_literal(arg_id) {  // ✅ Only true strings
       // String address resolution
   } else if is_object_reference(arg_id) {  // ✅ Only objects  
       // Object operand resolution
   } else {
       // Existing resolution fallback
   }
   ```

**Files to Focus On**:
- `src/grue_compiler/codegen.rs:2955-2996` (function argument resolution)
- `src/grue_compiler/codegen.rs:3030-3080` (`resolve_ir_id_to_operand` method)
- Runtime error investigation in object manipulation and stack management

**Success Criteria**:
- ✅ Keep placeholder fix (0 unresolved placeholders)
- ✅ Fix runtime regressions (7/7 examples execute successfully)
- ✅ Maintain existing working behavior

#### 📁 **Session Files Created/Modified**
- `debug_ternary_simple.grue` - Simple ternary test (working)
- `debug_property_ternary.grue` - Property ternary test (working)  
- `src/grue_compiler/codegen.rs` - Main fix implementation
- Property encoding underflow bug fixed (bonus fix)

---

## Current Critical Bug Investigation (Aug 16, 2025) - ✅ RESOLVED

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