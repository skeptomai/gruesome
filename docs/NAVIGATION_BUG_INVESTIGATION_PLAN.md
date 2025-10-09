# Navigation Bug Investigation Plan

## Problem Statement

When user types "north" (or any direction), the game responds "You can't go that way" even though:
- Exit properties are correctly compiled with dictionary addresses
- Exit pseudo-properties are implemented
- get_exit builtin code is generated
- Dictionary resolution works correctly (verified: location 0x1877 contains 0x07df for "north")

BUT the get_exit code never executes - Variable(1) contains 0x0001 (player object) instead of 0x07df (north dictionary address).

## What We KNOW For Certain

1. **Compilation produced correct code on ONE path:**
   - Location 0x1877 contains 0x07df (dictionary address for "north")
   - This is in verb "north" { default => handle_go("north") }
   - The DictionaryRef resolution worked correctly

2. **Runtime executes DIFFERENT code:**
   - Variable(1) = 0x0001 (player object) not 0x07df
   - Code at 0x1877 never executes
   - get_exit never calls get_prop_addr (confirmed by no logging)
   - Variable(244) is correctly set to 0x0002 (player.location)

3. **The mystery:**
   - Where does 0x0001 come from?
   - Which code path sets Variable(1) = 0x0001?
   - Why isn't verb "north" handler at 0x1877 running?

## Possible Avenues

1. **Grammar dispatcher routing** - Maybe verb "north" isn't matched, and "go" or another verb handles it
2. **Parser issue** - Maybe "north" isn't found in dictionary correctly
3. **Multiple handlers** - Maybe there are multiple code paths and we're executing the wrong one
4. **Function parameter** - Maybe Variable(1) is a local in a different function entirely
5. **Call stack issue** - Maybe handle_go is being called from unexpected context

## Investigation Plan

### Phase 1: Identify Which Code Executes

**Compiler instrumentation:**
```rust
// In codegen.rs grammar handler generation:
log::error!("📍 VERB_HANDLER: '{}' code range 0x{:04x}-0x{:04x}", verb, start_addr, end_addr);
log::error!("📍 PATTERN_HANDLER: '{}' pattern {:?} at 0x{:04x}", verb, pattern, handler_addr);
```

**Interpreter instrumentation:**
```rust
// In vm.rs write_variable():
if var == 1 {
    log::error!("🔍 WRITE_VAR: var=1, value=0x{:04x}, PC=0x{:04x}, opcode=0x{:02x}, call_depth={}",
        value, self.pc, self.game.memory[self.pc.saturating_sub(inst_size)], self.call_stack.len());
    // Log call stack
    for (i, frame) in self.call_stack.iter().enumerate() {
        log::error!("  Frame {}: return_pc=0x{:04x}, num_locals={}", i, frame.return_pc, frame.num_locals);
    }
}
```

**Test source instrumentation:**
```grue
fn handle_go(direction) {
    print("DEBUG: handle_go called\n");
    print("DEBUG: direction value = ");
    print_num(direction);
    print("\n");

    clear_quit_state();
    let exit = player.location.get_exit(direction);
    // ... rest
}
```

### Phase 2: Trace Grammar Dispatch

**Interpreter instrumentation for parse buffer:**
```rust
// After tokenize/parse operation:
log::error!("📖 PARSE_BUFFER: word_count={}, words:", word_count);
for i in 0..word_count {
    let dict_addr = parse_buffer[i];
    log::error!("  Word {}: dict_addr=0x{:04x}", i, dict_addr);
}
```

**Interpreter instrumentation for branches:**
```rust
// In do_branch():
if self.pc >= 0x1500 && self.pc <= 0x2000 {  // Grammar code range
    log::error!("🔀 BRANCH: PC=0x{:04x}, condition={}, branch_sense={}, target=0x{:04x}",
        self.pc, condition, branch.on_true, target);
}
```

### Phase 3: Find the 0x0001 Source

**Detailed instruction logging when Variable(1) is written:**
```rust
// In interpreter.rs before each instruction type:
if next_instruction_will_write_var_1 {
    log::error!("🎯 INSTRUCTION: PC=0x{:04x}, opcode=0x{:02x}, form={:?}, operands={:?}",
        self.vm.pc, inst.opcode, inst.form, operands);
}
```

## Key Questions to Answer

1. **Does "north" appear in parse buffer with address 0x07df?**
   - If NO: dictionary lookup failing
   - If YES: grammar dispatch is broken

2. **Which verb handler code actually runs (what PC range)?**
   - Is it verb "north" (0x082a region)?
   - Is it verb "go" (different region)?
   - Is it something else entirely?

3. **What instruction type writes Variable(1) = 0x0001?**
   - Store instruction (0x41/2OP:01)?
   - Function call return?
   - Parameter move from stack to local?

4. **Is this even in handle_go, or a completely different function?**
   - Check call stack depth when Variable(1) is written
   - Compare to expected depth for handle_go

## Execution Strategy

### Step 1: Minimal Compiler Instrumentation
Add logging to track:
- Verb handler code ranges
- Pattern handler locations
- Which handlers use string literals vs $noun

### Step 2: Minimal Interpreter Instrumentation
Add logging to track:
- Parse buffer contents (dict addresses)
- Variable(1) writes with full context (opcode, call stack, PC)

### Step 3: Run Test and Analyze
```bash
echo "north" | RUST_LOG=error ./target/debug/gruesome tests/mini_zork.z3 2>&1 | grep -E "(VERB_|WRITE_VAR|PARSE_|BRANCH)"
```

### Step 4: Based on Results
- If parse buffer wrong → investigate dictionary/tokenization
- If wrong verb matched → investigate grammar dispatcher logic
- If Variable(1) write is from unexpected place → trace backwards from that instruction

## Success Criteria

We'll know we've succeeded when we can answer:
1. What code path executes when "north" is typed
2. Why Variable(1) = 0x0001 instead of 0x07df
3. What needs to be fixed to make the correct path execute

## Notes

- Avoid adding too much logging at once - be surgical
- Focus on ONE question at a time
- Use grep/filtering to extract relevant log lines
- Consider adding source-level debug prints if bytecode tracing insufficient
