# Bug #18 Analysis - Branch Patch Location Mismatch

## Current Status: ROOT CAUSE IDENTIFIED

### The Bug
Branch patches are being written to the wrong memory addresses, causing branches to jump to incorrect locations and PC to land in the middle of instructions.

### Symptoms
1. Runtime error: "Invalid Long form opcode 0x00 at address 1231"
2. PC trace shows: PC jumps from 0x1122 to 0x1225 (should be 0x1226)
3. This causes PC to land at 0x1231, in the middle of an instruction

### The Smoking Gun

**What the compiler logged:**
```
[DEBUG] 📍 ADDRESS_TRANSLATE: Code[0x02e6] -> final=0x1122
[DEBUG] 🔧 BRANCH_PATCH_ATTEMPT: location=0x1122, target_address=0x1128
[DEBUG]  WRITE_BYTE_AT: Writing byte 0x80 directly to final_data[0x1122]
[DEBUG]  WRITE_BYTE_AT: Writing byte 0x06 directly to final_data[0x1123]
[DEBUG] 🟢 BRANCH_PATCHED: location=0x1122 ← [0x80 0x06] (offset=6, target=0x1128)
```

**What's actually in the file at runtime:**
- Address 0x1122-0x1126: `0x80 0x06 0x08 0x01 0x00`
- This decodes as:
  - 0x1122: 0x80 = jz opcode (SHORT 1OP:0)
  - 0x1123-0x1124: 0x06 0x08 = large constant operand 0x0608
  - 0x1125-0x1126: 0x01 0x00 = branch bytes (offset=256, WRONG!)

**The mismatch:**
- Compiler wrote `[0x80 0x06]` to addresses 0x1122-0x1123
- But those addresses contain the opcode and first operand byte
- The branch bytes are at 0x1125-0x1126 (3 bytes later)
- Those bytes contain `[0x01 0x00]` (wrong values)

### Root Cause Analysis

**Code offset vs Final address translation:**
- UnresolvedReference stored: `location = 0x02e6` (code space offset)
- Translated to final address: `0x02e6 + 0x0e3c = 0x1122`
- Patch wrote to: `final_data[0x1122]` and `final_data[0x1123]`

**The question:** Where are the branch bytes actually located?

If the jz instruction is:
- Opcode at 0x1122 (1 byte)
- Operand at 0x1123-0x1124 (2 bytes, large constant)
- Branch at 0x1125-0x1126 (2 bytes)

Then code offset 0x02e6 should map to where the branch bytes START in code_space.

**Hypothesis:** The UnresolvedReference `location` value (0x02e6) is NOT pointing to where the branch bytes are in code_space. It's pointing somewhere else (possibly the instruction start, or an intermediate position).

### Key Evidence from Logs

**Branch emission at code offset 0x02e6:**
```
[DEBUG] 📝 EMIT_BYTE: code_offset=741, byte=0xbc, code_address=0x02e5, space_len=742
[DEBUG] 🔵 BRANCH_PLACEHOLDER: Emitting 0xffff at code_address=0x02e6 for branch (offset=-1)
```

This shows:
- Some instruction byte (0xBC) emitted at code offset 741 (code_address 0x02e5)
- Branch placeholder emitted at code_address 0x02e6 (code offset 742+743)

**The 0xBC byte:** This is likely the low byte of a jump offset from a PREVIOUS instruction, NOT an rtrue opcode (common confusion - remember: when confused about rtrue, it's usually a jump byte).

### Critical Questions

1. **Where does the jz instruction that crashes actually start in code_space?**
   - If branch bytes are at code offset 0x02e6
   - And instruction is: opcode (1) + operand (2) + branch (2) = 5 bytes total
   - Then instruction starts at code offset 0x02e6 - 3 = 0x02e3

2. **What was actually emitted at code offset 0x02e3?**
   - Need to check logs for what PC_TRACK shows at that offset

3. **Is the branch_location value from emit_short_form_with_layout correct?**
   - Line 2394: `let loc = self.code_address;` captures address BEFORE emit_word
   - This should be where branch bytes start
   - So location SHOULD be correct

### Statistics from mini_zork compilation

- Branch placeholders emitted: 131
- Branch patches applied: 131 ✓
- All branches were patched, but at least one was patched to wrong location

### The Fix (Hypothesis)

The patch is writing to translated address correctly, but the TRANSLATION is wrong because:

**Option A:** The location stored in UnresolvedReference is the instruction START, not the branch bytes location. Need to verify what `instruction_start` vs `branch_location` values are in InstructionLayout.

**Option B:** The code_space offset to final address translation is off by the instruction prefix size (opcode + operands).

**Option C:** There's a second write happening that overwrites the correct patch.

### Next Steps

1. Find where the jz instruction actually starts in code_space (should be 3 bytes before 0x02e6)
2. Verify what `instruction_start` was for this instruction's InstructionLayout
3. Check if `branch_location` == `instruction_start` or if they're different
4. Determine if the bug is in:
   - How branch_location is calculated during emission
   - How location is translated from code offset to final address
   - How patch_branch_offset interprets the location parameter

### Files to Examine

- `src/grue_compiler/codegen_instructions.rs:2367-2400` - Branch location calculation
- `src/grue_compiler/codegen.rs:9183-9265` - patch_branch_offset implementation
- `src/grue_compiler/codegen.rs:1770-1850` - Branch resolution code

### Commands to Reproduce

```bash
# Compile mini_zork with instrumentation
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/mini_zork.z3 2>&1 | tee debug_bug18_full.log

# Run to see crash
echo "east" | RUST_LOG=error timeout 3 ./target/debug/gruesome tests/mini_zork.z3

# Check specific branch patch
grep "target_id=1054" debug_bug18_full.log
grep "WRITE_BYTE_AT.*0x112[2-7]" debug_bug18_full.log
```

### Verification Needed

**CRITICAL:** Need to determine from the logs where the jz instruction that crashes actually is in code_space, to verify whether the branch_location value is correct or off by the instruction prefix size.
