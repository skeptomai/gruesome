# Compiler Routine Header Implementation Plan

## Problem Summary

**Current State**: Compiler emits bare code at initial_pc (Z-Machine spec compliant for V1-V5)
**Issue**: Disassembler tools expect routine headers, causing only 8/25 routines to be discovered
**Solution**: Modify compiler to emit routine header before first instruction, adjust initial_pc to point to instruction

## Mechanism Explanation: How This Fixes The Disassembler

**Key Insight**: The disassembler validates initial_pc by checking if it points to a valid routine.

### Current Broken Flow:
1. Disassembler reads initial_pc from header (e.g., 0x677 in mini_zork)
2. Disassembler checks if initial_pc points to routine header
3. **FAILS**: initial_pc points to bare instruction bytes, not routine header
4. Disassembler marks initial_pc as invalid routine → boundary reset → only finds 8 routines

### Proposed Fixed Flow:
1. Compiler emits routine header (0x00) at address X
2. Compiler emits first instruction at address X+1
3. Compiler sets initial_pc = X+1 (points to instruction, NOT header)
4. **Disassembler validation**: Checks if initial_pc-1 has routine header → FINDS 0x00 → validates as routine
5. **Interpreter execution**: Starts at initial_pc (X+1) → executes instruction correctly
6. **Result**: Disassembler happy (finds routine), interpreter happy (executes code)

### Why This Works:
- **Disassembler**: Uses `initial_pc-1` during routine validation → finds routine header
- **Interpreter**: Uses `initial_pc` for execution → starts at first instruction
- **Both tools satisfied with same memory layout**

## Implementation Plan

### Phase 1: Locate Current Init Code Generation

**File**: `src/grue_compiler/codegen.rs`
**Function**: `generate_init_block` (around lines 6020-6080)

**Current Code Pattern**:
```rust
let init_routine_address = self.code_address;
// Generate V3 function header: Local count
let header_location = self.code_address;
self.emit_byte(self.current_function_locals)?; // Emit actual local count
```

### Phase 2: Modify Init Block Generation

**Changes Required**:

1. **Emit Dummy Routine Header Before First Instruction**:
   ```rust
   // NEW: Emit dummy routine header (0 locals) for disassembler compatibility
   let dummy_header_address = self.code_address;
   self.emit_byte(0)?; // 0 locals - satisfies disassembler validation

   // Store actual init code start (after dummy header)
   let actual_init_start = self.code_address;
   ```

2. **Generate Init Instructions After Header**:
   ```rust
   // Continue with normal init block generation
   // This code now starts at actual_init_start (dummy_header_address + 1)
   self.generate_ir_instructions(&init_instructions)?;
   ```

### Phase 3: Adjust Header PC Calculation

**File**: `src/grue_compiler/codegen_headers.rs`
**Current Code**:
```rust
// Bytes 6-7: PC initial value (start of executable code section)
header[6] = (pc_start >> 8) as u8;
header[7] = (pc_start & 0xFF) as u8;
```

**Modified Code**:
```rust
// Bytes 6-7: PC initial value (points to first instruction after dummy header)
let actual_pc_start = pc_start + 1; // Skip dummy header byte
header[6] = (actual_pc_start >> 8) as u8;
header[7] = (actual_pc_start & 0xFF) as u8;
```

### Phase 4: Implementation Steps

1. **Backup Current State**:
   ```bash
   git add -A && git commit -m "Before routine header implementation"
   ```

2. **Modify `generate_init_block` Function**:
   - Add dummy header emission before init instructions
   - Track both header address and actual init start address
   - Pass actual_init_start to header generation

3. **Update Header PC Calculation**:
   - Modify `codegen_headers.rs` to use actual_init_start
   - Ensure initial_pc points to first instruction, not header

4. **Test Compatibility**:
   - Verify interpreter still works (executes from initial_pc correctly)
   - Verify disassembler finds expected number of routines
   - Compare with commercial game disassembly patterns

### Phase 5: Verification Commands

```bash
# Test interpreter compatibility
RUST_LOG=debug ./target/release/gruesome tests/mini_zork.z3

# Test disassembler discovery
RUST_LOG=debug ./target/release/gruedasm-txd tests/mini_zork.z3

# Compare routine counts
echo "Mini_zork routines:" && ./target/release/gruedasm-txd tests/mini_zork.z3 | grep -c "Routine"
echo "Zork I routines:" && ./target/release/gruedasm-txd resources/test/zork1/DATA/ZORK1.DAT | grep -c "Routine"
```

## Expected Results

**Before**:
- Mini_zork: 8 routines found (boundary reset issue)
- Zork I: 450+ routines found (works by accident)

**After**:
- Mini_zork: ~25 routines found (proper discovery)
- Zork I: 450+ routines found (still works)
- **Both games**: Compatible with disassembler validation logic

## Risk Assessment

**Low Risk Changes**:
- Only affects init block generation
- Interpreter execution path unchanged (still starts at initial_pc)
- Z-Machine spec compliance maintained

**Rollback Plan**:
- Revert commits if interpreter compatibility breaks
- Original working state preserved via git

## Architecture Benefits

1. **Tool Compatibility**: Makes compiled games work with existing disassembler tools
2. **Commercial Game Mimicry**: Follows same pattern as Infocom commercial games
3. **Minimal Impact**: Changes only affect init block, rest of compiler unchanged
4. **Maintains Correctness**: Both interpreter and disassembler get what they expect

This solution provides the pragmatic fix the user requested while maintaining full compatibility with both interpreter execution and disassembler tool validation.