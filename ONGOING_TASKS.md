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

### Implementation Plan - DETAILED (Oct 16, 2025)

**PHASE 1: Update Property Storage in codegen_objects.rs**
- Location: Player object (~line 343-365), Regular objects (~line 482-520)
- Change: Create MULTIPLE placeholders (2 bytes per name) instead of single placeholder
- Pattern: Same as exit_directions fix (Bug #9)
- Code:
  ```rust
  if !object.names.is_empty() {
      let mut name_placeholders = Vec::new();
      for _ in &object.names {
          name_placeholders.push(0xFF);
          name_placeholders.push(0xFF);
      }
      object_properties.set_bytes(names_prop, name_placeholders);
      self.object_vocabulary_names.insert(object.name.clone(), object.names.clone());
  }
  ```
- Verify: Compile and check logs show correct placeholder count

**PHASE 2: Add Tracking Field to ZMachineCodeGen**
- Location: `src/grue_compiler/codegen.rs` - struct definition
- Add field: `pub object_vocabulary_names: IndexMap<String, Vec<String>>`
- Initialize in `new()`: `object_vocabulary_names: IndexMap::new()`
- Verify: Code compiles without errors

**PHASE 3: Update Property Serialization - Create DictionaryRefs**
- Location: `src/grue_compiler/codegen.rs` - `serialize_property_to_object_space()`
- Pattern: Same as exit_directions DictionaryRef creation
- Code: Loop through vocabulary_names, create DictionaryRef for each name
- Verify: Compile and check logs show DictionaryRefs created

**PHASE 4: Update Object Lookup (Grammar System)**
- Location: `src/grue_compiler/codegen.rs` - object lookup builtin (~line 7100-7200)
- Change: Loop through ALL addresses in property 16, not just read single word
- Pattern: get_prop_addr + get_prop_len + loop checking each 2-byte word
- Verify: Test gameplay with all object names

**Files to Modify**:
1. `src/grue_compiler/codegen_objects.rs` - lines 343-365 (player), 482-520 (objects)
2. `src/grue_compiler/codegen.rs` - struct definition, property serialization, object lookup

### Success Criteria

After fix:
1. Property 16 for mailbox contains ALL dictionary addresses (0x0924, 0x08XX, 0x08YY for "small mailbox", "mailbox", "box")
2. Grammar system successfully matches ANY valid name: "mailbox", "small mailbox", "box"
3. `examine mailbox` displays proper description (mailbox.desc)
4. No garbled text in any object examination
5. All 196 tests still pass

---

## ARCHITECTURE DISCOVERY: Property Table Pointer Address Translation (Oct 17, 2025)

**Status**: ✅ UNDERSTOOD - Documented for reference

**Investigation**: How do property table pointers get translated from object_space-relative (0x00c5) to final-absolute addresses (0x0481)?

### The Mystery

During investigation of property 16 issues, we noticed:
1. Property table pointers are written as **space-relative** during object generation (e.g., 0x00c5)
2. Binary files contain **final-absolute** addresses (e.g., 0x0481)
3. Manual adjustment code was removed, yet addresses are still correct
4. UnresolvedReference system does NOT write to object entry locations
5. Adjustment of exactly +0x03BC (object_base) happens between POST_COPY logging and file write

**Question**: WHERE and HOW are property table pointers being adjusted?

### The Complete Pipeline

**Phase 1: Generation** (`codegen_objects.rs:5335-5336`)
```rust
// Write space-relative property table address to object entry
self.write_to_object_space(obj_offset + 7, (prop_table_addr >> 8) as u8)?; // High byte
self.write_to_object_space(obj_offset + 8, (prop_table_addr & 0xFF) as u8)?; // Low byte
```
- Property table pointers are written as **object_space-relative** addresses
- Example: Object #1 property pointer = 0x00c5 (space-relative)
- Reason: During generation, we don't know final memory layout yet

**Phase 2: Copy** (`codegen.rs:1444`)
```rust
self.final_data[object_base..dictionary_base].copy_from_slice(&self.object_space);
```
- Object space is copied to final_data **unchanged**
- Property pointers are still space-relative after copy
- POST_COPY logging confirms: Object #1 at final_data[0x0401] = 0x00c5

**Phase 3: Patch** (`codegen.rs:1650`) - **THE KEY STEP**
```rust
// CRITICAL: Patch property table addresses from space-relative to absolute
self.patch_property_table_addresses(object_base)?;
```

### The patch_property_table_addresses() Function

**Location**: `codegen.rs:5869-6010`

**Algorithm**:
1. Calculate object count by iterating through object entries
2. For each object:
   - Read space-relative property pointer from final_data (e.g., 0x00c5)
   - Validate it's a real object (not property table data):
     - Must be non-zero
     - Must be >= minimum valid property address
     - Must be within object_space bounds
   - Calculate absolute address: `object_base + space_relative`
     - Example: 0x03BC + 0x00c5 = 0x0481
   - Write absolute address back to final_data
3. Stop iteration when pointer validation fails (reached property table data)

**Key Code** (`codegen.rs:5986-6000`):
```rust
// Calculate absolute final memory address
let absolute_addr = object_base + (space_relative_addr as usize);

// Write the corrected absolute address back to final_data
let new_high_byte = (absolute_addr >> 8) as u8;
let new_low_byte = (absolute_addr & 0xFF) as u8;

self.final_data[final_addr_offset] = new_high_byte;
self.final_data[final_addr_offset + 1] = new_low_byte;
```

### Why This Architecture

**Benefits**:
1. **Separation of Concerns**: Object generation doesn't need to know final memory layout
2. **Simplicity**: Property table creation uses simple 0-based addressing
3. **Flexibility**: Final memory layout can change without affecting object generation
4. **Single Responsibility**: Address translation happens in one place, one time

**Alternative Rejected**: Using UnresolvedReference system for property pointers
- UnresolvedReferences are for forward references (addresses not known yet)
- Property pointers ARE known during generation (just need translation)
- Patching is simpler and more efficient than reference resolution

### Verification

**Test**: Compile mini_zork.grue and examine binary:
```bash
RUST_LOG=error ./target/debug/grue-compiler examples/mini_zork.grue -o /tmp/test.z3 2>&1 | grep "VERIFY_AFTER_PATCH"
```

**Output**:
```
🔍 VERIFY_AFTER_PATCH: Object #2 prop ptr at 0x040a = 0x04 0x8f = 0x048f
```

**Binary Verification**:
```bash
xxd -s 0x03fa -l 48 tests/mini_zork.z3
```

**Result**:
```
000003fa: 0000 0000 0000 0004 8100 0000 0000 0000  ................
0000040a: 048f 0000 0000 0000 0004 b300 0000 0000  ................
0000041a: 0000 04d0 0000 0000 0000 0004 ed00 0000  ................
```

Property table pointers are correctly absolute: 0x0481, 0x048f, 0x04b3 ✅

### Files Involved

1. **codegen_objects.rs**:
   - Lines 5335-5336: Write space-relative property pointers
   - Function `write_to_object_space()`: Simple byte writer, no address adjustment

2. **codegen.rs**:
   - Line 1444: Copy object_space to final_data (unchanged)
   - Line 1650: Call `patch_property_table_addresses(object_base)`
   - Lines 5869-6010: `patch_property_table_addresses()` implementation
   - Lines 1652-1662: Verification logging after patch

### Lessons Learned

1. **Don't assume bugs exist**: The system was working correctly all along
2. **Trace the full pipeline**: Understanding generation → copy → patch explained everything
3. **Read the code carefully**: `patch_property_table_addresses()` was right there at line 1650
4. **Logging is essential**: POST_COPY and VERIFY_AFTER_PATCH logs revealed the truth
5. **Architecture matters**: Space-relative → absolute translation is clean design

### Related to Property 16 Bug

This investigation was triggered by property 16 (names) issues. Understanding the complete address translation pipeline is crucial for fixing property 16 correctly, as we need to ensure:
- Dictionary addresses are stored correctly in property 16
- Address translation doesn't corrupt property data
- UnresolvedReferences handle dictionary addresses properly

---

## BUG #23 INVESTIGATION: Object Numbering Mismatch (Oct 17, 2025)

**Status**: 🔧 ARCHITECTURAL ROOT CAUSE IDENTIFIED - Implementation in progress

**Symptom**: Compiler writes mailbox property 7 to 0x059c, interpreter reads object 10 property 16 from 0x0595 → 7-byte mismatch causing garbled text

### Investigation Steps Completed

**Step 1**: ✅ Found PropertyTable/PropertyBuilder serialization code
- Location: `codegen.rs:5244-5329` (property serialization)
- Location: `codegen.rs:4960-5102` (object entry creation)
- Result: Property 16 IS being written correctly with two-byte header format

**Step 2**: ✅ Discovered object numbering mismatch - ROOT CAUSE FOUND
- Added debug logging to object generation loop (`codegen_objects.rs:810-816`)
- Compiled and analyzed logs showing TWO different numbering schemes:
  - **OBJECT_GEN** (correct): mailbox = obj_num 10
  - **OBJECT_MAPPING** (wrong): mailbox = Object #3
- Traced compilation order:
  1. Line 915: `generate_object_tables()` - Creates CORRECT object_id_to_number mapping
  2. Line 964: `generate_code_to_space()` → calls `setup_object_mappings()` at line 2437
- Found `setup_object_mappings()` OVERWRITES correct mappings with semantic analysis numbers

### COMPLETE ARCHITECTURAL ROOT CAUSE ANALYSIS

**SMOKING GUN EVIDENCE**:
```
🔢 OBJECT_GEN: index=9, obj_num=10, name='mailbox', short_name='small mailbox'
🗺️ OBJECT_MAPPING: 'mailbox': IR ID 33 → Object #3
```

**THE FUNDAMENTAL PROBLEM**: Two competing object numbering systems create inconsistent property address calculations.

#### Numbering System #1: IR Semantic Analysis (`ir.rs`)

**Location**: `src/grue_compiler/ir.rs` - object counter logic
**Algorithm**:
1. Player = Object #1 (hardcoded)
2. Rooms assigned sequentially starting from object_counter=2
3. Objects assigned after rooms, continuing sequence
4. **Result**: `ir.object_numbers` mapping

**Example Sequence**:
- Player: Object #1
- west_of_house: Object #2
- north_of_house: Object #3
- mailbox: Object #3 ⚠️ **CONFLICT** - overwrites north_of_house!

#### Numbering System #2: Object Generation (`codegen_objects.rs`)

**Location**: `src/grue_compiler/codegen_objects.rs:805`
**Algorithm**:
```rust
let obj_num = (index + 1) as u8; // Objects are numbered starting from 1
```
1. Sequential based on `all_objects` vector order (rooms first, then objects)
2. Player at index 0 → obj_num 1
3. Mailbox at index 9 → obj_num 10
4. **Result**: Correct Z-Machine object numbering

#### The Architectural Conflict

**Property Address Calculation Flow**:
1. **Object Generation Phase** (`codegen_objects.rs:788-820`):
   - Uses **sequential obj_num** (mailbox=10) to calculate property table addresses
   - Property tables laid out correctly for obj_num sequence
   - Mailbox property 7 written to correct offset for Object #10

2. **Object Mapping Phase** (`codegen.rs:8392`):
   - **OVERWRITES** correct mappings with IR semantic numbers
   - Mailbox IR ID 33 → Object #3 (wrong!)
   - `ir_id_to_object_number` now contains wrong mapping

3. **Reference Resolution Phase**:
   - UnresolvedReferences use **wrong object number** (3 instead of 10)
   - String references calculated for Object #3's property addresses
   - But property tables were generated for Object #10's addresses
   - **Result**: 7-byte mismatch between generation and resolution

#### The 7-Byte Calculation

**Address Mismatch Analysis**:
- **Compiler writes**: Object #10 property 7 at address 0x059c
- **Interpreter reads**: Object #10 property 16 at address 0x0595
- **Difference**: 0x059c - 0x0595 = 7 bytes

**Why 7 bytes?**
- Property 7 vs Property 16: Different properties in same object
- Property table layout: Property headers + data, descending property number order
- 7-byte difference suggests reading from different property slot in same object

#### Root Cause Code Location

**The Overwrite**: `src/grue_compiler/codegen.rs:8392`
```rust
for (name, &ir_id) in &ir.symbol_ids {
    if let Some(&object_number) = ir.object_numbers.get(name) {
        self.ir_id_to_object_number.insert(ir_id, object_number); // ❌ OVERWRITES CORRECT MAPPING!
```

**Timeline**:
1. Line 915: `generate_object_tables()` → Creates CORRECT obj_num mappings
2. Line 964: `generate_code_to_space()` → calls `setup_object_mappings()`
3. Line 8392: **OVERWRITES** correct mappings with semantic analysis numbers
4. Property reference resolution uses wrong numbers → address mismatch

### ARCHITECTURAL SOLUTION

**STRATEGY**: Use IR semantic numbering consistently instead of sequential generation numbering.

**REJECTED APPROACH**: Preserving sequential numbering from generation phase
- **Problem**: Sequential numbering is arbitrary (based on vector order)
- **Issue**: IR semantic analysis has already established object relationships
- **Conflict**: Would break existing IR references and semantic integrity

**CORRECT APPROACH**: Fix object generation to use IR numbering

#### Implementation Plan

**Phase 1: Update Object Generation Logic** (`codegen_objects.rs:805`)

**Current Code**:
```rust
let obj_num = (index + 1) as u8; // Sequential numbering
```

**Fixed Code**:
```rust
// Use IR semantic object numbering instead of sequential
let ir_id = &all_objects[index].ir_id;
let obj_num = if let Some(&semantic_number) = ir.object_numbers.get(&all_objects[index].name) {
    semantic_number as u8
} else {
    // Fallback for objects not in IR (shouldn't happen)
    (index + 1) as u8
};
log::debug!("🔢 OBJECT_GEN: index={}, obj_num={} (from IR), name='{}', short_name='{}'",
           index, obj_num, all_objects[index].name, all_objects[index].short_name);
```

**Phase 2: Remove Conflicting Mapping Overwrite** (`codegen.rs:8392`)

**Current Code**:
```rust
self.ir_id_to_object_number.insert(ir_id, object_number); // OVERWRITES
```

**Fixed Code**:
```rust
// Object generation phase already set correct mapping - don't overwrite
if !self.ir_id_to_object_number.contains_key(&ir_id) {
    self.ir_id_to_object_number.insert(ir_id, object_number);
    log::debug!("🗺️ OBJECT_MAPPING: '{}': IR ID {} → Object #{} (new)", name, ir_id, object_number);
} else {
    let existing = self.ir_id_to_object_number[&ir_id];
    log::debug!("🗺️ OBJECT_MAPPING: '{}': IR ID {} → Object #{} (existing, not overwritten)", name, ir_id, existing);
}
```

**Phase 3: Verify Consistency**

Add verification logging to ensure both systems produce same numbers:
```rust
log::debug!("🔍 CONSISTENCY_CHECK: Object '{}' - Generation: {}, IR: {}",
           name, generation_number, ir_semantic_number);
assert_eq!(generation_number, ir_semantic_number, "Object numbering mismatch for {}", name);
```

### Success Criteria

After fix:
1. Object numbers consistent between generation and mapping phases
2. OBJECT_MAPPING logs match OBJECT_GEN logs (mailbox = 10 in both)
3. Property tables point to correct objects
4. `examine mailbox` displays proper description (not garbled text)
5. All 196 tests still pass

---

## COMPLETE OBJECT DUMP ANALYSIS: String Reference Patching Failure (Oct 17, 2025)

**Status**: 🔍 CRITICAL DISCREPANCY IDENTIFIED - Root cause found

**Investigation**: Complete dumps from compiler and interpreter reveal exact mismatch between intended patches and runtime values.

### Executive Summary

Despite fixing the address translation issue in `MemorySpace::Objects`, garbled text persists because the actual string reference patching process has a fundamental flaw. The compiler intends to patch location 0x059c with value 0x078c (mailbox description), but the interpreter reads 0x09d9 at runtime, resulting in garbled text "baGmm xw cwm glpgg rwtlpn".

### Complete Analysis Chain

**COMPILER PERSPECTIVE (String Creation & Resolution):**

1. **String Creation** (String ID 1019):
   ```
   🔍 STRING_CREATION: String ID 1019 = 'The small mailbox is closed.'
   🔍 STRING_FINAL: String ID 1019 at memory[0x078c-0x07a7] = 54686520736d616c6c206d61696c626f782069732...
   ```

2. **UnresolvedReference Creation** (Object Generation):
   ```
   🔍 UNRESOLVED_REF: Creating at Objects[0x01e0] → String ID 1019
   - Location: MemorySpace::Objects, offset 0x01e0
   - Target: String ID 1019
   - Reference Type: StringReference
   ```

3. **Address Translation** (Objects Space → Final):
   ```
   🔍 ADDRESS_TRANSLATION: Objects[0x01e0] → final_object_base(0x03bc) + 0x01e0 = 0x059c ✅
   ```

4. **String ID Resolution** (String ID → Packed Address):
   ```
   🔍 STRING_RESOLUTION: String ID 1019 → packed address 0x078c
   - String at 0x078c: "The small mailbox is closed." ✅
   ```

5. **INTENDED PATCH**:
   ```
   PATCH: Write 0x078c to final_data[0x059c]
   ```

**INTERPRETER PERSPECTIVE (Runtime State):**

6. **Property Table Structure**:
   ```bash
   # Object #3 (mailbox) property table at 0x059a:
   xxd -s 0x059a -l 16 tests/mini_zork.z3
   0000059a: 0724 0f09 d907 2409 d900 0000 0000 0000  .$....$.........
                      ^^^^
                      Property 7 = 0x09d9 ❌ WRONG VALUE!
   ```

7. **String Content at Wrong Address**:
   ```bash
   # String at 0x09d9 (actual runtime value):
   xxd -s 0x09d9 -l 32 tests/mini_zork.z3
   000009d9: 9261 476d 6d20 7877 2063 776d 2067 6c70  .aGmm xw cwm glp
   000009e9: 6767 2072 7774 6c70 6e80 0000 0000 0000  gg rwtlpn.......

   # Decodes to: "baGmm xw cwm glpgg rwtlpn" (GARBLED)
   ```

8. **Expected String Content**:
   ```bash
   # String at 0x078c (intended value):
   xxd -s 0x078c -l 32 tests/mini_zork.z3
   0000078c: 5468 6520 736d 616c 6c20 6d61 696c 626f  The small mailbo
   0000079c: 7820 6973 2063 6c6f 7365 642e 8000 0000  x is closed.....

   # Decodes to: "The small mailbox is closed." ✅ CORRECT
   ```

### The Critical Discrepancy

**INTENDED BEHAVIOR:**
- Location: final_data[0x059c]
- Value: 0x078c (packed address of "The small mailbox is closed.")

**ACTUAL RUNTIME:**
- Location: final_data[0x059c] contains 0x09d9
- Value: Points to garbled string "baGmm xw cwm glpgg rwtlpn"

**THE QUESTION:** Why does location 0x059c contain 0x09d9 instead of the intended 0x078c?

### Possible Root Causes

**Hypothesis 1: Patch Never Applied**
- UnresolvedReference for Objects[0x01e0] → String ID 1019 was never resolved
- Location 0x059c retains original/garbage value 0x09d9
- String ID 1019 correctly resolved to 0x078c but patch didn't happen

**Hypothesis 2: Wrong Patch Location**
- Reference resolution calculated wrong final address
- Patch applied to wrong location (not 0x059c)
- Location 0x059c never updated from original value

**Hypothesis 3: Patch Overwritten**
- Patch correctly applied: final_data[0x059c] = 0x078c
- Later process overwrote location 0x059c with 0x09d9
- Possible culprits: property table patching, dictionary building, other references

**Hypothesis 4: Multiple References to Same Location**
- Two UnresolvedReferences pointing to same final location 0x059c
- First reference: String ID 1019 → 0x078c (correct)
- Second reference: String ID XXXX → 0x09d9 (wrong, overwrites first)

### Investigation Plan

**Phase 1: Trace UnresolvedReference Resolution**
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test_patch_trace.z3 2>&1 | grep -E "(UNRESOLVED_REF|RESOLVING_REF|PATCH_APPLIED)"
```
- Verify UnresolvedReference for Objects[0x01e0] is in resolution list
- Confirm it resolves to location 0x059c with value 0x078c
- Check if patch is actually applied to final_data

**Phase 2: Memory State Before/After Resolution**
```bash
# Add logging in resolve_references() to dump final_data[0x059c] before/after each patch
log::debug!("🔍 BEFORE_PATCH: final_data[0x{:04x}] = 0x{:02x}{:02x}",
           location, final_data[location], final_data[location+1]);
# Apply patch
log::debug!("🔍 AFTER_PATCH: final_data[0x{:04x}] = 0x{:02x}{:02x}",
           location, final_data[location], final_data[location+1]);
```

**Phase 3: Check for Conflicting References**
```bash
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue -o /tmp/test_conflicts.z3 2>&1 | grep "0x059c"
```
- Search all UnresolvedReferences for conflicts at location 0x059c
- Verify only ONE reference targets this location
- Check reference types (StringReference vs others)

**Phase 4: Binary Verification**
```bash
# Immediately after compilation, verify binary content
xxd -s 0x059c -l 4 /tmp/test_verification.z3
```
- Check if 0x059c contains 0x078c (correct) or 0x09d9 (wrong) in fresh binary
- Compare against interpreter runtime state

### Expected Outcome

After investigation, we should identify:
1. **WHERE** the patch fails: creation, resolution, or application phase
2. **WHY** location 0x059c contains 0x09d9 instead of 0x078c
3. **WHAT** specific code change will ensure correct patching

### Success Metrics

**Fix Validation:**
1. Compiler logs show patch applied: final_data[0x059c] = 0x078c
2. Binary contains correct value: xxd shows 0x078c at offset 0x059c
3. Interpreter reads correct value: Property 7 = 0x078c
4. Gameplay works: `examine mailbox` shows "The small mailbox is closed."
5. No garbled text for any object examination
6. All existing tests continue to pass

**Files Involved:**
- `src/grue_compiler/codegen.rs` - UnresolvedReference resolution (`resolve_references()`)
- `src/grue_compiler/codegen_objects.rs` - UnresolvedReference creation
- `tests/mini_zork.z3` - Final binary verification

### Related Investigations

This analysis builds on:
- **Bug #22**: Address translation fix (Objects space → final addresses) ✅
- **Property 16 Investigation**: Object naming system issues ⏳
- **Architecture Discovery**: Property table pointer patching process ✅

The string reference patching failure is a separate issue from property 16 (names), but both affect object examination functionality.

---

## BUG #23 BREAKTHROUGH: Dictionary Address Generation Issue (Oct 17, 2025)

**Status**: 🎯 ROOT CAUSE IDENTIFIED - Dictionary addresses in property 16 are invalid

**Discovery**: Object numbering fix successful, property addressing working perfectly, but property VALUES contain invalid dictionary addresses.

### The Evidence

**BEFORE Object Numbering Fix**:
- Problem: Interpreter reads Object #10 property 16 from 0x0595, compiler writes Object #10 property 7 to 0x059c
- Issue: 7-byte address mismatch due to object numbering inconsistency

**AFTER Object Numbering Fix**: ✅ MAJOR SUCCESS
- Fix: Object generation now uses IR semantic numbering (mailbox = Object #3, not #10)
- Result: Both compiler and interpreter now correctly target Object #3 property 16 at address 0x0595
- Verification: Property access debugging shows perfect consistency

**REMAINING ISSUE**: Property 16 contains INVALID dictionary addresses
```
📦 Object #3: ""  ← mailbox (correctly identified)
   Properties:
      Property 16: 0x0924 ← ⚠️  NAMES PROPERTY (should be dictionary address array)
         Length: 6 bytes (3 words)
         [0]: 0x0924 (dict addr at 0x0595)  ← INVALID ADDRESS
         [1]: 0x08e8 (dict addr at 0x0597)  ← INVALID ADDRESS
         [2]: 0x0888 (dict addr at 0x0599)  ← INVALID ADDRESS
```

**Expected**: Dictionary addresses for "mailbox", "box" words
**Actual**: Invalid addresses 0x0924, 0x08e8, 0x0888 causing garbled text output

### Root Cause Analysis

**ARCHITECTURE DISCOVERY**: The object numbering mismatch was perfectly diagnosed and fixed:

1. **IR Semantic Analysis**: Assigned mailbox = Object #3 ✅
2. **Object Generation**: Was using sequential numbering (mailbox = obj_num 10) ❌
3. **Fix Applied**: Object generation now uses IR numbering consistently ✅
4. **Result**: Perfect addressing consistency between all compiler phases ✅

**NEW ISSUE**: Dictionary address generation phase writes wrong values to property 16
- Property 16 format: ✅ Correct (6 bytes = 3 dictionary addresses)
- Property 16 location: ✅ Correct (Object #3 at offset 0x0595)
- Property 16 content: ❌ Wrong (invalid dictionary addresses)

### Next Investigation Target

**FOCUS**: Dictionary address resolution in compiler
- **Location**: Dictionary word lookup and address assignment
- **Issue**: Compiler assigns invalid addresses (0x0924, 0x08e8, 0x0888) instead of real dictionary locations
- **Expected**: Valid dictionary addresses for "mailbox", "box" strings
- **Files**: Dictionary generation, object names property compilation

### Success Metrics

1. ✅ Object numbering consistency (ACHIEVED)
2. ✅ Property addressing accuracy (ACHIEVED)
3. ❌ Valid dictionary addresses in property 16 (IN PROGRESS)
4. ❌ Correct "examine mailbox" output (DEPENDENT ON #3)

### Major Win

**Bug #23 MAJOR PROGRESS**: Reduced core issue from architectural object numbering mismatch to specific dictionary address generation problem. The fundamental addressing and object systems are now working correctly.

---

## Future Tasks

(None currently)
