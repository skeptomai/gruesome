# PC Address Corruption - Comprehensive Architectural Analysis

**Date**: August 31, 2025  
**Issue**: Stack underflow runtime error due to PC pointing to invalid address  
**Root Cause**: Fundamental architectural flaws in init block, main loop, and code space generation  

## Executive Summary

Our Z-Machine compiler has a **critical architectural bug** where the Program Counter (PC) points to string data instead of executable code, causing immediate stack underflow during execution. This analysis identifies the root causes and provides a comprehensive fix plan.

### Key Findings:
- **PC points to 0x0f56** (string data) instead of executable code at 0x0344
- **first_instruction_address tracking system is fundamentally broken**
- **Address resolution across compilation phases is inconsistent**  
- **Entry point generation doesn't follow Z-Machine specification**

---

## 1. THE FUNDAMENTAL PROBLEM: PC Address Corruption

### Current Issue:
- **PC is set to 0x0f56** (points to string data, not executable code)  
- **First instruction executed: "and 0 0"** (tries to read stack operands)
- **Stack underflow** because stack is empty but instruction expects operands

### Root Cause Analysis:
From the logs, our `first_instruction_address` tracking system is **fundamentally broken**:

```
✅ First instruction tracked at: 0x0f56 (during emit_instruction)  
✅ Final code base calculated:   0x0344 (during memory layout)
⚠️  ADDRESS MISMATCH: PC will point to 0x0f56, but code starts at 0x0344
```

**The Problem**: We're tracking the first instruction address during IR translation (0x0f56), but this is an **offset within the old `story_data` array**, not a valid address in the final assembled Z-Machine image.

### Evidence from Runtime:
```
[2025-08-31T16:04:30Z INFO  gruesome] Initial PC: 0f56
[2025-08-31T16:04:30Z DEBUG gruesome::opcodes_math] and 0 0
Error during execution: Stack underflow
```

The interpreter is correctly reading the PC from the header (0x0f56) but this points to string data, not the intended executable code.

---

## 2. Z-MACHINE SPECIFICATION COMPLIANCE ANALYSIS

### What the Specification Says (Section 11.1):
- **Bytes 6-7**: "Initial value of program counter (byte address)"
- **For V1-5**: PC points directly to the first executable instruction
- **For V6**: PC field contains "Packed address of initial 'main' routine"

### What Zork I Actually Does:
From the disassembly: **Zork I PC = 0x4e36**, pointing to:
```
Main routine R0008, 0 locals ()
       CALL            R0022 (#8010,#ffff) -> -(SP)
       STOREW          (SP)+,#00,#01
       CALL            R0022 (#80f0,#ffff) -> -(SP)  
       ... [proper initialization sequence]
       JUMP            #ff66  [infinite main loop]
```

**Key Insights from Zork I:**
- **PC points to a routine header** (not raw instructions)
- **Main routine has 0 locals** (proper Z-Machine routine format)  
- **Initialization happens first** (object setup, global variables)
- **Ends with infinite main loop** (JUMP with negative offset)

### What We're Doing Wrong:
- **PC points to string data** instead of executable code
- **No proper routine header** for the entry point
- **Address tracking across compilation phases is broken**
- **Final assembly doesn't correctly map addresses**

---

## 3. CODE SPACE GENERATION ARCHITECTURAL ISSUES

### Our Current Approach - Separated Spaces Architecture:
```rust  
// During compilation, we maintain separate memory spaces
code_space: Vec<u8>,        // Code with placeholders
string_space: Vec<u8>,      // Encoded strings  
object_space: Vec<u8>,      // Object/property data
// ... then assemble them together
```

### The Problem:
1. **Address tracking is inconsistent** across compilation phases
2. **first_instruction_address** is captured during IR translation but becomes invalid after final assembly
3. **Placeholder resolution** happens after address tracking, corrupting the PC calculation

### Code Evidence:
From `src/grue_compiler/codegen.rs:1259-1268`:
```rust
if let Some(first_addr) = self.first_instruction_address {
    log::info!("  ✅ First instruction tracked at: 0x{:04x} (during emit_instruction)", first_addr);
    log::info!("  ✅ Final code base calculated:   0x{:04x} (during memory layout)", code_base);
    if first_addr == code_base {
        log::info!("  ✅ PERFECT MATCH: PC will point to start of code section");
    } else {
        log::warn!("  ⚠️  ADDRESS MISMATCH: PC will point to 0x{:04x}, but code starts at 0x{:04x}", first_addr, code_base);
        log::warn!("      This indicates instructions were generated before final assembly");
    }
}
```

**The Issue**: `first_addr` (0x0f56) is from the old compilation phase, but `code_base` (0x0344) is from final assembly. These are **completely different address spaces**.

### What the Z-Machine Spec Requires:
- **Single contiguous address space** with properly aligned sections
- **PC must point to valid executable code** (either instruction or routine header)
- **All addresses must be consistent** throughout the file

---

## 4. INIT BLOCK vs MAIN ROUTINE CONFUSION

### Our Current Confusion:
We're conflating two different concepts:
- **Init Block** (source code concept) - initialization logic from `init { ... }`
- **Main Routine** (Z-Machine concept) - the executable entry point  

### Current Implementation Issues:
From `src/grue_compiler/codegen.rs:2114-2121`:
```rust
// Phase 2.2: Generate init block if present
if let Some(init_block) = &ir.init_block {
    log::info!("🔧 TRANSLATING: Init block ({} instructions)", init_block.instructions.len());
    for (instr_i, instruction) in init_block.instructions.iter().enumerate() {
        // ... translation logic
    }
}
```

Then later, we generate a separate main loop:
```rust
// Phase 2.3: Add program flow control
self.generate_program_flow(ir)?;
```

**The Problem**: We're generating init block instructions and main loop separately, with broken address tracking between them.

### Correct Z-Machine Architecture:
1. **PC points to Main Routine** (Z-Machine entry point)
2. **Main Routine executes init block logic** as its first instructions
3. **Main Routine then enters main loop** (for interactive games)

### What We Should Do:
```
Z-Machine Entry Point (PC) → Main Routine Header (0 locals)
                          ↓
                         Init Block Instructions  
                          ↓
                         Main Loop (infinite)
```

---

## 5. MAIN LOOP IMPLEMENTATION ISSUES

### Our Current Main Loop:
From `src/grue_compiler/codegen.rs:5338-5379`:
```rust
// 1. Print prompt "> "
let layout = self.emit_instruction(
    0x8D, // print_paddr (print packed address string) - 1OP:141
    &[Operand::LargeConstant(placeholder_word())], // Placeholder for prompt string address
    None, None,
)?;

// 2. Read input (sread)  
self.emit_instruction(
    0x04, // sread opcode (VAR instruction)
    &[
        Operand::LargeConstant(text_buffer_addr),
        Operand::LargeConstant(parse_buffer_addr),
    ],
    None, None,
)?;

// 3. Jump back to step 1 (infinite loop)
```

### Problems:
- **Generates invalid bytecode** due to address resolution issues
- **Stack operations without proper setup**
- **No proper command parsing or game logic**

### What Zork I Does:
```
CALL R0025 -> -(SP)     // Call main game processing
JUMP #ff66              // Jump back (infinite loop with proper offset)
```

**Key Insight**: Zork I's main loop is **very simple** - it just calls subroutines and jumps back. All the complexity is in the subroutines.

---

## 6. CRITICAL ARCHITECTURAL FIXES NEEDED

### Phase 1: Fix PC Address Calculation (CRITICAL)
1. **Eliminate first_instruction_address tracking** - it's fundamentally broken
2. **PC should point to start of assembled code section** (after header)
3. **Generate proper routine header** as first bytes of code section

#### Implementation:
```rust
// In assemble_complete_zmachine_image()
let pc_start = self.final_code_base as u16;  // Always point to start of code
header[6] = (pc_start >> 8) as u8;
header[7] = (pc_start & 0xFF) as u8;
log::info!("✅ PC start address: 0x{:04x} (start of code section)", pc_start);

// First bytes of code_space MUST be routine header
self.code_space.insert(0, 0x00);  // 0 locals for main routine
```

### Phase 2: Unify Init Block and Main Routine (HIGH)
```rust
fn generate_main_routine(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
    // Write routine header (0 locals for main routine) as FIRST bytes
    self.write_to_code_space(0x00)?;  
    
    // Execute init block instructions first (if any)
    if let Some(init_block) = &ir.init_block {
        for instruction in &init_block.instructions {
            self.translate_ir_instruction(instruction)?;
        }
    }
    
    // Then enter main loop (for interactive mode)
    match ir.program_mode {
        ProgramMode::Script => {
            self.write_to_code_space(0xBA)?; // quit
        },
        ProgramMode::Interactive => {
            self.generate_infinite_main_loop()?;
        },
        ProgramMode::Custom => {
            self.generate_user_main_call()?;
        }
    }
}
```

### Phase 3: Fix Address Resolution (HIGH)
- **Single-pass address calculation** instead of multi-phase with placeholders
- **Consistent address space** throughout compilation
- **PC = final_code_base + 0** (always points to start of code after header)

### Phase 4: Proper Main Loop Implementation (MEDIUM)
- **Zork I-style simplicity**: call subroutines, jump back
- **Move parsing logic to subroutines** instead of inline main loop
- **Proper stack management** with balanced operations

---

## 7. COMPARISON WITH SPECIFICATION COMPLIANCE

### Current Status: ❌ NON-COMPLIANT
- PC points to invalid address (string data)
- No proper routine structure at entry point
- Address resolution system is broken
- Stack operations cause underflow

### Target Status: ✅ SPECIFICATION COMPLIANT
- PC points to valid main routine with proper header
- All addresses correctly resolved and consistent
- Proper Z-Machine routine calling conventions
- Stack operations balanced and valid

---

## 8. IMMEDIATE ACTION PLAN

### Priority 1 (Fix Today):
1. **Remove broken first_instruction_address tracking**
   - Delete `first_instruction_address: Option<usize>` field
   - Remove all `first_instruction_address` tracking code
   - Set PC to `final_code_base` directly

2. **Set PC to final_code_base** (start of code section)
   - Simplify PC calculation: `let pc_start = self.final_code_base as u16;`
   - Remove complex address translation logic

3. **Generate proper routine header** as first bytes
   - Ensure code_space starts with routine header (0x00 for 0 locals)
   - All subsequent instructions are part of this main routine

### Priority 2 (This Week):
1. **Unify init block and main routine generation**
   - Replace separate init block translation with unified main routine generation
   - Ensure single contiguous execution flow

2. **Implement proper address resolution**
   - Eliminate complex placeholder system
   - Use direct address calculation during final assembly

3. **Add comprehensive bytecode validation**
   - Verify PC points to valid routine header
   - Validate all instruction operands and addresses

### Success Criteria:
- `mini_zork.z3` starts execution without stack underflow
- PC points to valid executable code (routine header: 0x00)
- First instruction is proper Z-Machine instruction, not "and 0 0"
- Address resolution produces consistent results

---

## 9. TECHNICAL IMPLEMENTATION DETAILS

### Files to Modify:
1. **`src/grue_compiler/codegen.rs`**:
   - Remove `first_instruction_address` field and all tracking
   - Modify `assemble_complete_zmachine_image()` PC calculation
   - Rewrite `generate_all_zmachine_sections()` for unified main routine

2. **Address Resolution System**:
   - Simplify placeholder resolution to use final addresses directly
   - Remove multi-phase address translation

### Code Changes Required:

#### Remove Broken PC Tracking:
```rust
// DELETE these lines:
first_instruction_address: Option<usize>,
if self.first_instruction_address.is_none() {
    self.first_instruction_address = Some(instruction_start);
}
```

#### Fix PC Calculation:
```rust
// REPLACE complex PC calculation with:
let pc_start = self.final_code_base as u16;
header[6] = (pc_start >> 8) as u8;
header[7] = (pc_start & 0xFF) as u8;
```

#### Ensure Proper Entry Point:
```rust
// FIRST thing written to code_space:
self.code_space.push(0x00);  // Main routine header: 0 locals
```

---

## 10. VERIFICATION PLAN

### Test 1: Basic PC Validation
- **Run**: `cargo run --bin gruesome mini_zork.z3`
- **Expected**: PC should point to 0x0344 (or similar code start address)
- **Expected**: First byte at PC should be 0x00 (routine header)
- **Expected**: No "and 0 0" execution

### Test 2: Execution Flow
- **Expected**: Program starts executing proper Z-Machine instructions
- **Expected**: No stack underflow errors
- **Expected**: Proper routine calling conventions

### Test 3: Address Consistency
- **Verify**: All addresses in final image are consistent
- **Verify**: No 0xFFFF placeholders remain unresolved
- **Verify**: PC calculation matches actual code location

---

## CONCLUSION

Our current architecture has **fundamental design flaws** that prevent basic Z-Machine compliance. The separated spaces approach is sound, but our address tracking and PC calculation are completely broken. 

The core issue is that we're tracking addresses during IR translation phase, but these addresses become invalid after final assembly. The fix requires:

1. **Eliminate complex address tracking** across compilation phases
2. **Set PC to start of final code section** (simple, direct calculation)
3. **Generate proper Z-Machine routine structure** at entry point
4. **Unify init block and main routine generation** into single execution flow

This is an **architectural redesign**, not just a bug fix. We need to align our implementation with how real Z-Machine files (like Zork I) are structured according to the specification.

The fix is well-defined and should resolve the stack underflow issue completely, enabling proper execution of mini_zork and other compiled games.