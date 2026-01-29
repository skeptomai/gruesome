# Refactoring Branch Inversion Bug

**Date**: December 14, 2025
**Status**: ROOT CAUSE IDENTIFIED
**Introduced**: Commit 8a2c27d (refactor: Extract literal patterns handler)

## The Bug

The refactoring in commit 8a2c27d inverted the branch logic for literal pattern matching and added unnecessary complexity:

### Pre-Refactoring (Working ✅)

```
1. je word_count, 2 → branch-on-FALSE to skip_literal_label
2. loadw parse_buffer, 3 → Variable(7)
3. je Variable(7), literal_dict_addr → branch-on-FALSE to skip_literal_label
4. [Fall through] call handler
5. jump main_loop
6. skip_literal_label: [next pattern]
```

**Characteristics:**
- Branch-on-FALSE (0x7FFF) skips pattern when NO match
- Direct fall-through to handler when match
- No extra instructions

### Post-Refactoring (Broken ❌)

```
1. je word_count, 2 → branch-on-FALSE to skip_literal_label
2. loadw parse_buffer, 3 → Variable(7)
3. je Variable(7), literal_dict_addr → branch-on-TRUE to execute_literal_label ⚠️
4. [Fall through] jump skip_literal_label ⚠️ EXTRA INSTRUCTION
5. execute_literal_label: call handler ⚠️ EXTRA LABEL
6. jump main_loop
7. skip_literal_label: [next pattern]
```

**Characteristics:**
- Branch-on-TRUE (0x4000) jumps to handler when MATCH
- Extra JUMP instruction when no match
- Extra execute_literal_label that serves no purpose

## Why This Breaks

The extra jump instruction causes:
1. **Code size discrepancy**: Bytecode is 3 bytes longer per literal pattern
2. **Address misalignment**: All subsequent labels are offset by 3 bytes
3. **Branch resolution errors**: skip_literal_label points to wrong address

Additionally, the execute_literal_label:
1. Creates an extra ID in next_string_id sequence
2. Registered with `ir_id_to_address.insert()` instead of falling through
3. Adds complexity for zero benefit

## The Fix

Revert to pre-refactoring approach:

**Change this** (lines ~890-940):
```rust
// Compare - branch on TRUE to execute handler
let layout = self.emit_instruction_typed(
    Opcode::Op2(Op2::Je),
    &[
        Operand::Variable(7),
        Operand::LargeConstant(placeholder_word()),
    ],
    None,
    Some(0x4000_u16 as i16), // Branch-on-TRUE (match found)
)?;

// Register branch to execute_literal_label
let execute_literal_label = self.next_string_id;
self.next_string_id += 1;

if let Some(branch_location) = layout.branch_location {
    self.reference_context
        .unresolved_refs
        .push(UnresolvedReference {
            reference_type: LegacyReferenceType::Branch,
            location: branch_location,
            target_id: execute_literal_label,
            // ...
        });
}

// Jump to skip this pattern (no match)
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
            target_id: skip_literal_label,
            // ...
        });
}

// Label for executing literal pattern handler
self.reference_context
    .ir_id_to_address
    .insert(execute_literal_label, self.code_address);
```

**To this** (pre-refactoring logic):
```rust
// Compare - branch on FALSE to skip pattern
let layout = self.emit_instruction_typed(
    Opcode::Op2(Op2::Je),
    &[
        Operand::Variable(7),
        Operand::LargeConstant(placeholder_word()),
    ],
    None,
    Some(0x7FFF_u16 as i16), // Branch on FALSE (not equal) - skip if no match
)?;

// Register branch to skip_literal_label for word mismatch
if let Some(branch_location) = layout.branch_location {
    self.reference_context
        .unresolved_refs
        .push(UnresolvedReference {
            reference_type: LegacyReferenceType::Branch,
            location: branch_location,
            target_id: skip_literal_label,
            // ...
        });
}

// [Fall through to handler - NO extra jump needed]
```

## Benefits of Fix

1. **Removes 3-byte overhead** per literal pattern (jump instruction)
2. **Removes unnecessary label** (execute_literal_label)
3. **Restores correct address alignment** for all subsequent code
4. **Simplifies control flow** to match pre-refactoring working code
5. **Fixes pattern matching** by restoring correct bytecode structure

## Testing

After fix:
```bash
printf "look around\nquit\ny\n" | cargo run --bin gruesome tests/mini_zork_test_new.z3
# Expected: Room description (look_around() executed)

printf "look at mailbox\nquit\ny\n" | cargo run --bin gruesome tests/mini_zork_test_new.z3
# Expected: "The small mailbox is closed." (examine(mailbox) executed)
```
