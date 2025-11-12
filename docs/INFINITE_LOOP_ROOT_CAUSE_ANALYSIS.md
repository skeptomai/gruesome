# Infinite Loop Root Cause Analysis - Function Ordering Changes

## Summary
The infinite loop in container contents listing is NOT caused by GetObjectSibling logic errors, but by **function ordering changes between commits** that affect all branch offset calculations.

## Key Finding: Function Ordering Differences

### Working Commit (8c7312d):
```
dispatch_handle_climb (id=26)
dispatch_handle_read (id=27)
dispatch_handle_drop (id=28)
dispatch_handle_take (id=29)
```

### Current HEAD (broken):
```
dispatch_handle_read (id=26)
dispatch_handle_climb (id=27)
dispatch_handle_take (id=28)
dispatch_handle_drop (id=29)
```

## Impact Analysis

1. **Function ID Changes**: All dispatch functions have different IDs
2. **Grammar Rule Updates**: Function calls in grammar rules reference wrong functions
3. **Branch Offset Cascade**: Function reordering shifts all function positions
4. **Bytecode Differences**: Systematic address shifts affect all branch calculations

### Bytecode Evidence:
- Address 0x1fa0: Working=`0e 2b` (offset 43), Broken=`0e 1f` (offset 31)
- Address 0x2290: Working=`0e 1f` (offset 31), Broken=`0e 32` (offset 50)

## Function Position Shifts (Line Numbers):
- Working: `list_contents` at line 870, `contents` at line 874
- Current: `list_contents` at line 793, `contents` at line 797
- **Shift: 77 lines earlier in current version**

## IR Differences Show Grammar Impact:
```
Working:  verb: "take" -> FunctionCall(29, [RuntimeParameter("noun")])
         verb: "drop" -> FunctionCall(28, [RuntimeParameter("noun")])

Current:  verb: "take" -> FunctionCall(28, [RuntimeParameter("noun")])
         verb: "drop" -> FunctionCall(29, [RuntimeParameter("noun")])
```

## Resolution Strategy
The infinite loop occurs because:
1. Container iteration calls wrong function due to ID changes
2. Wrong function likely doesn't terminate iteration properly
3. Branch offsets point to wrong locations due to systematic shifts

## ROOT CAUSE IDENTIFIED
**Non-deterministic HashMap iteration in ir.rs:1472-1496**

```rust
let mut function_counts: std::collections::HashMap<
    String,
    Vec<(u32, ObjectSpecialization)>,
> = std::collections::HashMap::new();

// Later...
for (name, versions) in function_counts.iter() { // Non-deterministic order!
    if versions.len() > 1 {
        for (func_id, specialization) in versions {
            self.register_function_overload(name, *func_id, specialization.clone());
```

**HashMap iteration order is non-deterministic**, causing dispatch functions to be generated in different orders between program runs. This changes all function IDs, affecting:

1. Grammar rule function calls (take/drop swap)
2. Branch offset calculations (systematic address shifts)
3. Container iteration logic calling wrong functions

## SOLUTION
Replace `HashMap` with `IndexMap` to ensure deterministic function ordering:

```rust
let mut function_counts: IndexMap<
    String,
    Vec<(u32, ObjectSpecialization)>,
> = IndexMap::new();
```

This will restore consistent function ordering between builds.