# Property Encoding V3 Compliance Fix

## Problem Analysis

### Z-Machine V3 Specification (Section 12.4.1)
- Properties are limited to **1-8 bytes maximum**
- Single size byte format only: `32 * (size - 1) + property_number`
- NO two-byte size format exists in V3
- Property list terminated by size byte of 0

### V4+ Specification (Section 12.4.2)
- Properties up to 64 bytes allowed
- Can use either single-byte or two-byte size format
- Two-byte format: first byte has bit 7=1, second byte gives length

### Root Cause: Long String Properties
Mini Zork source contains very long string descriptions:
- **Leaflet desc**: 188 characters: `"\"WELCOME TO DORK!\n\nDORK is a game of adventure, danger, and low cunning. In it you will explore some of the most amazing territory ever seen by mortals. No computer should be without one!\""`
- **Egg desc**: 171 characters: `"The egg is covered with fine gold inlay, and ornamented in lapis lazuli and mother-of-pearl. Unlike most eggs, this one is hinged and closed with a delicate looking clasp."`

### Current Compiler Violations

File: `/Users/cb/Projects/infocom-testing-old/infocom/src/grue_compiler/codegen_strings.rs:996-1020`

**Violations:**
1. **Lines 996-1000**: Uses V4+ two-byte format when size > 8 (this should never happen in V3)
2. **Lines 1012-1020**: Uses V4+ two-byte format when size > 4 (should use V3 single-byte format for sizes 1-8)
3. **Line 1014**: Wrong formula `32 * (size - 1) + prop_num` only handles sizes 1-4

**Current Wrong Logic:**
```rust
if size > 8 {
    // Two-byte format required - WRONG! V3 doesn't support this
    let size_byte = 0x80 | prop_num;
    return (size_byte, data, string_id_opt, Some(size));
}

if size <= 4 {
    // Single-byte format - Only handles sizes 1-4
    let size_byte = 32 * (size - 1) + prop_num;
    (size_byte, data, string_id_opt, None)
} else {
    // Two-byte format (size > 4) - WRONG! Should be single-byte for sizes 5-8
    let size_byte = 0x80 | prop_num;
    (size_byte, data, string_id_opt, Some(size))
}
```

### Correct V3 Implementation

**String Properties Handled Correctly**: The compiler correctly stores strings as 2-byte packed addresses pointing to string data in high memory. This means string properties are always 2 bytes, well within V3's 8-byte limit.

**V3 Formula Works for All Sizes 1-8**:
```
size_byte = 32 * (size - 1) + property_number
```

**Examples:**
- Size 1: `32 * 0 + prop_num = prop_num`
- Size 2: `32 * 1 + prop_num = 32 + prop_num`
- Size 8: `32 * 7 + prop_num = 224 + prop_num`

Since property numbers are 1-31, the maximum size byte would be `224 + 31 = 255`, which fits in a single byte.

### Required Fix

**Remove all V4+ two-byte format logic** and use V3's single-byte format for all property sizes 1-8.

**New Logic:**
```rust
if size == 0 {
    // Empty property - use 1 byte minimum
    let size_byte = prop_num; // bits 6-5 = 00 means 1 byte
    return (size_byte, vec![0], None, None);
}

if size > 8 {
    // ERROR: V3 doesn't support properties > 8 bytes
    panic!("Property size {} exceeds V3 maximum of 8 bytes", size);
}

// V3 single-byte format for all sizes 1-8
let size_byte = 32 * (size - 1) + prop_num;
(size_byte, data, string_id_opt, None)
```

### Testing Plan

1. Verify commercial Zork I still works after fix
2. Verify Mini Zork text corruption is resolved
3. Check that all properties in compiled game use single-byte format
4. Confirm no properties exceed 8 bytes (they shouldn't with packed addresses)

## Implementation Status

- [x] Analysis completed
- [x] Fix implemented - Removed all V4+ two-byte format logic
- [x] Commercial Zork I tested - ✅ PASSES (perfect compatibility maintained)
- [x] Mini Zork tested - ✅ TEXT CORRUPTION FIXED (banner displays correctly)
- [x] Property size verification completed - No properties exceeded 8 bytes

## Results

**SUCCESS**: The property encoding fix resolved the text corruption issue while maintaining full commercial game compatibility.

**Fixed Issues:**
- Banner text now displays correctly: "DORK I: The Last Great Empire"
- No more garbled characters in compiled Mini Zork text
- All properties stay within V3's 8-byte limit
- Commercial Zork I continues working perfectly

**Separate Issue Identified:**
- Mini Zork has an infinite loop in object listing (unrelated to property encoding)
- This appears to be a different bug in object iteration/listing logic