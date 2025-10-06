# Header Generation Architecture Analysis

## Executive Summary

The Z-Machine compiler currently has **two conflicting header generation paths** that cause corruption. The primary path generates correct headers with serial numbers and proper formatting, but a secondary "fixup" path overwrites good data with stale addresses. This analysis documents the current architecture problems and proposes a clean solution.

## Current Architecture Problems

### **Two Competing Header Generation Paths**

#### Path 1: `generate_complete_header()` - PRIMARY (✅ Mostly Good)
- **Location**: Called at line 1312 during Phase 3c
- **Function**: Writes comprehensive header directly to `final_data[0..64]`
- **✅ Correct**: Serial number "250905", version info, format compliance
- **❌ Problem**: Uses preliminary addresses that may be incorrect for post-assembly fields

#### Path 2: `fixup_final_header()` - SECONDARY (❌ Harmful)
- **Location**: Called at line 1482 during Phase 3e
- **Function**: Overwrites header fields with "final" addresses
- **❌ Major Problem**: Corrupts good header data with stale/wrong values
- **❌ Specific Issue**: Uses `self.dictionary_addr` which can be stale

### **The Bug That Was Fixed (Temporary Hack)**

**Problem Identified:**
```
Header after generate_complete_header():  [GOOD - has serial number "250905"]
Header after fixup_final_header():        [BAD - serial overwritten with 0x0000]

PC Address:         0x0b78
Dictionary Address: 0x0b78  <-- SAME VALUE (impossible!)
```

**Temporary Fix Applied:**
```rust
// Phase 3e: Final header fixup with correct addresses - DISABLED
// The generate_complete_header() already writes correct header values directly to final_data  
// This fixup was overwriting correct serial numbers and addresses with bad values
log::debug!("🔧 Step 3e: Final header fixup - SKIPPED (generate_complete_header already correct)");
```

**Result of Fix:**
- ✅ Serial number "250905" now preserved
- ✅ Dictionary and PC addresses are now different (0x0329 vs 0x0360)
- ✅ Dictionary size is now 4 bytes (was 0)
- ❌ This is a hack that leaves dead code

## Z-Machine Header Field Requirements

### **Fields That Can Be Set Early (Static)**
- **0x00**: Version number (3)
- **0x01**: Flags 1 (capabilities)
- **0x10-11**: Flags 2 (game preferences)
- **0x12-17**: Serial number (6 ASCII bytes, e.g., "250905")
- **0x1E-1F**: Interpreter number/version (for V4+)

### **Fields That MUST Be Fixed Up Post-Assembly**
- **0x06-07**: PC (initial program counter) - depends on final code placement
- **0x08-09**: Dictionary address - depends on final memory layout
- **0x0A-0B**: Object table address - depends on final memory layout
- **0x0C-0D**: Global variables address - depends on final memory layout
- **0x0E-0F**: Static memory base - boundary between dynamic/static
- **0x18-19**: Abbreviations address - depends on final memory layout
- **0x1A-1B**: File length - total assembled file size
- **0x1C-1D**: Checksum - calculated over entire final file

### **Current Memory Layout Calculation**
```
Header (64 bytes)           -> 0x0040
↓
Dynamic Memory:
├── Globals                 -> varies
├── Abbreviations          -> varies  
├── Objects                -> varies
└── Static Memory Boundary -> varies
↓
Static Memory:
├── Dictionary             -> varies
├── Strings                -> varies (V3: even-aligned, V4+: 4-byte aligned)
└── Code (High Memory)     -> varies
```

## Recommended Clean Architecture

### **Proposed Three-Phase Header Generation**

#### Phase 1: Static Header Fields
```rust
fn generate_static_header_fields(&mut self) -> Result<(), CompilerError> {
    // Write only fields that don't depend on final assembly
    // - Version, flags, serial number, interpreter info
    // - Leave address fields as 0x0000 placeholders
}
```

#### Phase 2: Address Field Fixup  
```rust
fn fixup_header_addresses(
    &mut self,
    pc_start: u16,
    dictionary_addr: u16,
    objects_addr: u16, 
    globals_addr: u16,
    static_memory_base: u16,
    abbreviations_addr: u16
) -> Result<(), CompilerError> {
    // Update ONLY the address fields with final calculated values
    // Never touch static fields like serial number
}
```

#### Phase 3: File Metadata Finalization
```rust
fn finalize_header_metadata(&mut self) -> Result<(), CompilerError> {
    // Calculate and write file length and checksum
    // These must be absolute last since they depend on complete file
}
```

### **Proposed Call Sequence**
```rust
// Phase 3c: Static header (version, serial, flags)
self.generate_static_header_fields()?;

// Phase 3d: Assemble all memory spaces to final positions
// ... (existing assembly code)

// Phase 3e: Update address fields with calculated final addresses
self.fixup_header_addresses(
    pc_start,
    final_dict_addr,
    final_obj_addr, 
    final_globals_addr,
    static_memory_start,
    final_abbrev_addr
)?;

// Phase 3f: File length and checksum (must be last)
self.finalize_header_metadata()?;
```

## Current Status After Temporary Fix

### **What's Working:**
- ✅ Serial number "250905" appears in header
- ✅ Dictionary and PC addresses are different 
- ✅ Dictionary space is properly allocated (4 bytes minimum)
- ✅ Our Gruesome interpreter runs compiled games successfully
- ✅ File format is much more Z-Machine specification compliant

### **What's Still Broken:**
- ❌ Frotz still rejects files with "Fatal error: Story file read error"
- ❌ Dead code in `fixup_final_header()` function
- ❌ Architecture debt from competing header generation paths
- ❌ Potential for future bugs if fixup is re-enabled

## Next Steps

### **Option A: Clean Architecture Implementation** (Recommended)
1. Implement three-phase header generation as proposed
2. Remove `fixup_final_header()` entirely
3. Test that all functionality is preserved
4. Investigate remaining Frotz compatibility issues

### **Option B: Debug Frotz Compatibility First**  
1. Keep current temporary hack
2. Focus on remaining format validation issues preventing Frotz compatibility
3. Clean up architecture after Frotz works

### **Remaining Frotz Investigation Needed**
Even with corrected headers, Frotz still fails. Potential issues:
- Dictionary table content/format validation
- Abbreviations table structure  
- Object table format compliance
- File length calculation errors
- Checksum validation failures
- Missing or malformed required sections

## Files Modified in This Investigation

- `src/grue_compiler/codegen.rs`: Line 1482 - Disabled `fixup_final_header()` call
- Added debug logging for dictionary size tracking
- Various temporary debug statements (should be cleaned up)

## Code Quality Notes

The current fix is a **temporary hack** that works but leaves architectural debt. The proper solution requires refactoring the header generation system to have clear separation of concerns and eliminate the competing code paths that caused this corruption in the first place.

## Complete Session Status and Next Session Plan

### **What Was Accomplished This Session:**
1. ✅ **Identified root cause**: Two competing header generation paths causing corruption
2. ✅ **Fixed dictionary address bug**: PC and Dictionary no longer have same address
3. ✅ **Fixed missing serial number**: "250905" now appears correctly in header
4. ✅ **Fixed zero dictionary size**: Dictionary space now properly allocated (4 bytes minimum)
5. ✅ **Verified our interpreter compatibility**: Compiled games still run in Gruesome
6. ✅ **Documented architecture debt**: Complete analysis of header generation problems

### **Current State:**
- **Header Format**: Much improved, closer to Z-Machine spec compliance
- **Our Interpreter**: ✅ Works perfectly with compiled games
- **Frotz Compatibility**: ❌ Still fails with "Fatal error: Story file read error"
- **Architecture**: Has temporary hack that needs cleanup

### **Remaining Work for Full Frotz Compatibility:**

#### **Option A: Clean Architecture First (Recommended for Long-term)**
1. **Implement Three-Phase Header Generation**:
   - `generate_static_header_fields()` - version, serial, flags only
   - `fixup_header_addresses()` - post-assembly address fields only
   - `finalize_header_metadata()` - file length and checksum last
2. **Remove Dead Code**: Delete `fixup_final_header()` entirely
3. **Test Preservation**: Ensure all existing functionality works
4. **Then investigate remaining Frotz issues**

#### **Option B: Frotz Debugging First (Faster to Frotz Goal)**
1. **Keep current temporary hack** (works but has debt)
2. **Investigate specific Frotz validation failures**:
   - Dictionary table content/format
   - Abbreviations table structure  
   - Object table format compliance
   - File length calculation
   - Checksum validation
   - Required section validation
3. **Clean up architecture after Frotz works**

### **Critical Note: Frotz Testing Protocol**
**⚠️ IMPORTANT**: Running `frotz` breaks terminal handling. Always:
1. **Ask user to run frotz manually** and report results
2. **Never run frotz automatically** in tool commands
3. Use other Z-Machine interpreters for automated testing if needed

### **Files Modified This Session:**
- `src/grue_compiler/codegen.rs`: 
  - Line 1482: Disabled harmful `fixup_final_header()` call
  - Added proper debug logging for memory layout
  - Various temporary debug statements (should be cleaned up)
- `HEADER_ARCHITECTURE_ANALYSIS.md`: Complete documentation

### **Specific Test Protocol:**
```bash
# To test current state:
cargo run --bin grue-compiler examples/mini_zork.grue
xxd -l 32 mini_zork.z3  # Check header format

# Our interpreter test:
cargo run --bin gruesome mini_zork.z3  # Should work

# Frotz test (USER RUNS MANUALLY):
frotz mini_zork.z3  # User reports if it works or fails
```

### **Success Metrics:**
- **Phase Complete**: When frotz runs our compiled Z-Machine files without errors
- **Architecture Clean**: When single clean header generation path exists
- **Regression Free**: When all existing compiler functionality preserved

### **Next Session Priorities:**
1. **Decision**: Clean architecture vs immediate Frotz debugging  
2. **Implementation**: Chosen approach above
3. **Testing**: Systematic validation of remaining format issues
4. **Cleanup**: Remove temporary hacks and debug code

The major breakthrough this session was identifying and fixing the header corruption chain. The compiler now generates much more spec-compliant Z-Machine files, but there are remaining validation issues preventing full Frotz compatibility.