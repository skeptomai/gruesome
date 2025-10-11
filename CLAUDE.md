# Infocom Z-Machine Interpreter Project Guidelines

## CURRENT STATUS (October 10, 2025) - BUG #18 FIXED ✅

**PROGRESS**: Fixed Bug #18 (Jump instruction emission). Jump now correctly emitted as 1OP with offset operand, not as 0OP rtrue with branch parameter. Navigation commands work correctly.

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

### Bug 9: exit_data String ID Instead of Packed Address ✅ FIXED (Oct 9, 2025)
- **Issue**: Blocked exit messages showed garbage, then "Unimplemented VAR instruction: 0c"
- **Root Cause**: exit_data property stored raw string IDs (1002) instead of packed addresses (0x0568)
  - codegen_objects.rs:596-597 wrote string_id directly with no UnresolvedReference
  - At runtime, interpreter tried to use 1002 as packed address
  - Unpacking: 1002 / 2 = 501, then reading from address 501 caused garbage/errors
- **Fix**: Write placeholders and create StringRef UnresolvedReferences (`codegen_objects.rs:393-610, codegen.rs:4985-5016`)
  - Write placeholder 0xFFFF instead of string_id
  - Store (exit_index, string_id) in room_exit_messages HashMap
  - During property serialization, create StringRef with is_packed_address=true
  - Resolver patches placeholder with correct packed string address
- **Verification**: StringRef for string_id=1002 resolved to packed address 0x0568 at location 0x03b4
- **Impact**: Blocked exit messages now have correct packed addresses
- **Files**:
  - `src/grue_compiler/codegen_objects.rs:521-634` (tracking and placeholders)
  - `src/grue_compiler/codegen.rs:238-241, 369, 4985-5016` (UnresolvedReference creation)

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

---

## PREVIOUS FIXES (October 9, 2025)

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

---

## PREVIOUS ACTIVE INVESTIGATION (October 8-9, 2025)

### Navigation Bug Investigation (October 9, 2025) - RESOLVED

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

3. **Code Generation Verified** ✅:
   - Verb handler mapping complete:
     - 'go' handler: 0x07bc-0x0810 (noun pattern at 0x07dd)
     - 'north' handler: 0x0810-0x083b (default pattern at 0x0831 with args=1)
   - Default pattern codegen inspected (codegen.rs:6194-6338):
     - Correctly builds operands: function address + arguments
     - Emits call_vs instruction
     - Creates UnresolvedReferences for function call and dictionary words
   - Object lookup instrumentation added (codegen.rs:6537-6760):
     - Tracks Variables 2-5 (not Variable 1!)
     - Generates 29-byte lookup code per verb

4. **Debug Instrumentation Added**:
   - Debug breakpoint system implemented (`debug_break()` builtin)
   - Conditional compilation (cfg(debug_assertions))
   - Magic markers: 0xFFFE (intentional breakpoint), 0xFFFF (unresolved reference bug)
   - Call stack dump functionality
   - Comprehensive logging for:
     - Verb handler code ranges
     - Pattern handler locations
     - Object lookup code generation
     - Variable(1) writes during compilation

**Next Steps**:
- Investigate why grammar default pattern handler for 'north' is not calling `handle_go("north")`
- Possible causes:
  - Function address not resolved correctly during reference patching
  - Dictionary word "north" not resolved to correct address
  - Wrong function being called (address points to wrong routine)
  - Execution taking unexpected path in verb handler
- Use disassembly to examine actual bytecode at PC 0x1881 (offset 0x0831)
- Add runtime logging to show what function address is being called

**Files Modified**:
- `src/grue_compiler/ir.rs`: Added DebugBreak IR instruction
- `src/grue_compiler/semantic.rs`: Registered debug_break builtin
- `src/grue_compiler/codegen_builtins.rs`: Implemented debug_break codegen
- `src/grue_compiler/codegen_instructions.rs`: Wired DebugBreak to codegen
- `src/interpreter.rs`: Added breakpoint detection and call stack dump
- `src/grue_compiler/codegen.rs`: Added comprehensive logging for verb handlers, object lookup, Variable(1) writes
- `examples/mini_zork.grue`: Added debug_break() call in handle_go()
- `docs/CALL_STACK_DEBUGGING_IMPLEMENTATION.md`: Documented debug system

---

## PREVIOUS STATUS (October 8, 2025) - V3 TWO-BYTE PROPERTY BUG FIXED ✅

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

### Bug 5: Object Tree Iteration - Variable Source Tracking ✅ FIXED (Oct 9, 2025)
- **Issue**: "Invalid object number: 1000" when iterating over `contents()` results
- **Root Cause**: For-loops couldn't detect when variable holds `contents()` result
  - Direct `for item in obj.contents()` worked (inlined method call)
  - Indirect `let items = obj.contents(); for item in items` failed (variable indirection)
  - Without tracking, for-loop defaulted to array iteration using GetArrayElement
  - GetArrayElement read garbage memory at "array index", interpreted as object number
- **Fix**: Track `contents()` results in `variable_sources` IndexMap (`ir.rs:2704-2705, 2817-2824`)
  ```rust
  // Before method gets consumed
  let is_contents_method = method.as_str() == "contents";

  // After method execution
  if is_contents_method {
      self.variable_sources.insert(result_temp, VariableSource::ObjectTreeRoot(object_temp));
  }
  ```
- **Impact**: Both direct and indirect object tree iteration now work correctly
- **Infrastructure**: Complete (GetObjectChild/GetObjectSibling opcodes, variable_sources tracking)
- **Files**: `src/grue_compiler/ir.rs:2704-2705, 2817-2824`
- **Details**: See `docs/BUG_5_OBJECT_TREE_ITERATION_FIX.md`

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

**Affected bugs:** Bug #18 (value_is_none, exit_is_blocked), likely others in builtin functions

**Prevention:** Search codebase for `emit_instruction(0x0C` and verify operands are present

## Code Quality: emit_instruction vs emit_instruction_typed

**Current state (post-Bug #18 analysis):**
- 133 uses of `emit_instruction_typed` (type-safe, preferred) ✅
- 54 uses of raw `emit_instruction` (raw opcodes, error-prone)

**Legitimate uses of raw emit_instruction:**
1. **Placeholder + UnresolvedReference pattern** (27 uses)
   - Instructions with placeholders that need layout.operand_location tracking
   - Examples: call_vs with function address placeholder, print_paddr with string address placeholder
   - Pattern: `let layout = emit_instruction(...placeholder_word()...); unresolved_refs.push(UnresolvedReference { location: layout.operand_location })`
   - **Cannot use emit_instruction_typed** - need InstructionLayout for operand location
   - These are LEGITIMATE and should stay as-is ✅

2. **UNIMPLEMENTED_OPCODE markers** (3 uses)
   - Deliberate compile-time error markers for unimplemented features
   - Not real instructions, just placeholders that should fail
   - These are LEGITIMATE and should stay as-is ✅

3. **Simple instructions without placeholders** (18 uses)
   - Could be migrated to emit_instruction_typed for type safety
   - Examples: rtrue, sread with concrete operands, simple branches
   - **Refactoring opportunity**: Migrate these to emit_instruction_typed for better safety
   - Not causing bugs currently, so LOW priority

**Migration recommendation:**
- Keep emit_instruction for placeholder patterns (needs layout tracking)
- Migrate simple cases to emit_instruction_typed for better type safety
- Priority: LOW (not causing bugs, but would improve code quality)

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