# Object Lookup Dictionary Addresses Analysis

**Date**: October 28, 2025
**Status**: CRITICAL ARCHITECTURE ISSUE IDENTIFIED - SOLUTION DOCUMENTED
**Priority**: HIGH - Required to replace temporary hardcoded fix

## Problem Summary

The object lookup system in our Grue compiler causes infinite loops when users type commands like `"open mailbox"`. This was temporarily fixed with a hardcoded workaround, but a proper architectural solution is now documented based on analysis of commercial Zork I.

## Root Cause Analysis

### The Fundamental Type Mismatch

**Our Broken System:**
```rust
// Compiler stores STRING IDs in object properties:
Property 18: 1018  // String ID for "a small mailbox"

// Runtime lookup compares STRING ID vs DICTIONARY ADDRESS:
if (property_18_value == dictionary_address) {  // 1018 ≠ 0x0726
    // NEVER matches → infinite loop
}
```

**The Issue**: Two completely different value types being compared:
- **Property 18**: Contains string IDs (like 1018)
- **Dictionary lookup**: Provides dictionary addresses (like 0x0726)
- **Result**: Values never match, causing infinite loop in object search

## Commercial Zork I Analysis

### Object Table Dump Results

Analysis of `./resources/test/zork1/DATA/ZORK1.DAT` reveals the canonical implementation:

```
Object 1:
  Properties:
    [18]: 46 dc 42 c2 42 b4    <- 6 bytes = 3 dictionary addresses

Object 2:
  Properties:
    [18]: 4e 29               <- 2 bytes = 1 dictionary address

Object 10:
  Properties:
    [18]: 3c a9 42 59         <- 4 bytes = 2 dictionary addresses
```

### Key Discovery: Property 18 Format

**Property 18 stores DICTIONARY ADDRESSES, not string IDs!**

- Each object name gets looked up in dictionary during compilation
- The resulting dictionary addresses are stored as byte pairs
- Multiple names = multiple 2-byte dictionary addresses concatenated
- Runtime lookup compares dictionary addresses directly

## Required Implementation

### Option A: Store Dictionary Addresses in Property 18 (RECOMMENDED)

**Compilation Phase Changes:**
```rust
// During object compilation (ir.rs around line 1574):
for object in objects {
    let mut dict_address_bytes = Vec::new();

    for name in object.names {
        // Look up name in dictionary during compilation
        let dict_addr = dictionary.get_address(name);  // "mailbox" → 0x0726

        // Store as big-endian 2-byte address
        dict_address_bytes.extend_from_slice(&dict_addr.to_be_bytes());
    }

    // Store dictionary addresses in property 18 (not string IDs!)
    object.set_property(18, dict_address_bytes);
}
```

**Runtime Lookup** (No changes needed - already correct!):
```rust
// Object lookup system (already implemented correctly):
for object_num in 1..max_objects {
    let property_18_bytes = get_property(object_num, 18);

    // Iterate through 2-byte dictionary addresses
    for chunk in property_18_bytes.chunks(2) {
        let stored_dict_addr = u16::from_be_bytes([chunk[0], chunk[1]]);

        if stored_dict_addr == target_dictionary_address {
            return object_num; // Found matching object!
        }
    }
}
```

### Implementation Details

**Files to Modify:**
1. **`src/grue_compiler/ir.rs`** (around line 1574)
   - Change property 18 generation from string IDs to dictionary addresses
   - Look up each name in dictionary during object compilation
   - Store resulting addresses as byte array

**Dictionary Integration:**
- Dictionary must be available during object compilation phase
- Each object name gets dictionary lookup: `name → dictionary_address`
- Multiple names create concatenated address list: `[addr1, addr2, addr3]`

**Byte Format:**
- Each dictionary address: 2 bytes, big-endian format
- Multiple addresses: concatenated without delimiters
- Example: `"small mailbox"` → `[0x07, 0x26, 0x08, 0x15]` for 2 names

## Benefits of This Approach

1. **Matches Z-Machine Specification**: Identical to commercial Zork I implementation
2. **Lookup Compatibility**: Existing object lookup code already expects dictionary addresses
3. **Multiple Names Support**: Naturally handles objects with multiple names
4. **Performance**: Direct address comparison, no runtime string conversion
5. **Standards Compliance**: Follows established Infocom conventions

## Phase 2: Dictionary Integration Analysis - COMPLETE ✅ (October 28, 2025)

### Key Questions Answered

1. **When are object names added to dictionary?**
   - **Answer**: Step 2b: Dictionary generation phase (`generate_dictionary_space()`)
   - **Evidence**: Log shows `📚 Adding object name to dictionary: 'mailbox'`, `'small mailbox'`, etc.
   - **Location**: `codegen_strings.rs:395-400` iterates through `ir.objects` and adds all names to dictionary

2. **When are object properties generated?**
   - **Answer**: Step 2c: Object space generation phase (`generate_object_tables()`)
   - **Evidence**: Log shows `🏠 Step 2c: Generating object space` AFTER dictionary generation
   - **Location**: `codegen_objects.rs:284+` processes `ir.objects` and calls `set_string` for properties

3. **Is dictionary available during object property generation?**
   - **Answer**: YES! Dictionary is fully populated and accessible
   - **Evidence**: Compilation timeline shows Step 2b → Step 2c sequence
   - **Access Method**: `self.lookup_word_in_dictionary(word)` returns `u16` dictionary address

### Technical Specifications Discovered

4. **Dictionary Access Interface Available**:
   - **Method**: `self.lookup_word_in_dictionary(word: &str) -> Result<u16, CompilerError>`
   - **Location**: `codegen.rs:5656-5687`
   - **Data Source**: `self.dictionary_words: Vec<String>` populated during Step 2b
   - **Address Calculation**: `dict_base + header_size + (position * entry_size)`

5. **Property 18 Location Confirmed**:
   - **Current Code**: `ir.rs:1584` - `properties.set_string(StandardProperty::ShortName as u8, short_name.clone())`
   - **Issue**: StandardProperty::ShortName = 1, not 18
   - **Codegen Location**: `codegen_objects.rs:571+` processes `object.properties` from IR

6. **Implementation Path Clear**:
   - **OPTION A**: Modify `ir.rs:1584` to set property 18 instead of property 1
   - **OPTION B**: Add property 18 logic in `codegen_objects.rs` where object properties are processed
   - **Dictionary Access**: Available via `self.lookup_word_in_dictionary()` during codegen phase

## Current Status

- **Analysis**: COMPLETE ✅
- **Phase 2 Dictionary Integration**: COMPLETE ✅ (October 28, 2025)
- **Temporary Fix**: Hardcoded mailbox object #10 recognition (working) ✅
- **Permanent Solution**: Ready for implementation - dictionary access confirmed
- **Testing**: Commercial Zork I analysis confirms approach ✅

## Next Steps

1. **Implement dictionary address storage** in object property generation
2. **Test with multiple object names** to verify multiple address handling
3. **Remove hardcoded temporary fix** once permanent solution verified
4. **Regression test** with all existing objects to ensure compatibility

## Technical References

- **Z-Machine Specification**: Section 12.3 (Object Tables)
- **Commercial Implementation**: `./resources/test/zork1/DATA/ZORK1.DAT` analysis
- **Current Workaround**: `src/grue_compiler/codegen.rs:5869-5896` (temporary)
- **Target Fix Location**: `src/grue_compiler/ir.rs:1574` (property 18 generation)

---

**This analysis provides the definitive solution to replace the temporary hardcoded fix with a proper, standards-compliant object lookup system.**