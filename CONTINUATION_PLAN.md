# Compiler Continuation Plan - Direct Code Emission Issue

## CURRENT STATUS - MAJOR SUCCESS ✅

### COMPLETED (Session Sep 12, 2025):
1. **SYSTEMATIC UnresolvedReference RESOLUTION FIXED** - Critical architectural fix completed
2. **IR ID Mapping Consolidation Working** - Added centralized consolidation system
3. **Total IR ID mappings: 13 → 237** (1800% increase)
4. **All 80 UnresolvedReferences resolve successfully**
5. **mini_zork executes through initialization** and displays game banner correctly

### KEY ARCHITECTURAL FIX:
**Problem**: Compiler used separate tracking systems for IR ID types:
- `string_offsets: HashMap<IrId, usize>` (strings)
- `label_addresses: HashMap<IrId, usize>` (labels)  
- `ir_id_to_object_number: HashMap<IrId, u16>` (objects)

But only `ir_id_to_address` was used for UnresolvedReference resolution, causing hundreds of "target_id not found" failures.

**Solution**: Added `consolidate_all_ir_mappings()` function that merges all tracking systems into central `ir_id_to_address` table before resolution.

**Location**: `src/grue_compiler/codegen.rs` lines ~7490-7558

## REMAINING ISSUE - Direct Code Emission ⚠️

### SYMPTOM:
mini_zork crashes at PC=0x1221 with branch overflow error `0xffffff25`

### ROOT CAUSE:
Problematic bytes at 0x1222-0x1223: `0x00, 0x2d` 
- These are NOT from failed UnresolvedReference resolution (that's fixed)
- These are from **direct code emission paths** that bypass the reference system
- Pattern suggests a `je` (jump-if-equal) instruction with malformed operands

### HEX ANALYSIS:
```
00001220: 0181 0000 002d 0000 0d00 00a1 0000 4200
          ---- ----  ----
          je   op1   op2
          
0x01 = je instruction (2OP:1)
0x81 = operand types (Large, Large)  
0x00 0x00 = operand 1 (problematic - should be object/variable reference)
0x00 0x2d = operand 2 (problematic - 0x2d=45 suggests IR ID 45, but wrong encoding)
```

### INVESTIGATION PLAN:

#### Phase 1: Trace Direct Code Emission
1. **Find je instruction generation** - Search for code that emits opcode 0x01
2. **Trace operand emission** - Find where `0x00, 0x2d` bytes are being written
3. **Check UnresolvedReference creation** - Verify if references are created for these operands

#### Phase 2: Fix Direct Emission Paths  
1. **Ensure operands use UnresolvedReference system** instead of direct byte emission
2. **Fix any bypassed reference creation** in je instruction generation
3. **Verify proper Z-Machine operand encoding** (Large constants, variables, etc.)

#### Phase 3: Test Complete Execution
1. **Verify mini_zork reaches input loop** without crashes
2. **Test basic game commands** (look, north, take, etc.)
3. **Run full compiler test suite** to ensure no regressions

### DEBUG COMMANDS:

```bash
# Search for je instruction generation
grep -r "0x01\|je.*instruction" src/grue_compiler/

# Find operand emission around problematic area
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep -A10 -B10 "0x1220"

# Test execution with detailed logging
echo "look" | RUST_LOG=debug cargo run --bin gruesome -- mini_zork.z3 2>&1 | head -50

# Check for direct byte emission bypassing references
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue 2>&1 | grep -E "(emit_byte.*0x00|emit_byte.*0x2d)"
```

### KEY FILES TO INVESTIGATE:

1. **`src/grue_compiler/codegen_instructions.rs`** - Instruction emission logic
   - Look for je (2OP:1) instruction handling
   - Check operand encoding for object references

2. **`src/grue_compiler/codegen.rs`** - Core code generation
   - Search for direct emit_byte calls around comparison operations
   - Check emit_operand functions

3. **`src/grue_compiler/ir.rs`** - IR generation  
   - Look for PropertyAccess or comparison operations that might generate je instructions

### SUCCESS METRICS:
- ✅ UnresolvedReference resolution working (237 mappings)
- ⚠️ Direct code emission issue (PC 0x1221 crash)
- 🎯 **TARGET**: mini_zork executes to input loop without crashes

### ARCHITECTURAL NOTES:
- Keep separated memory spaces architecture (Header, Globals, Objects, Dictionary, Strings, Code)
- Preserve centralized IR ID mapping consolidation system
- Focus on instruction-level code emission, not memory layout

---
*Last Updated: September 12, 2025*  
*Context: Major IR ID mapping consolidation success - systematic UnresolvedReference failures resolved*