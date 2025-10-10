# Bug #18 Debugging Plan - PC Misalignment Investigation

## Strategy: Instrumentation Over Reverse Engineering

Use compiler and interpreter tools to identify the misencoded instruction causing PC to land at 0x1231 instead of 0x1230.

## Round 1: PC Trace (TRIVIAL)

### Objective
Find the exact instruction that has wrong size calculation by tracing PC advancement.

### Approach
```bash
# Re-enable interpreter PC logging (already exists in code)
# Add logging back to interpreter.rs execute_instruction():
log::debug!(
    "⚡ PC={:04x} opcode={:02x} form={:?} ops={} size={}",
    self.vm.pc - inst.size as u32,
    inst.opcode,
    inst.form,
    inst.operands.len(),
    inst.size
);

# Run with full logging
echo "east" | RUST_LOG=debug cargo run --bin gruesome tests/mini_zork.z3 2>&1 > /tmp/bug18_trace.log

# Find the problematic instruction
grep "⚡ PC=" /tmp/bug18_trace.log | tail -50
```

### What to Look For
- Last successful PC before error (should be close to 0x1231)
- Instruction where `PC + size` doesn't equal next instruction's PC
- Opcode and form of the problematic instruction

### Example Output
```
⚡ PC=1118 opcode=01 form=Long ops=2 size=5
⚡ PC=111d opcode=?? form=??? ops=? size=?  <- if this is wrong, next PC will be wrong
⚡ PC=???? <- this should be 1230 but lands at 1231 instead
```

**Complexity**: TRIVIAL - just add logging and grep
**Stop Condition**: We identify the instruction with wrong size

---

## Round 2: Compiler Output Analysis (SIMPLE)

### Objective
Understand what the compiler THINKS it emitted for the problematic instruction.

### Approach
```bash
# Compile with comprehensive logging
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3 2>&1 > /tmp/bug18_compile.log

# Find instructions emitted in the problematic range
grep "PC_TRACK: Emitting opcode=" /tmp/bug18_compile.log | grep -A2 -B2 "0x1118\|0x111\|0x112\|0x123"

# Look for the instruction that would precede 0x1230
# Check what opcode, form, and size were emitted
```

### What to Look For
- Opcode of instruction emitted just before 0x1230
- Form determination (VAR vs LONG vs SHORT)
- Size calculation (operand type bytes + operands + store/branch)
- Any warnings about form selection

### Cross-Reference
Compare compiler's size calculation with Z-Machine spec:
- SHORT form: 1 byte instruction + operands (0-1 bytes) + store/branch
- LONG form: 1 byte instruction + 2 operand bytes + store/branch
- VAR form: 1 byte instruction + 1 byte operand types + operands + store/branch

**Complexity**: SIMPLE - just grep logs and read

**Stop Condition**: We know what instruction was emitted and why its size is wrong

---

## Round 3: Disassembler Verification (SIMPLE)

### Objective
See what the disassembler thinks the instruction sequence should be.

### Approach
```bash
cargo run --bin gruedasm-txd tests/mini_zork.z3 2>&1 | grep -B10 -A5 "1230\|1231"
```

### What to Look For
- Does disassembler correctly decode 0x1230 as test_attr?
- What instruction does disassembler show BEFORE 0x1230?
- Does disassembler's sequence match compiler's intent?

### Note
If disassembler ALSO shows wrong instruction sequence, it means:
- Either: Binary is actually corrupted (unlikely)
- Or: Disassembler has same decoding bug as interpreter (possible)

**Complexity**: SIMPLE - just run and read

**Stop Condition**: We have third perspective on the instruction sequence

---

## Round 4: Form Determination Audit (MODERATE)

### Objective
Find the missing context check in determine_instruction_form().

### Approach
Based on Round 1-3 findings, examine the specific opcode:

```rust
// In codegen_instructions.rs, determine_instruction_form()
// Example: if Round 1 shows opcode 0x03 with wrong form

// Check current logic:
(0x03, ?) => Ok(InstructionForm::???)

// Cross-reference Z-Machine spec:
// Opcode 0x03 meanings:
// - 2OP:3 = div (2 operands) → LONG form
// - VAR:3 = put_prop (3 operands) → VAR form

// Add context check:
(0x03, 2) => Ok(InstructionForm::Long),  // div is 2OP
(0x03, _) => Ok(InstructionForm::Variable), // put_prop is VAR
```

### Common Ambiguous Opcodes
Based on Z-Machine spec, opcodes that can be both 2OP and VAR:
- 0x00: call_vs (VAR) - but should always be VAR (already handled)
- 0x01: je (can be 2OP or VAR depending on operand count)
- 0x02: jl (2OP only)
- 0x03: jg (2OP) vs put_prop (VAR)
- 0x04: dec_chk (2OP) vs sread (VAR)
- 0x05: inc_chk (2OP) vs print_char (VAR) - **FIXED in Bug #17**
- 0x06-0x09: Various 2OP vs VAR conflicts

### Check Method
For each opcode found in Round 1:
1. Look up in Z-Machine spec (sect15.html)
2. Check if it has 2OP and VAR variants
3. Verify determine_instruction_form() handles both cases
4. Add context check if missing

**Complexity**: MODERATE - requires spec lookup and logic analysis

**Stop Condition**: We've added the missing context check

---

## Round 5: Fix and Verify (TRIVIAL)

### Objective
Apply fix, recompile, and verify bug is resolved.

### Approach
```bash
# Add context check (example from Round 4)
# Edit src/grue_compiler/codegen_instructions.rs

# Recompile
cargo build

# Recompile mini_zork
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3

# Test
echo -e "east\ninventory\nquit" | cargo run --bin gruesome tests/mini_zork.z3

# Verify:
# 1. No "Invalid Long form opcode 0x00" error
# 2. Navigation works
# 3. All tests pass
cargo test --lib
```

**Complexity**: TRIVIAL - standard test procedure

**Stop Condition**: Bug is fixed, tests pass

---

## Decision Tree

```
START: Add PC logging (Round 1)
├─ Find problematic instruction at PC X
├─ Check compiler logs (Round 2)
│   ├─ See opcode N emitted with form F and size S
│   └─ Size S is wrong for form F
├─ Verify with disassembler (Round 3)
│   └─ Confirms opcode N is misencoded
├─ Audit form determination (Round 4)
│   ├─ Find opcode N doesn't have context check
│   └─ Add context-dependent form selection
└─ Fix and verify (Round 5)
    └─ Tests pass, bug resolved
```

## Technical Complexity Summary

| Round | Complexity | Tools | Output |
|-------|-----------|-------|--------|
| 1. PC Trace | TRIVIAL | Interpreter + grep | Exact failing instruction |
| 2. Compiler Logs | SIMPLE | Compiler + grep | What was emitted |
| 3. Disassembler | SIMPLE | gruedasm-txd | Third perspective |
| 4. Form Audit | MODERATE | Code + Z-spec | Missing context check |
| 5. Fix & Verify | TRIVIAL | cargo build/test | Confirmation |
| **Total** | **LOW-MODERATE** | | |

## Key Principles

1. **Instrumentation First**: Use logging to find the problem, not binary analysis
2. **Multiple Perspectives**: Compiler, interpreter, disassembler all provide clues
3. **Pattern Recognition**: This is Bug #17 pattern (context-dependent opcodes)
4. **Systematic Fix**: Audit ALL ambiguous opcodes, not just the failing one

## Expected Outcome

Based on Bug #16 and #17 patterns, we expect:
- **Root Cause**: Missing context check in determine_instruction_form()
- **Fix**: Add `(opcode, operand_count)` match arm for ambiguous opcode
- **Impact**: One-line fix, comprehensive comment explaining the ambiguity
- **Prevention**: Eventually audit all opcodes 0x00-0x1F for 2OP/VAR conflicts
