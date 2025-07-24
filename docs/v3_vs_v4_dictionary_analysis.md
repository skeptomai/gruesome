# V3 vs V4+ Dictionary Implementation Analysis

## Overview

The Z-Machine interpreter now has completely separate dictionary implementations for v3 and v4+ games. This separation was necessary because the dictionary format differs significantly between these versions according to the Z-Machine specification.

## Separate Encoding Functions

### V3 Encoding (`encode_word_v3()`)
- **Character Limit**: 6 Z-characters maximum
- **Output Format**: 2 words (4 bytes total)
- **Padding**: Filled with Z-character 5 (space) to reach 6 characters
- **End Marker**: End-of-string bit set on word2 (0x8000)

### V4+ Encoding (`encode_word_v4_plus()`)
- **Character Limit**: 9 Z-characters maximum  
- **Output Format**: 3 words (6 bytes total)
- **Padding**: Filled with Z-character 5 (space) to reach 9 characters
- **End Marker**: End-of-string bit set on word3 (0x8000)

## Separate Lookup Methods

### V3 Dictionary Lookup (`lookup_dictionary_v3()`)
- **Entry Size**: 4 bytes for encoded text + additional data bytes
- **Comparison**: 32-bit comparison (2 words)
- **Type Byte Location**: Address + 4 (after 4-byte encoded text)
- **Search Method**: Binary search on 32-bit encoded values

### V4+ Dictionary Lookup (`lookup_dictionary_v4_plus()`)
- **Entry Size**: 6 bytes for encoded text + additional data bytes  
- **Comparison**: 48-bit lexicographic comparison (3 words)
- **Type Byte Location**: Address + 6 (after 6-byte encoded text)
- **Search Method**: Binary search on 48-bit encoded values using lexicographic ordering

## Version-Aware Dispatcher

The main `lookup_dictionary()` method acts as a dispatcher:

```rust
pub fn lookup_dictionary(&self, word: &str) -> u16 {
    if self.game.header.version <= 3 {
        self.lookup_dictionary_v3(word)
    } else {
        self.lookup_dictionary_v4_plus(word)
    }
}
```

## Key Technical Differences

### 1. Entry Length Handling
- **V3**: Reads dictionary entries as 4-byte encoded text + data
- **V4+**: Reads dictionary entries as 6-byte encoded text + data

### 2. Character Encoding Capacity
- **V3**: Encodes maximum 6 characters per word (sufficient for most English words)
- **V4+**: Encodes maximum 9 characters per word (handles longer words better)

### 3. Binary Search Comparison
- **V3**: Simple 32-bit integer comparison on two 16-bit words
- **V4+**: Lexicographic comparison across three 16-bit words for proper 48-bit ordering

### 4. Data Access Offsets
- **V3**: Type byte and additional data start at offset +4 from entry address
- **V4+**: Type byte and additional data start at offset +6 from entry address

## Benefits of Separation

### 1. Specification Compliance
- Each implementation follows its version's exact Z-Machine specification
- No cross-version compatibility issues or workarounds needed

### 2. Debugging Clarity
- Clear separation makes it easy to debug version-specific dictionary issues
- Separate debug logging for each version shows format-specific details

### 3. Performance
- No runtime branching for format differences within lookup loops
- Optimized comparison logic for each format

### 4. Maintainability
- Changes to one version don't affect the other
- Easy to test and validate each implementation independently

## Testing Results

### Before Separation (V3-only implementation)
- **AMFV (v4)**: PPCC command not recognized - wrong dictionary encoding
- **Zork I (v3)**: Working correctly with v3 encoding

### After Separation (Version-aware implementation)
- **AMFV (v4)**: PPCC command properly recognized with v4+ encoding
- **Zork I (v3)**: Still working correctly with v3 encoding
- **Both versions**: Full gameplay compatibility achieved

## Implementation Files

### Current Implementation Location
- **File**: `src/dictionary.rs`
- **Functions**: 
  - `encode_word_v3()` - V3 encoding
  - `encode_word_v4_plus()` - V4+ encoding  
  - `lookup_dictionary_v3()` - V3 lookup
  - `lookup_dictionary_v4_plus()` - V4+ lookup
  - `lookup_dictionary()` - Version dispatcher

### Clean Reference Implementation
- **File**: `src/dictionary_clean.rs` (reference implementation)
- **Status**: Not currently integrated, available for comparison

## Z-Machine Specification References

### V3 Dictionary Format (Section 13.3)
- Encoded text: 4 bytes containing 6 Z-characters
- Always padded with Z-character 5's to make 4 bytes
- Entry length must be at least 4 bytes

### V4+ Dictionary Format (Section 13.4)  
- Encoded text: 6 bytes containing 9 Z-characters
- Always contains exactly 9 Z-characters
- Entry length must be at least 6 bytes

### Ordering Requirement (Section 13.5)
- Entries must be in numerical order of encoded text
- Treated as 32-bit (v3) or 48-bit (v4+) binary numbers
- Most-significant byte first ordering