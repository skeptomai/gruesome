# TXD SECOND COMPLIANCE ISSUE ANALYSIS (November 13, 2025)

## NEW ISSUE DISCOVERED

**After fixing the dictionary encoding issue, TXD still crashes on a different invalid packed address: `0x6468`**

## TXD ERROR DETAILS

```
*** ACCESSING address 0xc8d1 which is in page 100 (>= 13) ***
*** This would be UNPACKED from packed address 0x6468
*** Called from read_data_byte() at final address 0xc8d1
```

## TECHNICAL ANALYSIS

### 1. **Address Calculation**
- **Packed address**: `0x6468` (25704)
- **Unpacked address**: `0x6468 * 2 = 0xc8d0` (51408 bytes)
- **File size**: 8550 bytes
- **Exceeds by**: 42,858 bytes (5x the file size!)

### 2. **Address Source Investigation**
- **Direct search**: `0x6468` does NOT appear as a literal 16-bit value in the file
- **This means**: The address is being **calculated/computed** by some data structure
- **Likely sources**: Property values, string table calculations, or object data

### 3. **Context Difference from Previous Issue**
- **Previous**: `0x4a52` from systematic dictionary pattern `14a5 94a5 8000` (101 identical entries)
- **Current**: `0x6468` from computed/calculated address (not literal pattern)

## INVESTIGATION NEEDED

### 1. **Property Table Analysis**
Object properties might contain values that TXD interprets as packed addresses:
- Property values could be getting calculated incorrectly
- Property types might be misidentified

### 2. **String Table Address Calculation**
String addresses might be computed incorrectly:
- String offsets + base addresses
- String space boundary violations

### 3. **Object Relationships**
Object parent/child/sibling pointers might be invalid:
- Object numbers exceeding actual object count
- Circular references in object tree

## COMPARISON WITH WORKING FILE

Should compare with known-good Z-Machine file to understand:
1. What data structures TXD scans that could generate computed addresses
2. How property values should be encoded to avoid misinterpretation
3. Whether string table layout has systematic issues

## NEXT STEPS

1. **Compare memory layout** with original mini_zork_debug.z3 to see what changed
2. **Identify specific TXD scan pattern** that encounters the computed `0x6468`
3. **Trace object/property/string data** that could generate this calculated address
4. **Fix the computation source** (likely in object/property generation)

## STATUS

**ROOT CAUSE**: Unknown - requires deeper investigation into computed address generation
**IMPACT**: TXD compliance still failing despite dictionary fix
**PRIORITY**: High - affects professional tool compatibility