# STRING ADDRESS ISSUE ANALYSIS

## PROBLEM IDENTIFIED (November 13, 2025)

**The compiler is generating invalid string addresses by using offsets that exceed the string space size.**

## SPECIFIC ISSUE

From debug logs of mini_zork.grue compilation:

```
STRING_PACKED_RESOLVE: String ID 1018 offset=0x079a + base=0x0afa = final=0x1294 → packed=0x094a
```

**Breaking this down**:
- String offset within string space: `0x079a` (1946 bytes)
- String base address in file: `0x0afa` (2810 bytes)
- **Calculated final address**: `0x1294` (4756 bytes)
- **Packed address**: `0x094a` (which unpacks to 4756 * 2 = **9512 bytes**)

## ROOT CAUSE

**The string offset `0x079a` (1946 bytes) appears to exceed the actual string space size.**

The calculation `final_address = string_base + offset` assumes that `offset` is a valid position within the string space, but if the offset is larger than the string space itself, the final address will point beyond the end of the file.

## INVESTIGATION NEEDED

1. **What is the actual size of the string space?** (need to log `string_space.len()`)
2. **Where do these large offsets come from?** (need to trace `string_offsets` map)
3. **Are string offsets being calculated correctly?** (need to check string allocation logic)

## LIKELY CAUSES

1. **String offset calculation error**: Offsets may be accumulating incorrectly during string allocation
2. **String space size miscalculation**: The string space may be smaller than expected
3. **Double-counting or incorrect base**: Some addresses might be getting added twice

## MEMORY LAYOUT EXPECTATION

```
Dictionary: 0x0796 + 868 bytes = ends at 0x0afa
Strings: should start at 0x0afa, but offsets within this space should be < string_space.len()
Code: starts at 0x1466
```

If string ID 1018 has offset 1946 bytes, and the string space is smaller than 1946 bytes, then we're pointing into the code section or beyond the file entirely.

## NEXT STEPS

1. Add logging to show actual `string_space.len()` vs. string offsets
2. Find where string offsets exceed string space boundaries
3. Fix the string offset calculation to ensure all offsets are within bounds