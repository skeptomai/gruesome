# Next Session Roadmap - Reference Resolution & Call Handling

## 🎉 **Major Progress This Session**

### ✅ **BREAKTHROUGH: PC Calculation Fix**
**Root Cause Found**: PC pointed to function header instead of first instruction
- **Before**: PC = 0x03a2 (function header: 0x00 local count)  
- **After**: PC = 0x03a3 (first instruction: 0x8d opcode)
- **Fix**: Added function header offset (+1) to PC calculation when no init block
- **Expected Impact**: Should resolve "Invalid opcode 0x00" in 4-5 tests

### 🔍 **ROOT CAUSE IDENTIFIED: Dual Call Handling**
**Problem**: Two different paths handle Call instructions:
1. **Main codegen** `translate_call()` - has correct builtin handling + get_location fix
2. **Extracted instructions** `generate_instruction()` - has builtin detection but seems ineffective

**Result**: Function 9 (`get_location`) not detected as builtin → generates wrong opcode → sread error

## 🎯 **Next Session Priority Tasks**

### **Task 1: Fix Builtin Function Detection (30-45 minutes)**

**Issue**: Function 9 (`get_location`) not recognized as builtin in Call handling

**Debug Approach**:
1. **Check builtin registration**:
   ```bash
   RUST_LOG=debug ./target/release/grue-compiler test_property_simple.grue 2>&1 | grep -E "(register.*function.*9|function.*9.*get_location)"
   ```

2. **Check is_builtin_function(9)**:
   ```bash
   # Add debug logging to is_builtin_function() method
   # Verify builtin_function_names HashMap contents
   ```

3. **Fix registration timing**: Ensure builtin registration happens before Call translation

**Expected Fix**: Consolidate Call handling or fix builtin function ID mapping

### **Task 2: Verify PC Fix Impact (15-30 minutes)**

**Test the PC calculation fix across all failing tests**:
```bash
for file in test_property_simple test_array_compilation test_array_errors test_array_ops test_random test_variables; do
    echo "Testing ${file}.z3..."
    # Recompile with fixed PC calculation
    ./target/release/grue-compiler src/grue_compiler/tests/${file}.grue
    # Test execution  
    timeout 5 bash -c "echo '' | cargo run --bin gruesome -- ${file}.z3" 2>&1 | head -5
done
```

**Expected**: 4-5 tests should now pass or show different errors

### **Task 3: Address Remaining Issues (30+ minutes)**

**After PC+builtin fixes, remaining failures likely**:
- Memory bounds issues (test_variables.z3)
- Complex reference resolution edge cases

## 📊 **Success Metrics**

**Current**: 31 passing / 6 failing (84% success)
**Target**: 35+ passing / 2 failing (95% success)  
**Stretch**: 36+ passing (98% success - near perfection!)

## 🔧 **Technical Details**

### **Files Modified This Session**:
- `src/grue_compiler/codegen.rs` line 1020: PC calculation fix
- Documentation: SESSION_SUMMARY.md, NEXT_SESSION_ROADMAP.md

### **Architecture Insight**:
The compiler has **dual code paths** for Call instructions:
- **Main path**: `translate_call()` - comprehensive, has all fixes
- **Extracted path**: `generate_instruction()` - newer, but missing context

**Resolution**: Either consolidate paths or ensure builtin context propagation.

## 🚀 **Session Outlook**

**High Confidence**: PC fix should dramatically improve test pass rate
**Medium Effort**: Builtin detection fix is straightforward once root cause confirmed  
**High Impact**: Could achieve 95%+ success rate in next session

**The compiler is fundamentally sound** - these are final integration and edge case issues, not architectural problems.