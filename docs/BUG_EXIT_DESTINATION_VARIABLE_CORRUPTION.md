# Bug: exit.destination Returns 768 Instead of 2 - Variable Corruption

**Date**: October 11, 2025
**Status**: **ROOT CAUSE FOUND** - Property 21 (exit_types) not serialized to Z3 file
**Severity**: Critical - Navigation system completely broken

## Problem Statement

Navigation commands fail with error "Invalid object number: 768" when trying to move between rooms.

```grue
fn handle_go(direction) {
    let exit = player.location.get_exit(direction);
    if exit.none() { return; }
    if exit.blocked { print(exit.message); return; }
    move(player, exit.destination);  // ← FAILS HERE with object 768
}
```

Expected: `exit.destination` returns 2 (North of House object number)
Actual: `exit.destination` returns 768

## Root Cause

**Variable 2** (the `exit` local variable in handle_go) is being corrupted with value **0xC300** between the `.blocked` check and `.destination` access.

When `.destination` executes, it:
1. Reads Variable 2 → gets 0xC300 (49920)
2. Masks with 0x3FFF → gets 0x0300 (768)
3. Returns 768 as the destination object number
4. move() fails because object 768 doesn't exist (max is 14)

## Evidence Chain

### 1. Compilation Evidence

```
🔍 SLOT_2_ALLOC: Function 'handle_go' allocated slot 2 to 'exit' (IR ID 269)
```

Local variable allocation is correct - `exit` is properly assigned to slot 2.

### 2. Runtime Evidence - Variable Writes

```
🔍 WRITE_VAR_2: value=0x0001 (1), PC=0x1285, frame_depth=3
🔍 WRITE_VAR_2: value=0x07df (2015), PC=0x138f, frame_depth=2  # Different frame (list_objects)
🔍 WRITE_VAR_2: value=0x07df (2015), PC=0x13e2, frame_depth=2
...
🔍 WRITE_VAR_2: value=0xc300 (49920), PC=0x10fa, frame_depth=3  # ← THE BUG
```

**Key observation**: The write of 0xC300 happens at **frame_depth=3** (handle_go's frame), at **PC 0x10fa**.

### 3. Call Stack Context

Frame depths during execution:
- Frame 1: init block
- Frame 2: Functions called during init (like list_objects)
- Frame 3: handle_go (called in response to "north" command)

The corruption happens in handle_go's own frame, not a different function's frame.

### 4. The Corrupting Instruction

At PC 0x10fa, instruction bytes: `[41, 02, 00, 80, 09, 08, 00, 00]`

This writes 0xC300 to Variable 2 in handle_go's frame.

### 5. The Math

```
0xC300 = 49920 = 1100 0011 0000 0000
         ^^^^ ^^^^  ^^^^^^^^^^^^^^^^^^^^
         Type=11    Data=0x0300 (768)
         (invalid)

0xC300 & 0x3FFF = 0x0300 = 768
```

So 0xC300 looks like a partially-formed exit value with invalid type bits.

## Technical Architecture Issues Discovered

### Issue 1: HashMap Misuse - ir_id_to_local_var

The `ir_id_to_local_var` HashMap was being used for TWO DIFFERENT PURPOSES:

1. **Local variables** (slots 1-15): User-declared variables like `exit`, `direction`
2. **Global variables** (200+): Results from GetPropertyByNumber

**File**: `src/grue_compiler/codegen_instructions.rs:612`

**Original buggy code**:
```rust
let fresh_var = 200u8 + (self.ir_id_to_local_var.len() as u8 % 50);
self.ir_id_to_local_var.insert(*target, fresh_var);  // ← WRONG HashMap!
```

**Fix applied**: Changed GetPropertyByNumber to use `ir_id_to_stack_var` instead:
```rust
let fresh_var = self.allocate_global_for_ir_id(*target);
self.ir_id_to_stack_var.insert(*target, fresh_var);  // ← CORRECT HashMap
```

**Result**: This fixed the architectural confusion, but **the bug persists** - Variable 2 still gets corrupted with 0xC300.

### Issue 2: HashMap Not Cleared Between Functions

`ir_id_to_local_var` accumulates entries across ALL functions during compilation:
- Function 1 adds entries: {param1: 1, local1: 2}
- Function 2 adds entries: {param1: 1, local1: 2, local2: 3}
- Function 3 adds MORE entries...
- ...continues indefinitely

This means the HashMap grows to hundreds of entries. While this doesn't directly cause the bug (because we use the IR ID as the key, which is unique), it affects calculations that use `ir_id_to_local_var.len()`.

**Location**: No clearing in `generate_functions()` or `generate_function_header()`

**Impact**: Potentially causes the calculated global variable numbers to be less predictable.

### Issue 3: PC 0x10fa Mystery

handle_go starts at PC 0x026e. PC 0x10fa is **0xE8C (3724) bytes later**.

**Analysis**:
- handle_go cannot be 3.7KB of code (unrealistic for a simple navigation function)
- PC 0x10fa must be in INLINE code that executes in handle_go's frame
- Candidates: Property access, builtin functions, compiler-generated temporaries

**User functions end around PC 0x04c6**. So PC 0x10fa is in the range 0x0500-0x1700, which is likely:
- For-loop implementations
- Object tree iteration code
- Property checking code

## Critical Debugging Approach: Call Stack Return Addresses

**KEY INSIGHT FOR NEXT SESSION**: To understand the execution flow, we need to trace the COMPLETE call stack, including return addresses.

### Why This Matters

When we see `frame_depth=3`, we know there are 3 frames on the stack, but we don't know:
- What functions are in frames 1 and 2?
- What PC will we return to when the current function returns?
- What path of execution led to the current state?

### The Solution

Modify the write_variable logging to dump the ENTIRE call stack:

```rust
if var == 2 {
    log::error!("🔍 WRITE_VAR_2: value=0x{:04x}, PC=0x{:04x}, frame_depth={}",
        value, self.pc, self.call_stack.len());

    // Dump entire call stack
    for (i, frame) in self.call_stack.iter().enumerate() {
        log::error!("  Frame {}: return_addr=0x{:04x}, local_count={}, stack_pointer={}",
            i, frame.return_addr, frame.locals.len(), frame.stack_pointer);
    }
}
```

### What This Tells Us

With complete call stacks, we can:
1. **Weave together the execution flow** - see exactly what code path led to Variable 2 being written
2. **Identify the caller** - frame[frame_depth-2].return_addr tells us what code called the current function
3. **Distinguish between different executions** - see if Variable 2 writes in frame_depth=2 vs frame_depth=3 are truly different frames or the same frame at different times
4. **Map PCs to functions** - cross-reference return addresses with compiled function start addresses to identify the call chain

### Example Output (What We Want)

```
🔍 WRITE_VAR_2: value=0xc300, PC=0x10fa, frame_depth=3
  Frame 0: return_addr=0x0000, local_count=0, stack_pointer=0  (init/main)
  Frame 1: return_addr=0x0042, local_count=0, stack_pointer=2  (called from init)
  Frame 2: return_addr=0x028A, local_count=1, stack_pointer=5  (called from 0x028A in handle_go)
```

From this we can deduce:
- Frame 2's return address 0x028A is inside handle_go (which starts at 0x026e)
- So Frame 2 is a function CALLED BY handle_go
- The corruption happens while executing a function that handle_go called
- We can look at handle_go's code at PC 0x028A to see what function call that is

## Hypothesis: What's Really Happening

Based on the evidence, here's the likely scenario:

1. handle_go executes, allocates `exit` to Variable 2
2. get_exit() executes (inline code), returns packed exit value, stores in Variable 2 ✅
3. `.none()` check reads Variable 2, works correctly ✅
4. `.blocked` check reads Variable 2, works correctly ✅
5. **Between .blocked and .destination**: handle_go accesses `player.location.on_exit` property
6. This property access generates code that:
   - Calls GetObjectParent to get player.location
   - Calls GetPropertyByNumber to get the on_exit property value
   - **Somewhere in this chain, an instruction writes 0xC300 to Variable 2** ❌
7. `.destination` check reads Variable 2, gets 0xC300, returns 768 ❌

The instruction at PC 0x10fa that writes 0xC300 is likely:
- Part of GetObjectParent codegen
- Part of GetPropertyByNumber codegen
- Part of the property existence check (if player.location.on_exit)
- A temporary value calculation that incorrectly targets Variable 2

## What 0xC300 Might Be

Potential meanings of 0xC300:
1. **Property table address**: 0xC300 could be a pointer to a property table (common addresses are 0x03xx, so 0xC300 seems wrong by an order of magnitude)
2. **Intermediate calculation**: Part of a shift/multiply operation (e.g., `type << 14`)
3. **Corrupted packed value**: Bits 15-14 = 11 (invalid exit type), bits 13-0 = 0x0300 (768)
4. **Property number or offset**: Some calculation involving property 21 (on_exit)

The fact that masking gives 768 (0x0300) suggests 0xC300 = 0x4000 | 0x8000 | 0x0300, where 0x4000 and 0x8000 are the two type bits set.

## Next Steps (For Next Session)

### 1. Implement Call Stack Logging

Add full call stack dumps to Variable 2 write logging:

```rust
// In src/vm.rs, write_variable()
if var == 2 {
    log::error!("🔍 WRITE_VAR_2: value=0x{:04x}, PC=0x{:04x}, frame_depth={}",
        value, self.pc, self.call_stack.len());

    for (i, frame) in self.call_stack.iter().enumerate() {
        log::error!("  Frame[{}]: ret=0x{:04x}, locals={}, sp={}, fp={}",
            i, frame.return_addr, frame.locals.len(),
            frame.stack_pointer, frame.frame_pointer);
    }
}
```

### 2. Map Return Addresses to Functions

Cross-reference the return addresses with compiled function addresses to identify the call chain:

```
Function 'handle_go' starts at 0x026e
Function 'look_around' starts at 0x0034
Function 'list_objects' starts at 0x043e
...
```

If we see `return_addr=0x028A`, we know it's 0x1C bytes into handle_go.

### 3. Examine PC 0x10fa Code Generation

Find what IR instruction generates code at PC 0x10fa:
- Add PC mapping logs during compilation
- Check if 0x10fa is in GetObjectParent, GetPropertyByNumber, or property checking code
- Examine the actual Z-Machine instructions emitted around that PC

### 4. Audit GetObjectParent Codegen

**File**: `src/grue_compiler/codegen_instructions.rs:1069-1085`

Check if GetObjectParent incorrectly uses local variable slots:
```rust
IrInstruction::GetObjectParent { target, object } => {
    // Does this use Variable 2 anywhere?
    // Check all emit_instruction_typed calls
}
```

### 5. Examine IR for Property Access Between .blocked and .destination

The Grue source shows:
```grue
if exit.blocked { ... }
if player.location.on_exit {  // ← This code is between .blocked and .destination
    player.location.on_exit();
}
move(player, exit.destination);
```

Check the generated IR for the `if player.location.on_exit` check - does it generate instructions that could write to Variable 2?

### 6. Consider The Nuclear Option: Use Stack Instead of Locals

If we can't find the bug, consider changing the `exit` variable to use stack (Variable 0) or a global (Variable 200+) instead of a local variable:

**Option A**: Modify IR generation to mark `exit` as needing a global
**Option B**: Change codegen to detect critical variables and allocate them to globals
**Option C**: Use stack push/pop pattern for exit value instead of storing in a local

## Files Modified (This Session)

1. **src/grue_compiler/codegen_instructions.rs:602-620**
   - Changed GetPropertyByNumber to use `ir_id_to_stack_var` instead of `ir_id_to_local_var`
   - Uses `allocate_global_for_ir_id()` for proper global variable allocation
   - Fixed architectural confusion about variable allocation

2. **src/vm.rs:399-419**
   - Added frame_depth logging to Variable 2 writes
   - Added instruction bytes logging for debugging
   - **TODO**: Add full call stack dump (return addresses, locals count, stack pointers)

3. **src/opcodes_math.rs:54-58**
   - Added logging for AND operations with 0x3FFF (exit.destination)
   - Shows operand values for debugging

## Related Architecture Documentation

See `docs/ARCHITECTURE.md` - "Navigation System Architecture" section for:
- Complete exit system implementation details
- Exit bit encoding format
- Pseudo-property implementations
- get_exit() algorithm
- IR ID to object number translation

## Test Case

```bash
# Compile
RUST_LOG=warn cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3

# Create test input
echo -e "north\nlook\nquit\ny" > /tmp/nav_test.txt

# Run with logging
RUST_LOG=error timeout 3 ./target/debug/gruesome tests/mini_zork.z3 < /tmp/nav_test.txt 2>&1 | grep "WRITE_VAR_2"
```

Expected: Player moves north successfully
Actual: "Invalid object number: 768"

## Key Insight: The Call Stack Is The Key

The most important debugging tool we're missing is **complete call stack visibility**. Every time Variable 2 is written, we need to see:
- All frames on the stack
- Return address in each frame (tells us the calling function and exact call site)
- Local variable counts (validates frame structure)
- Stack pointers (validates stack integrity)

With this information, we can:
1. **Weave together the execution narrative** - see exactly what sequence of function calls led to the corruption
2. **Identify the guilty instruction** - map PC 0x10fa to the actual IR instruction and function that generated it
3. **Understand the timing** - see if corruption happens during property access, function call setup, or some other operation

**For next session: START HERE. Add call stack logging and let it tell us the story.**

## BREAKTHROUGH: Root Cause Identified (October 11, 2025 - Session 2)

### The Corruption Chain

Using call stack logging and memory dumps, the complete corruption chain has been identified:

1. **PC 0x10EC**: Variable 216 ← 0x0300 (768)
2. **PC 0x10F0**: Variable 216 ← 0xC300 (49920) [CORRUPTION HAPPENS HERE]
3. **PC 0x10F7**: Variable 2 ← Variable 216 (copies 0xC300 to exit local variable)
4. **PC 0x10FA**: `je Variable(2), 0x00` (exit.none() check reads corrupted value)

### The Smoking Gun

**Instruction at PC 0x10F7**: `store Variable(2), Variable(216)`
- Opcode bytes: `2D 02 D8`
- Decoded: `store Variable(2), Variable(216)`
- This copies the corrupted value from Variable 216 into Variable 2

**Variable 216 Corruption**:
- At PC 0x10EC: Variable 216 = 0x0300 (correct room object number 768... wait, that's wrong too!)
- At PC 0x10F0: Variable 216 = 0xC300 (corrupted with type bits)

### The Pattern

0xC300 = 0x0300 | 0xC000
- 0x0300 = 768 (decimal)
- 0xC000 = bits 15-14 set to `11` (invalid exit type)
- Bits 15-14 = `11` is NOT a valid exit type:
  - `00` = room destination
  - `01` = blocked
  - `10` = unused
  - `11` = unused

**This looks like code trying to OR type bits onto a value, but using the wrong type (11 instead of 00 or 01).**

### Call Stack at Time of Corruption

```
Frame[0]: return_pc=0x0000, num_locals=0, stack_base=0
Frame[1]: return_pc=0x0e63, num_locals=7, stack_base=1  
Frame[2]: return_pc=0x161e, num_locals=2, stack_base=1  ← handle_go
```

Frame[2] is handle_go (has 2 locals: direction and exit). The return address 0x161e tells us the code at PC 0x10F0-0x10FA is executing as part of a function CALLED BY handle_go.

### Memory Layout Around Corruption

```
PC 0x10E0-0x10EF:
  EF 00 D6 8F 00 40 00 00  6F ED EF D8  68 00 D8 D8

PC 0x10F0-0x10FF:
  8C 00 06 08 00 00 D8  2D 02 D8  41 02 00 80 09 08 00 00

Decoded instructions:
  PC 0x10E8: loadw Variable(237), 239 -> Variable(216)
  PC 0x10EC: or Variable(0), ??? -> Variable(216)  [WRITES 0x0300]
  PC 0x10F0: ??? instruction -> Variable(216)  [WRITES 0xC300]
  PC 0x10F7: store Variable(2), Variable(216)  [COPIES CORRUPTION]
  PC 0x10FA: je Variable(2), 0x00  [READS CORRUPTED VALUE]
```

### What's Generating This Code?

The code at PC 0x10E8-0x10FA is NOT in any user function (last user function ends at 0x04c6). This is compiler-generated code, likely from:
- Exit pseudo-property access (.destination, .blocked, .none)
- Inline property checking  
- For-loop iteration code
- Object tree navigation

The return address 0x161e (which is also beyond user functions) suggests this is deep in compiler-generated builtin/inline code.

### Next Steps

1. **Find what IR code generates PC 0x10E8-0x10FA**: Add compilation logging to map PC ranges to IR instructions
2. **Identify the OR operation at 0x10EC**: Understand why it writes 0x0300 (is 768 even correct?)
3. **Identify the instruction at 0x10F0**: What writes 0xC300? Is it trying to set exit type bits?
4. **Check Variable 237**: The loadw at 0x10E8 reads from Variable(237) at index 239 - what's that?

### Hypothesis

The compiler is generating code to:
1. Read property data into Variable 216
2. OR it with type bits to create a packed exit value
3. BUT it's using the wrong type bits (0xC000 = type 11 instead of 0x0000 = type 00)

OR alternatively:
1. The value 0x0300 (768) is already wrong - should be 0x0002 (object 2)
2. Then OR'ing with 0xC000 makes it worse

**Critical Question**: Why is the room object number 768 instead of 2?


## DEBUGGING PLAN: Use Our Own Tools (October 11, 2025 - Session 2)

### Why We're Failing to Debug This

We **own the entire toolchain** (compiler, interpreter, disassembler) yet we're struggling to answer basic questions like "why is Variable 216 = 768 instead of 2?" 

**Root Causes of Debugging Failure:**

1. **We're guessing at instruction decoding instead of USING our own decoder**
   - We have `src/instruction.rs` that decodes Z-Machine instructions perfectly
   - We're manually parsing hex dumps with Python scripts like archaeologists
   - We should use `Instruction::decode()` to tell us exactly what each instruction is

2. **We don't know what CODE GENERATED PC 0x10E8-0x10FA**
   - The compiler emits code but doesn't log "I just wrote bytes X at PC Y for IR instruction Z"
   - We can't map problematic runtime PCs back to the IR/builtin that generated them

3. **We can't trace Variable 216's value backwards**
   - We see it becomes 0x0300, but don't know where 0x0300 came from
   - We only log WRITES, not READS - missing half the data flow

4. **We're not using the disassembler we built**
   - We have `--disasm-range` flag but it didn't work in testing
   - We should fix it and use it, not xxd hex dumps

### Concrete 5-Phase Debugging Plan

#### Phase 1: Decode Instructions Properly (30 minutes)

**Goal**: Use our own instruction decoder to see what's actually executing

**Action 1.1**: Add instruction decoding to runtime logging
```rust
// In src/vm.rs or src/interpreter.rs
impl VM {
    pub fn decode_instruction_at(&self, pc: u32) -> Result<Instruction, String> {
        Instruction::decode(&self.game.memory, pc as usize)
            .map_err(|e| format!("Decode error at PC 0x{:04x}: {}", pc, e))
    }
    
    pub fn format_instruction_at(&self, pc: u32) -> String {
        match self.decode_instruction_at(pc) {
            Ok(inst) => format!("{:?}", inst),  // Or use Display trait
            Err(e) => format!("ERROR: {}", e),
        }
    }
}
```

**Action 1.2**: Use it in Variable 216 write logging
```rust
// In src/vm.rs, write_variable()
if var == 216 {
    // Decode the instruction that just executed
    // Note: self.pc might have advanced, so we need to figure out the actual instruction PC
    let inst_str = self.format_instruction_at(self.pc);
    
    log::error!(
        "🔍 WRITE_VAR_216: value=0x{:04x} ({}), PC=0x{:04x}, frame_depth={}, instruction: {}", 
        value, value, self.pc, self.call_stack.len(), inst_str
    );
}
```

**Action 1.3**: Also show the instruction that's ABOUT to execute (before write)
```rust
// In interpreter.rs, execute_instruction()
if inst.store_var == Some(216) {
    log::error!("🔍 ABOUT_TO_WRITE_VAR_216: instruction: {:?}", inst);
}
```

**Expected Output**: 
```
🔍 ABOUT_TO_WRITE_VAR_216: instruction: loadw Variable(237), 239 -> Variable(216)
🔍 WRITE_VAR_216: value=0x0300, PC=0x10ec, instruction: loadw Variable(237), 239 -> Variable(216)
```

This will IMMEDIATELY tell us what instructions are actually executing, not guesses.

#### Phase 2: Trace Variable 216 Reads AND Writes (15 minutes)

**Goal**: See the complete data flow of Variable 216

**Action 2.1**: Add read logging for Variable 216
```rust
// In src/vm.rs, read_variable()
pub fn read_variable(&mut self, var: u8) -> Result<u16, String> {
    let value = /* existing read logic */;
    
    if var == 216 {
        log::error!(
            "🔍 READ_VAR_216: value=0x{:04x} ({}), PC=0x{:04x}, frame_depth={}", 
            value, value, self.pc, self.call_stack.len()
        );
    }
    
    Ok(value)
}
```

**Action 2.2**: Track Variable 237 too (source of loadw)
```rust
if var == 237 {
    log::error!(
        "🔍 READ_VAR_237: value=0x{:04x} ({}), PC=0x{:04x}", 
        value, value, self.pc
    );
}
```

**Expected Output**: See the full lifecycle
```
🔍 WRITE_VAR_237: value=0x03bf (property address)
🔍 READ_VAR_237: value=0x03bf
🔍 ABOUT_TO_WRITE_VAR_216: loadw Variable(237), 239
🔍 WRITE_VAR_216: value=0x0300
🔍 READ_VAR_216: value=0x0300
🔍 ABOUT_TO_WRITE_VAR_216: or ...
🔍 WRITE_VAR_216: value=0xc300
```

#### Phase 3: Map PC to Generating IR (45 minutes)

**Goal**: Know exactly what IR instruction generated each block of Z-Machine code

**Action 3.1**: Add PC tracking to every code emission
```rust
// In src/grue_compiler/codegen.rs

impl CodeGenerator {
    pub fn emit_instruction_typed(&mut self, ...) -> Result<(), String> {
        let start_pc = self.current_pc;
        
        // ... existing emission code ...
        
        let end_pc = self.current_pc;
        
        // Log every instruction emission with PC range
        log::debug!(
            "📍 EMIT: PC 0x{:04x}-0x{:04x} ({} bytes): {:?} operands={:?} store={:?}", 
            start_pc, end_pc, end_pc - start_pc, opcode, operands, store_var
        );
        
        Ok(())
    }
}
```

**Action 3.2**: Track current IR instruction being processed
```rust
// In generate_ir_instruction() or wherever we process IR
pub fn generate_ir_instruction(&mut self, ir_inst: &IrInstruction) -> Result<(), String> {
    log::error!(
        "🔧 GEN_IR at PC 0x{:04x}: {:?}", 
        self.current_pc, ir_inst
    );
    
    // ... existing code generation ...
}
```

**Action 3.3**: Track builtin generation
```rust
// In codegen_builtins.rs, for each builtin
pub fn generate_exit_get_destination(&mut self, ...) -> Result<(), String> {
    let start_pc = self.current_pc;
    log::error!("🏗️ BUILTIN_START: exit.destination at PC 0x{:04x}", start_pc);
    
    // ... generation code ...
    
    log::error!("🏗️ BUILTIN_END: exit.destination, PC 0x{:04x}-0x{:04x}", 
        start_pc, self.current_pc);
}
```

**Expected Output**: During compilation:
```
🏗️ BUILTIN_START: exit.destination at PC 0x10e8
📍 EMIT: PC 0x10e8-0x10ec: loadw Variable(237), 239 -> Variable(216)
📍 EMIT: PC 0x10ec-0x10f0: or Variable(0), Variable(216) -> Variable(216)
📍 EMIT: PC 0x10f0-0x10f7: ??? -> Variable(216)
🏗️ BUILTIN_END: exit.destination, PC 0x10e8-0x10fa
```

This tells us EXACTLY what builtin/IR generated the problematic code.

#### Phase 4: Fix and Use Disassembler (20 minutes)

**Goal**: Get clean human-readable disassembly of problematic regions

**Action 4.1**: Test existing disassembler
```bash
# Try the existing flag
./target/debug/gruesome tests/mini_zork.z3 --disasm-range 0x10e8-0x1100 2>&1 | head -50
```

**Action 4.2**: If broken, add simple dump function
```rust
// In src/vm.rs or new file src/disassemble.rs
impl VM {
    pub fn disassemble_range(&self, start: u32, end: u32) -> Result<String, String> {
        let mut output = String::new();
        let mut pc = start;
        
        while pc < end && (pc as usize) < self.game.memory.len() {
            match Instruction::decode(&self.game.memory, pc as usize) {
                Ok(inst) => {
                    output.push_str(&format!("0x{:04x}: {}\n", pc, inst));
                    pc += inst.size as u32;
                }
                Err(e) => {
                    output.push_str(&format!("0x{:04x}: ERROR: {}\n", pc, e));
                    break;
                }
            }
        }
        
        Ok(output)
    }
}
```

**Action 4.3**: Use it in debugging
```bash
# Add a simple command to dump a range
./target/debug/gruesome tests/mini_zork.z3 --dump-code 0x10e8 0x1100
```

**Expected Output**: Clean disassembly
```
0x10e8: loadw Variable(237), 239 -> Variable(216)
0x10ec: or Variable(0), Variable(216) -> Variable(216)
0x10f0: <actual decoded instruction> -> Variable(216)
0x10f7: store Variable(2), Variable(216)
0x10fa: je Variable(2), 0x00 [branch to 0x1103]
```

#### Phase 5: Add Stack Tracing (15 minutes)

**Goal**: Track if values flow through the stack (Variable 0)

**Action 5.1**: Log stack operations
```rust
// In src/vm.rs
pub fn push(&mut self, value: u16) -> Result<(), String> {
    // Only log interesting values or when debugging
    if value == 0x0300 || value == 0xc300 || value == 768 {
        log::error!(
            "📥 PUSH: value=0x{:04x} ({}), PC=0x{:04x}, stack_depth={}", 
            value, value, self.pc, self.stack.len()
        );
    }
    
    self.stack.push(value);
    Ok(())
}

pub fn pop(&mut self) -> Result<u16, String> {
    let value = self.stack.pop()
        .ok_or("Stack underflow")?;
    
    if value == 0x0300 || value == 0xc300 || value == 768 {
        log::error!(
            "📤 POP: value=0x{:04x} ({}), PC=0x{:04x}, stack_depth={}", 
            value, value, self.pc, self.stack.len()
        );
    }
    
    Ok(value)
}
```

**Expected Output**: See if 0x0300 or 0xC300 flows through stack
```
📥 PUSH: value=0x0300, PC=0x10e0
📤 POP: value=0x0300, PC=0x10ec
```

### Expected Timeline and Results

**Phase 1** (decode properly): **10-15 minutes**
- Should IMMEDIATELY tell us what instructions at 0x10EC and 0x10F0 actually are
- No more guessing with Python scripts

**Phase 2** (trace reads): **10 minutes**
- Should show us the complete data flow of Variable 216
- See where 0x0300 comes from and where it goes

**Phase 3** (map to IR): **30-45 minutes**
- Should show us WHICH builtin/IR instruction generates the bad code
- Can then examine that specific codegen function

**Phase 4** (disassemble): **15-20 minutes**
- Should give us human-readable view of the entire problematic code region
- Can see the full context, not just individual instructions

**Phase 5** (stack trace): **10 minutes**
- Should show if values flow through stack or are created directly
- Helps understand the data flow

**Total estimated time**: 75-100 minutes (1.5-2 hours)

### The Questions We WILL Answer

After executing this plan, we will definitively know:

1. **What instruction writes 0x0300 to Variable 216?** (Phase 1)
2. **Where does 0x0300 come from?** (Phase 2 + 5)
3. **What instruction writes 0xC300 to Variable 216?** (Phase 1)
4. **What is being OR'd to turn 0x0300 into 0xC300?** (Phase 1)
5. **What IR instruction/builtin generated these instructions?** (Phase 3)
6. **Why is it 768 instead of 2?** (Combination of all phases)

### Implementation Order

**START WITH PHASE 1** - This is the highest leverage change. Proper instruction decoding will immediately clarify what's happening and inform the rest of the debugging.

Once Phase 1 is done, the other phases become much easier because we'll know exactly what we're looking at.

### Key Insight

**We should never manually decode hex bytes when we have a perfect decoder already built into our interpreter.** Use `Instruction::decode()` everywhere.

---

## ROOT CAUSE FOUND (October 11, 2025)

### Complete Corruption Chain Traced

After implementing toggleable stack and instruction logging (Phase 1), we traced the complete corruption chain:

#### The Corruption Chain

1. **PC 0x10b7**: `get_prop_addr Stack, 21 -> Variable(236)`
   - Object 2 (West of House) on stack
   - Tries to get address of property 21 (exit_types)
   - **Returns 0** - property not found!

2. **PC 0x10c6**: `store Variable(239), 0`
   - Explicitly stores 0 into Variable 239

3. **PC 0x10de**: `loadb Variable(236), Variable(239) -> Stack`
   - Variable 236 = 0x0000 (from failed get_prop_addr)
   - Variable 239 = 0x0000 (explicitly set)
   - **Loads byte from memory address 0+0 = 0**
   - Memory address 0 contains value **3** (Z-Machine version number)
   - Pushes 3 onto stack

4. **PC 0x10e2**: `mul Stack, 16384 -> Stack`
   - Pops 3 from stack
   - Multiplies: 3 × 16384 = 49152 = **0xC000**
   - This is equivalent to: 3 << 14 (shifting type bits into position 15-14)
   - Pushes 0xC000 onto stack

5. **PC 0x10f0**: `or Stack, Variable(216) -> Variable(216)`
   - Pops 0xC000 from stack
   - ORs with Variable 216 (contains 0x0300 = 768)
   - Result: 0xC000 | 0x0300 = **0xC300** (49920)
   - Stores in Variable 216

6. **PC 0x10f7**: `store Variable(2), Variable(216)`
   - Reads Variable 216 (0xC300)
   - Stores in Variable 2 (the `exit` local variable)
   - **Variable 2 now corrupted with 0xC300**

#### The Root Cause: Missing Property 21 (exit_types)

The compiler **creates** property 21 (exit_types) correctly:

```
🔍 ROOM_PROPS: Room 'west_of_house' has 7 properties:
🔍   Property 20: Bytes([255, 255, 255, 255, 255, 255])  # exit_directions
🔍   Property 21: Bytes([0, 0, 1])                        # exit_types ← EXISTS!
🔍   Property 22: Bytes([0, 2, 0, 3, 255, 255])           # exit_data
```

But at runtime:
- Property 20 (exit_directions) is found at address 0x03bf ✅
- **Property 21 (exit_types) returns 0 (not found)** ❌
- Property 22 (exit_data) would be found if we got there

**Conclusion**: Property 21 exists in the compiler's object properties but is **NOT being serialized to the Z3 file**.

#### What Value 3 Means

The value 3 loaded from memory address 0 is the **Z-Machine version number** stored in the game header:

```
Z3 file format:
0x00: Version (1 byte) = 0x03 for V3 games
0x01: Flags 1
...
```

The code is reading from memory address 0 because:
1. `get_prop_addr` failed to find property 21 → returned 0
2. Code treats 0 as a valid address
3. Loads byte from address 0 → gets version byte (3)
4. Shifts 3 << 14 → 0xC000 (bits 15-14 = 11 = invalid exit type)
5. ORs with 0x0300 → 0xC300
6. `.destination` masks → 0x0300 = 768

### The Fix

The issue is in property serialization. Property 21 (exit_types) is:
- Created in `codegen_objects.rs` at line 498: `room_properties.set_bytes(exit_types_prop, exit_types.clone());`
- **BUT** not being written to the Z3 file during object serialization

Need to investigate:
1. Why property 21 isn't in the serialized property table
2. Whether properties 20-22 have special size handling that's failing
3. Whether property ordering causes property 21 to be skipped

---

## ACTUAL ROOT CAUSE FOUND (October 12, 2025)

### Property 21 WAS Serialized Correctly

Further investigation revealed:
- Property 21 (exit_types) IS written to Z3 file ✅
- Property 21 IS found at runtime ✅
- The "Invalid object number: 768" error was NOT caused by missing property 21

### Real Bug: Object Tree vs Property Synchronization

The actual bug chain:

1. **At game start**: `player.location = west_of_house` (line 452 of mini_zork.grue)
   - This is a **property assignment** - sets property 10
   - Does NOT update object tree parent relationship
   - Player's parent in object tree remains uninitialized

2. **When "north" command executes**: `player.location.get_exit(direction)`
   - `.location` uses **GetObjectParent** to read from object tree (not property 10)
   - Object tree parent is uninitialized or wrong
   - Returns object 1 (player) instead of object 2 (West of House)
   - This is CORRECT behavior - `.location` must read from object tree because `move()` uses `insert_obj`

3. **In get_exit builtin**: Receives wrong object (object 1)
   - Tries to read property 21 from object 1 (player)
   - Player doesn't have property 21 → returns 0
   - loadb(0, 0) reads memory address 0
   - Memory[0] = 3 (Z-Machine version byte)
   - 3 << 14 = 0xC000 (invalid exit type)
   - Result: "Invalid object number: 768"

### The Architecture Issue

There's a **fundamental synchronization problem** between properties and object tree:

**Property Assignment**: `player.location = west_of_house`
- Sets property 10 to value 2
- Does NOT update object tree

**Property Read**: `player.location`
- Uses **GetObjectParent** (reads object tree)
- Does NOT read property 10
- Returns player's parent in object tree

**Why this architecture?**
- The `move()` builtin uses Z-Machine `insert_obj` instruction
- `insert_obj` updates object tree parent relationships
- Therefore `.location` MUST read from object tree to see moves
- But initialization via property assignment doesn't update the tree

### Current Status

**Crash Fixed**: ✅ No more "Invalid object number: 768"
- Added proper instrumentation to find root cause
- Identified object tree initialization issue

**Navigation Broken**: ❌ Player doesn't move between rooms
- Initial object tree not set up correctly
- `player.location` returns wrong value at game start
- Need to either:
  1. Initialize object tree at compile time, OR
  2. Use `move(player, west_of_house)` for initialization, OR
  3. Make `.location` property assignment also call `insert_obj`

### Files Involved

- `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ir.rs:2960-2965` - GetObjectParent for `.location`
- `/Users/cb/Projects/infocom-testing-old/infocom/examples/mini_zork.grue:452` - Property assignment initialization
- `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/codegen_builtins.rs:1118-1400` - get_exit builtin

### Next Steps

Need to resolve property vs object tree synchronization for `.location` property.

---

## Synchronization Solutions (October 12, 2025)

### The Core Issue

We have **two different systems** tracking object location:

**System 1: Object Tree** (Z-Machine native)
- Parent/child/sibling relationships built into Z-Machine spec
- Updated by `insert_obj` opcode (0x0E)
- Read by `get_parent` opcode (0x03)
- Used by `move()` builtin

**System 2: Property 10** (`location`)
- Standard object property in property table
- Updated by `put_prop` opcode (0x03 VAR form)
- Read by `get_prop` opcode (0x11)
- Set by `player.location = value` assignment

**Current Implementation**:
- `.location` **reads** from object tree via GetObjectParent ✅ (correct for runtime after move())
- `.location` **writes** to property 10 via SetPropertyByNumber ❌ (doesn't update tree)
- **Result**: Read and write are desynchronized!

### Option 1: Make `.location` Assignment Use insert_obj

**Implementation**:
- Intercept `player.location = value` at IR generation (ir.rs)
- Detect when left side is PropertyAccess with property="location"
- Generate `IrInstruction::InsertObj` instead of `SetPropertyByNumber`
- Keep `.location` reads using GetObjectParent

**Code Location**: `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/ir.rs:2186-2216` (assignment handling)

**Pros**:
- Works seamlessly with existing `move()` builtin
- No game code changes required
- `.location` behaves like native Z-Machine parent relationship
- Maintains abstraction - game code doesn't need to know about insert_obj

**Cons**:
- Property assignment has special behavior for exactly one property
- Creates implicit magic behavior (property assignment that's not really a property)
- May confuse users who expect property semantics

**Example**:
```grue
player.location = west_of_house;  // Compiles to insert_obj(player, west_of_house)
let loc = player.location;        // Compiles to get_parent(player)
```

### Option 2: Initialize Object Tree at Compile Time

**Implementation**:
- Compiler analyzes initial object containment from world definition
- Sets parent pointers in compiled object tree during codegen
- Player starts in west_of_house without runtime initialization

**Code Location**: `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/codegen.rs:4600-4800` (object tree generation)

**Pros**:
- Clean, matches how original Infocom games work
- No runtime overhead for initialization
- Object tree is correct from game start
- No special cases in IR generation

**Cons**:
- Requires compiler to understand initial object containment from world {} blocks
- Need to track where objects are defined (room contains object)
- Player object defined outside world {} - need special handling
- More complex compiler logic

**Example**:
```grue
world {
    room west_of_house {
        object mailbox { ... }  // Compiler sets mailbox.parent = west_of_house
    }
}

// Somewhere in init:
player.location = west_of_house;  // Compiler sets player.parent = west_of_house at compile time
```

**Implementation Details**:
- Parse `contains {}` blocks to build containment graph
- Track initial locations during semantic analysis
- Write correct parent pointers when building object tree
- Handle player object special case (defined separately)

### Option 3: Use move() for Initialization

**Implementation**:
- Change game code from property assignment to function call
- `player.location = west_of_house` → `move(player, west_of_house)`
- No compiler changes needed

**Code Location**: `/Users/cb/Projects/infocom-testing-old/infocom/examples/mini_zork.grue:452`

**Pros**:
- Simplest implementation - zero compiler changes
- Explicit and clear what's happening
- Uses existing, tested `move()` builtin
- No special cases or magic behavior

**Cons**:
- Requires game code change (breaking change)
- Less elegant than property assignment syntax
- Every game needs to remember to use move() for initialization
- Property assignment syntax becomes a trap (looks like it should work but doesn't)

**Example**:
```grue
// Before (doesn't work):
player.location = west_of_house;

// After (works):
move(player, west_of_house);
```

### Recommendation Analysis Needed

**Questions to answer**:
1. How should `.location` behave semantically? Is it a "real" property or an alias for object tree parent?
2. Should property assignment and property read always be symmetric?
3. Are there other properties that need tree synchronization? (probably not - location is unique)
4. What's the user mental model? Do they think of location as a property or as tree containment?
5. Should we support both? (property 10 for save/restore, tree parent for runtime)

**Test Coverage Needed**:
- Initial game state: player in starting location
- After move(): player.location returns new location
- After property assignment: verify behavior
- Nested containment: mailbox contains leaflet

---

## DECISION: Location is Containment Only (October 12, 2025)

### Analysis Question

"Can you see any shortcoming to containment being the only interpretation of location?"

### Answer: No Shortcomings Identified

**Conclusion**: Location should ONLY be object tree containment, with NO separate property 10.

### Reasoning

**1. Z-Machine Native Model**
- Object tree parent/child relationships are the native Z-Machine way to model containment
- `insert_obj` (0x0E) and `get_parent` (0x03) are built-in opcodes
- All Infocom games use tree containment for object location
- No need to duplicate this with a separate property

**2. No Synchronization Issues**
- Reads use GetObjectParent (IR) → get_parent (opcode 0x03)
- Writes use InsertObj (IR) → insert_obj (opcode 0x0E)
- Single source of truth - object tree parent pointer
- Impossible to have desynchronization between property and tree

**3. Semantically Correct**
- "Location" means "where is this object?"
- Object tree parent IS the answer to that question
- Property 10 called "location" was redundant and confusing
- Unified model is clearer

**4. Save/Restore Works Naturally**
- Z-Machine save/restore saves entire object tree state
- Object tree parents are preserved across save/restore
- No need for separate property to persist location

**5. No Special Cases Needed**
- `.location` property access compiles to GetObjectParent
- `.location =` assignment compiles to InsertObj
- No magic behavior, no special compiler logic
- Consistent with `.parent` property (which also uses tree)

### Implementation Plan

**IR Generation** (`src/grue_compiler/ir.rs`):
- Property read: `obj.location` → `GetObjectParent`
- Property write: `obj.location = value` → `InsertObj`
- No property 10 in standard properties list

**Object Initialization** (compile time):
- Analyze `world {}` blocks for initial containment
- Set object tree parent pointers during codegen
- Player object gets parent set based on `player.location = room` statements in init

**Backward Compatibility**:
- Remove property 10 from standard properties
- Existing game code continues to work (syntax unchanged)
- Only implementation changes (property → tree operations)

### What This Eliminates

**Eliminates**:
- Property 10 ("location") from standard properties ❌
- SetPropertyByNumber for location writes ❌
- GetPropertyByNumber for location reads ❌
- Synchronization bugs between property and tree ❌
- Confusion about two systems tracking same thing ❌

**Uses Instead**:
- GetObjectParent for reads ✅
- InsertObj for writes ✅
- Single source of truth (object tree) ✅
- Native Z-Machine semantics ✅

### Compatibility with move() Builtin

Perfect compatibility:
- `move(obj, dest)` compiles to `insert_obj` opcode
- `obj.location` compiles to `get_parent` opcode
- Both operations use object tree
- No special handling needed

### User Mental Model

**User writes**: `player.location = west_of_house`
**Compiler generates**: `insert_obj(player, west_of_house)`
**User reads**: `let loc = player.location`
**Compiler generates**: `let loc = get_parent(player)`

From the user's perspective, `.location` is a property. From the compiler's perspective, it's an alias for tree parent relationship.

**This is elegant**: The abstraction matches user intent while using the efficient native implementation.

### No Shortcomings

After analysis, **no shortcomings identified**:
- ✅ Works with save/restore
- ✅ Works with move() builtin
- ✅ Works with initial object placement
- ✅ No synchronization issues
- ✅ No performance overhead
- ✅ Matches Z-Machine native model
- ✅ Clean, simple implementation
- ✅ No special cases

**Decision: Proceed with containment-only implementation.**

