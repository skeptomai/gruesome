# Grue Compiler Architecture & Common Bug Patterns

## Critical Bug Patterns & Solutions

This document captures systematic issues we've encountered multiple times to prevent regression and aid future debugging.

## Grammar System Architecture

### Grammar System Overview

The **grammar system** is the core text adventure command processing engine that converts natural language input into executable game actions. It bridges the gap between raw player input and compiled game logic.

### Current vs Target Architecture

**Current Simplified Implementation:**
```
Input: "quit" → sread → dictionary lookup → Z-Machine quit instruction
```

**Target Grammar System:**
```
Input: "take red book" → sread → tokenization → grammar matching → object resolution → function execution
```

### Core Components

#### 1. Grammar Pattern Engine
```rust
pub struct GrammarPattern {
    pub verb: String,
    pub pattern: Vec<TokenType>,  // [Verb, Noun] or [Verb, "all"]
    pub handler: IRNode,          // Function call to execute
}

pub enum TokenType {
    Verb,
    Noun,
    Preposition(String),  // "from", "with", "to"
    Literal(String),      // "all", "inventory"
}
```

#### 2. Object Resolution System
```rust
pub struct ObjectResolver {
    pub visible_objects: Vec<ObjectRef>,
    pub disambiguation_needed: bool,
}
```

#### 3. Multi-word Noun Recognition
- **Challenge**: "jewel-encrusted egg" = 3 dictionary words → 1 object
- **Solution**: Noun phrase parsing with longest-match-first strategy
- **Dependencies**: Working property system for object name access

### Integration Points

#### Dictionary Enhancement
- **Current**: Simple word lookup for basic commands
- **Target**: Multi-word noun phrase recognition with alias matching

#### Property System Requirements
- **Current**: Property access fails (65534 placeholders)
- **Required**: Object name resolution needs `obj.name` property access
- **Blocking**: Must fix property placeholders before grammar implementation

#### Z-Machine Instruction Bridge
- **Current**: Direct Z-Machine calls (quit, sread)
- **Target**: Grammar patterns → Grue functions → compiled Z-Machine sequences

### Grammar DSL Examples

```grue
verb "take" {
    noun => handle_take($noun),
    "all" => take_all(),
    noun + "from" + noun => handle_take_from($1, $3)
}

verb "examine" {
    noun => examine($noun),
    default => look_around()
}

verb "put" {
    noun + "in" + noun => handle_put_in($1, $3),
    noun + "on" + noun => handle_put_on($1, $3)
}
```

### 0. Print Newline Architecture Bug ⚠️⚠️⚠️

**THIS HAS BEEN BROKEN AND FIXED MULTIPLE TIMES - CRITICAL PATTERN**

**Problem**: Print statements run together without line breaks, breaking game banner display.

#### WRONG Patterns (ALL cause broken formatting):

1. **❌ WRONG OPCODE**: Using 0x0D instead of 0x8D
```rust
// BROKEN: 0x0D is get_next_prop, not print_paddr
self.emit_instruction(0x0D, &[Operand::LargeConstant(0x0000)], None, None)?;
```

2. **❌ WRONG APPROACH**: Embedding newlines in string content
```rust
// BROKEN: Changes string length, breaks address calculations
let print_string = format!("{}\n", string_value);
```

3. **❌ WRONG APPROACH**: Using only string newlines without new_line instructions
```rust
// BROKEN: Z-Machine doesn't auto-add line breaks between print statements
// Just encoding \n in string content is not enough
```

#### ✅ CORRECT Pattern (Working as of Sep 13, 2025):

**Z-Machine Print Architecture Understanding**:
- `print_paddr` (0x8D) prints string content exactly as encoded
- Line breaks between separate `print()` calls require explicit `new_line` instructions
- **Each print() call needs two instructions**: print_paddr + new_line

```rust
// ✅ CORRECT: Two-instruction pattern in codegen_builtins.rs
// 1. Print the string content
self.emit_instruction(
    0x8D,                              // print_paddr opcode (1OP:141) 
    &[Operand::LargeConstant(0x0000)], // String reference (resolved later)
    None, None
)?;

// 2. Add line break after the text
self.emit_instruction(
    0xBB,  // new_line opcode (0OP:11)
    &[],   // No operands
    None, None
)?;
```

#### Symptoms:
- Game banner text runs together: "DORK I: The Last Great EmpireCopyright (c) 2025..."
- All print statements appear on single line without breaks
- Different strings print correctly but without line separation
- No interpreter errors - just broken visual formatting

#### Files Affected:
- `src/grue_compiler/codegen_builtins.rs` (generate_print_builtin function)

#### Detection Method:
```bash
# Test banner formatting immediately after print changes
cargo run --bin grue-compiler -- examples/mini_zork.grue --output /tmp/test.z3
./target/debug/gruesome /tmp/test.z3
# Should show properly separated banner lines
```

#### Working Output:
```
DORK I: The Last Great Empire
Copyright (c) 2025 Grue Games. All rights reserved.
ZORK is a registered trademark of Infocom, Inc.  
DORK is .... not
Revision 1 / Serial number 8675309
```

### 1. UnresolvedReference Location Calculation Bug ⚠️

**Problem**: References created with incorrect locations, causing systematic resolution failures.

#### WRONG Pattern (causes widespread crashes):
```rust
// ❌ BROKEN: Location calculated AFTER placeholder emission
self.add_unresolved_reference(
    LegacyReferenceType::StringRef,
    target_id,
    true,
    MemorySpace::Code,
)?;                                    // location = self.final_code_base + self.code_space.len()
self.emit_word(placeholder_word())?;   // Placeholder added AFTER location calculated
```

#### CORRECT Pattern:
```rust
// ✅ FIXED: Location calculated BEFORE placeholder emission
let code_space_offset = self.code_space.len();  // Record exact location first
self.emit_word(placeholder_word())?;             // Then emit placeholder

// Create reference with exact location
self.reference_context.unresolved_refs.push(UnresolvedReference {
    reference_type: LegacyReferenceType::StringRef,
    location: code_space_offset,  // Use exact offset
    target_id,
    is_packed_address: true,
    offset_size: 2,
    location_space: MemorySpace::Code,
});
```

#### Symptoms:
- Hundreds of `0x00 0x00` placeholder bytes left unresolved
- Game crashes during execution with "Branch to address 0xffffff25" errors
- Debug logs showing "Reality: 0x00 suggests missing UnresolvedReference"
- Initial program execution works but crashes partway through

#### Files Affected:
- `src/grue_compiler/codegen.rs` (lines ~7275-7287)
- Any code using `add_unresolved_reference()` followed by `emit_word(placeholder_word())`

#### Detection Method:
Search for this antipattern:
```bash
grep -A5 -B5 "add_unresolved_reference" src/grue_compiler/codegen.rs | grep -A10 -B10 "emit_word.*placeholder"
```

---

### 2. Z-Machine Branch Format Encoding ⚠️

**Problem**: Incorrect branch byte encoding causing arithmetic overflow during execution.

#### WRONG Pattern:
```rust
// ❌ BROKEN: Using emit_word for branch offset
self.emit_word(offset as u16)?;  // Produces incorrect byte order
```

#### CORRECT Pattern:
```rust
// ✅ FIXED: Manual byte-by-byte branch encoding
if offset < 64 && offset >= 0 {
    // 1-byte format: bit 7 = condition, bit 6 = 1, bits 5-0 = offset
    let branch_byte = 0xC0 | (offset as u8 & 0x3F);
    self.emit_byte(branch_byte)?;
} else {
    // 2-byte format: bit 7 = condition, bit 6 = 0, bits 13-0 = offset
    let offset_u16 = offset as u16;
    let first_byte = 0x80 | ((offset_u16 >> 8) as u8 & 0x3F);
    let second_byte = (offset_u16 & 0xFF) as u8;
    self.emit_byte(first_byte)?;
    self.emit_byte(second_byte)?;
}
```

#### Files Affected:
- `src/grue_compiler/codegen_instructions.rs` (`emit_branch_offset` function)

---

### 3. Reference Type Disambiguation ⚠️

**Problem**: Using wrong reference type for similar operations.

#### Key Rule:
- Use `LegacyReferenceType::Jump` for unconditional jumps
- Use `LegacyReferenceType::Branch` for conditional branches
- Never mix these types

#### Example Fix:
```rust
// Line 6232 in codegen.rs - was incorrectly using Branch for Jump
reference_type: LegacyReferenceType::Jump,  // Not Branch
```

---

## Architecture Principles

### UnresolvedReference System Flow

1. **Generation Phase**: Code emits placeholder bytes and records exact locations
2. **Collection Phase**: All UnresolvedReferences stored with code space offsets
3. **Translation Phase**: Code space offsets converted to final memory addresses  
4. **Resolution Phase**: Placeholders patched with actual addresses

**Critical Rule**: Location must be recorded BEFORE placeholder emission, not after.

### Memory Space Types

- `MemorySpace::Code`: Instructions in main code section
- `MemorySpace::CodeSpace`: Same as Code (legacy)
- `MemorySpace::Strings`: String literals section
- `MemorySpace::Objects`: Object table section

### Debug Logging Patterns

Always add comprehensive logging for:
```rust
log::debug!("REFERENCE_CREATE: offset=0x{:04x} target={} type={:?}", 
           offset, target_id, reference_type);

log::debug!("REFERENCE_RESOLVE: location=0x{:04x} -> address=0x{:04x}", 
           reference.location, target_address);
```

---

## Common Debugging Commands

### Check for unresolved placeholders:
```bash
xxd mini_zork.z3 | grep "0000"  # Look for 0x00 0x00 patterns
```

### Search for problematic patterns:
```bash
grep -r "add_unresolved_reference" src/grue_compiler/ | grep -A3 -B3 "emit_word"
```

### Verify tests still pass:
```bash
cargo test codegen_test 2>/dev/null | grep "test result"
```

---

## Lessons Learned

1. **Location timing is critical**: Always record locations before emitting bytes
2. **Systematic bugs affect many files**: One pattern fix can resolve hundreds of failures  
3. **Test early and often**: Compiler tests catch regressions effectively
4. **Debug logging is essential**: Comprehensive logging reveals exact failure points
5. **Pattern consistency matters**: Small variations in patterns can cause major failures

---

## Future Considerations

- Consider deprecating `add_unresolved_reference()` in favor of direct `UnresolvedReference` creation
- Add compile-time checks to prevent location-after-emission patterns
- Create automated tests that verify no `0x00 0x00` patterns in compiled output
- Document all reference creation patterns as they're discovered

---

---

## UPDATE: Systematic UnresolvedReference Resolution Fixed (Sep 12, 2025) ✅

### MAJOR BREAKTHROUGH: IR ID Mapping Consolidation Success

**Problem Solved**: Systematic UnresolvedReference resolution failures where hundreds of references failed with "target_id not found" errors.

**Root Cause**: Compiler used separate tracking systems (string_offsets, label_addresses, ir_id_to_object_number) but only ir_id_to_address was used for resolution.

**Solution**: Added `consolidate_all_ir_mappings()` function that merges all tracking systems into central ir_id_to_address table.

**Results**: 
- Total IR ID mappings: 13 → 237 (1800% increase)
- All 80 UnresolvedReferences now resolve successfully
- mini_zork executes through initialization and displays game banner
- Systematic reference resolution failures eliminated

**Implementation**: See `src/grue_compiler/codegen.rs` lines ~7490-7558

**Remaining**: One direct code emission issue at PC 0x1221 (separate from UnresolvedReference system). See CONTINUATION_PLAN.md for details.

---

## CRITICAL: Jump vs Branch Location Calculation Pattern Differences (Sep 15, 2025) ⚠️

**MAJOR FINDING**: Jump and Branch UnresolvedReference location calculations use DIFFERENT patterns. Mixing them causes systematic bugs.

### Jump Instructions (CORRECT Pattern):
- **Structure**: `[opcode][operand_high][operand_low]` (1OP instruction)
- **Location Calculation**: Points to operand location (after opcode)
```rust
// In translate_jump():
let operand_location = self.final_code_base + self.code_space.len() + 1; // +1 for opcode byte
UnresolvedReference { location: operand_location, ... }
```
- **Resolution**: Uses `-1` adjustment to get back to instruction start for offset calculation
```rust
// In resolve_unresolved_references():
let instruction_pc = reference.location - 1; // Back to instruction start from operand
let offset = resolved_address as i32 - (instruction_pc as i32 + 3);
```
- **Rationale**: Jump offset calculation needs instruction start position, but location points to operand

### Branch Instructions (CORRECT Pattern):
- **Structure**: `[opcode][operands...][branch_high][branch_low]` (branch offset is part of instruction)
- **Location Calculation**: Points directly to branch offset location
```rust
// In emit_instruction() codegen_instructions.rs:
let branch_location = if let Some(_offset) = branch_offset {
    let loc = self.code_address;  // Points to where placeholder bytes will be written
    self.emit_word(0xFFFF)?;      // Emit placeholder at that location
    Some(loc)
};
```
- **Resolution**: NO adjustment needed - location points directly to where offset should be patched
```rust
// In resolve_unresolved_references():
let result = self.patch_branch_offset(final_location, resolved_address); // No -1 needed
```
- **Rationale**: Branch offset location is calculated directly, no derivation needed

### ❌ CRITICAL BUG: Incorrect -1 Adjustment for Branches
```rust
// WRONG - copying jump pattern to branches:
let corrected_location = final_location - 1; // This breaks branch patching!
```

**Root Cause**: Someone copied the jump resolution pattern to branches, but they have different instruction structures.

**Fix**: Remove -1 adjustment for branches. The `emit_instruction` function already calculates the correct branch offset location.

**Impact**: The -1 hack was masking placeholder resolution - without it, placeholders resolve correctly but offset calculations need proper instruction positioning.

---

## Branch Placeholder Encoding Semantics (December 14, 2025) ✅

**CRITICAL UNDERSTANDING**: Branch placeholder values encode branch polarity via bit 15, not branch format.

### Z-Machine Branch Encoding Review

**Final Branch Format** (after resolution):
- Bit 7: Branch polarity (1 = branch on TRUE, 0 = branch on FALSE)
- Bit 6: Format (1 = 1-byte, 0 = 2-byte)
- Bits 5-0 (+ second byte if 2-byte): Branch offset

**Compiler Policy**: ALL branches use 2-byte format (bit 6 = 0 always)

### Placeholder Encoding System

The compiler uses placeholder values to communicate branch polarity to the resolution phase:

```rust
// In emit_comparison_branch (codegen_branch.rs:262-266):
let placeholder = if branch_on_true {
    0xBFFF_u16 as i16 // bit 15=1 encodes "branch on TRUE"
} else {
    0x7FFF_u16 as i16 // bit 15=0 encodes "branch on FALSE"
};
```

**Resolution Phase** (codegen.rs:5097-5100):
```rust
// Read placeholder to determine intended polarity
let high_byte = self.final_data[location] as u16;
let low_byte = self.final_data[location + 1] as u16;
let placeholder = (high_byte << 8) | low_byte;
let branch_on_true = (placeholder & 0x8000) != 0; // Check bit 15
```

### Why Mixed Encodings Are Correct

**Current codebase has**:
- 5 instances of 0xBFFF (branch on TRUE)
- 1 instance of 0x7FFF (branch on FALSE at line 829)

**All are semantically correct** - they encode the intended branch polarity:

| Location | Instruction | Intended Logic | Encoding | Correct? |
|----------|-------------|----------------|----------|----------|
| codegen_grammar.rs:121 | `jl word_count < 1` | Branch when TRUE (skip when no input) | 0xBFFF | ✅ |
| codegen_grammar.rs:243 | `je verb == this_verb` | Branch when TRUE (matched) | 0xBFFF | ✅ |
| codegen_grammar.rs:396 | `jl word_count < 2` | Branch when TRUE (insufficient words) | 0xBFFF | ✅ |
| codegen_grammar.rs:829 | `je word_count == 2` | Branch when FALSE (word count != 2) | 0x7FFF | ✅ |
| codegen_grammar.rs:1026 | `jl word_count < 3` | Branch when TRUE (insufficient words) | 0xBFFF | ✅ |
| codegen_lookup.rs:201 | `jg object > max` | Branch when TRUE (out of bounds) | 0xBFFF | ✅ |

### Common Misunderstanding: Bit 15 vs Bit 7

**WRONG interpretation**: "0xBFFF has bit 6=1, so it's 1-byte format"
- This confuses placeholder encoding (bit 15) with final branch encoding (bit 6)
- Placeholder values are TEMPORARY markers, not final branch bytes

**CORRECT interpretation**: "0xBFFF has bit 15=1, so `patch_branch_offset` will set bit 7=1 (branch on TRUE)"
- Placeholder bit 15 → Final branch bit 7 (polarity)
- Final branch bit 6 is ALWAYS 0 (compiler enforces 2-byte format)

### Previous Bug: Line 829 Encoding

**Original code**: Used 0xBFFF for `je word_count == 2` with "skip if NOT equal" intent
- Intent: Skip pattern handler when word count != 2
- Instruction: `je` branches when EQUAL
- Logic: Branch when word_count == 2 (TRUE) would skip when matched (WRONG!)
- Required: Branch when word_count != 2 (FALSE condition)

**Fixed code**: Changed to 0x7FFF (branch on FALSE)
- Now correctly skips when je is FALSE (word_count != 2)

### Detection vs Prevention

**Search Results** (December 14, 2025):
- Found 6 instances of 0xBFFF branch encoding
- Analyzed each for semantic correctness
- Confirmed 5/6 are correct, 1 fixed in previous session

**Prevention Strategy**:
- Always verify branch polarity matches intended control flow
- Test pattern matching commands after branch encoding changes
- Use descriptive comments explaining branch intent

### Files Referenced

- `src/grue_compiler/codegen_branch.rs:262-266` - Placeholder emission
- `src/grue_compiler/codegen.rs:5097-5100` - Placeholder reading
- `src/grue_compiler/codegen.rs:5103-5108` - Final encoding with forced 2-byte format
- `src/grue_compiler/codegen_grammar.rs` - Pattern matching logic (6 branch instances)
- `src/grue_compiler/codegen_lookup.rs` - Object lookup bounds check

---

## Deferred Features

### Computed Property System (Archived November 1, 2025)

**Concept**: Runtime-evaluated object property expressions
```grue
object mailbox {
    desc: "The mailbox is " + (mailbox.open ? "open" : "closed") + "."
}
```

**Implementation Status**: 70% complete (Phase 1 & 2 working, Phase 3 partial)
**Archive Location**: `computed-property-implementation-archive` branch
**Documentation**: `docs/COMPUTED_PROPERTY_IMPLEMENTATION_ARCHIVE.md`

**Architecture Overview**:
- **Phase 1**: IR registration of computed expressions ✅
- **Phase 2**: Z-Machine function generation ✅
- **Phase 3**: GetProperty function calling ❌ (complexity issues)

**Revert Reason**: Complexity vs. benefit analysis favored simpler conditional print approach

**Future Considerations**: Advanced language feature for consideration once core systems are fully stable

---

*Last Updated: November 1, 2025*
*Context: Computed property system archived, all major compiler systems functional*