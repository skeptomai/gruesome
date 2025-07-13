# Infocom Z-Machine Interpreter Project

## Project Overview
This is a Z-machine interpreter written in Rust, targeting Infocom games starting with Zork I. The project implements the Z-machine virtual machine specification for running interactive fiction games.

## Current Status
- Core infrastructure is complete (instruction decoding, memory management, stack operations)
- ~50% of Z-machine instructions are implemented
- Currently debugging Zork I startup issues

## Debugging Guidelines
When adding debug output to the codebase, use the appropriate logging level:

- **`info!`**: For important execution flow and disassembly output
  - Example: `info!("0x4f05: call #10809 #32784 #65535 -> sp")`
  - With `RUST_LOG=info`: You'll see the clean disassembly without extra details

- **`debug!`**: For additional debugging information that provides context
  - Example: `debug!("  STRING: \"{}\"", text)` when decoding strings
  - Example: `debug!("  CALL routine at byte addr: {:#06x}", byte_addr)`
  - With `RUST_LOG=debug`: You'll see the disassembly and string decoding

- **`trace!`**: For very detailed debugging that may be noisy
  - Example: Variable string detection that might produce false positives
  - With `RUST_LOG=trace`: You'll see everything including conservative variable string detection

This means:
- With `RUST_LOG=info`: You'll only see the clean disassembly without the extra string details
- With `RUST_LOG=debug`: You'll see the disassembly and string decoding
- With `RUST_LOG=trace`: You'll see everything including the conservative variable string detection

## Key Debugging Context
- Always use the Zork I file at ./resources/test/zork1/DATA/ZORK1.DAT as the gamefile for execution and debugging
- These are the attributes
```
Appendix C: Object Attributes and Properties in Zork 1
Attributes
$00 MAZEBIT Room is part of the maze.
$01 HOUSEBIT Room is part of the house.
$02 RLANDBIT Room is on dry land.
$03 ONBIT For objects, it gives light. For locations, it is lit. All
outdoor rooms should have ONBIT set.
$04 FLAMEBIT Object can be a source of re. LIGHTBIT should also be
set.
$05 VEHBIT Object can be entered or boarded by the player.
$06 LIGHTBIT Object can be turned on or o.
$07 KNIFEBIT Object can cut other objects.
$08 BURNBIT Object can be burned.
$09 READBIT Object can be read.
$0A SURFACEBIT Object is a container and hold objects which are always
visible. CONTBIT and OPENBIT should be set as well.
$0B SWITCHBIT Object can be turned on or o.
$0C TRYTAKEBIT object could be picked up but other values or routines need
to be checked.
$0D OPENBIT Object is can be opened or closed, refers to doors and
containers.
$0E CONTBIT Object is a container and can contain other objects or be
open/closed/transparent.
$0F TRANSBIT Object is transparent so objects inside it can be found even
if OPENBIT is clear.
$10 FOODBIT Object can be eaten.
$11 TAKEBIT Object can be picked up or carried
$12 ACCEPTBIT? (can accept objects)
$13 SACREDBIT
$14 PERSONBIT Object is a character in the game.
$15 DOORBIT Object is a door.
$16 DRINKBIT Object can be drunk.
$17 TOOLBIT Object can be used as a tool to open other things.
$18 CLIMBBIT Object can be climbed
$19 INTEGRALBIT Object cannot be taken separately from other objects, is
part of another object.
$1A INJUREDBIT Object is injured but not dead.
$1B ALIVEBIT Object is alive.
$1C TOUCHBIT For object, it has been taken or used. For rooms, it has
been visited.
$1D INVISIBLE Object is not detected by the game.
$1E CANTENTERBIT (or full of water bit)
$1F NONLANDBIT Room is in or near the water.
```
- And these are the properties
```
Properties
$01 NOT USED
$02 NOT USED
$03 NOT USED
$04 NOT USED
$05 SPECIALOBJS 0-2 entries of (dict addr, paddr for DESCFCN)
$06 LOCAL-GLOBALS array of obj #s that are valid to use with object
$07 BITCHECK attribute # to check???
$08 TEXT address of z-string
$09 SIZE weight or size of the object
$0A CAPACITY maximum weight or size that on object can hold
$0B FDESC addr to z-string of rst description
$0C TREASUREVALUE
$0D VALUE pts scores when taking a prized obj or entering a secret
room
$0E LDESC addr to z-string of "lie on ground" description
$0F HEALTH 0-5, 0 is healthy, 5 is dead
$10 ADJECTIVE adjective value in byte
$11 ACTION address to routine
$12 SYNONYM vocabulary addresses of synonymous tokens (must have
default name too)
$13 LAND exit
$14 OUT exit
$15 IN exit
$16 DOWN exit
$17 UP exit
$18 SW exit
$19 SE exit
$1A NW exit
$1B NE exit
$1C S exit
$1D W exit
$1E E exit
$1F N exit
```
### "Pitch Black" Issue - DETAILED ANALYSIS
- **Problem**: Game displays "It is pitch black" instead of "West of House" at startup
- **Root Cause**: Complex interaction between var82 (darkness flag) and the darkness routine at 0x6c62
- **Key Findings**:
  1. **Attribute 20 Investigation**:
     - Initially thought to be "PERSONBIT" based on the attribute table above
     - Analysis of all Zork objects shows attribute 20 is set on:
       - ALL outdoor locations (West of House, Forest, Clearing, etc.)
       - Some indoor locations with permanent lighting (Gallery, Living Room, Kitchen)
       - Light source objects (torch, candles)
     - This strongly suggests attribute 20 means "is lit" or "has light"
  
  2. **Darkness Routine Logic (at 0x6c62)**:
     - Called to determine if current location is dark
     - Checks if location has attribute 20 using `test_attr` at PC 0x6c7e
     - Branch instruction: `test_attr local1 #20 ?0x6c77` with branch byte 0x48
     - Branch byte analysis: bit 7 = 0 (branch on FALSE), bit 6 = 1 (short offset)
     - If location does NOT have attribute 20, branches backward
     - If location DOES have attribute 20, continues and sets local4 = 1 (dark)
     - **This logic appears inverted** - having attribute 20 (lit) returns 1 (dark)
  
  3. **Execution Flow**:
     - At PC 0x4f88: `store #82 #1` - initializes var82 to 1 (assumes dark)
     - Copyright/header is printed
     - At PC 0x8cb2: `jz g42 ?0x8cc1` - checks var82
     - If var82 != 0, prints "It is pitch black"
     - AFTER this, darkness routine is called to update var82
     - But the message was already printed based on the initial value
  
  4. **West of House Attributes**:
     - Raw attribute bytes: 02 40 08 00
     - Set attributes: 6 (LIGHTBIT), 9 (READBIT), 20
     - Attribute 20 being set should mean the location is lit
     - But the darkness routine treats this as dark due to inverted logic

- **Conclusion**: The attribute table in this file may be incorrect about attribute meanings, OR there's a bug in the original game's darkness routine. The routine appears to have inverted logic where lit locations are treated as dark.

### Other Known Issues
1. Game says "no verb in sentence" for valid commands
2. Revision/serial number corruption after commands

## Development Guidelines
- rust version is edition 2021

### Logging
- **IMPORTANT**: Use `info!` and `debug!` macros from the `log` crate instead of `eprintln!`
- This provides better control over logging levels and output formatting
- Example: `debug!("Setting location to object {}", value);`

### Testing
Run all tests:
```bash
cargo test
```

Run specific test suites:
```bash
cargo test call_tests
cargo test branch_tests
```

### Running the Game
```bash
cargo run -- resources/test/zork1/DATA/ZORK1.DAT
```

## Architecture Notes
- Z-machine version 3 (Zork era) implementation
- Memory layout: dynamic, static, and high memory regions
- Variable numbering: 0=stack, 1-15=locals, 16-255=globals (where global N = variable N+16)
- Object table structure with property defaults and object entries

## Z-Machine Specification Reference

### Local Copy Available
The complete Z-Machine Standards Document v1.0 is available locally at:
`/Users/cb/Projects/Z-Machine-Standard/`

Key sections:
- `sect04.html` - How instructions are encoded
- `sect05.html` - How routines are encoded  
- `sect14.html` - Complete table of opcodes
- `sect15.html` - Dictionary of opcodes (detailed descriptions)

### Instruction Encoding Summary

#### Instruction Forms
- **Long form**: Opcode byte has top bits != 11 or 10 (always 2OP)
- **Short form**: Opcode byte has top bits = 10 (0OP or 1OP)
- **Variable form**: Opcode byte has top bits = 11 (2OP or VAR)
- **Extended form**: Opcode byte = 0xBE (VAR, v5+)

#### Operand Types (2-bit encoding)
- `00` - Large constant (2 bytes)
- `01` - Small constant (1 byte)
- `10` - Variable (1 byte)
- `11` - Omitted (0 bytes)

#### Routine Structure (v1-4)
1. First byte: Number of locals (0-15)
2. Next 2 bytes per local: Default values
3. Followed by: Instruction stream

Example: LIT? routine at 0x6c62:
- Byte 0: `04` (4 locals)
- Bytes 1-8: Default values (0, 1, 0, 0)
- Byte 9+: First instruction

### Key Opcodes Reference

#### 2OP Opcodes
- `0x00` - **Reserved/undefined** (shows as unknown_2op)
- `0x01` - `je` (jump if equal)
- `0x02` - `jl` (jump if less)
- `0x03` - `jg` (jump if greater)
- `0x04` - `dec_chk` (decrement and check)
- `0x05` - `inc_chk` (increment and check)
- `0x06` - `jin` (jump if in)
- `0x07` - `test` (test bits)
- `0x08` - `or` (bitwise OR)
- `0x09` - `and` (bitwise AND)
- `0x0A` - `test_attr` (test object attribute)
- `0x0B` - `set_attr` (set object attribute)
- `0x0C` - `clear_attr` (clear object attribute)
- `0x0D` - `store` (store value in variable)
- `0x0E` - `insert_obj` (insert object)
- `0x0F` - `loadw` (load word)
- `0x10` - `loadb` (load byte)
- `0x11` - `get_prop` (get property)
- `0x12` - `get_prop_addr` (get property address)
- `0x13` - `get_next_prop` (get next property)
- `0x14` - `add` (addition)
- `0x15` - `sub` (subtraction)
- `0x16` - `mul` (multiplication)
- `0x17` - `div` (division)
- `0x18` - `mod` (modulo)
- `0x19` - `call_2s` (call with 2 args, store result)
- `0x1A` - `call_2n` (call with 2 args, no result)
- `0x1B` - `set_colour` (v5+)
- `0x1C` - `throw` (v5+)

### Important Implementation Notes

1. **2OP Opcode 0x00**: Reserved/undefined in the Z-machine spec. When bytes starting with 0x00 appear in disassembly as "unknown_2op", they are usually:
   - Data bytes (not instructions)
   - Bytes after unconditional jumps
   - Part of routine headers or other non-code sections

2. **Variable Storage**: 
   - Variable 0 = Stack top
   - Variables 1-15 = Local variables
   - Variables 16-255 = Global variables (global N = variable N+16)

3. **Branch Instructions**:
   - Bit 7: 0=branch on false, 1=branch on true
   - Bit 6: 1=short form (6-bit offset), 0=long form (14-bit signed offset)
   - Special offsets: 0="return false", 1="return true"

## Next Steps
1. Fix game initialization and restart loop
2. Debug why room description isn't shown after copyright
3. Fix command parser ("no verb in sentence" issue)
4. Address serial number corruption
