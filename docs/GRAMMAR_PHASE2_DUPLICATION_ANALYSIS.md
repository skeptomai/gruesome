# Phase 2 Analysis: Duplicate Code Patterns in codegen_grammar.rs

## Overview
After Phase 1 cleanup (removed 188 lines of dead code), analyzing remaining duplication patterns for helper extraction.

**Current State**: 1,364 lines in `codegen_grammar.rs`

## Duplicate Patterns Found

### 1. JUMP TO MAIN LOOP ⚡ (appears ~6 times)

**Locations**: Lines 525-544, 861-880, 1055-1074, 1231-1250, 1323-1342, 1360-1379

**Pattern** (~20 lines each occurrence):
```rust
// Jump back to main loop after handler
let layout = self.emit_instruction_typed(
    Opcode::Op1(Op1::Jump),
    &[Operand::LargeConstant(placeholder_word())],
    None,
    None,
)?;

if let Some(operand_location) = layout.operand_location {
    self.reference_context
        .unresolved_refs
        .push(UnresolvedReference {
            reference_type: LegacyReferenceType::Jump,
            location: operand_location,
            target_id: main_loop_jump_id,
            is_packed_address: false,
            offset_size: 2,
            location_space: MemorySpace::Code,
        });
}
```

**Proposed Helper**:
```rust
fn emit_jump_to_main_loop(&mut self, main_loop_label: u32) -> Result<(), CompilerError> {
    let layout = self.emit_instruction_typed(
        Opcode::Op1(Op1::Jump),
        &[Operand::LargeConstant(placeholder_word())],
        None,
        None,
    )?;

    if let Some(operand_location) = layout.operand_location {
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::Jump,
                location: operand_location,
                target_id: main_loop_label,
                is_packed_address: false,
                offset_size: 2,
                location_space: MemorySpace::Code,
            });
    } else {
        panic!("BUG: emit_instruction didn't return operand_location for jump");
    }

    Ok(())
}
```

**Savings**: 6 occurrences × 20 lines = ~120 lines → 6 calls × 1 line + 25 line helper = **~99 line reduction**

---

### 2. FUNCTION CALL WITH UNRESOLVED REF ⚡ (appears ~6 times)

**Locations**: Lines 495-512, 834-853, 1176-1195, 1279-1298, 1481-1500

**Pattern** (~20 lines each occurrence):
```rust
// Emit function call
let layout = self.emit_instruction_typed(
    Opcode::OpVar(OpVar::CallVs),
    &operands,
    Some(0), // Store return value but ignore it
    None,
)?;

// Register function reference
if let Some(operand_location) = layout.operand_location {
    self.reference_context
        .unresolved_refs
        .push(UnresolvedReference {
            reference_type: LegacyReferenceType::FunctionCall,
            location: operand_location,
            target_id: func_id,
            is_packed_address: true,
            offset_size: 2,
            location_space: MemorySpace::Code,
        });
}
```

**Proposed Helper**:
```rust
fn emit_handler_call(&mut self, func_id: IrId, operands: Vec<Operand>) -> Result<(), CompilerError> {
    let mut call_operands = vec![Operand::LargeConstant(placeholder_word())];
    call_operands.extend(operands);

    let layout = self.emit_instruction_typed(
        Opcode::OpVar(OpVar::CallVs),
        &call_operands,
        Some(0), // Store return value but ignore it
        None,
    )?;

    if let Some(operand_location) = layout.operand_location {
        self.reference_context
            .unresolved_refs
            .push(UnresolvedReference {
                reference_type: LegacyReferenceType::FunctionCall,
                location: operand_location,
                target_id: func_id,
                is_packed_address: true,
                offset_size: 2,
                location_space: MemorySpace::Code,
            });
    } else {
        panic!("BUG: emit_instruction didn't return operand_location for call_vs");
    }

    Ok(())
}
```

**Savings**: 6 occurrences × 20 lines = ~120 lines → 6 calls × 3 lines + 30 line helper = **~102 line reduction**

---

### 3. LOAD WORD FROM PARSE BUFFER 🔧 (appears ~4 times)

**Locations**: Lines 414-425, 690-701, (literal+noun section), (verb+noun section)

**Pattern** (~12 lines each occurrence):
```rust
// Load word N from parse buffer
// Parse buffer: [0]=max, [1]=count, [2]=word1_low, [3]=word1_high, [4]=word1_pos, [5]=word1_len, ...
self.emit_instruction_typed(
    Opcode::Op2(Op2::Loadw),
    &[
        Operand::Variable(PARSE_BUFFER_GLOBAL), // Parse buffer address
        Operand::SmallConstant(1),              // Word 1 offset (word N = offset N)
    ],
    Some(7), // Store in Variable(7)
    None,
)?;
```

**Proposed Helper**:
```rust
fn load_parse_buffer_word(&mut self, word_index: u8, dest_var: u8) -> Result<(), CompilerError> {
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Loadw),
        &[
            Operand::Variable(PARSE_BUFFER_GLOBAL),
            Operand::SmallConstant(word_index),
        ],
        Some(dest_var),
        None,
    )?;
    Ok(())
}
```

**Savings**: 4 occurrences × 12 lines = ~48 lines → 4 calls × 1 line + 12 line helper = **~40 line reduction**

---

### 4. OBJECT LOOKUP ✅ (already extracted!)

**Status**: `generate_object_lookup_from_noun()` already exists in `codegen_lookup.rs`

**Usage**: Correctly used throughout the file at lines 810, 1150, etc.

**No action needed** - already well-factored!

---

## Summary Statistics

| Pattern | Occurrences | Lines Each | Total Lines | After Helper | Savings |
|---------|-------------|------------|-------------|--------------|---------|
| Jump to Main Loop | 6 | ~20 | ~120 | ~31 | ~89 |
| Handler Call | 6 | ~20 | ~120 | ~48 | ~72 |
| Load Parse Buffer | 4 | ~12 | ~48 | ~16 | ~32 |
| **TOTAL** | **16** | - | **~288** | **~95** | **~193** |

**Expected Reduction**: ~193 lines (14% of current file size)

---

## Phase 2 Implementation Plan

### Step 1: Extract Jump Helper (15 minutes)
1. Add `emit_jump_to_main_loop` method to `impl ZMachineCodeGen`
2. Replace 6 occurrences with single-line calls
3. Test: compile mini_zork, verify bytecode identical
4. Commit

### Step 2: Extract Call Helper (15 minutes)
1. Add `emit_handler_call` method
2. Replace 6 occurrences
3. Test: compile mini_zork, verify bytecode identical
4. Commit

### Step 3: Extract Parse Buffer Helper (15 minutes)
1. Add `load_parse_buffer_word` method
2. Replace 4 occurrences
3. Test: compile mini_zork, verify bytecode identical
4. Commit

### Step 4: Verify & Cleanup (15 minutes)
1. Run full test suite
2. Test mini_zork gameplay
3. Check for any remaining duplication
4. Update GRAMMAR_FUNCTION_ANALYSIS.md with results

**Total Time**: ~60 minutes

---

## Additional Issues Found

### ⚠️ Suspicious Pattern: "Store but ignore return value"

Multiple locations have:
```rust
Some(0), // Store return value but ignore it
```

**Question**: Why store to Variable(0) (stack) if we're ignoring it?
- Variable 0 is the stack in Z-Machine
- Storing to stack and not using it seems wasteful
- Could we use `None` instead? (discard return value)
- Need to check Z-Machine spec for call_vs requirements

**Action**: Research later, don't change in Phase 2 (risky)

### ✅ No More Dead Code

- No `if false` blocks remaining
- No `#[allow(dead_code)]` markers
- No unused variables (`let _`)
- Clean after Phase 1 deletion

### 📝 Documentation Opportunities

Several complex sections could use better comments:
- RuntimeParameter resolution logic (lines 906-820)
- Literal+noun vs verb+noun distinction (lines 654-891)
- Parse buffer layout (scattered comments, could consolidate)

**Action**: Add documentation in Phase 4 refactoring

---

## Risk Assessment

**Low Risk**:
- Helper extraction is pure refactoring
- Logic remains identical
- Bytecode verification catches any issues
- Easy rollback with git

**Medium Risk**:
- None identified for Phase 2

**Mitigation**:
- Test after each helper extraction
- Verify bytecode identity after each step
- Keep commits small and atomic

---

## Success Criteria

✅ **3 new helper methods** created
✅ **~193 lines reduced** from main function
✅ **Bytecode identical** before and after
✅ **All tests pass**
✅ **Mini Zork gameplay verified**
✅ **Ready for Phase 3** (extract pattern handlers)

---

## Next Steps

After Phase 2 completion:
- **Phase 3**: Extract 6 pattern handler methods (literal, literal+noun, verb+noun, default, setup, finalize)
- **Phase 4**: Refactor main function as orchestrator calling helpers
