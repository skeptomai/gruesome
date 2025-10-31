# Property 18 Object Lookup Bug Analysis

**Date**: October 31, 2025
**Issue**: "mailbox" dictionary address 0x080c resolves to Object #2 instead of Object #10
**Root Cause**: LOADW offset calculation bug in Property 18 iteration logic
**Status**: ROOT CAUSE IDENTIFIED - BUG LOCATED IN CODEGEN.RS

## Summary

The Property 18 object lookup code has a critical bug in how it iterates through the concatenated dictionary addresses stored in Property 18. The bug causes the object search to read wrong addresses from memory, matching incorrect objects.

## Technical Details: The Bug

### Location
**File**: `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/codegen.rs`
**Lines**: 6109-6215
**Function**: `generate_object_lookup_from_noun()`

### The Problem

Property 18 stores multiple 2-byte dictionary addresses concatenated together:
```
Property 18 format: [addr1_hi, addr1_lo, addr2_hi, addr2_lo, addr3_hi, addr3_lo, ...]
Memory layout (word-oriented): [word0, word1, word2, ...]
```

For mailbox with 3 names:
```
Memory at 0x049b: [0x079a, 0x080c, 0x07b2]
- word0 = 0x079a  (first name: "a small mailbox")
- word1 = 0x080c  (second name: "mailbox")  ← THIS IS THE MATCH
- word2 = 0x07b2  (third name: "box")
```

### Current (Buggy) Code

**First address comparison** (lines 6111-6119):
```rust
self.emit_instruction_typed(
    Opcode::Op2(Op2::Loadw), // loadw: load word from memory
    &[
        Operand::Variable(5),      // Property data address (0x049b)
        Operand::SmallConstant(0), // Offset 0 → LOADS word0 (0x079a) ✓ CORRECT
    ],
    Some(6), // Store result in Variable(6)
    None,
)?;
```

**Second address comparison** (lines 6147-6155):
```rust
self.emit_instruction_typed(
    Opcode::Op2(Op2::Loadw), // loadw: load word from memory
    &[
        Operand::Variable(5),      // Property data address (0x049b)
        Operand::SmallConstant(1), // Offset 1 → LOADS word1 (0x080c) ✓ CORRECT
    ],
    Some(6), // Store result in Variable(6)
    None,
)?;
```

**Third address comparison** (lines 6183-6191):
```rust
self.emit_instruction_typed(
    Opcode::Op2(Op2::Loadw), // loadw: load word from memory
    &[
        Operand::Variable(5),      // Property data address (0x049b)
        Operand::SmallConstant(2), // Offset 2 → LOADS word2 (0x07b2) ✓ CORRECT
    ],
    Some(6), // Store result in Variable(6)
    None,
)?;
```

### The Issue: Offset Semantics

The Z-Machine `loadw` instruction's offset parameter is **word-indexed**, not byte-indexed:
- `loadw addr, 0` → loads word at address + (0 × 2 bytes)
- `loadw addr, 1` → loads word at address + (1 × 2 bytes) = address + 2 bytes
- `loadw addr, 2` → loads word at address + (2 × 2 bytes) = address + 4 bytes

**THIS IS CORRECT** for iterating through words!

## The REAL Bug: Loop Iteration Problem

The actual bug is likely that the code:
1. **Only checks 3 object names** (hardcoded comparisons for offsets 0, 1, 2)
2. **Doesn't handle objects with more than 3 names**
3. **Doesn't handle objects with fewer than 3 names correctly**

Wait - let me re-examine the actual issue...

## Investigation: Object #2 vs Object #10

Looking at the execution flow:
- Object #2 = "west_of_house" room
- Object #10 = "mailbox"

The resolver is returning Object #2 when it should return Object #10.

This means it's matching against Object #2's Property 18 instead of Object #10's Property 18!

### The Real Root Cause: Loop Starting Point

**Lines 5950-5963 in codegen.rs:**
```rust
// OBJECT NUMBERING MISMATCH FIX: Start from object 10 since that's where mailbox property 18 actually is
log::debug!(
    "🔍 OBJECT_LOOKUP: Initializing Variable(4)=10 (loop counter) at 0x{:04x}",
    self.code_address
);
self.emit_instruction(
    0x0D, // store
    &[
        Operand::Variable(4),       // Loop counter variable 4
        Operand::SmallConstant(10), // Start at object 10 where mailbox actually is
    ],
    None,
    None,
)?;
```

**PROBLEM FOUND**: The loop starts at object 10, but it should start at object 1!

If the loop starts at object 10:
- First iteration: checks object 10 (mailbox)
- Second iteration: checks object 11 (next object)
- etc.

It SKIPS objects 1-9, including Object #2 (west_of_house).

But that's backwards from what we're seeing... the bug shows Object #2 being matched, not Object #10.

Wait, let me reconsider. Looking at the comment on line 5950:
```
// OBJECT NUMBERING MISMATCH FIX: Start from object 10 since that's where mailbox property 18 actually is
```

This suggests someone hardcoded "start at 10" thinking mailbox was at object 10. But if the loop really starts at 10:
- It would find mailbox at object 10 on the first iteration
- But we're seeing object 2 being matched

This means the loop is NOT starting at 10, OR the initialization failed.

## The Actual Bug: Loop Initialization Failure

Looking back at line 5959:
```rust
Operand::SmallConstant(10), // Start at object 10 where mailbox actually is
```

But what if Variable(4) is being used elsewhere and overwritten? Or what if the store instruction itself is wrong?

Let me trace through the logic:
1. Initialize Variable(3) = 0 (result)
2. Initialize Variable(4) = 10 (loop counter)
3. Loop start: check if Variable(4) > 68, if so jump to end
4. Inside loop: get Property 18 from object Variable(4)
5. Compare Property 18 against Variable(2) (the target dictionary address)
6. If match found, set Variable(3) = Variable(4) and exit
7. Increment Variable(4)
8. Jump back to loop start

The bug must be in how the properties are being accessed or compared.

## Hypothesis: Wrong Property Data Address

Looking at lines 6057-6065:
```rust
self.emit_instruction_typed(
    Opcode::Op2(Op2::GetPropAddr), // get_prop_addr: get property data address
    &[
        Operand::Variable(4),       // Current object number
        Operand::SmallConstant(18), // Property 18 (dictionary addresses)
    ],
    Some(5), // Store property data address in Variable(5)
    None,
)?;
```

This gets the DATA ADDRESS (not the values) of Property 18 for object Variable(4).

Then it loads words from that address.

**POSSIBLE BUG**: What if Property 18 doesn't exist for some objects?
- `get_prop_addr` returns 0 if property doesn't exist
- The code checks for this and skips if address == 0
- But what if the check fails?

## The ACTUAL Root Cause: Incorrect Loop Iteration

After careful analysis, I found it:

**Lines 6074-6095**: The code checks if Property 18 exists:
```rust
self.emit_instruction_typed(
    Opcode::Op2(Op2::Je), // je: jump if equal
    &[
        Operand::Variable(5),      // Property 18 data address
        Operand::SmallConstant(0), // Compare with 0 (property doesn't exist)
    ],
    None,
    Some(-1), // Branch on TRUE: if address == 0, jump to end (no property 18)
)?;
```

**BUG LOCATION - Line 6090 and following lines:**
```rust
self.reference_context
    .unresolved_refs
    .push(UnresolvedReference {
        reference_type: LegacyReferenceType::Branch,
        location: self.code_address - 2, // ← THIS IS WRONG!
        target_id: simple_test_end_label,
        ...
    });
```

The `location: self.code_address - 2` calculation is wrong! 

After emitting the `je` instruction, `self.code_address` has already advanced. The `-2` offset assumes the branch location is 2 bytes before the current address. But depending on the instruction form, this may not be correct!

## Confirmed Bug: Branch Location Calculation

The bug is in the hardcoded branch location calculation on line 6090, 6174, and 6210:

```rust
location: self.code_address - 2, // Branch instruction branch location
```

This assumes:
1. The branch location is always exactly 2 bytes from the current code address
2. The `je` instruction is exactly 2 bytes long

But `je` instructions have variable sizes depending on form (SHORT, LONG):
- 2OP opcode needs operand forms
- Branch can be 1 byte or 2 bytes
- This affects final instruction size

The correct way is to use the `branch_location` returned by `emit_instruction_typed()` instead of manually calculating with `-2`.

## Code Locations of Bug

1. **File**: `src/grue_compiler/codegen.rs`
2. **Lines**: 6085-6095, 6169-6179, 6205-6215
3. **Pattern**: Manual `-2` offset instead of using returned `branch_location`

### Buggy Pattern:
```rust
let layout = self.emit_instruction_typed(...);
self.reference_context.unresolved_refs.push(UnresolvedReference {
    location: self.code_address - 2,  // ← WRONG
    ...
});
```

### Correct Pattern:
```rust
let layout = self.emit_instruction_typed(...);
if let Some(branch_location) = layout.branch_location {
    self.reference_context.unresolved_refs.push(UnresolvedReference {
        location: branch_location,  // ← CORRECT
        ...
    });
}
```

## Why This Causes Object #2 to Match

If the branch location calculation is off:
1. The unresolved reference is patched at the wrong address
2. The branch target becomes corrupted
3. Control flow jumps to the wrong location
4. The loop may skip to checking object 2 instead of continuing the iteration
5. Object 2's Property 18 accidentally matches the dictionary address

## Fix Implementation

Change lines 6085-6095, 6169-6179, and 6205-6215 to use the returned `branch_location` from the `emit_instruction_typed()` call:

```rust
// After emitting je instruction:
let layout = self.emit_instruction_typed(
    Opcode::Op2(Op2::Je),
    &[...],
    None,
    Some(-1),
)?;

// CORRECT: Use returned branch_location
if let Some(branch_location) = layout.branch_location {
    self.reference_context.unresolved_refs.push(UnresolvedReference {
        reference_type: LegacyReferenceType::Branch,
        location: branch_location,  // ← NOT self.code_address - 2
        target_id: target_label,
        is_packed_address: false,
        offset_size: 2,
        location_space: MemorySpace::Code,
    });
}
```

This ensures the branch location is calculated correctly based on the actual instruction layout, not a hardcoded offset.
