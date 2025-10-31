# Stack Discipline Fix Plan: Phased Approach

## Problem Summary
The compiler uses `use_push_pull_for_result()` for ALL Variable(0) operations, violating Z-Machine stack discipline. Most operations should use direct Variable(0) access for immediate consumption.

## Fix Strategy: Conservative Phased Approach

### Phase 1: Property Access Operations (Critical Path)
**Target:** Fix `handle_open` command failure

**Operations to Fix:**
- `GetProperty` (obj.property access)
- Property-based attribute access
- Direct property consumption

**Test Commands:**
- ✅ `open mailbox` (direct) - should work
- ✅ `examine mailbox` - property access test
- ✅ `look` then `open mailbox` - should still work

**Implementation:**
```rust
// In GetProperty codegen:
// BEFORE: self.use_push_pull_for_result(*target, "GetProperty operation")?;
// AFTER: self.ir_id_to_stack_var.insert(*target, 0);
```

### Phase 2: Attribute Testing Operations
**Target:** Comprehensive attribute access fix

**Operations to Fix:**
- Remaining `test_attr` operations beyond TestAttribute
- SetAttribute operations if they use stack
- Attribute-based conditionals

**Test Commands:**
- ✅ `open mailbox` - should still work
- ✅ `close mailbox` - attribute modification test
- ✅ `take mailbox` - takeable attribute test
- ✅ Full object interaction workflow

### Phase 3: Arithmetic and Logical Operations
**Target:** Expression evaluation stack discipline

**Operations to Fix:**
- Binary operations (`+`, `-`, `*`, `/`, `==`, `!=`)
- Unary operations (`!`, `-`)
- Comparison operations (`<`, `>`, `<=`, `>=`)

**Test Commands:**
- ✅ Complex expressions in conditions
- ✅ Mathematical operations in print statements
- ✅ String concatenation with arithmetic
- ✅ `inventory` (complex object listing with expressions)

### Phase 4: Array and Collection Operations
**Target:** Object iteration and collection handling

**Operations to Fix:**
- `CreateArray` operations
- Array indexing and access
- Collection iteration beyond object trees

**Test Commands:**
- ✅ Commands that create temporary arrays
- ✅ Complex object property iterations
- ✅ Multiple nested for-loops

### Phase 5: Function Call Optimization
**Target:** Selective function call stack usage

**Operations to Review:**
- User-defined function calls (keep stack for returns)
- Builtin function calls (evaluate case-by-case)
- Inline vs. persistent result handling

**Test Commands:**
- ✅ Function calls with return values
- ✅ Nested function calls
- ✅ Complex function argument expressions

## Testing Strategy

### Confirmation Testing at Each Phase
```bash
# Phase completion test battery
echo "open mailbox" | ./target/debug/gruesome tests/phase_N.z3           # Direct command
echo -e "look\nopen mailbox" | ./target/debug/gruesome tests/phase_N.z3  # Sequence
echo "examine mailbox" | ./target/debug/gruesome tests/phase_N.z3         # Property access
echo "inventory" | ./target/debug/gruesome tests/phase_N.z3               # Complex iteration
echo -e "take leaflet\ninventory" | ./target/debug/gruesome tests/phase_N.z3  # State changes
```

### Stack Instrumentation Verification
```bash
# Should show decreasing stack operations with each phase
RUST_LOG=error cargo run --bin grue-compiler -- examples/mini_zork.grue -o tests/phase_N.z3 2>&1 | grep "STACK_PUSH" | wc -l
```

### Final Gameplay Testing
**Complete Mini-Zork Walkthrough:**
```
open mailbox
read leaflet
take leaflet
go north
open window
go west
take water
go east
go up
take rope
go down
go south
open door with key
go west
take lamp
turn on lamp
go down
go south
[...continue full game...]
```

## Implementation Guidelines

### Direct Variable(0) Mapping Pattern
```rust
// NEW PATTERN: For immediate consumption operations
if let Some(target) = target {
    self.ir_id_to_stack_var.insert(target, 0);
    log::debug!("OPERATION: mapped IR ID {} directly to Variable(0)", target);
}
```

### Keep Stack Pattern (Selective)
```rust
// KEEP: For function boundaries and persistence
if let Some(target) = target {
    self.function_call_results.insert(target);
    self.use_push_pull_for_result(target, "function call result")?;
}
```

### Decision Criteria
**Use Direct Variable(0) When:**
- Result consumed immediately in same function
- Property access for conditional testing
- Arithmetic/logical operations for expressions
- Attribute testing for immediate decisions

**Use Stack When:**
- Function return values
- Values crossing function boundaries
- Results needed after other Variable(0) operations
- Persistent storage across multiple instructions

## Risk Mitigation

### Incremental Approach
- Fix one category at a time
- Test thoroughly at each phase
- Maintain backup files for rollback
- Document each change for review

### Validation Checks
- No new compilation errors
- No stack underflow/overflow at runtime
- All existing commands continue working
- Performance improvement (fewer stack ops)

### Rollback Plan
Each phase creates a separate test file (`tests/phase_N.z3`) allowing:
- Independent testing of each phase
- Easy rollback to last working state
- Comparative analysis of stack operations
- Incremental deployment confidence

## Success Metrics

### Phase 1 Success:
- `open mailbox` direct command works
- Property access operations functional
- No stack underflow in basic commands

### Phase 3 Success:
- Complex expressions work correctly
- Mathematical operations maintain accuracy
- No arithmetic operation stack issues

### Final Success:
- Complete Mini-Zork playable start to finish
- 50%+ reduction in unnecessary stack operations
- No stack discipline violations
- All commands work both directly and in sequences