# Infocom Z-Machine Project - Historical Bug Reports

This file archives detailed bug reports and fixes from the project's development history.

For current status and active work, see `CLAUDE.md`.

## October 2025 - Exit System Implementation & Bug Fixes

### Exit System Conversion (October 12, 2025)

All 5 exit builtins converted from inline code generation to real Z-Machine functions:
- `value_is_none(value) -> bool` (15 bytes)
- `exit_is_blocked(exit_value) -> bool` (18 bytes)
- `exit_get_data(exit_value) -> u16` (8 bytes)
- `exit_get_message(exit_value) -> u16` (8 bytes)
- `get_exit(room, direction) -> u16` (~220 bytes with loop logic)

**Benefits**: 53% code size reduction, proper calling conventions, eliminated stack/variable confusion.

**See**: `docs/BUILTIN_FUNCTION_CONVERSION_PLAN.md` for detailed implementation plan.

### Major Bug Fixes (October 8-12, 2025)

**Bug 19: exit_get_message Property Marking** ✅
- Issue: Blocked exit messages printed as number instead of text
- Fix: Mark exit_get_message results as properties in call_builtin_function
- File: `src/grue_compiler/codegen.rs:9595-9604`

**Bug 18: Jump Instruction Emission** ✅
- Issue: Jump with zero operands emitted as rtrue (0OP) instead of Jump (1OP)
- Fix: Use `translate_jump()` helper, never pass branch parameter to Jump
- Files: `src/grue_compiler/codegen_builtins.rs:932-950, 1044-1062`

**Bug 16: Store Instruction Form Selection** ✅
- Issue: Store with 1 operand classified as SHORT form (PrintPaddr) instead of LONG form (Store)
- Fix: Force LONG form for Store with 1-2 operands
- Files: `src/grue_compiler/codegen_instructions.rs:2084-2090`, `codegen_builtins.rs:1347-1355`

**Bug 15: ir_id_from_property Marking Propagation** ✅
- Issue: Property marking flag affected ALL uses of IR ID (including when value is 0)
- Fix: Caller checks for != 0 before printing exit messages
- File: `src/grue_compiler/codegen_builtins.rs:1181-1194`

**Bug 14: get_prop_len V3 Two-Byte Format** ✅
- Issue: get_prop_len didn't support V3 two-byte property format
- Fix: Check byte at data_addr-2 for bit 7 to detect two-byte format
- File: `src/opcodes_object.rs:70-136`

**Bug 13: String Concatenation with Runtime Values** ✅
- Issue: Runtime values in string concatenation printed as `[RUNTIME_LOCAL_470]` placeholders
- Fix: Detect runtime concatenations and emit multi-part print sequences
- Files: `src/grue_compiler/codegen.rs:192-200`, `codegen_strings.rs:701-778`, `codegen_builtins.rs:15-111`

**Bug 12: Score Corruption - Grammar Using G01** ✅
- Issue: Grammar handler used Global G01 (score variable) for temporary storage
- Fix: Changed to use Global G200 instead of G01
- File: `src/grue_compiler/codegen.rs:5844, 5891`
- Lesson: NEVER use G01-G02 for temporary storage!

**Bug 11: V3 Property Two-Byte Format - Interpreter Reading** ✅
- Issue: Interpreter couldn't read V3 two-byte properties (properties > 4 bytes)
- Fix: Check bit 7 for two-byte format in get_property_info
- File: `src/vm.rs:436-450`

**Bug 10: V3 Property Two-Byte Format - Compiler Writing** ✅
- Issue: Compiler wrote single size byte for properties > 4 bytes
- Fix: Write both bytes for two-byte format: `[0x96, 0x06, data...]`
- Files: `src/grue_compiler/codegen_strings.rs:759-852`, `codegen.rs:4892-4936`

**Bug 9: exit_data String ID Instead of Packed Address** ✅
- Issue: exit_data property stored raw string IDs instead of packed addresses
- Fix: Write placeholders and create StringRef UnresolvedReferences
- Files: `src/grue_compiler/codegen_objects.rs:521-634`, `codegen.rs:4985-5016`

**Bug 8: Branch Encoding - Placeholder Bit 15** ✅
- Issue: Branch placeholder 0x7FFF (bit 15=0) decoded as "branch on FALSE"
- Fix: Use -1 (0xFFFF) with bit 15=1 for "branch on TRUE"
- File: `src/grue_compiler/codegen_builtins.rs:1126, 1197, 1232`

**Bug 7: GetPropertyByNumber Variable Collision** ✅
- Issue: All GetPropertyByNumber instructions hardcoded Variable 241
- Fix: Allocate unique global variable per IR ID
- File: `src/grue_compiler/codegen_instructions.rs:552-590`

**Bug 6: Property Table Patching Loop Overrun** ✅
- Issue: Loop patched 126 objects when only 14 existed, corrupting property data
- Fix: Validate property table pointer, stop when invalid
- File: `src/grue_compiler/codegen.rs:5094-5109`

**Bug 5: Object Tree Iteration - Variable Source Tracking** ✅
- Issue: For-loops couldn't detect when variable holds contents() result
- Fix: Track contents() results in variable_sources IndexMap
- File: `src/grue_compiler/ir.rs:2704-2705, 2817-2824`

**Bug 4: For-Loop SSA Violations** ✅
- Issue: IR violated SSA semantics by reusing consumed stack values
- Fix: Reload index_var before each use
- File: `src/grue_compiler/ir.rs:2078-2090, 2095-2111`

**Bug 3: Grammar Argument Passing** ✅
- Issue: Grammar default pattern handler ignored args parameter
- Fix: Convert IrValue arguments to operands and pass them
- File: `src/grue_compiler/codegen.rs:5954-5994`

**Bug 2: Infinite Loop (Jump Offset 2)** ✅
- Issue: 32 jumps with offset=2 created infinite loops
- Fix: Eliminate redundant jumps in if-statements without else
- File: `src/grue_compiler/ir.rs:1940-1953`

**Bug 1: Opcode Form Selection** ✅
- Issue: Raw opcode 0x08 emitted as 2OP:OR instead of VAR:push
- Fix: emit_instruction_typed() respects Opcode enum variant

---

## Future Sections

Additional bug reports and development history can be added here as needed.
