# Ongoing Tasks

## CURRENT BUG: Garbled Object Examination Text (Oct 15, 2025)

**Status**: Investigation in progress

**Symptom**: `examine mailbox` displays garbled text like "amFym xw cwm glpgg rwoxnv"

### Current Evidence

- Property 16 (names) for object 1 (mailbox) returns value 0x0960
- Memory at 0x0960: `7a9a df0a 8000 132d 5e9a 31a0 65aa 0295` (garbled Z-string)
- Source defines: `names: ["small mailbox", "mailbox", "box"]`
- Grammar system reads property 16 to match objects against user input

### Root Cause - CONFIRMED ✅

**Architecture Bug**: Property 16 (names) only stores ONE dictionary address (first name from array), but objects have MULTIPLE valid names.

**The Problem**:
1. Mailbox has names: `["small mailbox", "mailbox", "box"]`
2. ALL three are added to dictionary with separate addresses
3. Property 16 stores ONLY "small mailbox" dictionary address (0x0924)
4. When user types "mailbox", it has DIFFERENT dictionary address
5. Object lookup compares input dict address vs property 16 values
6. NO MATCH found because property 16 only has "small mailbox"'s address!
7. Lookup fails, falls through to error case

**Evidence**:
- Compiler log: "small mailbox" at position 29 → address 0x0924
- Runtime: Object 10 property 16 = 0x0924
- User typed "mailbox" (different word, different dict address)
- Object lookup fails to find match

**The Fix**: Property 16 must store ALL dictionary addresses for all names, not just first one.
- Change from `set_word(names_prop, single_address)`
- To `set_bytes(names_prop, [addr1_hi, addr1_lo, addr2_hi, addr2_lo, ...])`
- Update object lookup to check if input address matches ANY address in property 16

### Verification Steps

1. **Check Dictionary Compilation**:
   ```bash
   RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test.z3 2>&1 | grep -i "dictionary\|mailbox"
   ```
   - Verify "mailbox", "small mailbox", "box" are in dictionary
   - Check their dictionary addresses

2. **Inspect Property 16 Generation**:
   ```bash
   RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test.z3 2>&1 | grep "property 16\|names property"
   ```
   - Find where property 16 is written
   - Check what value is written (string ID vs dictionary address)

3. **Compare Working vs Broken Objects**:
   - Rooms (objects 2-9) correctly have property 16 = 0x0000 (no names)
   - Items (objects 10-15) have non-zero property 16 values
   - Check if all items have garbled text or just mailbox

4. **Examine Compiled Property Table**:
   ```bash
   xxd -s 0x0481 -l 256 tests/mini_zork.z3
   ```
   - Find mailbox's property table structure
   - Verify property 16 format (should be word list, not packed string)

### Likely Issues

**Issue 1: names Property Compilation** (MOST LIKELY)
- File: `src/grue_compiler/codegen_objects.rs`
- Compiler may be treating `names: ["small mailbox", "mailbox", "box"]` as:
  - String concatenation → Z-string → packed address
- Should be treating it as:
  - Dictionary word list → multiple dictionary addresses

**Issue 2: Property 16 Format Mismatch**
- Property 16 should contain multiple words (dictionary addresses)
- Currently contains single word (0x0960 = packed string address)
- Grammar system expects to iterate through dictionary addresses, not unpack Z-strings

**Issue 3: Missing Dictionary Lookup**
- Compiler should look up each string in `names:` array in the dictionary
- Write list of dictionary addresses to property 16
- Instead it's creating a Z-string and storing its packed address

### Testing Approach

**Step 1: Add Compiler Logging** (codegen_objects.rs)
```rust
// When processing names property
log::debug!("🔍 NAMES: Processing names property for object {}, values: {:?}", obj_name, name_list);
log::debug!("🔍 NAMES: Writing to property 16, value=0x{:04x}", value);
```

**Step 2: Verify Dictionary** (codegen.rs or dictionary module)
```rust
log::debug!("🔍 DICT: Added word '{}' at address 0x{:04x}", word, address);
```

**Step 3: Test with Simple Case**
- Create minimal test with single object with names
- Verify property 16 structure
- Compare against Zork I property tables (working reference)

**Step 4: Check Property 16 Semantics**
- Search codebase for "property 16" or "names property"
- Verify if there's special handling for names vs other properties
- Look for dictionary address resolution code

### Implementation Plan

**Step 1: Update Property Storage (codegen_objects.rs:476-493)**
- Change from `object_properties.set_word(names_prop, 0xFFFF)` (single placeholder)
- To `object_properties.set_bytes(names_prop, vec_of_placeholders)` (multiple placeholders)
- Create placeholder for EACH name in `object.names` array
- Track ALL names for DictionaryRef resolution, not just first one

**Step 2: Update DictionaryRef Creation (codegen.rs:5408-5454)**
- Current: Creates ONE DictionaryRef for first name
- New: Loop through property bytes, create DictionaryRef for EACH 2-byte word
- Each DictionaryRef points to different word in names array

**Step 3: Update Object Lookup (codegen.rs:7111-7156)**
- Current: `get_prop` reads 2-byte value, compares with input
- New: `get_prop_len` to get property size, loop through all 2-byte words
- Match succeeds if input matches ANY dictionary address in property 16

**Files to Modify**:
1. `src/grue_compiler/codegen_objects.rs` - lines 476-493, 351-364 (player too)
2. `src/grue_compiler/codegen.rs` - lines 5408-5454 (DictionaryRef), 7111-7156 (lookup)

### Success Criteria

After fix:
1. Property 16 for mailbox contains ALL dictionary addresses (0x0924, 0x08XX, 0x08YY for "small mailbox", "mailbox", "box")
2. Grammar system successfully matches ANY valid name: "mailbox", "small mailbox", "box"
3. `examine mailbox` displays proper description (mailbox.desc)
4. No garbled text in any object examination
5. All 196 tests still pass

---

## Future Tasks

(None currently)
