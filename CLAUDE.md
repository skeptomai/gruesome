# Infocom Z-Machine Interpreter Project Guidelines

## Debugging Guidelines

For all debugging statements in this project, use the Rust `log` crate with `debug!` and `info!` macros instead of `println!`. This provides better control over debug output and follows Rust best practices.

### Usage:
- Use `debug!()` for detailed debugging information
- Use `info!()` for important runtime information
- Use `warn!()` for warnings
- Use `error!()` for errors

### Example:
```rust
use log::{debug, info};

// Instead of:
println!("Starting interpreter at PC: {:04x}", pc);

// Use:
info!("Starting interpreter at PC: {:04x}", pc);
```

### Object Debugging:
The `ObjectTable` struct has debug dump methods for inspecting Z-Machine objects:
- `debug_dump_object(obj_num)` - Dumps a specific object (1-based numbering)
- `debug_dump_all()` - Dumps all objects in the table

These methods show:
- Object number
- All 32 attribute flags (and which are set)
- Parent, sibling, and child relationships
- Description text
- All properties with their numbers and data

Example usage:
```rust
if let Some(obj_table) = game.get_object_table() {
    obj_table.debug_dump_object(1);  // Dump object #1
}
```

Example output:
```
[DEBUG gruesome::zobject] === Object #1 ===
[DEBUG gruesome::zobject]   Attributes: [14, 28]
[DEBUG gruesome::zobject]   Set attributes: [4, 5, 6, 11, 12, 13]
[DEBUG gruesome::zobject]   Parent: 247
[DEBUG gruesome::zobject]   Sibling: 2
[DEBUG gruesome::zobject]   Child: 0
[DEBUG gruesome::zobject]   Description: "pair of hands"
[DEBUG gruesome::zobject]   Properties address: 0x0bb8
[DEBUG gruesome::zobject]   Properties:
[DEBUG gruesome::zobject]     Property 18: [70, 220, 66, 194, 66, 180] (len=6)
[DEBUG gruesome::zobject]     Property 16: [130] (len=1)
```

## Project Structure

This is a Z-Machine interpreter implementation for playing Infocom text adventure games, starting with Zork I.

### Key Components:
- `vm.rs` - Virtual machine state (PC, stack, variables)
- `instruction.rs` - Instruction decoder for all Z-Machine instruction formats
- `interpreter.rs` - Main execution loop
- `disassembler.rs` - Debugging disassembler
- `opcode_tables.rs` - Opcode mappings per Z-Machine spec v1.1
- `zobject.rs` - Object system with debug dumping
- `game.rs` - Game file loading and parsing
- `header.rs` - Z-Machine file header parsing
- `dictionary.rs` - Game dictionary and text encoding

### Testing:
- Unit tests for each component
- Integration tests that load actual Zork I data file
- Test game file located at: `resources/test/zork1/DATA/ZORK1.DAT`

### Build and Test Commands:
```bash
# Run all tests
cargo test

# Run tests with debug output
RUST_LOG=debug cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture

# Format source code files
rustfmt src/*.rs

# Or format all Rust files in the project
cargo fmt
```

## NULL Call Handling

The interpreter correctly handles calls to address 0x0000 according to the Z-Machine specification:
- In `do_call()` function in interpreter.rs, there's a special check: `if packed_addr == 0`
- When calling address 0, it returns 0 (false) without executing any code
- This is NOT the source of the "w can you attack a spirit..." garbage text
- The garbage text comes from elsewhere in the execution flow after PERFORM returns

## Critical Game Execution Requirements

**IMPORTANT**: The Zork I game must follow the proper execution path to display correctly. Do NOT use workarounds or hacks that skip instructions.

### Expected Opening Display:
```
ZORK I: The Great Underground Empire
Copyright (c) 1981, 1982, 1983 Infocom, Inc. All rights reserved.
ZORK is a registered trademark of Infocom, Inc.
Revision 88 / Serial number 840726
 
West of House
You are standing in an open field west of a white house, with a boarded front door.
There is a small mailbox here.

>
```

### Common Execution Issues:
1. **Do NOT skip the intro sequence call at 0x4f82** - This prevents copyright text and revision/serial display
2. **Do NOT ignore instruction errors with hacks** - Fix the root cause instead
3. **Follow the disassembly execution path** - The working implementation shows the correct flow
4. **Implement proper SREAD** - Don't use hardcoded input that breaks game state

### Debugging Wrong Output:
If the game shows messages like "You are already standing, I think" or "Only bats can see in the dark" instead of the proper opening, this indicates:
- Wrong execution path from the start
- Skipped intro sequence 
- Broken piracy opcode handling
- Improper branch logic in early instructions

The fix is to remove workarounds and ensure proper instruction execution from PC 0x4f05.

## Important Zork I Object Information

### Object 4 (ADVENTURER / "cretin")
- This is a stub object that moves around the map
- Printed name: "cretin"
- Property 17 (ACTION): 00 00 (no function)
- This object represents the player's physical presence in game locations

### Object 5 (ME / "you")
- A global object which is always in scope
- Commands referring to ME hit this object, rather than ADVENTURER
- Property 17 (ACTION): 29 5c (has a valid action handler function)
- Because of this, parser messages mostly refer to you as "you", even though the ADVENTURER's printed name is "cretin"

### Key Insight
The game uses two separate objects for the player:
- Object 4 (ADVENTURER) for physical location tracking
- Object 5 (ME) for command processing and parser interactions

When processing commands, especially those with dictionary type 0x32 (like 'w'), the game checks property 17 (action handler) of objects. If object 4 is used instead of object 5, it will fail because object 4 has no action handler.

## TODO: Remaining Features to Implement

### 1. Timed Interrupts (Critical Missing Feature)
- **Current Status**: NOT IMPLEMENTED
- **Impact**: **71% of Zork I sread calls use timers** - this breaks core gameplay mechanics
- **Zork I Timer Analysis**:
  - 15 out of 21 sread instructions use 4 operands (text_buffer, parse_buffer, time, routine)
  - Timer values range from 0 to 42,271 tenths of seconds (0 to 70+ minutes)
  - Each timer calls different interrupt routines for various game mechanics
  - Examples: 0x1f58 uses 70-minute timeout, 0x011e uses 7.8-second timeout

#### Implementation Requirements:

**1. Multi-threaded/Async Input System:**
```rust
// Need to handle simultaneously:
// - User input from stdin
// - Timer interrupts every time/10 seconds  
// - Routine calls that can modify game state
// - Routine return values that terminate input
```

**2. Extended sread Implementation:**
```rust
0x04 => {
    // sread with optional timer support
    let text_buffer = operands[0];
    let parse_buffer = operands[1];
    
    if operands.len() >= 4 {
        let time = operands[2];      // tenths of seconds
        let routine = operands[3];   // interrupt routine address
        
        if time > 0 && routine > 0 {
            // Use timed input with interrupts
            return self.sread_with_timer(text_buffer, parse_buffer, time, routine);
        }
    }
    
    // Fall back to regular input
    self.sread_basic(text_buffer, parse_buffer)
}
```

**3. Timer Interrupt Handler:**
```rust
fn handle_timer_interrupt(&mut self, routine_addr: u16) -> Result<bool, String> {
    // Call the interrupt routine
    // If routine returns true: terminate input immediately
    // If routine returns false: continue waiting for input
    // Routine can modify game state (decrement counters, etc.)
}
```

**4. Technical Challenges:**
- **Async I/O**: Read from stdin while handling timer interrupts
- **Game State Mutations**: Interrupt routines modify VM state during input
- **Cross-platform Timing**: Precise timing across different operating systems
- **Input Termination**: Clean termination when routine returns true

**5. Real Gameplay Impact:**
- **Lantern countdown**: Without timers, lantern never burns out (infinite light)
- **Match mechanics**: Matches last forever instead of burning out quickly
- **Candles**: Don't diminish over time
- **Troll combat**: Troll waits indefinitely instead of getting impatient
- **General pacing**: Many timed elements completely broken

**6. Priority**: This is the **most critical missing feature** for authentic Zork I gameplay

### 2. Character Input (read_char opcode)
- **Current Status**: NOT IMPLEMENTED
- **Impact**: Required for v4+ games with real-time elements and menus
- **Note**: Zork I (v3) does NOT use read_char - it's a v4+ feature

#### Implementation Challenges:
1. **Cross-platform terminal control**: Different OSes handle raw input differently
2. **Character encoding**: Must return proper ZSCII codes
3. **Special keys**: Arrow keys, function keys need mapping
4. **Timeout handling**: Async input with interrupts
5. **Raw terminal mode**: Disable line buffering, no echo, immediate return

#### Technical Requirements:
```rust
// What read_char needs:
1. Raw terminal mode (disable line buffering)
2. No echo (don't show the character typed)
3. Immediate return (don't wait for Enter)
4. Optional timeout support
5. ZSCII character mapping
```

#### Games That Use read_char (v4+):
- **Border Zone** (v5): Real-time spy thriller with timed sequences
- **Arthur** (v6): Menu-driven interface
- **Journey** (v6): Graphical adventure with menus
- **Sherlock** (v5): Character-based menu navigation
- **Bureaucracy** (v4): Real-time elements

#### Use Cases:
- Real-time games (like Border Zone with its real-time train sequences)
- Menu navigation (arrow keys, Y/N prompts)
- Games with special controls (function keys, etc.)
- Timed puzzles where each keystroke matters
- Immediate responses without Enter key

### 3. Sound Effects
- **Current Status**: IMPLEMENTED ✓
- **Impact**: Minor - only beeps in v3
- **Implementation**:
  - sound_effect opcode (VAR:0x15) implemented
  - Supports bleeps 1 and 2 using terminal bell character (\x07)
  - Safely ignores other sound numbers (for The Lurking Horror compatibility)
  - Note: Zork I doesn't use sound effects, only The Lurking Horror v3 does

### 4. Display Opcodes
- **Current Status**: Partially implemented
- **Missing**:
  - set_text_style
  - buffer_mode
  - erase_line
  - set_cursor (stubbed)
  - get_cursor
  - split_window (stubbed)

## Completed Features

✓ Core Z-Machine execution (opcodes, stack, variables)
✓ Object system with properties and attributes
✓ Parser and dictionary lookup
✓ Text encoding/decoding with abbreviations
✓ Save/Restore with Quetzal format
✓ Random number generation
✓ Branch instructions
✓ Routine calls and returns
✓ Global and local variables
✓ Property get/set operations
✓ Stack operations

## Current State Summary

The interpreter is fully playable for most of Zork I. The main limitation is the lack of timed interrupts, which makes the game easier than intended (infinite light sources) but doesn't break core gameplay. Save/restore works correctly with the standard Quetzal format, and random events (combat, thief movement) now function properly.