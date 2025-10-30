# Comprehensive Z-Machine Function Call Architecture Fix Plan

## Current Architecture Analysis

### **CRITICAL ISSUE: Dual Architecture Violation**

The current implementation violates Z-Machine specification in two fundamental ways:

1. **Mixed Call Architecture**: Uses both `call_vs` (proper Z-Machine) AND `use_push_pull_for_result()` (architectural violation)
2. **Stack Discipline Violation**: 91 pushes vs 66 pulls = 25 unused values causing systematic stack overflow

### **Current Implementation Breakdown**

**Function Calls Flow**:
```rust
// Phase 1: call_builtin_function() uses proper call_vs
call_builtin_function() {
    emit call_vs(func_addr, args...) -> stores to Variable(target_var)
    // ✅ CORRECT: Direct variable storage, no stack involvement
}

// Phase 2: FunctionCall IR instruction ADDS push/pull on top
IrInstruction::FunctionCall { target: Some(id) } {
    call_builtin_function(); // Already stored result
    use_push_pull_for_result(id); // ❌ WRONG: Pushes Variable(0), not result!
}
```

**The Problem**: `use_push_pull_for_result()` always pushes `Variable(0)` to stack, but `call_vs` stores results in allocated global variables, not `Variable(0)`.

### **Z-Machine Specification Requirements**

**From CLAUDE.md lines 79-96**:
- **Stack (Variable 0)**: Function call return values, immediate consumption, expression evaluation
- **Local Variables (1-15)**: Function parameters, persistent variables, user-declared variables
- **Never use local variables for function return values**
- **Never bypass stack for function calls**

**Contradiction Discovered**: The documentation says "ALWAYS use stack for function returns" but the current `call_builtin_function()` uses direct variable storage.

## **Detailed Implementation Plan**

### **Phase 1: Architecture Decision & Analysis** (High Complexity, High Risk)

#### **Option A: Pure Push/Pull Architecture** ⭐ RECOMMENDED
Follow Z-Machine spec exactly - all function returns must go through Variable(0).

**Implementation**:
```rust
// Fix call_builtin_function to use Variable(0)
call_builtin_function() {
    emit call_vs(func_addr, args...) -> Variable(0)  // Store to stack
    // No push needed - result already on Variable(0)
}

// Fix use_push_pull_for_result to pull from Variable(0)
use_push_pull_for_result(target_id) {
    // Variable(0) already contains result
    let target_var = allocate_global_for_ir_id(target_id);
    emit store(target_var, Variable(0)); // Move stack to global
}
```

**Benefits**:
- Full Z-Machine specification compliance
- Consistent architecture across all function calls
- Eliminates the 25-item stack imbalance (no extra pushes)
- Follows "stack for function returns" requirement exactly

**Risks**:
- Variable(0) collision if multiple expressions use function calls
- Requires careful sequencing of Variable(0) consumption
- May break existing builtin functions that expect direct variable storage

#### **Option B: Pure Direct Variable Architecture**
Eliminate push/pull entirely, use direct variable storage for all function calls.

**Implementation**:
```rust
// Keep call_builtin_function as-is (direct variable storage)
call_builtin_function() {
    emit call_vs(func_addr, args...) -> Variable(target_var)  // Direct storage
}

// Remove use_push_pull_for_result entirely
IrInstruction::FunctionCall { target: Some(id) } {
    call_builtin_function(); // Direct variable storage - done
    // No additional operations needed
}
```

**Benefits**:
- Eliminates all stack imbalance issues immediately
- Simpler code generation (no push/pull complexity)
- Better performance (fewer instructions)

**Risks**:
- Violates Z-Machine specification requirement for stack usage
- May break complex expressions that depend on stack semantics
- Could cause issues with commercial Z-Machine interpreter compatibility

### **Phase 2: Implementation Strategy** (Medium Complexity, Medium Risk)

#### **A.1: Fix call_builtin_function for Option A**
**File**: `src/grue_compiler/codegen.rs:10782-10788`

**Current Code**:
```rust
let store_var = if let Some(target_id) = target {
    let result_var = self.allocate_global_for_ir_id(target_id);
    self.ir_id_to_stack_var.insert(target_id, result_var);
    Some(result_var)  // ❌ Direct variable storage
} else {
    None
};
```

**Fixed Code**:
```rust
let store_var = if target.is_some() {
    Some(0)  // ✅ Always store to Variable(0) for stack discipline
} else {
    None
};
```

#### **A.2: Fix use_push_pull_for_result for Option A**
**File**: `src/grue_compiler/codegen.rs:8120-8140`

**Current Code**:
```rust
// Phase C2: Emit actual push instruction for stack discipline
let push_operand = Operand::Variable(0);
self.emit_instruction_typed(Opcode::OpVar(OpVar::Push), &[push_operand], None, None)?;
```

**Fixed Code**:
```rust
// Phase C2: Variable(0) already contains function result - move to global
if self.function_call_results.contains(&target_id) {
    // This is a function call result - Variable(0) has the value
    let target_var = self.allocate_global_for_ir_id(target_id);
    self.emit_instruction_typed(
        Opcode::Op2(Op2::Store),
        &[Operand::SmallConstant(target_var), Operand::Variable(0)],
        None, None
    )?;
} else {
    // This is a regular expression result - push Variable(0) as before
    self.emit_instruction_typed(Opcode::OpVar(OpVar::Push), &[Operand::Variable(0)], None, None)?;
}
```

### **Phase 3: call_vs vs call_vn Implementation** (Low Complexity, Low Risk)

#### **Decision Logic**:
```rust
fn emit_function_call(&mut self, func_addr: u16, args: &[Operand], target: Option<IrId>) {
    if target.is_some() {
        // Result will be used - store it
        self.emit_instruction_typed(
            Opcode::OpVar(OpVar::CallVs),
            &build_operands(func_addr, args),
            Some(0), // Store to Variable(0)
            None
        )?;
    } else {
        // Result will be discarded
        self.emit_instruction_typed(
            Opcode::OpVar(OpVar::CallVn),  // ❌ V5+ only!
            &build_operands(func_addr, args),
            None,
            None
        )?;
    }
}
```

**Z-Machine Version Constraint**: `call_vn` is V5+ only. For V3 games:
```rust
if target.is_some() {
    // Use call_vs with Variable(0) storage
    emit call_vs -> Variable(0)
} else {
    // Use call_vs with dummy variable, ignore result
    emit call_vs -> Variable(dummy_var)
    // Don't track the dummy variable - result gets ignored
}
```

### **Phase 4: Comprehensive Testing Strategy** (High Complexity, Critical)

#### **4.1: Unit Testing**
```rust
#[test]
fn test_function_call_stack_discipline() {
    // Test that function calls store to Variable(0)
    // Test that use_push_pull_for_result consumes Variable(0)
    // Test no extra push operations for function results
}

#[test]
fn test_call_vs_vs_call_vn_selection() {
    // Test call_vs used when result needed
    // Test call_vs with dummy var when result discarded (V3)
    // Test no stack imbalance in either case
}
```

#### **4.2: Integration Testing**
```bash
# Test with complex expressions
cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/stack_test.z3
RUST_LOG=error ./target/debug/gruesome tests/stack_test.z3

# Verify stack instrumentation shows balance
# Expected: Equal push/pull operations, no leftover values
```

#### **4.3: Regression Testing**
```bash
# Commercial game compatibility
./target/debug/gruesome tests/zork1.z3
# Must still work perfectly

# All existing tests must pass
cargo test
```

### **Phase 5: Risk Mitigation** (Critical)

#### **5.1: Rollback Plan**
```bash
# Before starting
git add -A && git commit -m "Save point before Z-Machine function call fix"

# During implementation - commit each phase
git commit -m "Phase 1: Fixed call_builtin_function Variable(0) storage"
git commit -m "Phase 2: Fixed use_push_pull_for_result consumption"
git commit -m "Phase 3: Added call_vs/call_vn selection logic"
```

#### **5.2: Variable(0) Collision Detection**
```rust
// Add tracking to prevent Variable(0) overwrites
struct VariableZeroTracker {
    current_producer: Option<String>,  // What put value in Variable(0)
    consumption_expected: bool,        // Whether value should be consumed
}

fn emit_to_variable_zero(&mut self, context: &str) {
    if self.var_zero_tracker.consumption_expected {
        log::error!("Variable(0) collision: {} overwrites {}",
                   context, self.var_zero_tracker.current_producer);
    }
    self.var_zero_tracker.current_producer = Some(context);
    self.var_zero_tracker.consumption_expected = true;
}
```

### **Phase 6: Performance & Complexity Analysis**

#### **Execution Complexity**: **HIGH**
- **Files Modified**: 3-4 major codegen files
- **Function Call Sites**: ~50 locations need analysis
- **Testing Requirements**: Comprehensive regression testing
- **Z-Machine Compliance**: Deep specification analysis required

#### **Risk Assessment**: **MEDIUM-HIGH**
- **Breaking Changes**: High probability of breaking existing functionality
- **Commercial Compatibility**: Must maintain Zork I compatibility
- **Stack Semantics**: Complex interaction between expression evaluation and function calls
- **Variable(0) Management**: Critical timing requirements for Variable(0) access

#### **Implementation Timeline**:
1. **Analysis Phase**: Deep audit of all function call sites
2. **Implementation Phase**: Systematic fix of call_builtin_function and use_push_pull_for_result
3. **Testing Phase**: Comprehensive testing with instrumentation
4. **Validation Phase**: Commercial game compatibility verification

### **Recommended Approach**

**Start with Option A (Pure Push/Pull)** because:
1. Follows Z-Machine specification exactly
2. Fixes the documented 25-item stack imbalance
3. Maintains consistency with "ALWAYS use stack for function returns"
4. Provides clear architectural guidelines for future development

**Implementation Order**:
1. Fix `call_builtin_function()` to store in `Variable(0)`
2. Add function call result tracking to `use_push_pull_for_result()`
3. Implement comprehensive testing with stack instrumentation
4. Add Variable(0) collision detection for safety
5. Validate with commercial games

This plan addresses the fundamental architectural violation while maintaining Z-Machine specification compliance and providing comprehensive testing and rollback capabilities.

## **Implementation Status**

### **Current Phase**: Phase 1 - Architecture Decision & Analysis
- **Status**: Ready to begin implementation
- **Next Step**: Fix `call_builtin_function()` to store results in Variable(0)
- **Files to Modify**: `src/grue_compiler/codegen.rs:10782-10788`

### **Success Criteria**:
- ✅ Stack instrumentation shows balanced push/pull operations
- ✅ All function calls store results to Variable(0)
- ✅ No Variable(0) collisions detected
- ✅ Commercial game compatibility maintained
- ✅ All existing tests pass