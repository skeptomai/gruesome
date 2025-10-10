# Bug #17: erase_line Instruction in V3 Games

**Status**: ACTIVE - Under Investigation
**Priority**: HIGH - Blocks V3 game execution
**Date Identified**: October 10, 2025

## Problem Description

The compiler is emitting byte sequence 0xEE which the Z-Machine interpreter interprets as the `erase_line` instruction (VAR:14). This instruction is only available in Z-Machine V4+, but is appearing in V3-compiled games.

### Error Message
```
Error during execution: erase_line is only available in v4+
```

## Evidence

### 1. Game Version Verification
```bash
$ xxd tests/mini_zork.z3 | head -1
00000000: 0300 0001 0e3c 0e3d 078d 02e0 0040 078d  .....<.=.....@..
```
- First byte is `0x03` ✅ - Game correctly compiled as V3

### 2. Runtime Error
```bash
$ echo -e "east\nquit\ny" | cargo run --bin gruesome tests/mini_zork.z3
[Game boots successfully]
> east
Error during execution: erase_line is only available in v4+
```

### 3. Disassembly Evidence
```bash
$ cargo run --bin gruedasm-txd tests/mini_zork.z3 2>&1 | grep -B5 -A5 "ERASE_LINE"
GET_PROP_ADDR   Ge3,#16 -> Gdd
GET_PROP_LEN    Gdb -> Gde
DIV             Gde,#02 -> Gde
STORE           Gdf,#00
PRINT_CHAR      Gdf
ERASE_LINE      G17,#6feb,#ef00,#6100
JE              #80,#09 [FALSE] 1109
SET_CURSOR      #ef8c,#ffec,#70ec,#ef
```

**Observations**:
- Appears in Routine R0011 (likely `show_inventory` or `list_contents`)
- Immediately follows PRINT_CHAR instruction
- Has suspicious operands: G17, #6feb, #ef00, #6100
- Operands don't make sense for erase_line (should take 1 operand: fill value)

### 4. Opcode Information

**erase_line Specification**:
- Opcode: VAR:14 (0x0E in enum, 0xEE when encoded)
- Form: VAR (variable operand count)
- Encoding: 0xE0 + 14 = 0xEE
- Availability: V4+ ONLY
- Purpose: Clear from cursor to end of line
- Operands: 1 (value to use for filling)

**Related Opcodes**:
- PRINT_CHAR: VAR:5 (0xE5)
- Pattern: 0xE5 followed by 0xEE suggests sequential VAR opcodes

## Source Code Analysis

### Compiler Does NOT Explicitly Emit erase_line

Search results:
```bash
$ grep -rn "EraseLine\|erase_line\|0xEE" src/grue_compiler/ --include="*.rs"
src/grue_compiler/opcodes.rs:434:    EraseLine = 0x0E,
src/grue_compiler/opcodes.rs:537:            | OpVar::EraseLine
```

- Opcode is **defined** in opcodes.rs
- But **never emitted** in any codegen file
- No version checking around potential emission

### Interpreter Correctly Rejects V3 Usage

`src/display_v3.rs:199-201`:
```rust
fn erase_line(&mut self) -> Result<(), DisplayError> {
    Err(DisplayError::new("erase_line not supported in v3"))
}
```

### Context: Object Tree Iteration

The error occurs during object listing (inventory/contents display):

`examples/mini_zork.grue:427-432`:
```grue
fn list_contents(container) {
    let contents = container.contents();
    for item in contents {
        print("  " + item.name);
    }
}
```

**Hypothesis**: The `for item in contents` loop's codegen might be creating malformed instructions.

## Possible Root Causes

### Theory 1: Malformed Instruction Encoding
- Some instruction is being encoded incorrectly
- Creates 0xEE byte as part of operand data or opcode
- Most likely candidate: PRINT_CHAR (0xE5) followed by malformed operand encoding

### Theory 2: Data Misinterpreted as Code
- String data or property data contains 0xEE byte
- Jump or control flow lands in middle of data section
- Interpreter begins executing data as code

### Theory 3: Operand Type Encoding Error
- VAR form instructions use type bytes to specify operand types
- Type byte 0xEE could be misinterpreted as new instruction
- Especially if operand count calculation is off

### Theory 4: Corrupted Object Iteration Code
- Object tree iteration (`contents()`) generates complex code
- GetObjectChild/GetObjectSibling might have encoding issues
- Loop counter or iterator variable might create 0xEE byte

## Investigation Steps Performed

1. ✅ Verified game version in header (V3)
2. ✅ Confirmed runtime error occurs
3. ✅ Found disassembly location (Routine R0011)
4. ✅ Searched compiler for explicit erase_line emission (none found)
5. ✅ Identified context (object tree iteration/printing)
6. ⏸️ Raw byte analysis around 0xEE location (pending)
7. ⏸️ Instruction encoding validation (pending)

## Next Steps

### Debugging Philosophy

**DO NOT reverse engineer the binary .z3 file with xxd/hexdump.**

Instead, use our proper debugging infrastructure:
1. **Compiler debug output** - Shows what's being emitted and where
2. **Disassembler** - Provides instruction-level view with addresses
3. **Interpreter instrumentation** - Shows exactly what executes at runtime

If we can't debug with these tools, that's a failure of our debugging process, not a reason to resort to binary analysis.

### Immediate Actions

1. **Use Compiler Debug Output**
   - Compile with `RUST_LOG=debug` to see instruction emission
   - Look for PRINT_CHAR emission followed by unexpected bytes
   - Track exactly what opcodes/operands are being emitted during object tree iteration
   - Check code address tracking and PC advancement

2. **Instrument Interpreter Execution**
   - Add logging before opcode dispatch to show PC and instruction bytes
   - Log when erase_line is encountered (PC, prior instructions, call stack)
   - Trace execution from "east" command through to error
   - Identify what code path leads to the problematic instruction

3. **Use Disassembler Correctly**
   - Identify Routine R0011's source function (which Grue function?)
   - Cross-reference with compiler's USER_FUNCTION output
   - Understand code flow leading to ERASE_LINE
   - Look for patterns in surrounding instructions

4. **Create Minimal Reproduction**
   - Simplify mini_zork.grue to just the failing code path
   - Strip down to: object with contents, loop to print them
   - Compile and test if error reproduces
   - Narrow down exact IR/codegen pattern that causes issue

### Investigation Commands

```bash
# Step 1: See what compiler emits during object iteration
$ RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test.z3 2>&1 | grep -A10 "list_contents\|PRINT_CHAR"

# Step 2: Identify which function has the bug
$ RUST_LOG=error cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test.z3 2>&1 | grep USER_FUNCTION
# Then correlate with disassembler's Routine numbers

# Step 3: Trace interpreter execution to error point
$ echo "east" | RUST_LOG=debug cargo run --bin gruesome tests/mini_zork.z3 2>&1 | grep -B20 "erase_line"

# Step 4: Create minimal test case
$ cat > /tmp/test_minimal.grue <<EOF
world {
    room start "Start" {
        object box {
            names: ["box"]
            container: true
            contains {
                object item { names: ["item"] }
            }
        }
    }
}
fn test() {
    for thing in box.contents() {
        print(thing.name);
    }
}
EOF
$ cargo run --bin grue-compiler -- /tmp/test_minimal.grue -o /tmp/test_minimal.z3
$ echo "test" | cargo run --bin gruesome /tmp/test_minimal.z3
```

### Code Areas to Examine

1. **PRINT_CHAR emission** (`src/grue_compiler/codegen_builtins.rs` or `codegen_instructions.rs`)
   - Check emit_instruction_typed calls with OpVar::PrintChar
   - Validate operand encoding

2. **Object tree iteration** (`src/grue_compiler/codegen_instructions.rs:GetObjectChild/GetObjectSibling`)
   - Review loop counter initialization
   - Check variable allocation for temporaries

3. **VAR form encoding** (`src/grue_compiler/codegen_instructions.rs:emit_instruction`)
   - Verify operand type byte generation
   - Check operand count limits

4. **String concatenation** (if used in loop)
   - May generate complex instruction sequences
   - Check for buffer overruns or incorrect byte counts

## Workarounds

### Option 1: Disable Object Listing (Temporary)
- Skip `list_contents` implementation
- Return early or print placeholder

### Option 2: Compile to V4 (Not Ideal)
- Changes target platform
- Loses V3 compatibility goal
- Not acceptable for production

### Option 3: Implement V3 erase_line Stub
- Make interpreter silently ignore erase_line in V3
- **NOT RECOMMENDED**: Masks real bug
- Could hide other instruction encoding issues

## Success Criteria

Bug is fixed when:
1. ✅ V3 games compile without 0xEE byte (unless intentionally placed in data)
2. ✅ `echo "east" | cargo run --bin gruesome tests/mini_zork.z3` completes without error
3. ✅ Object listing/inventory commands work correctly
4. ✅ All existing tests still pass
5. ✅ No version-inappropriate opcodes in any V3 game

## Related Bugs

- **Bug #5**: Object tree iteration (partial fix, may be related)
- **Bug #16**: Store instruction form selection (similar instruction encoding issue)

## References

- Z-Machine Standard: VAR opcodes section
- `src/display_v3.rs:199-201` - erase_line rejection
- `src/grue_compiler/opcodes.rs:434` - EraseLine definition
- `examples/mini_zork.grue:427-432` - list_contents function
- `docs/EXIT_SYSTEM_IMPLEMENTATION_PLAN.md` - Original mention of this issue

## Debugging Tools Reference

### Compiler Debug Output
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- file.grue -o out.z3
```
Shows:
- Instruction emission with opcodes and operands
- Code addresses as instructions are generated
- IR translation decisions
- Function boundaries and labels

### Interpreter Instrumentation
Add to `src/interpreter.rs` before instruction execution:
```rust
log::debug!("PC={:04x} opcode={:02x} form={:?}",
    self.vm.pc, inst.opcode, inst.form);
```

### Disassembler Usage
```bash
cargo run --bin gruedasm-txd file.z3 | grep -A10 "Routine R00XX"
```
Cross-reference routine numbers with compiler's USER_FUNCTION output.

### Key Principle
**If these tools can't find the bug, improve the tools - don't bypass them with binary analysis.**

---

**Last Updated**: October 10, 2025
**Assigned To**: Investigation in progress
**Blocked By**: None
**Blocking**: V3 game completion, exit system testing
