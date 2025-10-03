# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 3, 2025) - MAIN LOOP HEADER BUG FIXED ✅

**CRITICAL FIX**: Main loop routine header was missing required local variable initialization bytes.

**The Bug**: Z-Machine V3 routine headers require 2-byte initial values for each declared local variable. Main loop declared 5 locals but didn't emit the 10 bytes of initialization data.

**The Fix** (codegen.rs line 5071-5074):
```rust
// V3 requires initial values for each local variable (2 bytes each)
for _ in 0..5 {
    self.emit_word(0x0000)?; // Initialize all locals to 0
}
```

**Impact**: Simple test cases now execute past init, grammar system tests can run.

## CRITICAL: NEVER MODIFY THE INTERPRETER

**ABSOLUTE PROHIBITION**: Never modify `src/interpreter.rs` or any interpreter code.

**Rationale**: The interpreter correctly executes real Zork I and other commercial Z-Machine games. Any execution failures with compiled games are **compiler bugs**, not interpreter bugs.

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
