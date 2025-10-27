# 🔧 OBJECT ITERATION SYSTEM: CRITICAL OPCODE BUG FIXED (October 27, 2025)

## 🎯 BREAKTHROUGH: Wrong Opcode Bug Found and Fixed

**CRITICAL DISCOVERY**: The get_object_contents function was emitting the **WRONG Z-Machine opcode**!
- **Bug**: Emitting opcode 0x01 (get_sibling) instead of 0x02 (get_child)
- **Impact**: get_object_contents was calling get_sibling instead of get_child
- **Result**: Always returned 0 because player object has no sibling

**COMPLETED FIX**: ✅ **Fixed opcode from 0x01 to 0x02 in get_object_contents builtin**
- **Location**: `src/grue_compiler/codegen_builtins.rs:812`
- **Change**: `0x01, // get_child opcode (1OP:1)` → `0x02, // get_child opcode (1OP:2)`

**VERIFICATION**: ✅ **get_object_contents now works correctly**
- **Empty container test**: ✅ Returns 0 correctly
- **With objects test**: ✅ Returns object 3 (coin) when coin inserted into player
- **Runtime evidence**: `insert_obj: obj=3, dest=1` followed by `value=0x0003 (3)` and `push 3`
- **Compiler evidence**: Now emits `opcode=0x02` instead of `opcode=0x01`

## 🔍 CURRENT INVESTIGATION: Stack Underflow in Print System (October 27, 2025)

### **Problem Statement**
The fundamental object iteration fix is working, but there's a **separate stack underflow issue**:
- `get_object_contents` correctly returns object 3 (coin)
- Object tree relationships properly established via insert_obj
- Stack underflow occurs at **print_paddr instruction at PC 0x07fc**
- Error: `print_paddr` trying to read from empty stack

### **Theory: String Concatenation Stack Management**
The stack underflow likely occurs in string concatenation/printing logic:
1. `get_object_contents` successfully returns object 3 and pushes to stack
2. String concatenation code (`"Player contents result: " + result`) may consume stack values incorrectly
3. Later `print_paddr` instruction expects a string address on stack but finds empty stack

**Evidence**:
- Test code: `print("Player contents returned: " + result);`
- Stack underflow at `print_paddr` (0x8d) instruction
- Bytecode sequence: `8d 03 a9` where 0x8d tries to read Variable(0) from empty stack

### **Investigation Plan**

**Phase 1: Isolate the Stack Underflow**
1. Create minimal test that triggers stack underflow without object iteration
2. Test string concatenation with simple integer values
3. Verify if issue is in string concatenation vs printing logic

**Phase 2: Trace Stack Operations**
1. Add stack depth logging around string concatenation operations
2. Track push/pull operations in string building
3. Identify where stack becomes empty when it shouldn't

**Phase 3: Fix Stack Management**
1. Ensure string concatenation preserves stack discipline
2. Fix any missing push operations or extra pop operations
3. Verify all Variable(0) reads have corresponding stack values

### **Diagnostic Commands**
```bash
# Test string concatenation without objects
cargo run --bin grue-compiler -- test_simple_string_concat.grue -o tests/string_test.z3
RUST_LOG=debug ./target/debug/gruesome tests/string_test.z3

# Track stack operations around the crash
RUST_LOG=debug ./target/debug/gruesome tests/test_debug_get_child.z3 2>&1 | grep -E "(push|pop|stack|0x07fc)" -A3 -B3
```

### **Success Criteria**
- String concatenation with runtime values works without stack underflow
- `print("text: " + variable)` completes successfully
- Object iteration tests run to completion showing actual object names
- All existing functionality remains working

---

## 📋 CURRENT STATUS SUMMARY

### ✅ **COMPLETED GOALS**
1. **Critical Opcode Bug**: ✅ get_object_contents now calls get_child (0x02) instead of get_sibling (0x01)
2. **Object Tree Traversal**: ✅ Successfully finds objects after insert_obj operations
3. **Empty Container Handling**: ✅ Returns 0 correctly for empty containers
4. **Fundamental Architecture**: ✅ Object iteration system architecture now correct

### 🎯 **ACTIVE INVESTIGATION**
**STACK UNDERFLOW IN PRINT SYSTEM**: Separate issue from object iteration bug
- **Priority**: MEDIUM - object iteration core functionality is working
- **Issue**: print_paddr instruction reading from empty stack during string operations
- **Impact**: Prevents completion of tests that should now work
- **Solution**: Fix stack management in string concatenation/printing pipeline

### 📊 **VERIFICATION METRICS**
- ✅ Object iteration opcode: Fixed (get_child=0x02 working correctly)
- ✅ Object tree population: Working (insert_obj operations successful)
- ✅ Object discovery: Working (get_child returns correct object IDs)
- 🔧 String printing: Stack underflow needs investigation
- ✅ Regression testing: All basic functionality preserved

---

**Historical fixes moved to**: `OBJECT_ITERATION_HISTORY.md`