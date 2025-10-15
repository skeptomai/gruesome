# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 14, 2025) - ROOM HANDLER IMPLEMENTATION 🔧

**ACTIVE TASK**: Implementing room event handlers (on_enter, on_exit, on_look) per `docs/ROOM_HANDLER_IMPLEMENTATION_PLAN.md`

**STATUS**: Phase 1 in progress - IR generation for room handlers

**PROBLEM**: Room handlers exist in mini_zork.grue source but don't execute at runtime:
- `behind_house.on_look` should print kitchen message when window is open
- `west_of_house.on_enter` should print welcome message on first visit
- Handlers are parsed but never called during gameplay

**See**: `docs/ROOM_HANDLER_IMPLEMENTATION_PLAN.md` for complete implementation plan

### Recent Fixes (October 13, 2025)

**Random Builtin Bug** ✅
- Fixed `generate_random_builtin` incorrectly marking results as strings
- Now properly maps result to stack variable 0
- File: `src/grue_compiler/codegen_builtins.rs:719-728`

**Object Numbers Timing Bug** ✅
- Fixed timing issue where object numbers were transferred too late
- Now transferred immediately in `generate_complete_game_image` after `setup_comprehensive_id_mappings`
- Fixes "Room 'X' has no object number from IR" errors in unit tests
- File: `src/grue_compiler/codegen.rs:582`

**Test Infrastructure** ✅
- Enhanced `create_minimal_ir()` to initialize object_numbers with player
- Created `add_room_to_ir()` helper for proper room registration
- All 6 exit-related unit tests now passing
- File: `src/grue_compiler/codegen_tests.rs:13-51`

### Exit System Complete (October 12, 2025) ✅

All 5 exit builtins converted to real Z-Machine functions with proper calling conventions. Navigation commands ("north", "east", "south", "west") work correctly with both blocked and unblocked exits.

**See**: `docs/BUILTIN_FUNCTION_CONVERSION_PLAN.md` and `CLAUDE_HISTORICAL.md` for detailed history.

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
