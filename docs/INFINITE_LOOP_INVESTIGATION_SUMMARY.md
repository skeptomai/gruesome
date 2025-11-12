# Infinite Loop Investigation Summary - Nov 12, 2025

## Current Status: PARTIALLY FIXED

### ✅ ROOT CAUSE IDENTIFIED AND FIXED
**Non-deterministic HashMap in ir.rs:1472** causing random function ordering between builds:

```rust
// FIXED: Changed from HashMap to IndexMap for deterministic iteration
let mut function_counts: IndexMap<
    String,
    Vec<(u32, ObjectSpecialization)>,
> = IndexMap::new();
```

**Impact**: Function dispatch ordering now deterministic, fixing systematic branch offset issues.

### ❌ INFINITE LOOP PERSISTS
Container iteration still prints infinite "leaflet" entries. This is a **separate logical bug** in container iteration, not the HashMap determinism issue.

## Investigation Timeline

1. **PrintPaddr Opcode Fix**: Fixed 0x8D→0x0D for object recognition
2. **Function Ordering Discovery**: Found dispatch functions reordering between commits
3. **HashMap Root Cause**: Identified non-deterministic HashMap iteration
4. **IndexMap Fix Applied**: Function ordering now deterministic
5. **Logical Bug Remains**: Container iteration still infinite loops

## Current Function Ordering (IndexMap Fixed)
- dispatch_handle_take (id=26)
- dispatch_handle_drop (id=27)
- dispatch_handle_climb (id=28)
- dispatch_handle_read (id=29)

## Next Investigation Required
The infinite loop in `list_contents()` is a separate logical bug in:
- `container.contents()` method implementation
- Container iteration for-loop logic
- GetObjectSibling termination conditions

## Files Modified
- `src/grue_compiler/ir.rs:1472` - HashMap→IndexMap fix
- `src/grue_compiler/opcodes.rs` - PrintPaddr opcode fix

## Commercial Game Impact
**MUST VERIFY**: Zork I gameplay protocol still passes after IndexMap fix.