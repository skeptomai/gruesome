# Z-Machine Stack Architecture Redesign Plan
*Created: September 24, 2025*

## Current Problem Analysis

**Fundamental Issue**: Our compiler violates Z-Machine stack specification by treating it as random-access storage when it's strictly LIFO (Last-In-First-Out).

### Z-Machine Stack Specification Compliance
Based on `/Users/cb/Projects/Z-Machine-Standard/sect06.html`:

- **Variable(0) = stack pointer**: Writing pushes, reading pulls (LIFO only)
- **No random access**: Stack can only be accessed at the top
- **Stack per routine**: Empty at start/end of each routine
- **Critical exception**: Indirect variable references access stack in-place (dangerous)

### What We Do Wrong:
```rust
// Multiple IR values assigned to different "depths"
use_stack_for_result(ir_13) → stack_depth = 2
use_stack_for_result(ir_14) → stack_depth = 3

// But all resolve to same Variable(0)
resolve_ir_id_to_operand(ir_13) → Variable(0) // First pop
resolve_ir_id_to_operand(ir_14) → Variable(0) // Second pop → UNDERFLOW!
```

### Symptoms in For-Loops:
```grue
for i in [1, 2] {
    // 1. Create array [1, 2] → pushes array address to stack
    // 2. Get array length → needs array address, pushes length to stack
    // 3. Compare i < length → needs both i and length from stack
    // PROBLEM: Both resolve to Variable(0) but stack is LIFO!
}
```

## Redesign Strategy

### **Phase 1: Immediate Expression Evaluation**
**Principle**: Use stack only for immediate consumption, never for persistent storage

#### Current Pattern (BROKEN):
```grue
for i in [1, 2] {
    // Creates: array → stack, length → stack, both try to read Variable(0)
}
```

#### Fixed Pattern:
```grue
for i in [1, 2] {
    // Creates: array → local_var_1, length → local_var_2, comparison uses locals
}
```

### **Phase 2: Stack Usage Classification**

Redesign `use_stack_for_result()` to be more selective:

#### ✅ **USE STACK FOR**:
1. **Function return values** (Z-Machine specification requirement)
2. **Single-use expression results** consumed immediately
3. **Push/pull operations** with explicit 0xE8/0xE9 instructions
4. **Temporary calculations** where result is used once then discarded

#### ❌ **USE LOCAL VARIABLES FOR**:
1. **Loop variables and counters**
2. **Array references that persist across operations**
3. **Intermediate values needed multiple times**
4. **Complex expression operands that must coexist**
5. **Any value that needs to survive while other stack operations occur**

### **Phase 3: Implementation Plan**

#### **Step 1: Audit Current Stack Usage**
```bash
# Find all use_stack_for_result() calls
grep -r "use_stack_for_result" src/grue_compiler/
# Classify each usage as "immediate" vs "persistent"
# Priority order: for-loops first, then function calls, then property access
```

#### **Step 2: Create Smart Assignment Function**
```rust
#[derive(Debug, Clone, Copy)]
pub enum UsagePattern {
    ImmediateConsumption,  // Use stack - consumed by next instruction
    PersistentValue,       // Use local var - needed across multiple operations
    FunctionReturn,        // Use stack - Z-Machine requirement
}

pub fn assign_result_storage(&mut self, target_id: IrId, usage_pattern: UsagePattern) {
    match usage_pattern {
        UsagePattern::ImmediateConsumption => self.use_stack_for_result(target_id),
        UsagePattern::PersistentValue => self.use_local_var_for_result(target_id),
        UsagePattern::FunctionReturn => self.use_stack_for_result(target_id), // Z-Machine requirement
    }
}
```

#### **Step 3: Fix For-Loop Code Generation**
```rust
// OLD (broken): Everything goes to stack
array_creation → use_stack_for_result()      // Multiple values on stack!
array_length → use_stack_for_result()        // Stack depth increases!
index_comparison → use_stack_for_result()    // All resolve to Variable(0)!

// NEW (correct): Persistent values use locals
array_creation → assign_result_storage(target, PersistentValue)     // → local var
array_length → assign_result_storage(target, PersistentValue)       // → local var
index_comparison → assign_result_storage(target, ImmediateConsumption) // → stack (used once)
```

#### **Step 4: Implement Local Variable Allocation**
```rust
pub fn use_local_var_for_result(&mut self, target_id: IrId) {
    let next_local = self.next_available_local_var();
    self.ir_id_to_local_var.insert(target_id, next_local);
    log::debug!("use_local_var_for_result: IR ID {} -> Variable({})", target_id, next_local);
}

fn next_available_local_var(&mut self) -> u8 {
    // Variables 1-15 are local variables in Z-Machine
    // Variable 0 = stack, Variables 16+ = globals
    for var_num in 1..=15 {
        if !self.ir_id_to_local_var.values().any(|&v| v == var_num) {
            return var_num;
        }
    }
    panic!("Ran out of local variables (max 15 per routine in Z-Machine)");
}
```

#### **Step 5: Update Resolve Function**
```rust
// Current resolve_ir_id_to_operand already handles both cases correctly:
// - ir_id_to_stack_var.get(&ir_id) → Variable(0)
// - ir_id_to_local_var.get(&ir_id) → Variable(N)
// No changes needed to resolution logic, only to assignment logic
```

### **Phase 4: Testing Strategy**

#### **Progression Testing** (in this exact order):
1. **Step 1**: Fix `bare_for_loop.grue` (simplest case - just loop construct)
2. **Step 2**: Fix `minimal_for_loop.grue` (moderate complexity - has objects/functions)
3. **Step 3**: Ensure `basic_test.grue` still works (regression check - no for-loops)
4. **Step 4**: Test `mini_zork.grue` (complex case - full game logic)

#### **Validation Commands**:
```bash
# Compile and test each step
env RUST_LOG=error cargo run --bin grue-compiler -- examples/bare_for_loop.grue --output /tmp/test_step1.z3
env RUST_LOG=error ./target/debug/gruesome /tmp/test_step1.z3

# Debug stack vs local variable usage
env RUST_LOG=debug cargo run --bin grue-compiler -- examples/bare_for_loop.grue --output /tmp/debug.z3 2>&1 | grep -E "(stack|Variable|local)"

# Expected: No more "STACK UNDERFLOW" errors
# Expected: Debug logs show proper local variable assignments
```

#### **Success Criteria**:
- ✅ `bare_for_loop.grue` executes without stack underflow
- ✅ `minimal_for_loop.grue` executes basic loop iteration
- ✅ `basic_test.grue` continues to work (no regression)
- ✅ Debug logs show appropriate stack vs local variable usage

## Implementation Priority

### **PHASE 1 - IMMEDIATE (This Session)**:
Focus exclusively on for-loop constructs:
- `IrInstruction::CreateArray` → use local variables
- `IrInstruction::ArrayLength` → use local variables
- `IrInstruction::GetArrayElement` → use local variables
- Binary comparisons in loop conditions → evaluate immediately

### **PHASE 2 - NEXT SESSION**:
- Function call return values (must remain on stack per Z-Machine spec)
- Property access patterns
- Complex expression evaluation

### **PHASE 3 - FUTURE SESSIONS**:
- Performance optimization
- Advanced control flow (nested loops, complex conditionals)
- Error handling and edge cases

## Expected Benefits

### **Immediate Improvements**:
- ✅ Stack underflow errors eliminated in for-loops
- ✅ Complex expressions work correctly
- ✅ Z-Machine specification compliance
- ✅ Foundation for more complex language features

### **Long-term Improvements**:
- 🔄 More efficient code generation (fewer stack operations)
- 🔄 Better debugging (clearer variable lifetime tracking)
- 🔄 Foundation for advanced features (nested loops, complex expressions)
- 🔄 Proper separation of concerns (stack for flow, locals for storage)

## Risk Assessment

### **Low Risk Changes**:
- Local variable allocation (Variables 1-15 are well-defined in Z-Machine spec)
- Stack usage classification (improves existing logic)
- For-loop specific changes (isolated scope)

### **Medium Risk Changes**:
- Modifying `use_stack_for_result()` usage patterns across codebase
- Changing expression evaluation order

### **High Risk Changes**:
- Function call return value handling (must remain Z-Machine compliant)
- Global changes to expression evaluation

### **Mitigation Strategy**:
- **Incremental implementation** with testing at each step
- **Preserve existing functionality** for `basic_test.grue`
- **Comprehensive logging** to track assignment decisions
- **Focused scope**: Start with for-loops only, expand gradually
- **Regression testing**: Ensure no golden file changes

## Files to Modify

### **Primary Changes**:
- `src/grue_compiler/codegen_instructions.rs` - for-loop instruction generation
- `src/grue_compiler/codegen.rs` - add `use_local_var_for_result()` function

### **Secondary Changes**:
- `src/grue_compiler/codegen.rs` - `assign_result_storage()` smart function
- Documentation updates in `CLAUDE.md`

### **Testing Files**:
- `examples/bare_for_loop.grue` (primary test case)
- `examples/minimal_for_loop.grue` (secondary test case)
- `examples/basic_test.grue` (regression test)

This plan provides a systematic approach to fixing the fundamental stack architecture violation while maintaining Z-Machine specification compliance and minimizing risk of regression.