# Bug #17 Debugging Plan - Complexity-Based Approach

## Investigation Strategy Overview

The plan follows increasing complexity - start with simplest approaches that give maximum signal, only proceed to complex analysis if simpler methods fail.

## Phase 1: Minimal Reproduction (TRIVIAL - High Value)

### Why Start Here
- **Pros**:
  - Eliminates 90% of code complexity
  - Creates repeatable test case for iteration
  - Fast feedback loop during fix attempts
  - Can test variations easily
- **Cons**:
  - Might not reproduce if bug is context-specific
- **Complexity**: TRIVIAL - Just write small Grue file and compile

### Approach
```bash
# Minimal case: object tree iteration + print
cat > /tmp/test_bug17.grue <<'EOF'
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
        print("  " + thing.name);
    }
}

verb "test" {
    default => test()
}
EOF

cargo run --bin grue-compiler -- /tmp/test_bug17.grue -o /tmp/test.z3
echo "test" | cargo run --bin gruesome /tmp/test.z3
```

### Test Variations (TRIVIAL)
```grue
// Variation 1: Remove concatenation
for thing in box.contents() {
    print(thing.name);
}

// Variation 2: Remove loop
print("  " + box.item.name);

// Variation 3: Loop without print
for thing in box.contents() {
    let x = thing.name;
}
```

### Decision Point
- **Reproduces**: Continue with minimal case (simplifies all subsequent phases)
- **Doesn't reproduce**: Bug is in navigation/exit system interaction (switch to Phase 1B)

## Phase 1B: If Minimal Doesn't Reproduce (SIMPLE)

### Why Navigation Might Be Special
The error triggers on "east" command which:
1. Calls handle_go()
2. Calls get_exit()
3. May check exit.blocked (unimplemented pseudo-property!)
4. May print blocked message

### Alternative Minimal Case
```grue
// Test navigation code path
room start "Start" {
    exits: {
        east: blocked("You can't go that way.")
    }
}

verb "go" {
    default => handle_go()  // Use existing handler
}
```

**Complexity**: SIMPLE - Still isolated, but includes navigation system

## Phase 2: Interpreter Instrumentation (SIMPLE - High Signal)

### Why This Is Valuable
- **Pros**:
  - Shows EXACT instruction sequence that executes
  - Reveals if 0xEE is really erase_line or misinterpreted bytes
  - PC values tell us where in code the error occurs
  - Can see instruction immediately before error
- **Cons**:
  - Requires interpreter code modification
  - Generates large log output
- **Complexity**: SIMPLE - Just add logging, no logic changes

### Implementation
Edit `src/interpreter.rs` at instruction execution point:

```rust
// Before instruction dispatch
log::debug!(
    "PC={:04x} opcode={:02x} form={:?} ops={} size={}",
    self.vm.pc - inst.size as u32,
    inst.opcode,
    inst.form,
    inst.operands.len(),
    inst.size
);

// Special case for the bug
if inst.opcode == 0xEE {
    log::error!("🚨 erase_line at PC={:04x}", self.vm.pc - inst.size as u32);
}
```

### Execution
```bash
echo "test" | RUST_LOG=debug cargo run --bin gruesome /tmp/test.z3 2>&1 | grep -B20 "erase_line"
```

### What We Learn
- Exact PC where 0xEE appears
- Previous instruction's opcode, operands, size
- Whether size calculation is wrong (if PC advancement is off)

## Phase 3: Disassembler Mapping (SIMPLE - Context)

### Why This Helps
- **Pros**:
  - Shows which source function contains the bug
  - Reveals instruction context (what's around erase_line)
  - Can see if pattern is consistent
- **Cons**:
  - Disassembler might also misinterpret if bytes are corrupted
- **Complexity**: SIMPLE - Just run tools and correlate

### Approach
```bash
# Get function addresses from compiler
RUST_LOG=error cargo run --bin grue-compiler -- /tmp/test.z3 2>&1 | grep USER_FUNCTION

# Disassemble
cargo run --bin gruedasm-txd /tmp/test.z3 2>&1 | grep -B20 -A5 "ERASE_LINE"
```

### Correlation
Match PC from interpreter logs to disassembly output.
Match routine address to USER_FUNCTION output.

### What We Learn
- Which Grue function has the bug
- Instruction sequence pattern
- Whether "erase_line operands" look like garbage (suggests misinterpretation)

## Phase 4: Compiler Output Analysis (MODERATE - Deep Insight)

### Why This Is More Complex
- **Pros**:
  - Shows exactly what compiler emits
  - Can see opcode choices and operand encoding
  - Reveals IR -> codegen translation
- **Cons**:
  - Very verbose output (thousands of lines)
  - Requires understanding compiler internals
  - Must correlate with addresses
- **Complexity**: MODERATE - Need to filter/search large output

### Approach
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- /tmp/test.z3 2>&1 > /tmp/compile.log

# Search for suspicious patterns
grep "PrintChar\|0xE5" /tmp/compile.log
grep "VAR.*14\|0x0E" /tmp/compile.log  # Should be empty for V3!
grep "GetObjectChild\|GetObjectSibling" /tmp/compile.log
```

### What We Learn
- Whether compiler explicitly emits VAR:14 (smoking gun)
- PRINT_CHAR emission pattern
- Object iteration codegen pattern
- Operand encoding for VAR form instructions

## Phase 5: Code Path Analysis (MODERATE to COMPLEX)

### Based on Evidence from Phases 1-4

**Scenario A: PRINT_CHAR encoding bug (MODERATE)**

If phases 2-3 show 0xEE immediately after PRINT_CHAR:

```bash
# Find PRINT_CHAR emission code
grep -rn "emit_instruction_typed.*PrintChar" src/grue_compiler/

# Examine each call site
# Check: operand count, operand types, size calculation
```

**Pros**:
- Focused search area
- PRINT_CHAR is simple instruction
- Likely an operand type byte error

**Cons**:
- Must understand VAR form encoding rules
- Could be in string concatenation (more complex)

**Complexity**: MODERATE

---

**Scenario B: PC advancement error (MODERATE)**

If Phase 2 shows previous instruction has wrong size:

```bash
# Check instruction size calculation
grep -rn "size.*calculation\|total_size" src/grue_compiler/codegen_instructions.rs

# Look at emit_instruction_typed return value
```

**Pros**:
- Clear symptom (size mismatch)
- Centralized fix location

**Cons**:
- Affects all VAR form instructions
- Could have many edge cases

**Complexity**: MODERATE

---

**Scenario C: String concatenation bug (COMPLEX)**

If only fails with `"  " + thing.name`:

```bash
# Find string concatenation codegen
grep -rn "concat\|StringConcat" src/grue_compiler/

# Trace IR generation for binary + on strings
```

**Pros**:
- Clear trigger condition
- Can test with/without concat

**Cons**:
- String handling is complex subsystem
- May involve multiple instructions
- Temporary string storage adds complexity

**Complexity**: COMPLEX

---

**Scenario D: Object iteration bug (COMPLEX)**

If only fails with `for thing in contents()`:

```bash
# Find contents() builtin and iteration codegen
grep -rn "GetObjectChild\|GetObjectSibling" src/grue_compiler/

# Check for-loop over object tree translation
```

**Pros**:
- Known problematic area (Bug #5 history)
- Can isolate with test variations

**Cons**:
- Object tree iteration is complex
- Involves multiple Z-Machine opcodes
- Variable allocation for loop counter adds complexity

**Complexity**: COMPLEX

---

**Scenario E: Version check missing (TRIVIAL)**

If Phase 4 shows explicit VAR:14 emission:

```bash
grep -rn "EraseLine\|0x0E.*emit" src/grue_compiler/
```

**Pros**:
- Trivial fix: add `if self.version >= 4`
- Clear root cause

**Cons**:
- Seems unlikely (no code emits this)

**Complexity**: TRIVIAL

## Decision Tree (Complexity-Based)

```
START: Create minimal reproduction
├─ Reproduces (CONTINUE)
├─ Doesn't reproduce → Try navigation variant (SIMPLE)
└─ Still doesn't reproduce → Bug is elsewhere (COMPLEX - full investigation)

Add interpreter logging (SIMPLE)
├─ See 0xEE after PRINT_CHAR → Scenario A (MODERATE)
├─ See size mismatch → Scenario B (MODERATE)
├─ See weird instruction sequence → Check variations:
│   ├─ Fails without concat → Scenario D (COMPLEX)
│   └─ Fails without loop → Scenario C (COMPLEX)
└─ Can't determine → Need compiler output (MODERATE)

Run compiler with debug logging (MODERATE)
├─ See explicit VAR:14 → Scenario E (TRIVIAL FIX)
├─ See PRINT_CHAR pattern → Scenario A (MODERATE)
└─ See object iteration pattern → Scenario D (COMPLEX)
```

## Recommended Execution Order

### Round 1: Simplest Approaches (All TRIVIAL to SIMPLE)
1. Create minimal reproduction
2. Test variations to isolate trigger
3. Add interpreter logging
4. Run disassembler correlation

**Why**: These are all simple and give maximum information. Combined, they should identify the scenario.

**Stop If**: We clearly identify it's Scenario E (just add version check) or Scenario A (PRINT_CHAR encoding).

### Round 2: Targeted Investigation (MODERATE)
Based on Round 1 results, focus on one scenario:
- Scenario A: Examine PRINT_CHAR emission
- Scenario B: Examine instruction size calculation

**Why**: These are moderate complexity with high likelihood.

**Stop If**: We find the bug and can fix it.

### Round 3: Complex Subsystems (COMPLEX)
Only if Round 1+2 don't reveal issue:
- Scenario C: String concatenation deep dive
- Scenario D: Object iteration deep dive

**Why**: These are complex and time-consuming. Only pursue if evidence points here.

## Technical Pros/Cons Summary

| Phase | Complexity | Signal | Pros | Cons |
|-------|-----------|--------|------|------|
| Minimal repro | TRIVIAL | HIGH | Fast, repeatable, simplifies debugging | Might not reproduce |
| Interpreter logs | SIMPLE | VERY HIGH | Shows exact execution, clear symptoms | Verbose output |
| Disassembler | SIMPLE | MEDIUM | Context, source mapping | Could also misinterpret |
| Compiler logs | MODERATE | HIGH | Shows what's emitted | Very verbose, requires filtering |
| PRINT_CHAR analysis | MODERATE | HIGH | Focused area, likely cause | Requires encoding knowledge |
| PC advancement | MODERATE | MEDIUM | Centralized fix | Affects many instructions |
| String concat | COMPLEX | MEDIUM | Clear trigger | Complex subsystem |
| Object iteration | COMPLEX | MEDIUM | Known issue area | Multiple moving parts |

## Best-Case Scenario (Simplest Path)

1. Minimal repro works (TRIVIAL)
2. Interpreter logs show 0xEE after PRINT_CHAR (SIMPLE)
3. Find PRINT_CHAR operand type byte error (MODERATE)
4. Fix operand encoding (TRIVIAL)
5. Verify (TRIVIAL)

**Complexity**: LOW overall

## Worst-Case Scenario (Complex Path)

1. Minimal repro doesn't work (need navigation context)
2. Interpreter logs ambiguous
3. Compiler logs show complex interaction between:
   - String concatenation
   - Object tree iteration
   - Property access
4. Bug is in interaction of multiple subsystems
5. Requires understanding 3+ complex areas to fix

**Complexity**: HIGH overall

## Recommended Starting Point

**Start with Round 1 (all SIMPLE or easier)** because:
- Low complexity investment
- High probability of identifying the scenario
- Fast iteration
- Even if doesn't fully solve, narrows search space dramatically

**Only proceed to Round 2/3 if Round 1 is inconclusive.**
