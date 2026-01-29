# Gruesome Quick Reference

## Essential Commands

```bash
# Build and run
cargo build --release
cargo run --bin gruesome resources/test/zork1/DATA/ZORK1.DAT

# Debug mode
RUST_LOG=debug cargo run --bin gruesome game.dat 2>debug.log

# Run tests  
cargo test
cargo test test_zork_starts -- --nocapture

# Format code
cargo fmt
```

## Key File Locations

| What | Where |
|------|-------|
| Entry point | `src/main.rs` |
| VM core | `src/vm.rs` |
| Instruction execution | `src/interpreter.rs` |
| Game file loading | `src/game.rs` |
| Test game | `resources/test/zork1/DATA/ZORK1.DAT` |
| Debug tools | `src/bin/` |

## Instruction Format Quick Reference

```
┌─────────────┬───────────┬────────────┬──────────┐
│ Form        │ Byte 1    │ Operands   │ Store?   │
├─────────────┼───────────┼────────────┼──────────┤
│ Long        │ 0xxxxxx   │ 2 small    │ Optional │
│ Short       │ 10xxxxxx  │ 0-1 any    │ Optional │
│ Short       │ 11xxxxxx  │ (omitted)  │ Optional │
│ Variable    │ 11xxxxx   │ 2-8 any    │ Optional │
│ Extended    │ 11111110  │ 0-8 any    │ Optional │
└─────────────┴───────────┴────────────┴──────────┘
```

## Variable Numbers

```
0x00      = Stack (push/pop)
0x01-0x0F = Local variables L01-L15  
0x10-0xFF = Global variables G00-G239
```

## Common Opcodes

| Op | Name | Description |
|----|------|-------------|
| 0x01 | je | Jump if equal |
| 0x02 | jl | Jump if less |
| 0x03 | jg | Jump if greater |
| 0x04 | dec_chk | Decrement and check |
| 0x05 | inc_chk | Increment and check |
| 0x0D | store | Store value in variable |
| 0x0E | insert_obj | Insert object in tree |
| 0x10 | loadb | Load byte |
| 0x11 | get_prop | Get object property |
| 0x14 | add | Add two values |
| 0x15 | sub | Subtract two values |
| 0x1A | call_2s | Call routine (store result) |
| 0x20 | call_vs | Call routine (variable args) |
| 0x54 | read_char | Read single character |
| 0xB0 | rtrue | Return true (1) |
| 0xB1 | rfalse | Return false (0) |
| 0xB2 | print | Print literal string |
| 0xB3 | print_ret | Print string and return |
| 0xE0 | call_vs | Call with 0-3 args |
| 0xE1 | storew | Store word |
| 0xE4 | sread | Read line of input |
| 0xE5 | print_char | Print single character |
| 0xE6 | print_num | Print number |

## Memory Map

```
0x00 ┌─────────────────┐
     │     Header      │ 64 bytes
0x40 ├─────────────────┤
     │                 │
     │ Dynamic Memory  │ Modifiable
     │                 │
???? ├─────────────────┤ (from header)
     │                 │
     │ Static Memory   │ Read-only
     │                 │
EOF  └─────────────────┘
```

## Text Encoding

```
Z-String: 5-bit characters, high bit ends string
Alphabets:
  A0: a-z (codes 6-31)
  A1: A-Z (codes 6-31)  
  A2: punctuation/digits
Special:
  0: space
  1: abbrev 0-31
  2: abbrev 32-63
  3: abbrev 64-95
  4: shift to A1
  5: shift to A2
```

## Object Format (V3)

```
Object Entry (9 bytes):
  [0-3]: 32 attribute flags (4 bytes)
  [4]:   Parent object
  [5]:   Sibling object
  [6]:   Child object
  [7-8]: Property table address

Property Entry:
  Size byte: bits 7-5 = size-1, bits 4-0 = prop number
  Data: 1-8 bytes
```

## Parse Buffer Format

```
Text Buffer:
  [0]: Max length
  [1]: Actual length
  [2+]: Characters

Parse Buffer:
  [0]: Max words
  [1]: Word count
  [2+]: Word entries (4 bytes each)
  
Word Entry:
  [0-1]: Dictionary address (or 0)
  [2]:   Word length
  [3]:   Text buffer position
```

## Debug Functions

```rust
// In code
debug!("PC {:04x}: {}", pc, message);
info!("Important: {}", event);

// Object debugging
if let Some(obj_table) = game.get_object_table() {
    obj_table.debug_dump_object(180); // West of House
}

// Disassembly
let disasm = Disassembler::new(&game);
println!("{}", disasm.disassemble_range(0x4f05, 0x4f20)?);
```

## Common Issues

1. **Parse buffer positions**: Remember to add 2 for text buffer header
2. **Variable 0**: Always means stack, not a variable
3. **Packed addresses**: Multiply by 2 in V3 to get byte address
4. **dec_chk/inc_chk**: First operand is variable number, not value
5. **String decoding**: High bit set marks end of string

## Useful Breakpoints

```rust
// Stop at specific PC
if self.vm.pc == 0x5fda {
    debug!("Hit breakpoint at WORD-PRINT");
}

// Stop on specific instruction
if inst.opcode == 0xE4 {  // sread
    debug!("SREAD at {:04x}", pc);
}

// Stop on error
if location == 0 {
    debug!("ERROR: Location is 0!");
}
```