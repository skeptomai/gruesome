# NEXT SESSION PLAN - September 24, 2025 (Current Session - Stack Rearchitecture)

## 🎉 MAJOR PROGRESS ACHIEVED: Opcode Classification Fixed, Stack Issue Root Cause Identified

### ✅ **What Was Accomplished This Session**

**OPCODE CLASSIFICATION FIX COMPLETED**:
- **Critical Bug**: VAR/2OP opcode classification conflict in `is_true_var_opcode()` function
- **Root Cause**: Raw opcodes 0x00-0x1F were forced to VAR form, preventing 2OP:0 (je) for loop conditions
- **Solution**: Removed conflicting raw opcode mappings, kept only genuinely VAR-only opcodes (0x08, 0x09)
- **Result**: "Invalid Long form opcode 0x00" errors eliminated ✅

**STACK ARCHITECTURE ROOT CAUSE IDENTIFIED**:
- **Fundamental Issue**: Compiler violates Z-Machine stack specification
- **Problem**: Treats stack as random-access with "stack slots" when Z-Machine only supports LIFO via Variable(0)
- **Impact**: Stack underflow when multiple intermediate values try to access same Variable(0)
- **📋 Comprehensive Plan Created**: `STACK_REARCHITECTURE_PLAN.md`

### ✅ **Test Results After Opcode Fix**

**BEFORE FIX**:
- ❌ `bare_for_loop.grue` → Array index crash
- ❌ `minimal_for_loop.grue` → "Invalid Long form opcode 0x00"

**AFTER FIX**:
- 🔄 `bare_for_loop.grue` → Stack underflow at PC 0x038d (stack architecture issue)
- 🔄 `minimal_for_loop.grue` → Stack underflow at PC 0x08d5 (stack architecture issue)
- ✅ `basic_test.grue` → Continues to work correctly (no regressions)

**PROGRESS**: Opcode classification errors eliminated, underlying stack architecture issue exposed.

## 🎯 **CURRENT SESSION: Stack Architecture Implementation**

### **IMMEDIATE FOCUS: For-Loop Stack Redesign**

Following `STACK_REARCHITECTURE_PLAN.md` Phase 1 implementation:

#### **Step 1: Implement Local Variable Assignment Function**
```rust
// Add to src/grue_compiler/codegen.rs
pub fn use_local_var_for_result(&mut self, target_id: IrId) {
    let next_local = self.next_available_local_var();
    self.ir_id_to_local_var.insert(target_id, next_local);
}
```

#### **Step 2: Fix For-Loop Instructions**
Target these specific instructions in `src/grue_compiler/codegen_instructions.rs`:
- `IrInstruction::CreateArray` → use local variables instead of stack
- `IrInstruction::ArrayLength` → use local variables instead of stack
- `IrInstruction::GetArrayElement` → use local variables instead of stack

#### **Step 3: Test Incremental Progress**
```bash
# Primary test case
env RUST_LOG=error cargo run --bin grue-compiler -- examples/bare_for_loop.grue --output /tmp/stack_fix_test.z3
env RUST_LOG=error ./target/debug/gruesome /tmp/stack_fix_test.z3
# Expected: No stack underflow errors

# Regression test
env RUST_LOG=error ./target/debug/gruesome /tmp/debug_basic.z3
# Expected: Continue to work
```

## 📋 **IMPLEMENTATION ORDER (This Session)**

### **Priority 1: Core Infrastructure**
1. **Add `use_local_var_for_result()` function** to codegen.rs
2. **Add `next_available_local_var()` helper function**
3. **Test infrastructure with debug logging**

### **Priority 2: For-Loop Specific Changes**
1. **Fix `CreateArray` instruction** to use local variables
2. **Fix `ArrayLength` instruction** to use local variables
3. **Test `bare_for_loop.grue` after each change**

### **Priority 3: Validation**
1. **Ensure `basic_test.grue` still works** (regression check)
2. **Test `minimal_for_loop.grue`** (more complex case)
3. **Debug logging verification** (stack vs local usage)

## ✅ **SUCCESS CRITERIA FOR THIS SESSION**

**Phase 1 Goals**:
- ✅ Implement local variable assignment infrastructure
- ✅ Fix stack underflow in `bare_for_loop.grue`
- ✅ Maintain functionality of `basic_test.grue`
- ✅ Show clear debug logs of stack vs local variable usage

**Test Commands Ready**:
```bash
# Quick validation suite
env RUST_LOG=error cargo run --bin grue-compiler -- examples/bare_for_loop.grue --output /tmp/test1.z3
env RUST_LOG=error ./target/debug/gruesome /tmp/test1.z3  # Should not show "STACK UNDERFLOW"

env RUST_LOG=error cargo run --bin grue-compiler -- examples/basic_test.grue --output /tmp/test2.z3
env RUST_LOG=error ./target/debug/gruesome /tmp/test2.z3  # Should continue to work
```

## 🔄 **NEXT SESSIONS AFTER THIS ONE**

### **Phase 2**: Expand Beyond For-Loops
- Function call return values (must stay on stack per Z-Machine spec)
- Property access patterns
- Complex expression evaluation

### **Phase 3**: Optimization and Edge Cases
- Performance improvements
- Nested loops and complex control flow
- Error handling

## 🚨 **CRITICAL REMINDERS**

- **Stack architecture is the root cause** - opcode issues were symptoms
- **Z-Machine only supports LIFO stack** - no random access to "stack slots"
- **Local variables (1-15) are the correct solution** for persistent intermediate values
- **Function returns must stay on stack** per Z-Machine specification
- **Incremental testing is critical** - fix one instruction type at a time

The fundamental architectural problem has been identified and planned. Implementation begins now.