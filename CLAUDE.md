# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 5, 2025) - SSA VIOLATIONS FIXED, PLACEHOLDER ARRAY ISSUE 🔍

**PROGRESS**: Fixed five bugs, mini_zork now progresses past for-loops.

### Bug 1: Opcode Form Selection ✅ FIXED
- **Issue**: Raw opcode 0x08 emitted as 2OP:OR (0xC8) instead of VAR:push (0xE8)
- **Fix**: `emit_instruction_typed()` respects Opcode enum variant
- **Details**: See `OPCODE_FORM_BUG_ANALYSIS.md`

### Bug 2: Infinite Loop (Jump Offset 2) ✅ FIXED
- **Issue**: 32 jumps with offset=2 created infinite loops (jump to self)
- **Root Cause**: LoadImmediate doesn't emit code → labels defer to next instruction
- **Fix**: Eliminate redundant jumps in if-statements without else (`ir.rs:1940-1953`)
- **Result**: No offset-2 jumps from if-statements (Option A implemented)

### Bug 3: Function Address Bug ✅ FIXED
- **Issue**: Functions called at wrong address (first instruction instead of header)
- **Root Cause**: Code was "updating" function addresses after header generation
- **Fix**: Functions now correctly point to headers (`codegen.rs:2045-2052`)
- **Impact**: Interpreter reads local count from header, allocates locals correctly

### Bug 4: For-Loop SSA Violations ✅ FIXED
- **Issue**: Stack underflow at PC 0x13e3, 0x13ec - loop counter reused without reload
- **Root Cause**: IR violated SSA semantics by reusing consumed stack values
  - `index_temp` consumed by Less comparison, then reused by GetArrayElement
  - `index_temp` consumed by GetArrayElement, then reused by Add operation
- **Fix**: Reload `index_var` before each use (`ir.rs:2078-2090, 2095-2111`)
- **Impact**: Proper SSA semantics, stack underflow eliminated

### Bug 5: Object Tree Iteration - Variable Source Tracking ⚠️ INCOMPLETE
- **Current Error**: "Invalid object number: 1000" at PC 0x140c
- **Root Cause**: For-loops cannot detect when variable holds `contents()` result
- **Issue**: Detection only works for direct `for item in obj.contents()`, not `let items = obj.contents(); for item in items`
- **Impact**: Mini_zork inventory command fails, any indirect iteration over `contents()` fails
- **Partial Solution**: Added GetObjectChild/GetObjectSibling IR instructions and codegen
- **Missing**: Variable source tracking (HashMap<IrId, VariableSource>)
- **Technical Debt**: ⚠️ Current implementation incomplete, will fail on real code
- **Required**: Implement Option A from ARCHITECTURE.md before production use
- **Details**: See `docs/ARCHITECTURE.md` - "Object Tree Iteration Implementation"

**Tests**: All 174 tests passing.

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
