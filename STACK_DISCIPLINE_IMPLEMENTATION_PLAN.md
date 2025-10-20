# Z-Machine Stack Discipline Implementation Plan

## Executive Summary

**OBJECTIVE**: Implement proper Z-Machine stack discipline to resolve Variable(0) collision bugs (particularly Property 28 crash) while maintaining system stability and performance.

**ROOT CAUSE**: Current compiler treats Variable(0) like concurrent storage slots instead of LIFO stack, causing 20+ operations to overwrite each other's results.

**SOLUTION**: Implement proper push/pull symmetry with balanced stack operations, stack depth tracking, and consumption distance analysis.

## Phase 1: Discovery & Analysis ✅ COMPLETE

### 1.1 Variable(0) Collision Scope Analysis

**FINDINGS**:
- ✅ **33+ operations** currently use `use_stack_for_result()` → Variable(0)
- ✅ **Property access ALREADY FIXED** - GetProperty/GetPropertyByNumber use globals
- ✅ **Collision sources**: Call, CallIndirect, Array operations, Object operations, TestProperty, GetObjectChild, GetObjectSibling, LogicalComparisonOp, etc.
- ✅ **Property 28 crash** likely from expression evaluation Variable(0) collisions, not property access itself

### 1.2 Current Operations Using Variable(0)

**From codegen_instructions.rs**:
- Call/CallIndirect (function returns)
- Array operations (ArrayLength, ArrayEmpty, ArrayContains, etc.)
- Object operations (GetObjectChild, GetObjectSibling, TestProperty)
- Logical operations (LogicalComparisonOp)
- Property operations (GetNextProperty)

**From codegen.rs**:
- Various expression evaluation contexts
- Return value handling
- Variable assignments

## Phase 2: Stack Management Infrastructure Architecture

### 2.1 Stack State Management

**Add to CodegenContext** (`src/grue_compiler/codegen.rs`):
```rust
pub struct CodegenContext {
    // ... existing fields ...

    // Stack Management Infrastructure
    pub stack_depth: usize,
    pub stack_contents: Vec<IrId>,  // Debug: track what's on stack
    pub max_stack_depth: usize,     // Debug: track maximum depth reached

    // Expression Evaluation Planning
    pub expression_nesting_level: usize,
    pub pending_stack_operations: Vec<StackOperation>,
}

#[derive(Debug, Clone)]
enum StackOperation {
    Push(IrId),
    Pull(IrId, u8), // IrId, target_variable
    DirectAccess(IrId), // Variable(0) direct access
}
```

### 2.2 Push/Pull Helper Functions

**Stack Operation Helpers**:
```rust
impl CodegenContext {
    /// Push value onto Z-Machine stack (VAR:232)
    pub fn emit_push(&mut self, operand: Operand, ir_id: IrId) -> Result<(), CompilerError> {
        self.emit_instruction_typed(
            Opcode::Var(VarOp::Push),
            &[operand],
            None, // Push doesn't store result
            None,
        )?;

        // Update stack state
        self.stack_depth += 1;
        self.stack_contents.push(ir_id);
        self.max_stack_depth = self.max_stack_depth.max(self.stack_depth);

        log::debug!("🔼 STACK_PUSH: IR ID {} pushed, depth now {}", ir_id, self.stack_depth);
        Ok(())
    }

    /// Pull value from Z-Machine stack (VAR:233)
    pub fn emit_pull(&mut self, target_var: u8, ir_id: IrId) -> Result<(), CompilerError> {
        if self.stack_depth == 0 {
            return Err(CompilerError::StackUnderflow(format!(
                "Attempted to pull IR ID {} but stack is empty", ir_id
            )));
        }

        self.emit_instruction_typed(
            Opcode::Var(VarOp::Pull),
            &[Operand::Variable(target_var)],
            None, // Pull stores to operand, not store_var
            None,
        )?;

        // Update stack state
        self.stack_depth -= 1;
        let pulled_ir_id = self.stack_contents.pop().unwrap_or(0);

        log::debug!("🔽 STACK_PULL: IR ID {} pulled to var {}, depth now {}",
                   pulled_ir_id, target_var, self.stack_depth);

        // Verification: check if we pulled what we expected
        if pulled_ir_id != ir_id {
            log::warn!("⚠️  STACK_MISMATCH: Expected IR ID {}, pulled {}", ir_id, pulled_ir_id);
        }

        Ok(())
    }

    /// Access stack top directly (Variable 0) - for immediate consumption
    pub fn emit_stack_access(&mut self, ir_id: IrId) -> Result<(), CompilerError> {
        // Register that this IR ID uses Variable(0) directly
        self.ir_id_to_stack_var.insert(ir_id, 0);
        log::debug!("📍 STACK_ACCESS: IR ID {} uses Variable(0) directly", ir_id);
        Ok(())
    }

    /// Validate stack balance at function boundaries
    pub fn validate_stack_balance(&self, function_name: &str) -> Result<(), CompilerError> {
        if self.stack_depth != 0 {
            return Err(CompilerError::StackImbalance(format!(
                "Function '{}' ended with {} items on stack (should be 0)",
                function_name, self.stack_depth
            )));
        }
        log::debug!("✅ STACK_BALANCE: Function '{}' has balanced stack", function_name);
        Ok(())
    }
}
```

### 2.3 Consumption Distance Analysis

**Smart Stack Usage Decision**:
```rust
impl CodegenContext {
    /// Analyze how an IR result will be consumed to determine storage strategy
    pub fn analyze_consumption_pattern(&self, ir_id: IrId, ir: &IrProgram) -> ConsumptionPattern {
        // Look ahead in IR to see how this value is used
        let usage_count = self.count_ir_id_usage(ir_id, ir);
        let consumption_distance = self.calculate_consumption_distance(ir_id, ir);

        match (usage_count, consumption_distance) {
            (1, 0..=2) => ConsumptionPattern::ImmediateConsumption,
            (1, 3..=10) => ConsumptionPattern::ShortTermPersistence,
            (_, _) => ConsumptionPattern::LongTermPersistence,
        }
    }

    /// Decide storage strategy based on consumption pattern
    pub fn decide_storage_strategy(&self, pattern: ConsumptionPattern, ir_id: IrId) -> StorageStrategy {
        match pattern {
            ConsumptionPattern::ImmediateConsumption => {
                // Use Variable(0) direct access - most efficient
                StorageStrategy::DirectStackAccess
            }
            ConsumptionPattern::ShortTermPersistence => {
                // Use push/pull sequence - handles interference
                StorageStrategy::PushPullSequence
            }
            ConsumptionPattern::LongTermPersistence => {
                // Use global variable - like current property access
                StorageStrategy::GlobalVariable
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum ConsumptionPattern {
    ImmediateConsumption,   // Used in next 1-2 instructions
    ShortTermPersistence,   // Used within 3-10 instructions
    LongTermPersistence,    // Used much later or multiple times
}

#[derive(Debug, PartialEq)]
enum StorageStrategy {
    DirectStackAccess,      // Variable(0) direct - current use_stack_for_result()
    PushPullSequence,       // push then pull when needed
    GlobalVariable,         // Allocated global - current property access pattern
}
```

## Phase 3: Incremental Migration Strategy

### 3.1 Migration Phases (Risk-Ordered)

**Phase 3A: Infrastructure Implementation** (SAFEST) ⬅️ **START HERE**
1. **Add stack management functions** (no behavior change)
2. **Add consumption analysis** (analysis only, no changes)
3. **Add debug logging** for current Variable(0) usage
4. **Add validation** for stack balance (warnings only)

**Phase 3B: Simple Direct Access Cases** (LOW RISK)
1. **Function returns** where result used immediately
2. **Simple expressions** with single operation
3. **Test each change** with mini_zork compilation

**Phase 3C: Complex Expression Cases** (MEDIUM RISK)
1. **Nested function calls** - implement push/pull sequences
2. **Array operations** - convert to appropriate storage strategy
3. **Mathematical expressions** - proper operator precedence

**Phase 3D: Critical Path Cases** (HIGH RISK)
1. **Property 28 collision** - target the specific crash case
2. **Grammar system integration** - complex expression evaluation
3. **Full system validation** - comprehensive testing

### 3.2 Implementation Sequence

**Step 1: Enhanced use_stack_for_result()**
```rust
pub fn use_stack_for_result_smart(&mut self, target_id: IrId, ir: &IrProgram) {
    // Existing safety check (don't overwrite globals)
    if self.ir_id_to_stack_var.contains_key(&target_id) {
        return;
    }

    // NEW: Analyze consumption pattern
    let pattern = self.analyze_consumption_pattern(target_id, ir);
    let strategy = self.decide_storage_strategy(pattern, target_id);

    match strategy {
        StorageStrategy::DirectStackAccess => {
            // Current behavior - Variable(0) direct access
            self.ir_id_to_stack_var.insert(target_id, 0);
            log::debug!("SMART_STACK: IR ID {} -> Variable(0) direct access", target_id);
        }
        StorageStrategy::PushPullSequence => {
            // NEW: Mark for push/pull handling
            self.pending_stack_operations.push(StackOperation::Push(target_id));
            log::debug!("SMART_STACK: IR ID {} -> push/pull sequence", target_id);
        }
        StorageStrategy::GlobalVariable => {
            // Use existing global allocation (like property access)
            let fresh_var = self.allocate_global_for_ir_id(target_id);
            self.ir_id_to_stack_var.insert(target_id, fresh_var);
            log::debug!("SMART_STACK: IR ID {} -> global variable {}", target_id, fresh_var);
        }
    }
}
```

**Step 2: Gradual Migration by Operation Type**
```rust
// Replace use_stack_for_result() calls one IR instruction type at a time:

// Week 1: Simple cases
IrInstruction::LoadImmediate => use_stack_for_result_smart(),
IrInstruction::UnaryOp => use_stack_for_result_smart(),

// Week 2: Function calls
IrInstruction::Call => use_stack_for_result_smart(),
IrInstruction::CallIndirect => use_stack_for_result_smart(),

// Week 3: Array operations
IrInstruction::ArrayLength => use_stack_for_result_smart(),
IrInstruction::ArrayEmpty => use_stack_for_result_smart(),

// Week 4: Complex cases
IrInstruction::LogicalComparisonOp => use_stack_for_result_smart(),
```

**Step 3: Property 28 Targeted Fix**
Once infrastructure is stable, target the specific Property 28 collision:
1. **Identify exact collision sequence** causing crash
2. **Apply push/pull discipline** to that specific case
3. **Verify crash resolution**
4. **Expand to similar patterns**

### 3.3 Rollback Strategy

**At each step**:
1. **Commit working state** before changes
2. **Feature flag new behavior** (can disable if problems)
3. **Comprehensive testing** of existing functionality
4. **Performance benchmarking** (ensure no regression)

## Phase 4: Verification and Testing Approach

### 4.1 Static Verification (Compile-Time)

**Stack Balance Validation**:
```rust
// Add to compilation pipeline - verify every function has balanced stack
pub fn validate_function_stack_balance(&self, func: &IrFunction) -> Result<(), CompilerError> {
    let mut simulated_depth = 0;

    for instruction in &func.instructions {
        match instruction {
            IrInstruction::Call { .. } => simulated_depth += 1, // Return value pushed
            IrInstruction::GetProperty { .. } => simulated_depth += 1, // Property value pushed
            // ... analyze all stack-affecting operations
        }
    }

    if simulated_depth != 0 {
        return Err(CompilerError::StackImbalance(format!(
            "Function '{}' has net stack change of {}, should be 0",
            func.name, simulated_depth
        )));
    }
    Ok(())
}
```

**Variable(0) Conflict Detection**:
```rust
// Detect potential Variable(0) conflicts during compilation
pub fn detect_variable_0_conflicts(&self) -> Vec<ConflictWarning> {
    let mut conflicts = Vec::new();

    // Find all IR IDs that map to Variable(0)
    let variable_0_users: Vec<IrId> = self.ir_id_to_stack_var
        .iter()
        .filter(|(_, &var)| var == 0)
        .map(|(&ir_id, _)| ir_id)
        .collect();

    if variable_0_users.len() > 1 {
        conflicts.push(ConflictWarning {
            message: format!("Multiple IR IDs use Variable(0): {:?}", variable_0_users),
            severity: Severity::High,
        });
    }

    conflicts
}
```

### 4.2 Runtime Verification (Execution-Time)

**Stack State Monitoring**:
```rust
// Add to Z-Machine interpreter for stack validation during execution
impl VM {
    pub fn validate_stack_operation(&mut self, operation: &str) -> Result<(), RuntimeError> {
        match operation {
            "push" => {
                if self.stack.len() >= MAX_STACK_SIZE {
                    return Err(RuntimeError::StackOverflow);
                }
            }
            "pull" => {
                if self.stack.is_empty() {
                    return Err(RuntimeError::StackUnderflow);
                }
            }
            _ => {}
        }

        log::debug!("🔍 STACK_STATE: {} operation, depth now {}", operation, self.stack.len());
        Ok(())
    }
}
```

**Property 28 Crash Detection**:
```rust
// Add specific monitoring for Property 28 access patterns
pub fn monitor_property_28_access(&self, object_id: u16, property_num: u8) {
    if property_num == 28 {
        log::debug!("🎯 PROPERTY_28_ACCESS: Object {} accessing Property 28", object_id);

        // Log current stack state
        log::debug!("🔍 STACK_STATE: depth={}, top_value={:?}",
                   self.stack.len(),
                   self.stack.last());
    }
}
```

### 4.3 Functional Testing (Game Behavior)

**Property 28 Crash Test**:
```bash
# Test script to reproduce Property 28 crash
#!/bin/bash
echo "Testing Property 28 crash resolution..."

# Compile with debug logging
RUST_LOG=debug ./target/debug/grue-compiler examples/mini_zork.grue -o tests/mini_zork_stack_test.z3

# Run the specific sequence that causes Property 28 crash
echo -e "look\n" | RUST_LOG=debug ./target/debug/gruesome tests/mini_zork_stack_test.z3 2>&1 | grep -E "(PROPERTY_28|STACK_|ERROR)"

# Verify crash doesn't occur
if [ $? -eq 0 ]; then
    echo "✅ Property 28 access completed successfully"
else
    echo "❌ Property 28 crash still occurs"
fi
```

**Comprehensive Game Testing**:
```bash
# Test all major game functionality with stack validation
test_game_functionality() {
    local test_file="tests/mini_zork_stack_test.z3"

    # Test navigation
    echo -e "north\nsouth\neast\nwest\n" | ./target/debug/gruesome "$test_file"

    # Test object examination
    echo -e "examine mailbox\nexamine tree\nexamine leaflet\n" | ./target/debug/gruesome "$test_file"

    # Test complex expressions
    echo -e "take leaflet\nopen mailbox\nread leaflet\n" | ./target/debug/gruesome "$test_file"

    # Test grammar system
    echo -e "get mailbox\nlook at tree\nclimb tree\n" | ./target/debug/gruesome "$test_file"
}
```

### 4.4 Performance Validation

**Stack Operation Benchmarking**:
```rust
// Measure performance impact of push/pull vs Variable(0) direct access
#[cfg(test)]
mod stack_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_stack_operations() {
        let start = Instant::now();

        // Test current Variable(0) direct access performance
        for _ in 0..1000 {
            // Simulate current approach
            // store Variable(0), value
            // load Variable(0) -> result
        }

        let direct_time = start.elapsed();

        let start = Instant::now();

        // Test push/pull sequence performance
        for _ in 0..1000 {
            // Simulate push/pull approach
            // push value
            // pull result_var
        }

        let push_pull_time = start.elapsed();

        println!("Direct access: {:?}, Push/pull: {:?}", direct_time, push_pull_time);

        // Verify performance regression is acceptable (< 10%)
        assert!(push_pull_time < direct_time * 110 / 100);
    }
}
```

### 4.5 Regression Testing

**Automated Test Suite**:
```bash
# Complete validation script
validate_stack_implementation() {
    echo "🧪 STACK VALIDATION SUITE"

    # 1. Compile-time validation
    echo "📋 Static analysis..."
    cargo test test_stack_balance_validation
    cargo test test_variable_0_conflict_detection

    # 2. Runtime validation
    echo "🏃 Runtime validation..."
    cargo test test_stack_state_monitoring
    cargo test test_property_28_monitoring

    # 3. Functional testing
    echo "🎮 Game functionality..."
    test_game_functionality

    # 4. Performance validation
    echo "⚡ Performance benchmarking..."
    cargo test benchmark_stack_operations

    # 5. Regression testing
    echo "🔄 Regression testing..."
    cargo test --all

    echo "✅ Stack validation complete"
}
```

### 4.6 Success Criteria

**Must Pass ALL of**:
1. ✅ **No Property 28 crashes** during room description access
2. ✅ **All 174+ tests pass** - no functionality regressions
3. ✅ **Stack balance validation** - all functions have balanced stacks
4. ✅ **No Variable(0) conflicts** - at most one operation uses Variable(0) at a time
5. ✅ **Performance acceptable** - < 10% regression in compilation/execution time
6. ✅ **Game fully playable** - all navigation, examination, interaction works

## Implementation Schedule

### Week 1: Infrastructure (Phase 3A)
- [ ] Add stack management data structures to CodegenContext
- [ ] Implement emit_push(), emit_pull(), emit_stack_access() helpers
- [ ] Add stack depth tracking and validation
- [ ] Add debug logging for current Variable(0) usage patterns
- [ ] **VERIFICATION**: No behavior changes, just enhanced logging and infrastructure

### Week 2: Analysis Tools (Phase 3A continued)
- [ ] Implement consumption distance analysis
- [ ] Implement storage strategy decision logic
- [ ] Add Variable(0) conflict detection
- [ ] Add stack balance validation
- [ ] **VERIFICATION**: Analysis tools work correctly, identify current conflicts

### Week 3: Simple Cases (Phase 3B)
- [ ] Implement use_stack_for_result_smart()
- [ ] Migrate LoadImmediate and UnaryOp operations
- [ ] Test with mini_zork compilation and basic functionality
- [ ] **VERIFICATION**: Simple operations work correctly with new infrastructure

### Week 4: Property 28 Focus (Phase 3D)
- [ ] Identify exact Property 28 collision sequence
- [ ] Apply targeted fix to Property 28 case
- [ ] Verify Property 28 crash resolution
- [ ] **VERIFICATION**: Property 28 crash eliminated while maintaining functionality

## Key Architectural Insights

1. **Property access is already fixed** - focus on remaining Variable(0) collisions
2. **Stack is primary mechanism** for expression evaluation per Z-Machine spec
3. **Push/pull symmetry** is the correct pattern, not Variable(0) avoidance
4. **Consumption distance analysis** enables optimal storage strategy selection
5. **Incremental migration** provides safety and verification at each step

## Critical Success Factors

1. **Start with infrastructure** - build foundation before changing behavior
2. **Verify at each step** - commit working state before proceeding
3. **Target Property 28 specifically** - solve the immediate crash problem
4. **Maintain compatibility** - all existing tests must continue to pass
5. **Performance monitoring** - ensure no significant regression

---

## Status Tracking

- [x] **Phase 1**: Discovery & Analysis Complete
- [x] **Phase 3A**: Infrastructure Implementation Complete
- [x] **Phase 3B**: Infrastructure & Global Variable Migration Complete
- [ ] **Phase C**: Actual Push/Pull Implementation ⬅️ **CURRENT FOCUS**
- [ ] **Phase 4**: Comprehensive Validation

---

# PHASE C: ACTUAL PUSH/PULL IMPLEMENTATION ACTION PLAN

**Current State (Commit b769d96)**: Infrastructure complete but NO actual collision reduction yet.
- ✅ 3 operations using dedicated globals (19% real improvement)
- ❌ 2 operations marked for push/pull but still using Variable(0) (no improvement yet)
- ❌ 11 operations still competing for Variable(0)

**CRITICAL**: Property 28 crash still possible until actual push/pull opcodes implemented.

## Phase C1: Implement Core Push/Pull Mechanics

### Step C1.1: Modify use_push_pull_for_result() for Actual Push/Pull
**Goal**: Replace Variable(0) mapping with actual VAR:232 (push) instruction emission

**Implementation**:
```rust
// Instead of: self.ir_id_to_stack_var.insert(target_id, 0);
// Implement:
// 1. Emit VAR:232 (push) instruction after operation completes
// 2. Mark IR ID for pull when consumed
// 3. Track stack depth for validation
```

**Test Plan**: Single operation test (arithmetic negation)
**Success Criteria**: Same 7.5k bytecode, same gameplay, different stack usage

### Step C1.2: Implement Pull-on-Consumption System
**Goal**: Emit VAR:233 (pull) when push-marked values are consumed

**Implementation**:
```rust
// When resolve_ir_id_to_operand() encounters push-marked IR ID:
// 1. Emit VAR:233 (pull) to temporary global variable
// 2. Return Variable(temp_global) as operand
// 3. Update stack depth tracking
```

**Test Plan**: End-to-end push→pull sequence verification
**Success Criteria**: LIFO stack discipline maintained, no stack underflow

### Step C1.3: Add Stack Depth Safety Validation
**Goal**: Prevent stack corruption and validate LIFO order

**Implementation**:
- Stack underflow prevention during pull operations
- LIFO order validation
- Error handling for stack discipline violations

**Test Plan**: Error condition testing and stack state validation
**Success Criteria**: Clean error handling, no stack corruption

## Phase C2: Incremental Testing & Conversion

### Step C2.1: Test Single Operation (Start with Arithmetic Negation)
**Goal**: Prove push/pull system works for one operation

**Process**:
1. Implement push after `IrUnaryOp::Minus` result
2. Implement pull when negation result is consumed
3. Verify: same behavior, different stack usage
4. Test: compile mini_zork, verify 7.5k output, test gameplay

**Success Criteria**: Property 28 crash frequency reduction measurable

### Step C2.2: Convert Additional Expression Operations
**Goal**: Expand to multiple operations incrementally

**Target Operations**:
- object_is_empty builtin
- Remaining arithmetic operations
- Test each conversion individually

**Success Criteria**: Cumulative collision reduction without instability

### Step C2.3: Measure Stack Collision Reduction
**Goal**: Quantify actual vs theoretical improvement

**Metrics**:
- Debug logging of Variable(0) access patterns
- Actual collision frequency measurement
- Property 28 crash frequency reduction

**Success Criteria**: Measurable collision reduction, Property 28 improvement

## Phase C3: Full Implementation & Validation

### Step C3.1: Convert All Remaining Suitable Operations
**Goal**: Maximize Variable(0) collision reduction

**Target**: ~8 more operations for push/pull conversion
**Goal**: 75%+ reduction in Variable(0) contention
**Final State**: ~3 operations on globals, ~2 operations on stack, minimal Variable(0) usage

### Step C3.2: Comprehensive Testing
**Goal**: Verify full system stability and Property 28 resolution

**Test Suite**:
- Full mini_zork gameplay testing
- Property access stress testing
- Navigation and command testing
- Property 28 crash elimination verification

### Step C3.3: Performance & Stability Validation
**Goal**: Ensure no performance regression

**Validation**:
- Bytecode size comparison (should remain ~7.5k)
- Runtime performance testing
- Memory usage validation
- Stack depth monitoring

## Technical Implementation Details

### Key Z-Machine Opcodes:
- **VAR:232 (0xE8)**: `push value` - Push value onto game stack
- **VAR:233 (0xE9)**: `pull -> variable` - Pull value from stack to variable

### Implementation Strategy:
1. **Producer Side**: After instruction completion → emit push
2. **Consumer Side**: Before operand use → emit pull to temp global
3. **Tracking**: Mark IR IDs for push/pull vs direct Variable(0) access

### Safety Mechanisms:
- Stack underflow prevention
- Temp global allocation for pull targets
- LIFO order validation
- Rollback points at each phase

## Success Metrics:
- **Primary**: Property 28 crash eliminated
- **Secondary**: 70%+ reduction in Variable(0) contention
- **Tertiary**: Maintain 7.5k bytecode size and gameplay stability

## Rollback Points:
- **Current**: `b769d96` (infrastructure ready)
- **After C1**: Core push/pull mechanics working
- **After C2**: Incremental testing complete
- **After C3**: Full implementation verified

## Next Action:
**Phase C1.1**: Implement actual push/pull in use_push_pull_for_result()

**Next Action**: Begin Phase 3A - Infrastructure Implementation