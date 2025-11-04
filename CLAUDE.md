# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 29, 2025)

**All major bugs fixed. System functional.**

**See**: `CLAUDE_HISTORICAL.md` for complete history of 18 fixed bugs and detailed investigations.

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
8. **File opening**: NEVER use `filename:linenumber` syntax - it creates a bare buffer, not opening the actual file

## CRITICAL: NO HACKS OR WORKAROUNDS

**NEVER implement hack solutions, workarounds, or placeholder fixes without explicit user approval.**

**Prohibited Actions:**
- ❌ NEVER hardcode values instead of implementing proper functionality
- ❌ NEVER implement "temporary" fixes that provide incorrect behavior
- ❌ NEVER choose lazy solutions over proper implementation
- ❌ NEVER make implementation decisions that affect functionality without asking

**Required Approach:**
- ✅ ALWAYS ask user for implementation direction when facing technical challenges
- ✅ ALWAYS present multiple implementation options with trade-offs
- ✅ ALWAYS implement proper solutions, even if more complex
- ✅ ALWAYS prioritize correctness over expedience

**When Implementation Questions Arise:**
1. Stop and ask the user for direction
2. Present the technical challenge clearly
3. Offer multiple solution approaches with pros/cons
4. Wait for explicit user choice before proceeding
5. Implement the chosen solution properly

**Example Violation:** Changing `print("Your score is " + to_string(player.score))` to `print("Your score is 0")` - this is a hack that provides incorrect information.

**Correct Approach:** Ask user whether to implement proper `to_string()` function, use alternative display method, or other solution approach.

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
```
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

## CRITICAL: Branch Encoding and Instruction Emission Standards

**ALL BRANCHES MUST BE 2-BYTE ENCODING:**
- ❌ NEVER allow 1-byte branch format in compiler output (e.g., `0x80` single-byte)
- ❌ NEVER let branch resolution convert 2-byte placeholders to 1-byte format
- ✅ ALWAYS emit 2-byte branch placeholders (`0xFFFF`) for forward branches
- ✅ ALWAYS resolve to 2-byte branch format (bit 7=0 in first branch byte)
- ✅ ALWAYS verify compiled instructions have correct 5-byte size for branches

**Rationale:** The codebase explicitly states "Always emit 2-byte placeholder for branches" and size calculations assume 2-byte format. Mixed 1-byte/2-byte encoding causes instruction alignment issues and systematic loop structure emission problems.

**ALL INSTRUCTION EMISSION MUST USE emit_instruction_typed:**
- ❌ NEVER use raw `emit_instruction()` except for placeholder+UnresolvedReference patterns
- ❌ NEVER pass raw opcode numbers without type safety validation
- ✅ ALWAYS use `emit_instruction_typed(Opcode::Op2(Op2::Je), operands, store_var, branch_offset)`
- ✅ ALWAYS leverage type-safe opcode enums (Op1, Op2, OpVar) for validation
- ✅ ALWAYS let emit_instruction_typed handle form determination and encoding

**Branch Resolution Verification:**
- Before: `41 05 00 FF FF` (5 bytes with 2-byte placeholder)
- After: `41 05 00 40 XX` (5 bytes with 2-byte branch, bit 7=0)
- NEVER: `41 05 00 80` (4 bytes with 1-byte branch, bit 7=1)

**Impact:** Fixes systematic loop structure emission problems and instruction alignment issues throughout the compiler.

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

## Important Reminders

- Never give percentages of completion or time estimates
- Use IndexSet and IndexMap rather than HashSet or HashMap for determinism
- **NEVER compile test files to `/tmp`** - Always use `tests/` directory in the repository for compiled Z3 files
- **NEVER write analysis documents, plans, or reports to `/tmp`** - ALWAYS write ALL analysis files to the `docs/` directory in the repository
- **CRITICAL: Documentation Location Rule** - Any technical analysis, architectural documentation, refactoring plans, bug investigations, or implementation strategies MUST be written to `docs/` directory, NEVER to temporary locations

## CRITICAL ARCHITECTURE DIRECTIVE: NO INLINE BUILTIN FUNCTIONS

**ALL builtin functions MUST be implemented as real Z-Machine functions, NOT inline generation.**

**CORRECT Architecture** (like `get_exit`):
- Builtin functions created in `generate_builtin_functions()` as real Z-Machine routines
- Called via `call_builtin_function()` using `call_vs` instructions
- Each builtin has proper function header, local variables, and return instruction

**INCORRECT Architecture** (to be eliminated):
- Inline generation via `generate_*_builtin()` methods
- Code generated directly into calling function
- No proper function boundaries or local variable scoping

**Implementation Requirements**:
1. Move ALL `generate_*_builtin()` implementations to `generate_builtin_functions()`
2. Convert ALL inline calls to use `call_builtin_function()`
3. Each builtin MUST be a complete Z-Machine function with header and locals
4. NO exceptions - even simple builtins like `print` must be real functions

**Benefits**:
- Proper function call semantics and stack discipline
- Local variable scoping eliminates global variable conflicts
- Code size reduction (generate once, call many times)
- Consistent architecture across all builtins
- Easier debugging with proper call stack frames

## CRITICAL: DYNAMIC BOUNDS CALCULATION RULE

**NEVER use hardcoded maximum/minimum values for loop bounds or array sizes.**

**ALWAYS calculate bounds dynamically from actual data structures.**

**Common violations to avoid:**
- ❌ NEVER hardcode object counts: `SmallConstant(68)`
- ❌ NEVER hardcode array sizes: `for i in 0..50`
- ❌ NEVER guess maximum values: `max_items = 100`
- ✅ ALWAYS calculate from data: `ir.objects.len() + ir.rooms.len() + 1`
- ✅ ALWAYS use dynamic maximum: `*mapping.values().max().unwrap_or(&default)`
- ✅ ALWAYS verify bounds at runtime: `if index >= collection.len()`

**Rationale**: Hardcoded bounds cause infinite loops, crashes, or missed data when actual sizes differ from assumptions. This was the root cause of the "open mailbox" infinite loop bug where hardcoded 68 objects caused loops to check non-existent objects 16-68 infinitely.

**Implementation Pattern**:
```rust
// ❌ WRONG: Hardcoded maximum
let max_objects = 68;

// ✅ CORRECT: Calculate from actual data
let max_objects = if self.ir_id_to_object_number.is_empty() {
    default_value
} else {
    *self.ir_id_to_object_number.values().max().unwrap_or(&default_value)
};
```