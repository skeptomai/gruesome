# Infocom Z-Machine Interpreter Project - Historical Documentation

## ARCHIVED BUGS (All Fixed ✅)

### Bug 18: Jump Instruction Emission - 0OP rtrue Instead of 1OP Jump ✅ FIXED (Oct 10, 2025)
- **Issue**: "Invalid Long form opcode 0x00 at address 1231" when typing "east" in mini_zork
- **Symptoms**: PC jumped from 0x1122 to 0x1225 (off by 1), landed at 0x1231 in middle of instruction
- **Root Cause**: Jump (1OP:12, opcode 0x0C) emitted with no operands and branch parameter
  - `emit_instruction(0x0C, &[], None, Some(-1))` in value_is_none() and exit_is_blocked() builtins
  - With zero operands, form determination chose SHORT form → 0OP, not 1OP
  - 0OP:12 = rtrue (return true), NOT jump!
  - rtrue with branch parameter emitted, but rtrue NEVER branches in Z-Machine
  - Interpreter correctly executed rtrue without reading branch bytes
  - PC advanced to branch bytes location where 0x80 (patched branch byte) was interpreted as jz opcode
- **The Cascade**:
  1. Compiler emitted rtrue at 0x1121 with branch placeholder 0xFFFF at 0x1122-0x1123
  2. Branch resolver patched 0x1122-0x1123 with offset bytes 0x80 0x06
  3. Runtime: rtrue executed, returned true, didn't read branch bytes
  4. PC advanced to 0x1122 (where branch bytes are)
  5. Interpreter decoded 0x80 as jz opcode, executed wrong instruction
  6. Eventually PC landed at 0x1231 in middle of instruction → crash
- **Fix**: Replaced `emit_instruction(0x0C, &[], None, Some(-1))` with `translate_jump(end_label)`
  - Fixed in both `value_is_none()` (line 932-950) and `exit_is_blocked()` (line 1044-1062)
  - `translate_jump()` correctly emits Jump as 1OP with signed offset as operand
- **Prevention**: Added CRITICAL section to CLAUDE.md documenting Jump vs Branch distinction
  - Jump takes offset as OPERAND, not as branch parameter
  - ALWAYS use translate_jump() helper for forward jumps
  - NEVER pass branch parameter to Jump instruction
- **Verification**: All 183 tests pass, "east" command now prints "You can't go that way." correctly
- **Files**:
  - `src/grue_compiler/codegen_builtins.rs:932-950, 1044-1062` (fix)
  - `CLAUDE.md:523-545` (prevention documentation)
- **Lesson**: Instructions that look like they should branch (Jump) often don't - Jump uses operand encoding, only conditional instructions use branch encoding

### Bug 16: Store Instruction Form Selection ✅ FIXED (Oct 10, 2025)
- **Issue**: "Unimplemented VAR:0x0c" error at PC=0x10fe when typing "east"
- **Root Cause**: Store instruction (2OP:13, opcode 0x0D) with 1 operand incorrectly classified as SHORT form
  - get_exit builtin called: `emit_instruction(0x0D, [SmallConstant(0)], Some(239), None)`
  - Passed 1 operand (value) + store_var (destination variable)
  - Form determination saw 1 operand → chose SHORT form (1OP)
  - SHORT form opcode 0x0D = PrintPaddr (1OP:13), NOT Store (2OP:13)!
  - Emitted bytes: `0x9D 0x00 0xEF` = PrintPaddr(0) → Variable(239)
  - This is wrong instruction with wrong size (3 bytes instead of expected Store)
- **PC Corruption Mechanism**:
  - Store should be LONG form 2OP: `0x4D 0xEF 0x00` (3 bytes)
  - Instead emitted SHORT form 1OP PrintPaddr: `0x9D 0x00 0xEF` (3 bytes)
  - Same size, but wrong instruction! Loop initialization set var=0 didn't execute
  - Later instructions calculated sizes based on what they thought was emitted
  - Cumulative misalignment caused PC to land at operand bytes (0xEC at 0x10fe)
- **Discovery Process**:
  1. Added comprehensive bytecode emission logging (emit_instruction_typed)
  2. Found emission gap: 0x02a4 (Div) → 0x02b1 (Loadw), missing Store at 0x02a4-0x02a7
  3. Searched for Store to Variable 239, found NONE in emission trace
  4. Located raw `emit_instruction(0x0D, ...)` call in get_exit builtin (line 1348)
  5. Traced form determination: 1 operand → SHORT form → wrong opcode!
- **Fix Part 1**: Force LONG form for Store with 1-2 operands (`codegen_instructions.rs:2084-2090`)
  ```rust
  // Opcode 0x0D: Context-dependent!
  // - 1-2 operands: store (2OP form) - MUST use Long form
  // - 3+ operands: output_stream (VAR form)
  (0x0D, 1 | 2) => Ok(InstructionForm::Long),
  (0x0D, _) => Ok(InstructionForm::Variable),
  ```
- **Fix Part 2**: Pass Store operands correctly (`codegen_builtins.rs:1347-1355`)
  ```rust
  // Store (2OP:13) takes 2 operands: (variable, value)
  // It does NOT use store_var field!
  self.emit_instruction_typed(
      Opcode::Op2(Op2::Store),
      &[Operand::Variable(index_var), Operand::SmallConstant(0)],
      None,  // Store does NOT use store_var
      None,
  )
  ```
- **Verification**: Bytes at 0x10E4 now: `0x4D 0xEF 0x00` = LONG form 2OP Store ✅
  - Binary 01001101: bits[7-6]=01 (LONG), bits[5-4]=00 (Variable), bits[3-2]=11 (SmallConstant), bits[1-0]=01 (opcode 13)
  - All PC advances now correct, no corruption!
- **Impact**: Bug #16 COMPLETELY FIXED, navigation system works until hitting erase_line bug (Bug #17)
- **Files**:
  - `src/grue_compiler/codegen_instructions.rs:2084-2090` (form determination fix)
  - `src/grue_compiler/codegen_builtins.rs:1347-1355` (Store call fix)
- **Lesson**: Z-Machine opcode semantics depend on form; same opcode number = different instructions in different forms

### Bug 15: ir_id_from_property Marking Propagation ✅ FIXED (Oct 9, 2025)
- **Issue**: Garbled output when typing "east" - print_paddr executed with address 0
- **Symptoms**: After get_exit calculations succeeded (num_exits=3), garbled text appeared followed by "Unimplemented VAR:0x0c" error
- **Root Cause**: `ir_id_from_property` set used to track which IR IDs contain string addresses (for print_paddr vs print_num)
  - `exit_get_message` builtin marked its return value (IR ID 290) in ir_id_from_property (codegen_builtins.rs:1183)
  - This flag is global/persistent - affects ALL uses of that IR ID throughout compilation
  - When IR ID 290 was later used as a function return value containing 0 (no blocked message)
  - Any code trying to print IR ID 290 would emit print_paddr instead of print_num
  - At runtime: print_paddr with operand Variable(0) (stack) containing 0 → tried to print from address 0 → garbled output
- **Discovery Process**:
  1. Added interpreter instrumentation: logged PC=0x10e4, opcode=0x0d (print_paddr), operands=[0]
  2. Added compiler instrumentation: found print_paddr emission at offset 0x0311 with Variable(0), arg_id=290
  3. Traced IR: t290 = call func#291(...) - return value from function call
  4. Found that exit_get_message was marking its return value, causing downstream print() to use print_paddr
- **Fix**: Removed ir_id_from_property marking from exit_get_message (`codegen_builtins.rs:1181-1194`)
  - The flag is too broad - affects all uses of the IR ID, including when value is 0
  - Caller must check != 0 before printing exit messages
  - exit.message property should only print when non-zero (blocked exit present)
- **Impact**: No more print_paddr with address 0, garbled output eliminated
- **Files**: `src/grue_compiler/codegen_builtins.rs:1181-1194`
- **Lesson**: IR ID tracking flags must be scoped carefully - they affect ALL uses of an IR ID, not just immediate context

### Bug 14: get_prop_len V3 Two-Byte Format Support ✅ FIXED (Oct 9, 2025)
- **Issue**: get_exit returned num_exits=1 instead of 3, loop failed to iterate through all exits
- **Root Cause**: `get_prop_len` opcode didn't support V3 two-byte property format (`opcodes_object.rs:70-115`)
  - Given property DATA address, must check byte at address-2 to detect two-byte format
  - For two-byte: byte-2=0x80|prop_num, byte-1=size, byte0=data
  - Old code only checked byte-1, misread second size byte as single-byte format
  - Example: exit_directions at 0x03bf, size bytes at 0x03bd-0x03be (0x94, 0x06)
    - get_prop_len(0x03bf) read 0x03be = 0x06, extracted size = 1 (wrong!)
    - Should check 0x03bd first, see 0x94 (bit 7 set), then read size from 0x03be = 6
- **Fix**: Check byte at data_addr-2 for bit 7 to detect two-byte format (`opcodes_object.rs:70-136`)
  - If bit 7 set at data_addr-2, read actual size from data_addr-1
  - Otherwise, extract size from bits 7-5 of data_addr-1
  - Handles both V3 and V4+ formats correctly
- **Impact**: get_prop_len now correctly returns 6 bytes for exit_directions, num_exits=3 ✅
- **Note**: Similar to Bug 11 (get_property_info), but for different opcode with different addressing
- **File**: `src/opcodes_object.rs:61-139`

### Bug 13: String Concatenation with Runtime Values ✅ FIXED (Oct 9, 2025)
- **Issue**: Expressions like `print("There is " + obj.name + " here.")` printed placeholder strings like `[RUNTIME_LOCAL_470]`
- **Root Cause**: String concatenation generated compile-time placeholder strings for runtime values
  - IR: `t470 = obj.prop#1` (get name property) → runtime value in local variable
  - IR: `t471 = "There is " + t470` → codegen created placeholder `"There is [RUNTIME_LOCAL_470]"`
  - IR: `t473 = t471 + " here."` → codegen created `"There is [RUNTIME_LOCAL_470] here."`
  - IR: `t474 = print(t473)` → codegen saw t473 in `ir_id_to_string`, emitted PRINTPADDR with **literal placeholder string**
  - Runtime: Printed placeholder string literally instead of actual runtime value
- **Fix**: Detect runtime concatenations and emit multi-part print sequences (`codegen.rs:192-200, codegen_strings.rs:701-778, codegen_builtins.rs:15-111`)
  1. Added `StringPart` enum: `Literal(usize)` for string IDs, `RuntimeValue(IrId)` for runtime values
  2. Added `runtime_concat_parts: IndexMap<IrId, Vec<StringPart>>` to track multi-part concatenations
  3. Modified `translate_string_concatenation()` to:
     - Detect when left/right operands are runtime values (in stack/local variables or nested concatenations)
     - Build `Vec<StringPart>` flattening nested concatenations
     - Reuse existing string IDs instead of creating duplicates
  4. Modified `generate_print_builtin()` to:
     - Check for runtime concatenations before string literal check
     - Emit separate print instructions for each part (print_paddr for literals and properties, print_num for integers)
     - Emit single new_line after all parts
- **Technical Details**:
  - Nested concatenation handling: `("a" + b) + "c"` flattens to `["a", b, "c"]`
  - String ID reuse: Check `encoded_strings.contains_key(ir_id)` before creating new string ID
  - Property detection: Use `ir_id_from_property` set to determine if runtime value is a property string
- **Impact**: String concatenation with runtime values now works correctly in compiler
- **Limitation**: Exit pseudo-properties (.message, .blocked, .destination) not yet implemented, so some game features still fail
- **Files**:
  - `src/grue_compiler/codegen.rs:192-200, 226-229, 375` (StringPart enum and infrastructure)
  - `src/grue_compiler/codegen_strings.rs:701-778` (runtime concatenation detection and flattening)
  - `src/grue_compiler/codegen_builtins.rs:15-111` (multi-part print emission)

### Bug 12: IR Generation for Builtin Pseudo-Methods ✅ FIXED (Oct 9, 2025)
- **Issue**: Method calls like `player.location.get_exit(direction)` generated conditional property checks
- **Root Cause**: IR generator wrapped ALL method calls in "if property exists" branches
  - For builtin pseudo-methods (`get_exit`, `empty`, `none`), property check always returned 0
  - Branch took `else` path, skipped the actual method call
  - Function called with NO arguments instead of correct arguments
- **Fix**: Detect builtin pseudo-methods and generate direct calls without property checks (`ir.rs:2645-2717`)
  ```rust
  let is_builtin_pseudo_method = matches!(method.as_str(), "get_exit" | "empty" | "none");
  if is_builtin_pseudo_method {
      // Generate direct Call instruction without property check
  }
  ```
- **Impact**: Navigation commands now work correctly, `get_exit()` receives both room and direction arguments
- **File**: `src/grue_compiler/ir.rs:2645-2717`
- **See**: `docs/ARCHITECTURE.md` - "IR Generation for Builtin Pseudo-Methods"

### Bug 11 (Score Corruption): Grammar Using G01 for Temp Storage ✅ FIXED (Oct 11, 2025)
- **Issue**: Score display showing dictionary addresses (1973, 2027, etc.) instead of actual game score
- **Symptoms**: Status line corrupted during grammar pattern matching for verbs
- **Root Cause**: Grammar handler was using Global G01 (Variable 17 = SCORE) for temporary dictionary address storage
  - Line 5838: `storew` instruction with offset=1 wrote to G01 (globals_addr + 2 = score)
  - Line 5891: `je` comparison used Variable(17) to compare dictionary addresses
  - G01 is displayed in status line as score, so temp values appeared as score
  - Dictionary addresses: "east" = 0x07b5 (1973), "quit" = 0x07eb (2027)
- **The Fix**: Changed grammar handler to use Global G200 (Variable 216) instead of G01
  - Line 5844: Changed `Operand::SmallConstant(1)` to `Operand::SmallConstant(200)`
  - Line 5891: Changed `Operand::Variable(17)` to `Operand::Variable(216)`
  - G200 is safe temp storage range (G200-G249), far from game state globals
  - Added comprehensive comments explaining why G01-G02 must NEVER be used for temps
- **Instrumentation**: Added logging to `vm.rs write_word()` to track writes to 0x0042 (score address)
  - Found multiple writes from PCs 0x13c0, 0x1413, 0x1467 all from grammar handler
  - Call stack showed return_pc 0x0e6f with locals [1, 1973, 0, 0, 0, 0, 0]
- **Prevention**: Created `docs/GLOBAL_VARIABLES_ALLOCATION.md` with comprehensive allocation map
  - Reserved globals: G00=player, G01=score, G02=moves (NEVER use for temps!)
  - Safe temp ranges: G200-G239 for compiler-generated temporary storage
  - Prevention checklist and quick reference table
- **Verification**: All 183 tests pass ✅, score stays at 0 during gameplay
- **Files**:
  - `src/grue_compiler/codegen.rs:5828-5831, 5844, 5891` (fix)
  - `src/vm.rs:248-267` (instrumentation, can be removed)
  - `docs/GLOBAL_VARIABLES_ALLOCATION.md` (prevention documentation)
- **Lesson**: NEVER use G01-G02 for temporary storage - they are displayed in status line!

### Bug 10: Navigation Bug Investigation (October 9, 2025) - RESOLVED
**Problem**: When user types "north", the message "You can't go that way." is printed, but `handle_go()` is never called.

**Investigation Summary**:
1. **Variable(1) Mystery SOLVED** ✅:
   - Variable(1) writes at runtime PC 0x15f0-0x1864 are from grammar verb handlers initializing word count
   - Formula: `Runtime PC = Code Offset + 0x1050 (base address)`
   - Example: PC 0x15f0 = code offset 0x05a0 (in 'look' verb handler at 0x059c)
   - This is CORRECT behavior - not related to navigation bug

2. **Navigation Bug IDENTIFIED** ⚠️:
   - Debug breakpoint added at `handle_go()` entry (mini_zork.grue:323) using `debug_break("handle_go entry")`
   - Breakpoint compiles to address 0x12D0 with magic marker 0xFFFE
   - When user types "north":
     - Execution reaches 'north' verb handler at PC 0x1881 (code offset 0x0831)
     - Variable(1) written at PC 0x1810 (in 'go' handler) and 0x1864 (in 'north' handler)
     - Message "You can't go that way." is printed
     - Debug breakpoint NEVER triggers (handle_go never called!)
   - "You can't go that way." only exists in handle_go(), yet handle_go() wasn't called
   - Conclusion: Grammar dispatch for 'north' verb is NOT calling `handle_go("north")`

### Complete List of Previously Fixed Bugs (1-10)
- Bug 1: Opcode Form Selection ✅ FIXED
- Bug 2: Infinite Loop (Jump Offset 2) ✅ FIXED
- Bug 3: Grammar Argument Passing ✅ FIXED (Oct 6, 2025)
- Bug 4: Function Address Bug ✅ FIXED
- Bug 5: For-Loop SSA Violations ✅ FIXED
- Bug 6: Object Tree Iteration - Variable Source Tracking ✅ FIXED (Oct 9, 2025)
- Bug 7: Property Table Patching Loop Overrun ✅ FIXED
- Bug 8: GetPropertyByNumber Variable Collision ✅ FIXED (Oct 8, 2025)
- Bug 9: Branch Encoding - Placeholder Bit 15 ✅ FIXED (Oct 8, 2025)
- Bug 10: exit_data String ID Instead of Packed Address ✅ FIXED (Oct 8, 2025)

## ARCHIVED ARCHITECTURAL INVESTIGATIONS

### V3 Property Two-Byte Format Investigation
Two separate bugs required fixes:
- **Compiler Writing** (`codegen_strings.rs:759-852, codegen.rs:4892-4936`)
- **Interpreter Reading** (`vm.rs:433-459`)

Both now handle V3 two-byte property format correctly with comprehensive regression testing.

### Debug Instrumentation Added
- Debug breakpoint system implemented (`debug_break()` builtin)
- Conditional compilation (cfg(debug_assertions))
- Magic markers: 0xFFFE (intentional breakpoint), 0xFFFF (unresolved reference bug)
- Call stack dump functionality

### Files Modified During Investigation Period
- `src/grue_compiler/ir.rs`: Added DebugBreak IR instruction
- `src/grue_compiler/semantic.rs`: Registered debug_break builtin
- `src/grue_compiler/codegen_builtins.rs`: Implemented debug_break codegen
- `src/grue_compiler/codegen_instructions.rs`: Wired DebugBreak to codegen
- `src/interpreter.rs`: Added breakpoint detection and call stack dump
- `src/grue_compiler/codegen.rs`: Added comprehensive logging for verb handlers, object lookup, Variable(1) writes
- `examples/mini_zork.grue`: Added debug_break() call in handle_go()
- `docs/CALL_STACK_DEBUGGING_IMPLEMENTATION.md`: Documented debug system

## TECHNICAL DEBT AND REFACTORING OPPORTUNITIES

### Code Quality: emit_instruction vs emit_instruction_typed
**Current state (post-Bug #18 analysis):**
- 133 uses of `emit_instruction_typed` (type-safe, preferred) ✅
- 54 uses of raw `emit_instruction` (raw opcodes, error-prone)

**Legitimate uses of raw emit_instruction:**
1. **Placeholder + UnresolvedReference pattern** (27 uses) - KEEP AS-IS ✅
2. **UNIMPLEMENTED_OPCODE markers** (3 uses) - KEEP AS-IS ✅
3. **Simple instructions without placeholders** (18 uses) - Could migrate to emit_instruction_typed

**Migration recommendation:**
- Keep emit_instruction for placeholder patterns (needs layout tracking)
- Migrate simple cases to emit_instruction_typed for better type safety
- Priority: LOW (not causing bugs, but would improve code quality)

## ARCHIVED STATUS SUMMARIES

**Tests**: All 174-183 tests consistently passing throughout investigation period.

**Z-Machine Interpreter**: Complete ✅
- v3 Games: Fully playable (Zork I, Seastalker, The Lurking Horror)
- v4+ Games: Fully playable (AMFV, Bureaucracy, Border Zone)

**Grue Z-Machine Compiler**: V3 Production Ready ✅
- Full Pipeline: Lexer → Parser → Semantic → IR → CodeGen
- V3 Support: Production ready with comprehensive test coverage
- V4/V5 Support: Experimental (debug builds only)