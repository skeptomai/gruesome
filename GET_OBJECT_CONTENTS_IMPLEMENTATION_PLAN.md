# GET_OBJECT_CONTENTS Implementation Plan

## Overview

Replace the broken placeholder implementation in `get_object_contents` builtin with proper Z-Machine object tree traversal using GetObjectChild/GetObjectSibling opcodes.

## Current Architecture Analysis

### Working Infrastructure ✅

1. **IR Generation**: `contents()` method calls are correctly detected as builtin pseudo-methods
   - Location: `src/grue_compiler/ir.rs:2681-2682, 2731-2742`
   - Pattern: `matches!(method.as_str(), "get_exit" | "empty" | "none" | "contents")`
   - Result: Direct call to `get_object_contents` builtin without property lookup

2. **Object Tree Opcodes**: GetObjectChild and GetObjectSibling are fully implemented
   - Location: `src/grue_compiler/codegen_instructions.rs:1033-1109`
   - GetObjectChild: `0x01` (1OP:1) - returns first child object, branches if no child
   - GetObjectSibling: `0x02` (1OP:2) - returns next sibling object, branches if no sibling
   - Both store result to stack (Variable 0) and use proper branch encoding

3. **For-Loop Object Tree Iteration**: Complete implementation for direct iteration
   - Location: `src/grue_compiler/ir.rs:2070-2119`
   - Uses GetObjectChild to get first child, then GetObjectSibling to traverse siblings
   - Handles empty containers by branching to loop_end when no children

4. **Variable Source Tracking**: Tracks contents() results for indirect iteration
   - Location: `src/grue_compiler/ir.rs:2867-2869`
   - Pattern: `if is_contents_method { self.variable_sources.insert(result_temp, VariableSource::ObjectTreeRoot(object_temp)); }`
   - Enables: `let items = obj.contents(); for item in items { ... }`

### Broken Component ❌

**get_object_contents Builtin**: Placeholder OR(1|0=1) instead of object tree traversal
- Location: `src/grue_compiler/codegen_builtins.rs:778-815`
- Current: `Opcode::Op2(Op2::Or), [LargeConstant(1), SmallConstant(0)]` → always returns 1
- Problem: For-loop iteration expects proper object tree results, placeholder causes stack underflow

## Implementation Strategy

### Option A: Return First Child Object ID (Recommended)

**Approach**: get_object_contents returns the first child object ID (or 0 if no children), letting the existing for-loop infrastructure handle GetObjectSibling traversal.

**Advantages**:
- Minimal changes - reuses existing for-loop object tree iteration code
- Consistent with how direct `for item in obj.contents()` works
- Simple mapping: GetObjectChild → first child → for-loop handles rest

**Implementation**:
```rust
// Replace OR(1|0=1) placeholder with GetObjectChild call
let child_temp = self.next_id();
let no_child_label = self.next_id();
let end_label = self.next_id();

// GetObjectChild: get first child of container object
self.emit_instruction(
    0x01, // get_child opcode (1OP:1)
    &[container_operand],
    Some(0), // Store result to stack
    Some(0x7FFF), // Branch on FALSE (no child) to no_child_label
)?;

// Child exists: store child object ID as result
self.emit_instruction_typed(
    Opcode::Op2(Op2::Or),
    &[Operand::Variable(0), Operand::SmallConstant(0)], // child_id | 0 = child_id
    Some(0),
    None,
)?;
// Jump to end
self.translate_jump(end_label)?;

// No child: store 0 as result (empty container)
self.emit_label(no_child_label);
self.emit_instruction_typed(
    Opcode::Op2(Op2::Or),
    &[Operand::SmallConstant(0), Operand::SmallConstant(0)], // 0 | 0 = 0
    Some(0),
    None,
)?;

self.emit_label(end_label);
self.use_push_pull_for_result(store_var, "get_object_contents builtin")?;
```

### Option B: Array-Like Interface (Complex)

**Approach**: Return array-like object with indexing support to match array iteration patterns.

**Disadvantages**:
- Requires implementing array interface for object tree
- Complex integration with existing array vs object tree detection
- More code changes across multiple files

## Detailed Implementation Plan

### Phase 1: Replace Placeholder with GetObjectChild

**File**: `src/grue_compiler/codegen_builtins.rs:800-815`

1. **Remove Broken Placeholder**:
   - Delete OR(1|0=1) instruction that causes stack underflow
   - Remove misleading comments about "non-empty container"

2. **Add GetObjectChild Logic**:
   - Use existing emit_instruction pattern from GetObjectChild codegen
   - Handle branch for empty containers (no children)
   - Store result to stack using existing use_push_pull_for_result

3. **Branch Handling**:
   - Use 0x7FFF placeholder for branch-on-FALSE (consistent with existing GetObjectChild)
   - Create labels for no_child and end cases
   - Return 0 for empty containers, first child ID for non-empty

### Phase 2: Test with Minimal Reproduction

**Verification Steps**:
1. Compile `examples/minimal_object_iteration_repro.grue`
2. Run and verify no stack underflow
3. Test with objects that have children (create enhanced test case)
4. Verify for-loop iteration works correctly

### Phase 3: Integration Testing

**Test Cases**:
1. **Empty Container**: `player.contents()` with no items → should return 0, for-loop should not execute
2. **Single Item**: Container with one child → should return child ID, for-loop executes once
3. **Multiple Items**: Container with multiple children → should return first child, for-loop iterates through all siblings
4. **Nested Containers**: Objects containing other containers → verify recursive behavior

## Z-Machine Specification Reference

### GetObjectChild (1OP:1, opcode 0x01)
- **Input**: Object number
- **Output**: Child object number (0 if no child)
- **Branch**: Branches when result is 0 (no child)
- **Usage**: `get_child container_obj → child_obj, branch_if_no_child`

### GetObjectSibling (1OP:2, opcode 0x02)
- **Input**: Object number
- **Output**: Sibling object number (0 if no sibling)
- **Branch**: Branches when result is 0 (no sibling)
- **Usage**: `get_sibling current_obj → next_obj, branch_if_no_sibling`

## Expected Behavior Changes

### Before Fix
```
player.contents() → OR(1|0=1) → returns 1
for item in items → stack underflow (iteration expects object tree results)
```

### After Fix
```
player.contents() → GetObjectChild(player) → returns first_child_id or 0
for item in items → GetObjectSibling traversal (existing working code)
```

## Risk Assessment

**Low Risk Changes**:
- Replacing placeholder with GetObjectChild reuses existing, tested opcode codegen
- No changes to IR generation or for-loop infrastructure
- Consistent with existing object tree architecture

**Testing Requirements**:
- Verify all existing object tree tests continue to pass
- Test both empty and non-empty containers
- Confirm no regression in direct `for item in obj.contents()` syntax

## Success Criteria

1. ✅ `examples/minimal_object_iteration_repro.grue` compiles and runs without stack underflow
2. ✅ Empty containers (like player with no items) iterate correctly (0 iterations)
3. ✅ Containers with items iterate through all children
4. ✅ All existing object tree tests continue to pass
5. ✅ No regression in direct vs indirect iteration patterns

## Implementation Timeline

1. **Phase 1**: Replace placeholder (~30 minutes)
2. **Phase 2**: Test minimal reproduction (~15 minutes)
3. **Phase 3**: Enhanced testing with actual object children (~30 minutes)
4. **Documentation**: Update comments and commit (~15 minutes)

**Total Estimated Time**: ~90 minutes

## Follow-up Work

After successful implementation:
1. Create comprehensive object iteration test suite
2. Document object tree vs array iteration patterns
3. Consider performance optimizations for large object trees
4. Investigate object tree caching strategies if needed