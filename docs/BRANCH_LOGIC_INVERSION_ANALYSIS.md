# Branch Logic Inversion Analysis

## Fix 2: Removing Extra Jump and Execute Label

This fix addresses the control flow structure change introduced during refactoring.

## Pre-Refactoring Control Flow (Working ✅)

```
Code:
  1. je word, literal_dict → branch-on-FALSE to skip_literal_label
  2. [fall through] call handler
  3. jump main_loop
  4. skip_literal_label: [next pattern]

Assembly-level view:
  0x0100: je @7, literal → branch if NOT equal to skip_literal_label
  0x0105: call_vs handler → executed when words MATCH
  0x010A: jump main_loop
  0x010D: skip_literal_label: [next pattern code]
```

**Control Flow**:
- If word matches → fall through → call handler → jump to main loop ✓
- If word doesn't match → branch to skip_literal_label → try next pattern ✓

**Size**: 3 instructions, no intermediate labels

## Post-Refactoring Control Flow (Broken ❌)

```
Code:
  1. je word, literal_dict → branch-on-TRUE to execute_literal_label
  2. [fall through] jump skip_literal_label
  3. execute_literal_label: call handler
  4. jump main_loop
  5. skip_literal_label: [next pattern]

Assembly-level view:
  0x0100: je @7, literal → branch if EQUAL to execute_literal_label
  0x0105: jump skip_literal_label → executed when words DON'T match
  0x0108: execute_literal_label: call_vs handler
  0x010D: jump main_loop
  0x0110: skip_literal_label: [next pattern code]
```

**Control Flow**:
- If word matches → branch to execute_literal_label → call handler → jump to main loop ✓
- If word doesn't match → fall through → jump to skip_literal_label → try next pattern ✓

**Size**: 4 instructions, 1 extra label

## Is This a Logic Bug?

**No - both approaches work logically!**

Both control flows:
- Call handler when pattern matches ✓
- Skip to next pattern when no match ✓
- Return to main loop after handler ✓

The Z-Machine would execute both correctly.

## So Why Does It Break?

**Answer: Address Resolution Assumptions**

The compiler's label resolution code makes assumptions based on:
1. **When labels are defined** (after handler code)
2. **What code exists between branch and label** (nothing vs jump instruction)

### Problem 1: Extra Instruction Creates Offset

Pre-refactoring:
```
Branch target: skip_literal_label
Branch location: 0x0103 (2 bytes for offset)
Label defined at: 0x010D (after handler + jump)
Offset: 0x010D - 0x0105 = 8 bytes
```

Post-refactoring:
```
Branch target: skip_literal_label
Branch location: 0x0103 (2 bytes for offset)
Jump to skip_literal_label at: 0x0105 (3 bytes: opcode + 2-byte offset)
Label defined at: 0x0110 (after handler + jump + extra jump)
Offset: 0x0110 - 0x0105 = 11 bytes
```

The extra jump adds 3 bytes to every pattern's code, shifting all subsequent addresses.

### Problem 2: Label Registration Timing

The refactoring registers execute_literal_label differently:

**Pre-refactoring**:
```rust
// No intermediate label needed
// Handler code immediately follows the branch
```

**Post-refactoring**:
```rust
// Register execute_literal_label
self.reference_context
    .ir_id_to_address
    .insert(execute_literal_label, self.code_address);
```

This creates an EXTRA entry in `next_string_id` sequence, which may interfere with ID allocation for other labels.

### Problem 3: Compound Effects

Each literal pattern in the refactored code:
- Adds 1 extra label to ID sequence
- Adds 3 bytes of code (jump instruction)
- Shifts all subsequent code by 3 bytes

For mini_zork with 1 literal pattern ("around"):
- ID sequence: Off by 1
- Code size: +3 bytes
- All addresses after literal pattern: +3 bytes offset

This compounds across multiple verbs and patterns.

## Why Doesn't the Compiler Handle This?

The compiler SHOULD handle different code structures, but:

### Issue 1: Label ID Conflicts

The monotonic `next_string_id` counter is used for:
- skip_literal_label (ID N)
- execute_literal_label (ID N+1) ← EXTRA ID
- skip_literal_noun_label (ID N+2) ← Should be N+1

If other code expects specific ID ordering, the extra ID breaks assumptions.

### Issue 2: Size Calculation Fragility

Some parts of the compiler may calculate expected code size:
- Pattern matching section: Expected size X
- Actual size with extra jumps: X + (3 * num_literal_patterns)

If size checking exists, this mismatch causes failures.

### Issue 3: `ir_id_to_address.insert()` vs `record_final_address()`

Pre-refactoring used direct insertion:
```rust
self.reference_context
    .ir_id_to_address
    .insert(skip_literal_label, self.code_address);
```

Refactoring tried to use:
```rust
self.record_final_address(skip_literal_label, self.code_address);
```

Looking at `record_final_address()`:
```rust
pub fn record_final_address(&mut self, ir_id: IrId, address: usize) {
    // Detect and warn about mapping conflicts (potential bugs)
    if let Some(&existing_addr) = self.reference_context.ir_id_to_address.get(&ir_id) {
        if existing_addr != address {
            log::warn!("IR_MAPPING_CONFLICT: ...");
        }
    }
    self.reference_context.ir_id_to_address.insert(ir_id, address);
}
```

The difference: `record_final_address()` checks for conflicts. If execute_literal_label was already registered somewhere, this would warn but still overwrite.

However, the REAL issue is that we tried using `record_final_address()` when we should have used direct insert to match pre-refactoring behavior.

## Conclusion

**The branch logic inversion IS a logic change, but not a logic BUG**

Both approaches are logically valid, but the post-refactoring approach:

1. **Violates compiler size assumptions** (+3 bytes per pattern)
2. **Disrupts label ID sequencing** (extra ID per pattern)
3. **Creates address alignment issues** (all code shifts)
4. **Adds unnecessary complexity** (extra jump serves no purpose)

The fix removes this complexity and restores:
- Pre-refactoring control flow (simpler)
- Pre-refactoring code size (smaller)
- Pre-refactoring label registration (direct insert)
- Pre-refactoring address alignment (no extra offsets)

**Key insight**: The refactoring didn't introduce a SEMANTIC bug (wrong behavior), it introduced an ARCHITECTURAL mismatch (violates compiler invariants about code structure and sizing).
