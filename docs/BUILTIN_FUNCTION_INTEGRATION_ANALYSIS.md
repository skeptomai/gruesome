# Builtin Function Integration Analysis

## Problem Statement

Builtin functions (specifically `get_exit`) are being generated correctly in memory during compilation, but they are not being saved to the final Z-Machine output file. This causes runtime errors when the interpreter tries to execute the functions.

**Evidence:**
- Debug logs show function generation: `0x0034 → 0x009a (102 bytes)`
- Runtime calls work: `call routine at packed address 001a` succeeds
- File reality: Address `0x0034` contains all zeros in final output
- Runtime failure: `Invalid opcode 0x00 at address 0035`

## Investigation Required

### 1. Builtin Function Generation
- **Where**: Identify where `create_builtin_*` functions are called
- **When**: Determine the phase of compilation when builtins are generated
- **Storage**: Understand what memory space builtin functions use

### 2. Code Space Management
- **Main Code Space**: How regular functions are stored and managed
- **Builtin Code Space**: Whether builtins use separate memory space
- **Merging Process**: How different code spaces are combined

### 3. File Writing Process
- **Save Pipeline**: Trace the `save_game_state` or similar file writing process
- **Code Inclusion**: Verify what code spaces are included in final output
- **Address Mapping**: Ensure builtin function addresses are preserved

### 4. Integration Points
- **Generation Phase**: When builtin functions should be integrated
- **Address Resolution**: How function addresses are resolved and maintained
- **Memory Layout**: Final memory layout and file structure

## Root Cause Analysis ✅

**PROBLEM IDENTIFIED**: Builtin functions are generated correctly but to wrong memory space during Phase 2A.5

### Architecture Flow
1. **Phase 2A.5**: `generate_builtin_functions()` called at line 2212
   - This calls `create_builtin_get_exit()`
   - Function generates bytecode to `self.code_space`
   - Debug logs show function creation: `0x0033 → 0x009a (102 bytes)`

2. **Phase 2B**: Regular function code generation
   - User functions generated to same `self.code_space`
   - Overwrites builtin function space!

3. **Phase 3**: `assemble_complete_zmachine_image()`
   - Line 1233: `self.final_data[code_base..total_size].copy_from_slice(&self.code_space)`
   - Copies `code_space` to final image
   - But builtin functions already overwritten by regular functions

### Evidence Chain
- ✅ Builtin generation works: Debug logs show instructions emitted
- ✅ File assembly works: `code_space` copied to final image correctly
- ❌ Memory conflict: Regular functions overwrite builtin functions in `code_space`

### The Bug
**Phase 2A.5 writes builtins to `code_space[0x0033..0x009a]`**
**Phase 2B writes regular functions starting at `code_space[0x0000..]`**
→ **Regular functions overwrite builtin functions in shared `code_space`**

## Fix Plan

### Option A: Separate Builtin Space (Recommended)
1. Add `builtin_code_space: Vec<u8>` to store builtin functions separately
2. Modify `generate_builtin_functions()` to use builtin space
3. In `assemble_complete_zmachine_image()`, copy builtin space AFTER regular code space
4. Update address calculations to account for builtin space offset

### Option B: Reserve Builtin Section
1. Calculate builtin space requirements before regular function generation
2. Reserve space at end of `code_space` for builtins
3. Generate regular functions first, then builtins at reserved location
4. Ensure no overlap between regular and builtin function ranges

### Option C: Integration Before Regular Functions
1. Generate builtins first in `code_space`
2. Track builtin space end address
3. Start regular function generation after builtin space
4. Maintain proper address offset tracking