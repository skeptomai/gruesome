# Grue Compiler Execution Architecture

## Overview

This document describes the complete execution architecture of the Grue Z-Machine compiler, including how Grue source code maps to Z-Machine bytecode and how execution flows at runtime.

## Grue Language Execution Model

### Source Code Structure

```grue
init {
    print("ZORK I: The Great Underground Empire");
    print("Copyright (c) 2025 Grue Games. All rights reserved.");
}

main {
    print("> ");
    // Main game loop logic
}
```

### Generated Z-Machine Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Z-Machine File Structure                                    │
├─────────────────────────────────────────────────────────────┤
│ Header                                                      │
│  ├─ Initial PC → Points to Init Block Start                │
│  ├─ Static Memory Base                                     │
│  └─ Other standard Z-Machine header fields                 │
├─────────────────────────────────────────────────────────────┤
│ Static Memory                                               │
│  ├─ Property Tables                                        │
│  ├─ Object Tables                                          │
│  ├─ Dictionary                                             │
│  └─ Global Variables                                       │
├─────────────────────────────────────────────────────────────┤
│ Dynamic Memory                                              │
│  ├─ String Table                                           │
│  ├─ Main Loop Routine                                      │
│  │   ├─ Routine Header (0x00 = 0 locals)                  │
│  │   ├─ Print Prompt ("> ")                               │
│  │   ├─ SREAD instruction (wait for input)                │
│  │   └─ Jump back to main loop start                      │
│  ├─ Init Block (Direct Execution)                          │
│  │   ├─ User's init{} code compiled directly              │
│  │   └─ CALL main_loop_routine (packed address)           │
│  └─ Other generated routines                               │
└─────────────────────────────────────────────────────────────┘
```

## Execution Flow

### Runtime Execution Sequence

1. **Z-Machine Startup**
   - Interpreter loads game file
   - Sets PC to initial address from header
   - Begins execution at init block start

2. **Init Block Execution**
   ```
   PC → Init Block Start
         ├─ Execute user's init{} code directly
         ├─ Print statements, setup code, etc.
         └─ CALL main_loop_routine (0x20 opcode)
   ```

3. **Main Loop Routine Execution**
   ```
   Main Loop Routine:
         ├─ Routine Header (0x00)
         ├─ Print prompt "> "
         ├─ SREAD (wait for user input)
         ├─ Parse input
         ├─ Execute commands
         └─ Jump back to routine start (infinite loop)
   ```

### Memory Layout Example (mini_zork.z3)

```
Address Range | Content
--------------|------------------------------------------
0x0000-0x003F | Z-Machine Header
0x0040-0x05BE | Static Memory (objects, dictionary, etc.)
0x05BF-0x0BCDh| Dynamic Memory (strings, routines)
0x0BCE        | Init Block Start ← Initial PC points here
              | ├─ User init code (print statements)
              | └─ CALL 0x05F7 (main loop routine)
0x05F7        | Main Loop Routine Start
              | ├─ 0x00 (routine header)
              | ├─ Print prompt
              | ├─ SREAD instruction
              | └─ Jump back to 0x05F8
```

## Key Architectural Decisions

### 1. Init Block as Direct Execution

**Design Choice:** `init {}` blocks compile to direct execution code, not callable routines.

**Rationale:**
- Matches real Z-Machine games (Zork I starts with direct execution)
- Init code typically runs once at startup
- Simpler than making init callable
- Follows Z-Machine specification for program entry

**Implementation:**
```rust
// Generate the actual init block code directly (no routine call overhead)
for instruction in &init_block.instructions {
    self.generate_instruction(instruction)?;
}

// At end of init, call main loop
self.emit_instruction(
    0x20, // call_vs opcode (VAR form of call)
    &[Operand::LargeConstant(0x0000)], // Placeholder for main loop routine address
    None, // No store (main loop doesn't return a value)
    None, // No branch
)?;
```

### 2. Main Loop as Callable Routine

**Design Choice:** Main game loop is a proper Z-Machine routine with header.

**Rationale:**
- Enables proper calling convention (CALL instruction)
- Allows routine to be called repeatedly
- Matches Z-Machine specification for routines
- Supports packed address resolution

**Implementation:**
```rust
// Main loop should be a routine with 0 locals (like Zork I)
self.emit_byte(0x00)?; // Routine header: 0 locals

// Record the routine address (including header) for function calls
self.function_addresses.insert(main_loop_id, main_loop_routine_address);
self.record_address(main_loop_id, main_loop_routine_address);
```

### 3. CALL vs JUMP for Control Transfer

**Design Choice:** Use CALL instruction to transfer from init to main loop.

**Rationale:**
- Main loop is a proper routine, should be called not jumped to
- Matches real Z-Machine games (Zork I uses CALL for major control transfer)
- Enables proper return semantics (though main loop never returns)
- Uses packed addresses correctly

**Evidence from Zork I:**
```
Zork I first instruction: call #2a39, #8010, #ffff -> V00
```

## Comparison with Real Z-Machine Games

### Zork I Architecture

```
Header (0x4f05) → CALL #2a39 → Main Game Routine
                  ^
                  Single CALL instruction
```

### Grue Compiler Architecture

```
Header (0x0bce) → Init Block Code → CALL main_loop → Main Loop Routine
                  ^                                   ^
                  User's init{} code                  Generated main loop
```

### Key Similarities

1. **Both use CALL instructions** for major control transfer
2. **Both have main routines with headers** (0x00 for 0 locals)
3. **Both follow Z-Machine calling conventions** (packed addresses)
4. **Both start with direct execution** (not routine calls)

### Key Differences

1. **Zork I**: Single CALL → complex main routine
2. **Grue**: Init code → CALL → simpler main loop

## Reference Resolution

### Function Calls (Packed Addresses)

```rust
// Add unresolved reference for main loop call
self.reference_context.unresolved_refs.push(UnresolvedReference {
    reference_type: ReferenceType::FunctionCall,
    location: self.current_address - 2,
    target_id: main_loop_id,
    is_packed_address: true, // Function calls use packed addresses
    offset_size: 2,
});
```

### Jump Instructions (Byte Addresses)

```rust
// Jump within routine uses byte addresses
self.reference_context.unresolved_refs.push(UnresolvedReference {
    reference_type: ReferenceType::Jump,
    location: jump_location,
    target_id: target_id,
    is_packed_address: false, // Jumps use byte addresses
    offset_size: 2,
});
```

## Instruction Generation

### Routine Headers

All callable routines must start with a header:

```rust
fn generate_function_header(&mut self, function: &IrFunction) -> Result<(), CompilerError> {
    // Z-Machine function header: 1 byte for local count + 2 bytes per local (v3 only)
    let local_count = function.local_vars.len();
    self.emit_byte(local_count as u8)?;
    
    // In v3, emit default values for locals
    if self.version == ZMachineVersion::V3 {
        for _i in 0..local_count {
            self.emit_word(0)?; // Default local value = 0
        }
    }
    Ok(())
}
```

### Address Calculation

Routine addresses must include the header:

```rust
// Record main loop routine address for function calls
let main_loop_routine_address = self.current_address;

// Emit routine header
self.emit_byte(0x00)?; // 0 locals

// Record address for calls (points to header, not code)
self.record_address(main_loop_id, main_loop_routine_address);
```

## Testing and Validation

### Basic Functionality Test

```bash
# Simple game compilation and execution
RUST_LOG=info cargo run --bin grue-compiler -- debug_object_error.grue
RUST_LOG=info timeout 5s cargo run --bin gruesome debug_object_error.z3
```

**Expected Output:**
```
Simple test - no objects
[Game waits for input - main loop working correctly]
```

### Complex Game Test

```bash
# Complex game with full banner
RUST_LOG=warn timeout 3s cargo run --bin gruesome mini_zork.z3
```

**Expected Output:**
```
ZORK I: The Great Underground Empire
Copyright (c) 2025 Grue Games. All rights reserved.
ZORK is a registered trademark of Infocom, Inc.
Revision 1 / Serial number 250109
[Executes successfully until advanced features]
```

## Architecture Benefits

### 1. Z-Machine Compliance
- Follows official Z-Machine specification
- Compatible with standard Z-Machine interpreters
- Uses proper calling conventions

### 2. Real Game Compatibility
- Matches patterns from actual Infocom games
- Proven architecture through Zork I analysis
- Handles both simple and complex games

### 3. Extensibility
- Routine-based main loop enables future enhancements
- Proper address resolution supports complex function calls
- Clear separation between init and runtime phases

### 4. Debugging Support
- Clear execution flow makes debugging easier
- Proper PC tracking and instruction boundaries
- Verifiable against real Z-Machine behavior

## Future Enhancements

### Potential Improvements

1. **Multiple Init Blocks**: Support for ordered initialization
2. **Init Parameters**: Pass parameters to main routine
3. **Error Handling**: Graceful handling of init failures
4. **Optimization**: Inline simple init blocks when beneficial

### Compatibility Considerations

Any future changes must maintain:
- Z-Machine specification compliance
- Compatibility with existing Grue code
- Performance characteristics
- Debugging capabilities

## Conclusion

The Grue compiler's execution architecture successfully balances:
- **Simplicity** for Grue language users
- **Compliance** with Z-Machine specifications  
- **Compatibility** with real Z-Machine games
- **Extensibility** for future features

This architecture has been validated through:
- Analysis of real Z-Machine games (Zork I)
- Successful compilation and execution of test games
- Compatibility testing with standard interpreters
- Performance verification under various conditions