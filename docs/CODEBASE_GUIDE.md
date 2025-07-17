# Gruesome Codebase Guide for New Contributors

## Quick Start

If you're new to the codebase, here's the recommended reading order:

1. **Start here**: `main.rs` - See how the interpreter is launched
2. **Game loading**: `game.rs` - Understand how game files are loaded
3. **Core VM**: `vm.rs` - Learn about the virtual machine state
4. **Execution**: `interpreter.rs` - See how instructions are executed
5. **Instructions**: `instruction.rs` - Understand instruction encoding

## Module Relationships

```
main.rs
  ├── game.rs (Game::from_file)
  │     ├── header.rs (Header::from_bytes)
  │     └── zobject.rs (ObjectTable::new)
  │
  ├── vm.rs (VM::new) 
  │     ├── Uses Game struct
  │     ├── Manages stack/call frames
  │     └── Handles memory access
  │
  └── interpreter.rs (Interpreter::new)
        ├── Uses VM struct
        ├── instruction.rs (decode)
        ├── text.rs (printing)
        ├── dictionary.rs (parsing)
        └── debug_symbols.rs (debugging)
```

## Key Code Paths

### 1. Starting the Game

```rust
// main.rs
let game = Game::from_file(&args[1])?;
let vm = VM::new(game);
let mut interpreter = Interpreter::new(vm);
interpreter.run()?;
```

### 2. Main Execution Loop

```rust
// interpreter.rs - simplified
pub fn run(&mut self) -> Result<(), String> {
    loop {
        // Fetch
        let pc = self.vm.pc;
        let inst = Instruction::decode(&self.vm.game.memory, pc)?;
        
        // Update PC
        self.vm.pc += inst.size as u32;
        
        // Execute
        match self.execute_instruction(&inst)? {
            ExecutionResult::Continue => continue,
            ExecutionResult::Quit => break,
            // ... other cases
        }
    }
}
```

### 3. Reading User Input (SREAD)

```rust
// interpreter.rs - handle_sread()
1. Read text from user
2. Store in text buffer
3. Parse into words (dictionary.rs)
4. Execute timer if specified
5. Continue execution
```

### 4. Printing Text

```rust
// Text output flow:
print instruction -> decode_string() -> print!()
                          ↓
                  Handle abbreviations
                  Handle alphabet shifts  
                  Handle special chars
```

## Important Functions to Know

### Core Functions

| Module | Function | Purpose |
|--------|----------|---------|
| `game.rs` | `Game::from_memory()` | Load and validate game file |
| `vm.rs` | `VM::read_word()` | Read 16-bit value from memory |
| `vm.rs` | `VM::write_byte()` | Write to dynamic memory |
| `vm.rs` | `VM::read_variable()` | Get variable value (stack/local/global) |
| `interpreter.rs` | `execute_instruction()` | Main instruction dispatcher |
| `interpreter.rs` | `do_call()` | Handle routine calls |
| `interpreter.rs` | `do_branch()` | Handle conditional branches |
| `instruction.rs` | `Instruction::decode()` | Decode instruction at address |
| `text.rs` | `decode_string()` | Decode Z-string to UTF-8 |
| `dictionary.rs` | `lookup_dictionary()` | Find word in game dictionary |

### Debugging Functions

| Module | Function | Purpose |
|--------|----------|---------|
| `disassembler.rs` | `disassemble_range()` | Disassemble instructions |
| `debug_symbols.rs` | `format_address()` | Show routine names |
| `zobject.rs` | `debug_dump_object()` | Display object details |

## Common Patterns

### 1. Error Handling

```rust
// Always use Result<T, String> for errors
pub fn some_operation(&self) -> Result<u16, String> {
    if condition_failed {
        return Err("Description of error".to_string());
    }
    Ok(value)
}

// Propagate errors with ?
let value = self.read_word(addr)?;
```

### 2. Memory Access

```rust
// Reading is straightforward
let byte = self.vm.read_byte(addr);
let word = self.vm.read_word(addr);

// Writing checks for dynamic memory
self.vm.write_byte(addr, value)?;  // Can fail if static memory
```

### 3. Debug Logging

```rust
use log::{debug, info, warn, error};

debug!("Detailed trace: PC={:04x}", pc);
info!("Important event: {}", description);
warn!("Potential issue: {}", warning);
error!("Error occurred: {}", error);
```

### 4. Instruction Patterns

```rust
match inst.opcode {
    0x01 => {
        // je - jump if equal
        let condition = op1 == op2;
        self.do_branch(inst, condition)
    }
    0x0D => {
        // store - save value to variable
        let var = inst.operands[0] as u8;
        self.vm.write_variable(var, op2)?;
        Ok(ExecutionResult::Continue)
    }
    // ... many more
}
```

## Testing and Debugging

### Running Tests

```bash
# Unit tests
cargo test

# Integration test with Zork
cargo test test_zork_starts

# Specific test
cargo test test_name
```

### Debugging Tools

```bash
# Disassemble a routine
cargo run --bin disasm_routine <address>

# Trace execution
RUST_LOG=debug cargo run --bin gruesome game.dat

# Analyze specific issue
cargo run --bin analyze_word_print
```

### Adding Debug Output

1. **Temporary debugging**: Use `debug!()` with PC checks:
```rust
if self.vm.pc == 0x5fda {
    debug!("At WORD-PRINT: locals={:?}", frame.locals);
}
```

2. **Permanent traces**: Add to appropriate execution points:
```rust
debug!("Calling routine at {:04x} with {} args", addr, args.len());
```

## Code Style Guidelines

1. **Naming**:
   - Snake_case for functions and variables
   - CamelCase for types and structs
   - UPPER_CASE for constants

2. **Comments**:
   - Document "why" not "what"
   - Reference Z-Machine spec sections
   - Explain non-obvious algorithms

3. **Structure**:
   - Keep functions focused and small
   - Extract complex logic to helper functions
   - Group related functionality

4. **Testing**:
   - Unit test pure functions
   - Integration test game scenarios
   - Add regression tests for bugs

## Common Pitfalls

1. **Off-by-one errors**: Z-Machine uses both 0-based and 1-based indexing
2. **Signed vs unsigned**: Some operations need i16, others u16
3. **Address modes**: Byte addresses vs word addresses vs packed addresses
4. **Text positions**: Parse buffer positions include text buffer header offset
5. **Variable 0**: Means stack, not a real variable

## Getting Help

- **Z-Machine Spec**: Version 1.1 is the definitive reference
- **CLAUDE.md**: Project-specific implementation notes
- **ROUTINE_ADDRESSES.md**: Known routine documentation
- **Test files**: `src/bin/test_*.rs` show component usage
- **Git history**: Shows how bugs were fixed

## Next Steps

1. Run the game and try various commands
2. Pick a small bug or feature from issues
3. Write a test that demonstrates the issue
4. Fix it following existing patterns
5. Submit a PR with clear description

Welcome to the project! The Z-Machine is a fascinating piece of gaming history, and we're excited to have you help preserve it.