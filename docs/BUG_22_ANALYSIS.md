# Bug #22: Corrupted Object Property Table Addresses - Analysis & Fix Plan

## Problem Statement

When running mini_zork.z3, the interpreter reports 90+ "BOUNDS ERROR: prop_table_addr >= file size" errors at startup. Property table addresses point far beyond the file boundaries (0x2707, 0xe621, 0x94a5, etc.) instead of valid addresses (~0x0400-0x0600 range).

## Investigation Timeline

### Phase 1: Initial Discovery (Oct 15, 2025 23:29)
- **Comprehensive gameplay test** revealed systematic corruption
- **ALL objects** have invalid property table pointers
- Game navigation works, but examining ANY object shows garbled text
- Inventory display completely corrupted

### Phase 2: Compilation Analysis (Oct 15, 2025 23:32)
- Property tables created correctly:
  ```
  obj#1 'yourself': prop_table=0x00ce
  obj#2 'West of House': prop_table=0x00d9
  ...
  obj#15 'egg': prop_table=0x0219
  ```
- Patching appears correct:
  ```
  obj#1: space_relative=0x00ce → absolute=0x048a
  obj#2: space_relative=0x00d9 → absolute=0x0495
  ...
  ```

### Phase 3: Hex Dump Analysis (Oct 15, 2025 23:30-23:35)

**Header**:
- Object table address (bytes 0x0A-0x0B): `03 bc` = 0x03bc ✓ Correct

**Expected Layout**:
```
0x03bc: Property defaults (62 bytes, 0x3e)
0x03fa: First object entry (player)
```

**Actual Layout** (from hex dump):
```
0x03bc-0x03f9: Property defaults (all zeros) ✓ Correct
0x03fa-0x03ff: SIX ZERO BYTES (unexpected gap!)
0x0400-0x0487: Object entries start HERE (6 bytes late!)
```

**Object Entry Analysis** (at 0x0400):
```
00000400: 00 04 8a 00 00 00 00 00 00 04 95 00 00 00 00
          ↑attributes↑ ↑P ↑S ↑C ↑prop_addr
```

Breaking down object #1 (V3 format: 4+1+1+1+2 = 9 bytes):
- Bytes 0x0400-0x0403: Attributes = `00 04 8a 00`  ← SUSPICIOUS!
- Byte  0x0404: Parent = `00`  ✓
- Byte  0x0405: Sibling = `00`  ✓
- Byte  0x0406: Child = `00`  ✓
- Bytes 0x0407-0x0408: Property table pointer = `04 95` = 0x0495  ✓ CORRECT!

**Pattern Recognition**:
- Property table pointers ARE correct after patching (0x0495, 0x04b9, 0x04d6...)
- But there's a **6-byte offset** - objects should start at 0x03fa, they start at 0x0400
- This offset causes interpreter to read attribute bytes as property table pointers!

## Root Cause Hypothesis

**MISALIGNMENT IN OBJECT SPACE COPY**

When `object_space` is copied to `final_data`, there may be:
1. **Padding/alignment** added that shifts everything by 6 bytes
2. **Incorrect base address** calculation when copying object_space
3. **Header size mismatch** between what's written and what's expected

The fix correctly patches the property table pointers WITHIN the object entries, but the **object entries themselves** are 6 bytes offset from where the header says they should be.

## Debugging Plan

### Step 1: Trace Object Space Copy
**Goal**: Find where the 6-byte offset is introduced

**Actions**:
1. Add logging to `copy_object_space_to_final()` (or equivalent)
2. Log:
   - Source: `object_space.len()`, first 20 bytes
   - Destination: `final_data` offset where copied, first 20 bytes after copy
   - Expected object table address vs actual
3. Check if there's any padding or header bytes inserted

**Expected findings**:
- Object table header says 0x03bc
- Object space should be copied to start at 0x03bc in final_data
- Objects should start at 0x03bc + 0x3e = 0x03fa
- **Actual**: Objects appear at 0x0400 (6 bytes late)

### Step 2: Check Header Generation
**Goal**: Verify header object table pointer matches actual data location

**Actions**:
1. Log when header is written:
   - Object table address written to header
   - Where in final_data object_space is actually copied
2. Check for off-by-N errors in address calculation

### Step 3: Examine Object Space Content
**Goal**: Verify object_space has correct layout BEFORE copy

**Actions**:
1. Dump first 100 bytes of `object_space` after generation:
   ```rust
   log::error!("🔍 OBJECT_SPACE dump: {:02x?}", &self.object_space[0..100]);
   ```
2. Verify:
   - Bytes 0x00-0x3d: Property defaults
   - Bytes 0x3e onwards: Object entries
   - No unexpected padding

### Step 4: Compare Expected vs Actual
**Goal**: Create side-by-side comparison of what SHOULD be vs what IS

**Expected** (object_space layout):
```
0x00-0x3d: Property defaults (62 bytes)
0x3e-0x46: Object #1 entry (9 bytes)
  - 0x3e-0x41: Attributes (4 bytes)
  - 0x42: Parent
  - 0x43: Sibling
  - 0x44: Child
  - 0x45-0x46: Property table pointer (space-relative, e.g., 0x00ce)
```

**Actual** (from hex dump at final address 0x03fa):
```
0x03fa-0x03ff: SIX ZERO BYTES ← BUG!
0x0400-0x0408: Object #1 entry (9 bytes)
  - 0x0400-0x0403: Attributes
  - 0x0404: Parent
  - 0x0405: Sibling
  - 0x0406: Child
  - 0x0407-0x0408: Property table pointer (absolute, 0x0495)
```

### Step 5: Find the Code Responsible
**Goal**: Locate exactly where object_space gets copied with wrong offset

**Candidate locations**:
1. `finalize()` or similar final assembly method
2. `copy_object_space()` or similar method
3. Header generation that writes object table address

**Search commands**:
```bash
grep -r "object_space.*final_data" src/
grep -r "copy.*object" src/ | grep -i space
grep -r "object_table_addr" src/ | grep -i write
```

## Fix Strategy

Once we find where the 6-byte offset is introduced:

**Option A: Fix the copy offset**
- If object_space is being copied 6 bytes too late, adjust the offset

**Option B: Fix the header address**
- If header is written with wrong address, add 6 to match actual location

**Option C: Fix object_space generation**
- If object_space has 6 bytes of unexpected padding at start, remove it

## Success Criteria

After fix:
1. ✅ Object table header points to correct address
2. ✅ Objects start immediately after property defaults (no gap)
3. ✅ Property table pointers are correct
4. ✅ No BOUNDS ERROR messages at startup
5. ✅ "examine mailbox" shows proper description
6. ✅ Inventory display shows proper item names
7. ✅ All 183 tests pass

## Files to Examine

Priority order:
1. `src/grue_compiler/codegen.rs` - `patch_property_table_addresses()`, finalize methods
2. `src/grue_compiler/codegen_objects.rs` - object space generation
3. Header writing code (wherever object table address is written to bytes 0x0A-0x0B)

## Key Insight

**The property table patching is CORRECT!** The bug is NOT in `patch_property_table_addresses()`.

The bug is in the **ALIGNMENT** - object entries are 6 bytes offset from where the header says they should be. When the interpreter reads object #1 at the address in the header (0x03fa), it gets 6 zero bytes + the first 3 bytes of object #1's attributes. This garbage gets interpreted as the property table pointer, causing the BOUNDS ERROR.
