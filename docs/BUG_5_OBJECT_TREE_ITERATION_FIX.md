# Bug #5: Object Tree Iteration Fix - Implementation Plan

## Root Cause

The infrastructure is **100% complete**. The bug is simply: `contents()` method calls don't populate the `variable_sources` IndexMap, so for-loops can't detect object tree iteration when the result is stored in a variable.

## Current State

### ✅ Already Implemented

- `VariableSource` enum (ObjectTreeRoot, Array, Scalar)
- `variable_sources: IndexMap<IrId, VariableSource>` in IrGenerator (stable iteration)
- `GetObjectChild` / `GetObjectSibling` IR instructions
- `generate_object_tree_iteration_with_container()` function
- For-loop checks `variable_sources` to select iteration strategy
- Array literals populate `variable_sources`
- Codegen for object tree opcodes (get_child, get_sibling)

### ❌ Missing

- `contents()` method calls don't add entries to `variable_sources`

## The Fix (5 Lines of Code)

### Location

`src/grue_compiler/ir.rs`, line ~2760-2800 in `Expr::MethodCall` handling

### Change

After creating `result_temp` for method call, add tracking:

```rust
// Track contents() results as object tree roots for iteration
if method == "contents" {
    self.variable_sources.insert(
        result_temp,
        VariableSource::ObjectTreeRoot(object_temp)
    );
}
```

**Insert after:** Result assignment in BOTH branches (property exists / property fallback)

## Why This Works

### Before Fix

```grue
let items = player.contents();  // result_temp created, NOT tracked
for obj in items {              // No source found → defaults to array iteration → ERROR
    print("There is " + obj.name + " here.");  // obj=1000 (garbage) → crash
}
```

### After Fix

```grue
let items = player.contents();  // result_temp tracked as ObjectTreeRoot(player) ✅
for obj in items {              // Source found → uses get_child/get_sibling ✅
    print("There is " + obj.name + " here.");  // obj=2 (real object) → works ✅
}
```

## Implementation Steps

### Step 1: Locate method call result handling

Line ~2760-2800 in `src/grue_compiler/ir.rs`
- Find where `result_temp` is set after method execution
- Need to track in BOTH branches (property exists, property fallback)

### Step 2: Add contents() tracking

- Check `if method == "contents"`
- Insert `ObjectTreeRoot(object_temp)` into `variable_sources`

### Step 3: Test coverage

- Verify direct iteration: `for obj in player.contents()` (already works)
- Verify indirect iteration: `let x = player.contents(); for obj in x` (broken → fixed)
- Verify arrays still work: `let arr = [1,2,3]; for n in arr` (already works)

## Edge Cases

### Handled ✅

- Direct `for obj in player.contents()` ✅
- Indirect `let items = player.contents(); for obj in items` ✅
- Array iteration unchanged ✅
- Scalar usage of result (first child ID) ✅

### Not Handled (Acceptable Limitations) ❌

- Multiple assignment: `let x = player.contents(); let y = x; for obj in y` ❌
  - Would require SSA value propagation (future work)
  - Rare case, document limitation

## Testing

### Validation Checklist

1. All 174 existing tests pass
2. Mini Zork compiles without errors
3. `look` command lists objects (currently fails with "Invalid object number: 1000")
4. Inventory command works

### Risk Assessment

**EXTREMELY LOW**

- Single file modified (ir.rs)
- 5 lines added
- No changes to codegen, no changes to interpreter
- Only adds tracking, doesn't modify generation logic
- Surgical change in isolated location

## Why This Is The Right Solution

### Compared to Alternatives

- ❌ **Option B (Runtime type tagging)**: Complex, wastes value space
- ❌ **Option C (Language syntax changes)**: Breaking change, over-engineered
- ❌ **Option D (Always use object iteration)**: Breaks arrays
- ✅ **Option A (Variable source tracking)**: Simple, complete, zero risk

This is NOT an architectural problem - it's a missing 5-line tracking call.

## Technical Background

### Variable Source Tracking System

The IR generator uses `variable_sources: IndexMap<IrId, VariableSource>` to track where each temporary value originated (IndexMap ensures deterministic iteration order):

```rust
pub enum VariableSource {
    ObjectTreeRoot(IrId),  // From contents() call - use get_child/get_sibling
    Array(IrId),           // From array literal - use get_array_element
    Scalar(IrId),          // Other values - not iterable
}
```

### For-Loop Strategy Selection

In `Stmt::For` generation (line ~2295-2320):

```rust
let iterable_temp = self.generate_expression(for_stmt.iterable, block)?;

// Check variable source to select iteration strategy
let container_object = self.variable_sources
    .get(&iterable_temp)
    .and_then(|source| {
        if let VariableSource::ObjectTreeRoot(container_id) = source {
            Some(*container_id)
        } else {
            None
        }
    });

if let Some(container_id) = container_object {
    // Use get_child/get_sibling iteration
    return self.generate_object_tree_iteration_with_container(...);
}

// Otherwise use array iteration (get_array_element)
```

### The Missing Piece

Array literals already track source (line ~3087):

```rust
// In array literal generation
self.variable_sources.insert(array_temp, VariableSource::Array(array_temp));
```

But `contents()` method calls don't:

```rust
// In Expr::MethodCall for "contents"
// result_temp is created but NOT tracked ❌
// Fix: Add this line
self.variable_sources.insert(result_temp, VariableSource::ObjectTreeRoot(object_temp));
```

## Files to Modify

- `src/grue_compiler/ir.rs` - Add contents() source tracking (~5 lines)
- `CLAUDE.md` - Update Bug #5 status from ⚠️ INCOMPLETE to ✅ FIXED

## Success Criteria

1. **Compilation**: Mini Zork compiles without errors
2. **Execution**: `look` command prints room description AND lists objects
3. **No errors**: No "Invalid object number: 1000" error
4. **Tests**: All 174 existing tests still pass
5. **Arrays**: Array iteration still works correctly
