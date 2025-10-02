# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 1, 2025) - FIXING HARDCODED BRANCH OFFSETS 🔧

**CRITICAL RULE ESTABLISHED**: **NEVER** use hardcoded branch offsets or instruction size estimates. **ALWAYS** use label-based UnresolvedReference system.

### ⚠️ ONGOING: Systematic Hardcoded Offset Elimination (Session Oct 1, 2025)

**Cardinal Rule Violations Found**: Multiple locations in grammar system using hardcoded offset calculations instead of proper label-based resolution.

**Why Hardcoded Offsets Always Fail**:
- Instruction sizes vary based on operand types and values
- Code changes invalidate offset calculations instantly
- Branch encoding (1-byte vs 2-byte) depends on final resolved offset
- No way to accurately predict final instruction layout during emission

**Fixes Applied**:
1. ✅ **Object Lookup Loop Branches** (lines 5596-5687): Fixed 3 branch instructions to use `layout.branch_location`
2. 🔧 **Verb Word Count Branch** (line 5364-5378): Currently fixing hardcoded offset 66

**Current Error**: Branch to 0x0c30 outside bounds at PC 0x0933
- **Root Cause**: Hardcoded `branch_offset_to_verb_only = 66` at line 5364
- **Symptom**: Placeholder `0xffff` only partially patched, resulting in invalid offset 767 (0x02ff)
- **Fix Required**: Replace with label-based UnresolvedReference to verb-only handler

**Correct Pattern** (ALWAYS use this):
```rust
// 1. Create label
let skip_to_target_label = self.next_string_id;
self.next_string_id += 1;

// 2. Emit branch with placeholder
let layout = self.emit_instruction(
    0x02, // jl
    &[operands...],
    None,
    Some(0x7FFF), // Placeholder triggers 2-byte branch encoding
)?;

// 3. Register UnresolvedReference using layout.branch_location
if let Some(branch_location) = layout.branch_location {
    self.reference_context.unresolved_refs.push(UnresolvedReference {
        reference_type: LegacyReferenceType::Branch,
        location: branch_location,
        target_id: skip_to_target_label,
        ...
    });
}

// 4. Later: Register label at actual target
self.label_addresses.insert(skip_to_target_label, self.code_address);
self.record_final_address(skip_to_target_label, self.code_address);
```

**WRONG Pattern** (NEVER use this):
```rust
// ❌ NEVER calculate offsets manually
let offset = if some_condition { 66 } else { 12 };
self.emit_instruction(opcode, operands, None, Some(offset))?;

// ❌ NEVER use self.code_address - N for reference locations
location: self.code_address - 2  // WRONG!
```

**Current Working Files**:
- **Test File**: `/tmp/test_fixed_branches.z3` (partial fixes applied)
- **Source**: `src/grue_compiler/codegen.rs` (line 5364 needs fix)

## PREVIOUS STATUS (September 28, 2025) - OBJECT LOOKUP LIMITATION IDENTIFIED ⚠️

**CRITICAL INVESTIGATION COMPLETE**: "Invalid object 608" runtime error fully analyzed and root cause identified.

### 🔍 OBJECT LOOKUP INVESTIGATION COMPLETE (Session Sep 28, 2025):

**Root Cause Identified**: Grammar system passes parse buffer dictionary addresses (e.g., 608) to handler functions as `$noun` parameters, but these addresses are incorrectly treated as object IDs in property operations.

**Technical Details**:
- **Parse Buffer Address**: Variable(110) correctly stores parse buffer address 608
- **Grammar Handler**: Functions like `look_at_obj($noun)` receive dictionary addresses as parameters
- **Property Assignment**: Code like `obj.open = false` generates `clear_attr(608, 1)` instead of `clear_attr(validObjectID, 1)`
- **Crash Location**: PC 0x141e with instruction `opcode=0x0c` (clear_attr) using object 608

**Current Implementation Status**:
- ✅ **Parse Buffer Handling**: Correctly extracts dictionary addresses from user input
- ✅ **Grammar Pattern Matching**: Successfully identifies verbs and nouns
- ⚠️ **Object Lookup**: Only supports hardcoded objects 1-2, insufficient for real games
- ❌ **Dictionary-to-Object Mapping**: Missing comprehensive address→ID resolution system

**Architectural Fix Required**: Complete implementation of `generate_object_lookup_from_noun()` function to properly map dictionary addresses to Z-Machine object IDs (1-255) before property operations.

**Current Baseline**: Stable compilation and execution up to user input processing. Ready for architectural improvement to object lookup system.

## PREVIOUS STATUS (September 28, 2025) - BANNER FULLY RESTORED ✅

**CRITICAL SUCCESS**: Banner display completely restored through systematic commit analysis and restoration from BANNER_WORKS baseline!

### ✅ BANNER RESTORATION COMPLETE (Session Sep 28, 2025):

**Banner "Canary in the Coal Mine" Working Perfectly** - Systematic analysis and restoration strategy:
- **Root Cause Found**: String corruption introduced between cfee3a8 (BANNER_WORKS) and 681be3b
- **Restoration Method**: Created banner-restoration branch from working BANNER_WORKS commit (cfee3a8)
- **Banner Output**: Displays correctly - "ZORK is a registered trademark of Infocom, Inc." (NO corruption)
- **Runtime Concatenation**: Preserved and working - dynamic object descriptions functional
- **Result**: Banner canary is healthy - system stable with working string display architecture

### 🎯 CURRENT STATE - Banner Restoration Branch:
- **✅ BANNER WORKING**: All banner text displays correctly without string corruption
- **✅ RUNTIME CONCATENATION**: Dynamic string expressions work (mailbox "open/closed" descriptions)
- **✅ GAME FUNCTIONALITY**: Executes through banner display and initial setup
- **⚠️ Known Issue**: Object 65534 error after banner display (separate from banner functionality)
- **Branch**: banner-restoration (based on cfee3a8 BANNER_WORKS)

### 📊 EXECUTION PROOF - Banner Display:
```
DORK I: The Last Great Empire
Copyright (c) 2025 Grue Games. All rights reserved.
ZORK is a registered trademark of Infocom, Inc.
DORK is .... not
Revision 1 / Serial number 8675309
```

### 🎯 REMAINING ISSUES (Documented for next session):

#### 1. **Object 65534 (0xFFFE) Error - ROOT CAUSE IDENTIFIED** ✅ [INVESTIGATION COMPLETE]

**CRITICAL BUG FOUND**: Main loop generation violates Z-Machine specification causing stack corruption.

**Root Cause**: `src/grue_compiler/codegen.rs` line ~5000 (search "Implicit init: calling main loop routine"):
```rust
let layout = self.emit_instruction(
    0x20,                                          // call_vs opcode (VAR form of call)
    &[Operand::LargeConstant(placeholder_word())], // Placeholder for main loop routine address
    None,                                          // ❌ WRONG: No store (main loop doesn't return)
    None,                                          // No branch
)?;
```

**The Problem**:
- ALL Z-Machine call instructions MUST store their result somewhere
- When `store_var = None`, the result gets pushed onto the **stack**
- Main loop returns 0 or garbage value (possibly 65534/0xFFFE)
- This garbage value corrupts the stack for subsequent operations
- Later stack operations find 65534 and treat it as an object ID
- Results in "Property 16 not found for object 3" error

**Architecture Issue**:
- **Interactive Mode**: Generates automatic main loop (ID 9000) that gets called from init
- **Script Mode**: No main loop, just executes init (works perfectly - test_minimal.grue, basic_test.grue)
- **Custom Mode**: Calls user main function (may have same issue)

**Evidence**:
- Simple cases (test_minimal.grue, basic_test.grue) work: Script mode, init-only, no main loop
- Complex cases (mini_zork.grue) fail: Interactive mode, automatic main loop generation
- Value 65534 appears on stack before PC 0x12f3 (confirmed via execution tracing)

**Fix Required**: Change `None` to proper stack storage: `Some(0x00)` to store result on stack Variable(0) instead of pushing garbage

#### 2. **Placeholder/Reference Audit Needed**
- Found 26 UnresolvedReference creations in codegen.rs
- Only 9 have clear placeholder emissions nearby
- Need to verify all references have corresponding placeholders
- Some might be relying on emit_instruction to handle placeholders

#### 3. **Code Pattern Inconsistencies**
- Found places using hardcoded 0xFFFF instead of placeholder_word()
- Fixed main instances but debug messages still have hardcoded values
- Need systematic review of placeholder patterns

#### 4. **Potential Dead Code**
- Removed emit_branch_offset() which wasn't following placeholder/fixup pattern
- May be other functions not following modern patterns
- Need audit of unused or outdated functions

## 📊 MAIN BRANCH ANALYSIS (September 28, 2025)

**MERGE TEST RESULTS**: Tested merging banner-restoration fixes with main branch improvements. **OUTCOME**: Main branch has serious regressions preventing basic functionality.

### **MAIN BRANCH ARCHITECTURAL IMPROVEMENTS** (Valid concepts, broken implementation):

#### 1. **Stack vs Local Variable Architecture** 📚
- **Goal**: Implement Z-Machine specification-compliant variable usage patterns
- **Implementation**: Modified codegen to distinguish stack (Variable 0) vs local variables (Variable 1-15)
- **Status**: Breaking basic property access - "Property 14 not found for object 2" immediately on startup
- **Code Changes**: Enhanced `resolve_ir_id_to_operand()` with proper Z-Machine compliance checks

#### 2. **End-of-Routine Alignment Architecture** 🎯
- **Goal**: Fix routine boundary alignment and PC calculation consistency
- **Implementation**: Enhanced PC calculation logic to handle function headers vs instruction offsets
- **Status**: Unknown impact due to property access failures preventing execution
- **Code Changes**: Modified routine header generation and PC offset calculations

#### 3. **VAR Opcode Classification System** 🔧
- **Goal**: Comprehensive classification of VAR opcodes (0x00-0x1F) per Z-Machine specification
- **Implementation**: Added systematic VAR opcode handling in `is_true_var_opcode()` function
- **Status**: Array index crash prevention (positive improvement)
- **Code Changes**: Enhanced opcode classification in `opcodes_math.rs`

#### 4. **Enhanced UnresolvedReference System** 🔗
- **Goal**: More robust placeholder tracking and resolution for complex references
- **Implementation**: Expanded reference tracking with better error handling
- **Status**: Unknown effectiveness due to basic property access failing

### **CONCLUSION**:
Main branch contains **valid architectural concepts** but applied them to an **unstable foundation**. The improvements would be beneficial if applied to the stable banner-restoration branch incrementally with proper testing.

**DECISION**: Continue on banner-restoration branch with stable baseline, cherry-pick main branch improvements individually after fixing object 65534 error.

### 🔍 KEY DEBUGGING PATTERNS (Session Sep 17, 2025):

#### Useful Commands for Debugging:
```bash
# Check specific offset in compiled file
xxd -s 0x127f -l 10 mini_zork.z3

# Find where specific values appear
xxd mini_zork.z3 | grep -E "ff fe|fe ff"

# Trace execution with debug logging
RUST_LOG=debug ./target/debug/gruesome mini_zork.z3 2>&1 | grep "PC=0x12fc"

# Find placeholder patterns
grep -r "emit_word(0xFFFF)" src/grue_compiler --include="*.rs" | grep -v ".bak"

# Check for UnresolvedReference without placeholders
grep -B 5 "UnresolvedReference {" src/grue_compiler/codegen.rs | grep -E "(emit_word|placeholder)"
```

#### Code Patterns to Watch:
- **Correct**: `Operand::LargeConstant(placeholder_word())`
- **Wrong**: `Operand::LargeConstant(0xFFFF)`
- **Correct**: Create UnresolvedReference AFTER getting location from emit_instruction layout
- **Wrong**: Calculate location manually before/after emission

### ✅ COMPLETED FIXES (All Sessions):
1. **Branch System Fixed** - UnresolvedReference system for conditional branches
2. **Object Mapping Fixed** - Unique Z-Machine object numbers  
3. **IR ID Mapping Fixed** - Array instruction target registration
4. **PC Calculation Fixed** - Unified calculation logic for all scenarios
5. **UnresolvedReference Location Fixed** - Systematic reference resolution bug
6. **Print Function Text Formatting Fixed** - Game banner displays with proper newlines

### 📋 CRITICAL ARCHITECTURE DOCUMENTATION:
- Created `COMPILER_ARCHITECTURE.md` documenting systematic bug patterns
- **UnresolvedReference Pattern**: Location must be recorded BEFORE placeholder emission
- **Branch Encoding Pattern**: Manual byte-by-byte encoding, not `emit_word()`
- **Reference Type Pattern**: Jump vs Branch disambiguation rules

**Current Status**: mini_zork compiler now generates functionally correct Z-Machine bytecode that executes ~90% successfully. Game banner displays properly with correct newlines, major systematic reference resolution working.

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
- never give me percentages of completion and never give me time estimates to complete tasks
- don't give percentages complete or percentage estimates for completion
- we want to maintain determinism build over build. Use IndexSet and IndexMap rather than HashSet or HashMap. The Index versions have deterministic enumeration