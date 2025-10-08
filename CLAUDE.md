# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 8, 2025) - V3 TWO-BYTE PROPERTY BUG FIXED ✅

**PROGRESS**: Fixed critical interpreter bug preventing V3 two-byte properties from being read. Exit system properties now accessible at runtime.

### Bug 1: Opcode Form Selection ✅ FIXED
- **Issue**: Raw opcode 0x08 emitted as 2OP:OR (0xC8) instead of VAR:push (0xE8)
- **Fix**: `emit_instruction_typed()` respects Opcode enum variant
- **Details**: See `OPCODE_FORM_BUG_ANALYSIS.md`

### Bug 2: Infinite Loop (Jump Offset 2) ✅ FIXED
- **Issue**: 32 jumps with offset=2 created infinite loops (jump to self)
- **Root Cause**: LoadImmediate doesn't emit code → labels defer to next instruction
- **Fix**: Eliminate redundant jumps in if-statements without else (`ir.rs:1940-1953`)
- **Result**: No offset-2 jumps from if-statements (Option A implemented)

### Bug 3: Grammar Argument Passing ✅ FIXED (Oct 6, 2025)
- **Issue**: Verb commands (north, south, etc.) returned "I don't understand that"
- **Root Cause**: Grammar default pattern handler ignored `args` parameter, called functions with 0 arguments
  - Line 5954: `IrHandler::FunctionCall(func_id, _args)` - underscore meant arguments were discarded
  - Line 5962-5967: Called function with only function address, no arguments
  - Result: `handle_go()` received no direction parameter, `get_exit()` always returned 0
- **Fix**: Convert `IrValue` arguments to operands and pass them (`codegen.rs:5954-5994`)
  - For `IrValue::String`: Look up dictionary address, pass as `LargeConstant`
  - For `IrValue::Integer`: Pass as `SmallConstant` or `LargeConstant` based on size
  - Emit `call_vs` with full operand list: `[function_addr, ...args]`
- **Impact**: Navigation commands now recognized and executed (still debugging exit system)
- **File**: `src/grue_compiler/codegen.rs:5954-5994`

### Bug 4: Function Address Bug ✅ FIXED
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

### Bug 6: Property Table Patching Loop Overrun ✅ FIXED
- **Issue**: "Property 14 has size 8 (>2), cannot use put_prop" when setting `player.location.visited = true`
- **Root Cause**: `patch_property_table_addresses()` looped through 126 objects when only 14 existed
  - Calculated max_objects from `(object_space.len() - defaults_size) / obj_entry_size`
  - This includes property table data, not just object headers
  - Loop treated property table bytes as object headers, corrupting property data
- **Symptom**: Object 17 (fake) at offset 0x00ce had property table pointer at 0x03b5-0x03b6
  - This overlapped with West of House's property 14 size byte at 0x03b6
  - Writing Object 17's corrected address (0x02ee) overwrote size byte: 0x0e → 0xee
- **Fix**: Added validation to stop loop when property table pointer is invalid (`codegen.rs:5094-5109`)
  - Property table pointers must be >= defaults_size (0x3e for V3)
  - Changed `continue` to `break` - stop iteration entirely when invalid pointer detected
  - Loop now patches exactly 14 objects, stops at boundary between headers and property tables
- **Impact**: Compiled games no longer corrupt object property data
- **File**: `src/grue_compiler/codegen.rs` lines 5035-5109
- **Prevention**: NEVER calculate object count from remaining space - it includes non-object data

### Bug 7: GetPropertyByNumber Variable Collision ✅ FIXED (Oct 8, 2025)
- **Issue**: `get_exit()` returned 0 instead of packed exit value
- **Root Cause**: All `GetPropertyByNumber` instructions hardcoded Variable 241 for storage
  - When `player.location.get_exit("east")` executed, it accessed multiple properties sequentially
  - Each property access overwrote Variable 241, destroying previous values
  - Final `get_exit` call found Variable 241 = 0
- **Error**: "Invalid object number: 1002" when trying to use string ID as object
- **Fix**: Allocate unique global variable per IR ID (`codegen_instructions.rs:552-590`)
  - Changed from `let result_var = 241u8` to dynamic allocation
  - Each IR ID gets its own variable: 200 + (ir_id_to_local_var.len() % 50)
- **Impact**: Property accesses no longer collide, get_exit works
- **File**: `src/grue_compiler/codegen_instructions.rs:552-590`

### Bug 8: Branch Encoding - Placeholder Bit 15 ✅ FIXED (Oct 8, 2025)
- **Issue**: get_exit's first check `if addr == 0` always branched to not_found, even when addr != 0
- **Root Cause**: Branch placeholder 0x7FFF has bit 15 = 0, decoded as "branch on FALSE"
  - Z-Machine: bit 15 of placeholder encodes branch sense (1=true, 0=false)
  - 0x7FFF = 0111111111111111 (bit 15 clear) = branch on false
  - Should use -1 (0xFFFF) = 1111111111111111 (bit 15 set) = branch on true
- **Symptom**: JE at PC=0x12df compared 0x03bd vs 0x0000, condition=false, branch={on_true: false}
  - Since addr (0x03bd) != 0, condition is false, so "branch on false" executed → jumped to not_found
- **Fix**: Changed get_exit branch placeholders from 0x7FFF to -1 (`codegen_builtins.rs:1126, 1197, 1232`)
- **Impact**: Branches now have correct sense, get_exit continues past first check
- **File**: `src/grue_compiler/codegen_builtins.rs` lines 1115-1233

### Bug 9: exit_data String ID Instead of Packed Address ✅ FIXED (Oct 8, 2025)
- **Issue**: Blocked exit messages showed garbage, then "Unimplemented VAR instruction: 0c"
- **Root Cause**: exit_data property stored raw string IDs (1002) instead of packed addresses (0x0568)
  - codegen_objects.rs:596-597 wrote string_id directly with no UnresolvedReference
  - At runtime, interpreter tried to use 1002 as packed address
  - Unpacking: 1002 / 2 = 501, then reading from address 501 caused garbage/errors
- **Fix**: Write placeholders and create StringRef UnresolvedReferences (`codegen_objects.rs:593-610, codegen.rs:4985-5016`)
  - Write placeholder 0xFFFF instead of string_id
  - Store (exit_index, string_id) in room_exit_messages HashMap
  - During property serialization, create StringRef with is_packed_address=true
  - Resolver patches placeholder with correct packed string address
- **Verification**: StringRef for string_id=1002 resolved to packed address 0x0568 at location 0x03b4
- **Impact**: Blocked exit messages now have correct packed addresses
- **Files**:
  - `src/grue_compiler/codegen_objects.rs:521-634` (tracking and placeholders)
  - `src/grue_compiler/codegen.rs:238-241, 369, 4985-5016` (UnresolvedReference creation)

### Bug 10: V3 Property Two-Byte Format - Compiler Writing ✅ FIXED (Oct 8, 2025)
- **Issue**: Properties > 4 bytes appeared corrupted at runtime - "Property 14 not found for object 2"
- **Root Cause**: Formula `32 * (size - 1) + prop_num` correctly set bit 7=1 for sizes > 4, but code didn't write second size byte
  - For property 22 with 6 bytes: `32 * (6 - 1) + 22 = 0xB6` (bit 7=1, two-byte format)
  - Compiler wrote 0xB6 then immediately wrote data, skipping second size byte
  - Runtime read 0xB6, then next byte (0x00 from data) as size, giving size=0
  - Property table appeared to terminate (size=0 treated as terminator)
- **Fix**: Explicit two-byte format handling (`codegen_strings.rs:759-852, codegen.rs:4892-4936`)
  - Detect when size > 4, return 4-tuple with optional second size byte
  - Write both bytes: `[0x96, 0x06, data...]` instead of `[0xB6, 0x00, data...]`
- **Impact**: Property 22 (exit_data) and all properties > 4 bytes now correctly encoded
- **Files**:
  - `src/grue_compiler/codegen_strings.rs:759-852` (encode_property_value)
  - `src/grue_compiler/codegen.rs:4892-4936` (property writing)
- **See**: `docs/ARCHITECTURE.md` - "CRITICAL: V3 Property Size Encoding - Two-Byte Format"

### Bug 11: V3 Property Two-Byte Format - Interpreter Reading ✅ FIXED (Oct 8, 2025)
- **Issue**: After fixing compiler, still got "Property 14 not found" - interpreter couldn't read two-byte properties
- **Root Cause**: `get_property_info()` for V3 never checked bit 7 to detect two-byte format (`vm.rs:436-440`)
  - Always extracted size from bits 7-5: `((size_byte >> 5) & 0x07) + 1`
  - Always returned `size_bytes=1`, never read second byte
  - For 0x96: calculated size = 4 (wrong!), next byte (0x06) treated as new property header
- **Fix**: Check bit 7 for two-byte format, mirror V4+ logic (`vm.rs:436-450`)
  ```rust
  if size_byte & 0x80 != 0 {
      // Two-byte header: read second byte
      let size_byte_2 = self.game.memory[prop_addr + 1];
      let prop_size = if size_byte_2 == 0 { 64 } else { size_byte_2 as usize };
      Ok((prop_num, prop_size, 2))
  }
  ```
- **Impact**: All exit properties (20, 21, 22) now readable, navigation commands work
- **Regression**: All 174 tests pass ✅, commercial Infocom games still work ✅
- **File**: `src/vm.rs:433-459` (get_property_info)
- **See**: `docs/ARCHITECTURE.md` - "CRITICAL: V3 Property Interpreter Bug - Two-Byte Format Support"

**Tests**: All 174 tests passing.

**Remaining Work**: Exit pseudo-properties (.blocked, .destination, .message, .none) not implemented - see EXIT_SYSTEM_IMPLEMENTATION_PLAN.md

## CRITICAL: INTERPRETER MODIFICATION POLICY

**FUNCTIONALITY CHANGES PROHIBITED**: Never modify the functionality of `src/interpreter.rs` or any interpreter code. Never "fix bugs" in the interpreter.

**Rationale**: The interpreter correctly executes real Zork I and other commercial Z-Machine games. Any execution failures with compiled games are **compiler bugs**, not interpreter bugs.

**EXCEPTION - INCOMPLETE IMPLEMENTATIONS**: You MAY fix incomplete interpreter implementations if:
1. The feature is documented in Z-Machine spec but not implemented
2. Commercial Infocom games don't use the feature (so interpreter wasn't tested)
3. Compiler-generated code needs the feature
4. Fix is verified with comprehensive regression testing

**Example**: Bug 11 (V3 two-byte properties) - Feature exists in spec, commercial V3 games don't use properties > 4 bytes, so interpreter had incomplete implementation. Fix verified: all tests pass ✅, commercial games still work ✅.

**LOGGING ALLOWED**: You MAY add temporary debug logging to the interpreter to diagnose issues. Clean up logging after debugging is complete.

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
- **NEVER compile test files to `/tmp`** - Always use `tests/` directory in the repository for compiled Z3 files
