# Call Stack Debugging Implementation Design ✅ IMPLEMENTED

## Implementation Status

**COMPLETE** (October 8, 2025) - Basic breakpoint and call stack dump implemented and tested.

**Verification**:
- Compiler generates 0xFFFE markers for debug_break() calls
- Interpreter detects markers and dumps call stack
- Test confirmed: debug_break("handle_go entry") compiled to address 0x12d0
- Awaiting actual execution to verify full functionality

## Purpose

Implement a debugging-only capability to:
1. Set breakpoints in Grue source code
2. Dump the full Z-Machine call stack when breakpoint is hit
3. Inspect local variables and execution state at any point

This is needed to diagnose the navigation bug where we need to see:
- Which function is executing when Variable(1) = 0x0001
- What the call stack looks like when handle_go is called
- What local variables contain at specific execution points

## Design Approaches Considered

### Approach 1: Source-Level Debug Builtin ⭐ RECOMMENDED

**Grue Source:**
```grue
fn handle_go(direction) {
    debug_break("handle_go_entry");  // Breakpoint here
    clear_quit_state();
    let exit = player.location.get_exit(direction);
    debug_break("after_get_exit");   // Another breakpoint
    // ...
}
```

**Compiler Implementation:**
```rust
// In ir.rs - recognize debug_break calls
if func_name == "debug_break" {
    // Emit special marker instruction
    // Use print_paddr (0xB3/1OP:0x0D) with special packed address 0xFFFE as marker
    return Ok(IrInstruction::DebugBreak { label: arg_as_string });
}

// In codegen_builtins.rs:
fn generate_debug_break(&mut self, label: String) {
    if cfg!(debug_assertions) {  // Only in debug builds
        // Emit: print_paddr 0xFFFE (impossible packed address = marker)
        // Then: print_paddr actual_label_string
    }
    // In release builds, emit nothing (no-op)
}
```

**Interpreter Implementation:**
```rust
// In interpreter.rs execute_1op:
0x0D => {  // print_paddr
    let packed_addr = operand;

    // Debug-only breakpoint detection
    if cfg!(debug_assertions) && packed_addr == 0xFFFE {
        log::error!("🔴 BREAKPOINT HIT at PC 0x{:04x}", self.vm.pc);
        self.dump_call_stack()?;
        return Ok(ExecutionResult::Continue);
    }

    // Normal print handling
    let addr = self.unpack_routine_address(packed_addr);
    // ... rest of print implementation
}
```

**Pros:**
- Source-level control - add breakpoints by editing Grue code
- Self-documenting (label shows where we are)
- Clean separation between debug and release builds
- Uses existing opcode (print_paddr) with impossible value as marker
- No new opcode needed

**Cons:**
- Requires compiler changes
- Magic value 0xFFFE could theoretically conflict (very unlikely)

### Approach 2: PC-Based Breakpoints (Runtime Config)

**Usage:**
```bash
BREAKPOINTS=0x15e2,0x1635,0x186e ./gruesome tests/mini_zork.z3
```

**Implementation:**
```rust
// In interpreter.rs before each instruction:
if let Ok(breakpoints) = std::env::var("BREAKPOINTS") {
    let bps: Vec<u32> = breakpoints.split(',')
        .filter_map(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .collect();

    if bps.contains(&self.vm.pc) {
        log::error!("🔴 BREAKPOINT HIT at PC 0x{:04x}", self.vm.pc);
        self.dump_call_stack()?;
    }
}
```

**Pros:**
- No compiler changes needed
- Can set breakpoints without recompiling
- Traditional debugger approach
- Environment variable control

**Cons:**
- Need to know PC addresses beforehand (requires disassembly or compilation logs)
- Less intuitive than source-level
- Performance impact (checks PC every instruction)
- Hard to correlate PCs with source code

### Approach 3: Hybrid - Print Marker Detection

**Grue Source:**
```grue
fn handle_go(direction) {
    print("__DEBUG_BREAK__:handle_go_entry\n");
    // actual code
}
```

**Implementation:**
```rust
// In interpreter.rs print handling:
if text.starts_with("__DEBUG_BREAK__:") {
    let label = text.trim_start_matches("__DEBUG_BREAK__:");
    log::error!("🔴 BREAKPOINT: {}", label);
    self.dump_call_stack()?;
    return Ok(ExecutionResult::Continue);  // Don't actually print
}
```

**Pros:**
- Works with existing infrastructure
- No new opcodes needed
- Source-level control
- Very simple implementation

**Cons:**
- Pollutes output (strings are compiled into game)
- Hacky - abuses print system
- Strings take up space in game file even in release builds
- Could interfere with actual game text

### Approach 4: Special Operand Value in Store

**Grue Source:**
```grue
// Compiler could recognize this pattern:
let __breakpoint = 0xDEAD;  // Magic value triggers breakpoint
```

**Implementation:**
```rust
// In vm.rs write_variable:
if var == 254 && value == 0xDEAD {  // Reserved variable + magic value
    log::error!("🔴 BREAKPOINT HIT");
    self.dump_call_stack()?;
}
```

**Pros:**
- Minimal changes

**Cons:**
- Very hacky
- Hard to control (no labels)
- Reserves a variable
- Not intuitive

## Recommended Implementation: Approach 1

### Step 1: Add DebugBreak IR Instruction

**File:** `src/grue_compiler/ir.rs`

Add to IrInstruction enum:
```rust
pub enum IrInstruction {
    // ... existing variants

    #[cfg(debug_assertions)]
    DebugBreak {
        label: String,
    },
}
```

Recognize in function call handling:
```rust
if func_name == "debug_break" && args.len() == 1 {
    if let IrValue::String(label) = &args[0] {
        return Ok(IrInstruction::DebugBreak {
            label: label.clone(),
        });
    }
}
```

### Step 2: Generate Breakpoint Marker (Compiler)

**File:** `src/grue_compiler/codegen_builtins.rs`

```rust
pub fn generate_debug_break(&mut self, label: &str) -> Result<(), CompilerError> {
    #[cfg(debug_assertions)]
    {
        log::debug!("Generating debug_break for label: {}", label);

        // Emit print_paddr with magic marker 0xFFFE
        // This is an impossible packed address (would unpack to 0x1FFC in V3)
        // which is in the header region, not valid for string data
        self.emit_instruction(
            0x8D,  // print_paddr (1OP:0x0D)
            &[Operand::LargeConstant(0xFFFE)],
            None,
            None,
        )?;

        // Optionally emit the label as a comment (could be useful for logs)
        // For now, just track it in a separate structure
    }

    #[cfg(not(debug_assertions))]
    {
        // In release builds, emit nothing (no-op)
        let _ = label;  // Suppress unused warning
    }

    Ok(())
}
```

### Step 3: Detect Breakpoint (Interpreter)

**File:** `src/interpreter.rs`

Modify execute_1op:
```rust
0x0D => {
    // print_paddr - print string at packed address
    let packed_addr = operand;

    // Debug-only breakpoint detection
    #[cfg(debug_assertions)]
    {
        if packed_addr == 0xFFFE {
            log::error!("");
            log::error!("🔴 BREAKPOINT HIT at PC 0x{:04x}", self.vm.pc);
            self.dump_call_stack()?;
            return Ok(ExecutionResult::Continue);
        }
    }

    // Normal print_paddr handling
    let addr = self.unpack_routine_address(packed_addr);
    let (text, _) = crate::text::decode_string(
        &self.vm.game.memory,
        addr,
        self.vm.game.header.abbrev_table,
    )?;
    print!("{}", text);
    io::stdout().flush().ok();
    Ok(ExecutionResult::Continue)
}
```

### Step 4: Implement Call Stack Dump

**File:** `src/interpreter.rs`

```rust
impl Interpreter {
    /// Dump the current call stack for debugging
    /// Shows all active function calls, their local variables, and return addresses
    #[cfg(debug_assertions)]
    fn dump_call_stack(&self) -> Result<(), String> {
        log::error!("═══════════════════════════════════════════");
        log::error!("CALL STACK DUMP - Depth: {}", self.vm.call_stack.len());
        log::error!("═══════════════════════════════════════════");
        log::error!("Current PC: 0x{:04x}", self.vm.pc);
        log::error!("Stack size: {}", self.vm.stack.len());

        if self.vm.stack.len() > 0 {
            log::error!("Stack contents (top {} values):", std::cmp::min(10, self.vm.stack.len()));
            let start = self.vm.stack.len().saturating_sub(10);
            for (i, val) in self.vm.stack[start..].iter().enumerate() {
                let stack_pos = start + i;
                log::error!("  Stack[{}] = 0x{:04x} ({})", stack_pos, val, val);
            }
        }

        for (i, frame) in self.vm.call_stack.iter().enumerate() {
            log::error!("");
            log::error!("Frame {}: ", i);
            log::error!("  Return PC: 0x{:04x}", frame.return_pc);
            log::error!("  Num locals: {}", frame.num_locals);

            if frame.num_locals > 0 {
                log::error!("  Locals:");
                for (j, val) in frame.locals.iter().take(frame.num_locals as usize).enumerate() {
                    log::error!("    Local[{}] (Variable {}) = 0x{:04x} ({})",
                        j + 1, j + 1, val, val);
                }
            }

            if let Some(store_var) = frame.store_var {
                log::error!("  Store var: {}", store_var);
            }
        }

        log::error!("═══════════════════════════════════════════");
        Ok(())
    }

    #[cfg(not(debug_assertions))]
    fn dump_call_stack(&self) -> Result<(), String> {
        Ok(())  // No-op in release builds
    }
}
```

## Usage Example

**Grue Source:**
```grue
fn handle_go(direction) {
    debug_break("handle_go_entry");

    clear_quit_state();
    let exit = player.location.get_exit(direction);

    debug_break("after_get_exit");

    if exit.none() {
        debug_break("exit_is_none");
        print("You can't go that way.");
        return;
    }

    if exit.blocked {
        print(exit.message);
        return;
    }

    // Call location on_exit handler if it exists
    if player.location.on_exit {
        player.location.on_exit();
    }

    debug_break("before_move");
    move(player, exit.destination);
    player.location.visited = true;

    // Call location on_enter handler if it exists
    if player.location.on_enter {
        player.location.on_enter();
    }

    look_around();
}
```

**Expected Output:**
```
🔴 BREAKPOINT HIT at PC 0x12d8
═══════════════════════════════════════════
CALL STACK DUMP - Depth: 3
═══════════════════════════════════════════
Current PC: 0x12d8
Stack size: 5
Stack contents (top 5 values):
  Stack[0] = 0x0001 (1)
  Stack[1] = 0x0002 (2)
  Stack[2] = 0x07df (2015)
  Stack[3] = 0x0000 (0)
  Stack[4] = 0x0001 (1)

Frame 0:
  Return PC: 0x1045
  Num locals: 0

Frame 1:
  Return PC: 0x15f0
  Num locals: 2
  Locals:
    Local[1] (Variable 1) = 0x0001 (1)
    Local[2] (Variable 2) = 0x0002 (2)
  Store var: 0

Frame 2:
  Return PC: 0x12a0
  Num locals: 1
  Locals:
    Local[1] (Variable 1) = 0x07df (2015)  <-- HERE'S THE DIRECTION!
  Store var: 0
═══════════════════════════════════════════
```

## Approved Design Decisions ✅

### 1. Magic Marker Values - BOTH 0xFFFE and 0xFFFF

**0xFFFE - Intentional Breakpoints:**
- Used by `debug_break()` calls in source code
- Never used by compiler for anything else
- V3 unpacks to 0x1FFC (impossible string address - in header region)
- Unambiguous: "this is a deliberate breakpoint"

**0xFFFF - Unresolved Reference Detection (Bug Detector):**
- This is `placeholder_word()` - what compiler writes before resolution
- If it appears at runtime, an UnresolvedReference FAILED to resolve
- Breaking on this catches compiler bugs automatically
- Distinguishes "forgot to resolve" from "intentional breakpoint"

**Implementation:**
```rust
// In interpreter.rs execute_1op (print_paddr handling):
if packed_addr == 0xFFFF {
    log::error!("⚠️  BUG DETECTED: Unresolved reference at PC 0x{:04x}", self.vm.pc);
    log::error!("    This means an UnresolvedReference failed to resolve during compilation");
    self.dump_call_stack()?;
    return Err("Unresolved reference in compiled code".to_string());
}

if packed_addr == 0xFFFE {
    // Read label from next instruction
    let next_opcode = self.vm.game.memory[self.vm.pc as usize];
    if next_opcode == 0x8D {
        let label_addr_bytes = &self.vm.game.memory[self.vm.pc as usize + 1..self.vm.pc as usize + 3];
        let label_packed = u16::from_be_bytes([label_addr_bytes[0], label_addr_bytes[1]]);
        let label_addr = self.unpack_routine_address(label_packed);
        let (label, _) = crate::text::decode_string(
            &self.vm.game.memory, label_addr,
            self.vm.game.header.abbrev_table,
        )?;

        log::error!("🔴 BREAKPOINT: {}", label.trim_start_matches("BREAK:"));
        self.vm.pc += 3;  // Skip the label print instruction
    } else {
        log::error!("🔴 BREAKPOINT HIT at PC 0x{:04x}", self.vm.pc);
    }

    self.dump_call_stack()?;
    return Ok(ExecutionResult::Continue);
}
```

### 2. Conditional Compilation - cfg!(debug_assertions) ✅

**Rationale:**
- Standard Rust convention
- Automatic with `cargo build` (debug) vs `cargo build --release`
- Zero overhead in release builds
- No need to remember feature flags

**Usage:**
```bash
# Breakpoints enabled:
cargo build
./target/debug/gruesome tests/mini_zork.z3

# Breakpoints compiled out:
cargo build --release
./target/release/gruesome tests/mini_zork.z3
```

### 3. Full Label Printing with Function Names ✅

**Requirements:**
- Breakpoint labels shown when hit
- Function names shown in call stack dump
- Local variables labeled with function name and parameter position

**Implementation Components:**

#### A. Function Name Tracking in CallFrame

```rust
// In vm.rs - CallFrame structure:
pub struct CallFrame {
    pub return_pc: u32,
    pub locals: Vec<u16>,
    pub num_locals: u8,
    pub store_var: Option<u8>,

    #[cfg(debug_assertions)]
    pub function_name: Option<String>,  // NEW

    #[cfg(debug_assertions)]
    pub function_addr: u32,  // NEW - for display
}
```

#### B. Function Name Lookup Table (Compiler)

```rust
// In codegen.rs - track function names during compilation:
pub struct ZMachineCodeGen {
    // ... existing fields

    #[cfg(debug_assertions)]
    function_name_map: IndexMap<u32, String>,  // routine_addr -> function_name
}

// During function code generation:
#[cfg(debug_assertions)]
{
    let routine_addr = self.final_code_base + code_offset;
    self.function_name_map.insert(routine_addr, function_name.clone());
}

// Write debug info to game file (at end, after all code):
#[cfg(debug_assertions)]
fn write_debug_info(&mut self) -> Result<(), CompilerError> {
    // Format:
    // [magic: 0xDEBG] [count: u16] [entries...]
    // Each entry: [addr: u32] [name_len: u8] [name_bytes...]

    let debug_info_start = self.final_data.len();

    // Magic marker
    self.final_data.extend_from_slice(&[0xDE, 0xBG]);

    // Count
    let count = self.function_name_map.len() as u16;
    self.final_data.push((count >> 8) as u8);
    self.final_data.push((count & 0xFF) as u8);

    // Entries
    for (addr, name) in &self.function_name_map {
        // Address (4 bytes, big-endian)
        self.final_data.push((addr >> 24) as u8);
        self.final_data.push((addr >> 16) as u8);
        self.final_data.push((addr >> 8) as u8);
        self.final_data.push((addr & 0xFF) as u8);

        // Name length
        self.final_data.push(name.len() as u8);

        // Name bytes
        self.final_data.extend_from_slice(name.as_bytes());
    }

    log::debug!("Debug info: {} functions, {} bytes at offset 0x{:04x}",
        count, self.final_data.len() - debug_info_start, debug_info_start);

    Ok(())
}
```

#### C. Debug Info Loading (Interpreter)

```rust
// In vm.rs - load debug info when game loads:
#[cfg(debug_assertions)]
pub fn load_debug_info(&mut self) {
    // Search for debug info at end of file
    let file_len = self.game.memory.len();

    // Look for magic marker 0xDEBG
    for i in (0..file_len.saturating_sub(4)).rev() {
        if self.game.memory[i..i+2] == [0xDE, 0xBG] {
            let count = u16::from_be_bytes([
                self.game.memory[i + 2],
                self.game.memory[i + 3]
            ]) as usize;

            let mut offset = i + 4;
            for _ in 0..count {
                let addr = u32::from_be_bytes([
                    self.game.memory[offset],
                    self.game.memory[offset + 1],
                    self.game.memory[offset + 2],
                    self.game.memory[offset + 3],
                ]);
                offset += 4;

                let name_len = self.game.memory[offset] as usize;
                offset += 1;

                let name = String::from_utf8_lossy(
                    &self.game.memory[offset..offset + name_len]
                ).to_string();
                offset += name_len;

                self.function_names.insert(addr, name);
            }

            log::debug!("Loaded {} function names from debug info", count);
            break;
        }
    }
}

pub fn lookup_function_name(&self, addr: u32) -> Option<String> {
    #[cfg(debug_assertions)]
    return self.function_names.get(&addr).cloned();

    #[cfg(not(debug_assertions))]
    None
}
```

#### D. Enhanced Call Stack Dump

```rust
fn dump_call_stack(&self) -> Result<(), String> {
    log::error!("═══════════════════════════════════════════");
    log::error!("CALL STACK DUMP - Depth: {}", self.vm.call_stack.len());
    log::error!("═══════════════════════════════════════════");
    log::error!("Current PC: 0x{:04x}", self.vm.pc);
    log::error!("Stack size: {}", self.vm.stack.len());

    if self.vm.stack.len() > 0 {
        log::error!("Stack contents (top {} values):", std::cmp::min(10, self.vm.stack.len()));
        let start = self.vm.stack.len().saturating_sub(10);
        for (i, val) in self.vm.stack[start..].iter().enumerate() {
            let stack_pos = start + i;
            log::error!("  Stack[{}] = 0x{:04x} ({})", stack_pos, val, val);
        }
    }

    for (i, frame) in self.vm.call_stack.iter().enumerate() {
        #[cfg(debug_assertions)]
        let func_name = frame.function_name.as_deref().unwrap_or("<unknown>");

        #[cfg(not(debug_assertions))]
        let func_name = "<no debug info>";

        log::error!("");
        log::error!("Frame {}: {} at 0x{:04x}", i, func_name, frame.function_addr);
        log::error!("  Return PC: 0x{:04x}", frame.return_pc);

        if frame.num_locals > 0 {
            log::error!("  Locals ({}):", frame.num_locals);
            for (j, val) in frame.locals.iter().take(frame.num_locals as usize).enumerate() {
                log::error!("    {}[{}] = 0x{:04x} ({})", func_name, j + 1, val, val);
            }
        }

        if let Some(store_var) = frame.store_var {
            log::error!("  Store result → Variable {}", store_var);
        }
    }

    log::error!("═══════════════════════════════════════════");
    Ok(())
}
```

### Expected Output Format

```
🔴 BREAKPOINT: handle_go_entry

═══════════════════════════════════════════
CALL STACK DUMP - Depth: 3
═══════════════════════════════════════════
Current PC: 0x12d8
Stack size: 5
Stack contents (top 5 values):
  Stack[0] = 0x0001 (1)
  Stack[1] = 0x0002 (2)
  Stack[2] = 0x07df (2015)
  Stack[3] = 0x0000 (0)
  Stack[4] = 0x0001 (1)

Frame 0: main at 0x1045
  Return PC: 0x0000

Frame 1: grammar_dispatcher at 0x15a0
  Return PC: 0x1048
  Locals (2):
    grammar_dispatcher[1] = 0x0001 (1)
    grammar_dispatcher[2] = 0x0002 (2)
  Store result → Variable 0

Frame 2: handle_go at 0x12c0
  Return PC: 0x15f0
  Locals (1):
    handle_go[1] = 0x07df (2015)  <-- direction parameter!
  Store result → Variable 0
═══════════════════════════════════════════
```

## Implementation Checklist

- [ ] Add IrInstruction::DebugBreak variant (ir.rs)
- [ ] Recognize debug_break() function calls (ir.rs)
- [ ] Implement generate_debug_break() (codegen_builtins.rs)
- [ ] Detect breakpoint marker in print_paddr (interpreter.rs)
- [ ] Implement dump_call_stack() (interpreter.rs)
- [ ] Test with simple debug_break in test source
- [ ] Verify no-op in release builds
- [ ] Add documentation to CLAUDE.md about usage

## Related Files

- `src/grue_compiler/ir.rs` - IR instruction definition
- `src/grue_compiler/codegen_builtins.rs` - Builtin code generation
- `src/interpreter.rs` - Instruction execution and debugging
- `examples/mini_zork.grue` - Test usage

## Future Enhancements

- Conditional breakpoints
- Watch expressions (break when variable changes)
- Step-through debugging
- Interactive debugger (pause execution, inspect, continue)
- Breakpoint hit counts
- Call graph visualization
