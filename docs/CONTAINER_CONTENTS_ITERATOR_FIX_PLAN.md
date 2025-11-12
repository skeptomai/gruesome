# Container Contents Iterator Fix Plan

## Problem Analysis

### Root Cause
The infinite loop in "open mailbox" is caused by two broken builtin implementations that violate Z-Machine low-memory design principles:

1. **`list_contents` builtin** - Just prints "[CONTENTS_LIST]" placeholder instead of iterating
2. **`get_object_contents` builtin** - Returns only first child instead of implementing proper iterator

### Z-Machine Design Philosophy
- **Use iterators, not arrays** - designed for low memory environments
- **Object tree traversal** using `get_child` + `get_sibling` instructions
- **Memory efficient** - only hold one object reference at a time
- **Dynamic bounds** - no hardcoded object limits

### Current Broken Behavior
```
User: "open mailbox"
Game: "The a small mailbox contains:"
Game: "leaflet" (repeated infinitely)
```

**Why it loops infinitely:**
- `list_contents()` calls `container.contents()`
- `get_object_contents()` returns only first child (leaflet)
- `for obj in contents` expects iterator but gets single object
- Iteration logic breaks and repeats infinitely

## Implementation Plan

### Phase 1: Fix `list_contents` builtin (Critical - stops infinite loop)

**File**: `src/grue_compiler/codegen_builtins.rs:1068-1104`

**Current broken implementation:**
```rust
// Just prints placeholder "[CONTENTS_LIST]"
let placeholder_str = self.get_builtin_message("builtin_contents_list_placeholder", "[CONTENTS_LIST]");
let string_id = self.find_or_create_string_id(&placeholder_str)?;
// Emit print_paddr with placeholder
```

**Z-Machine Iterator Pattern Implementation:**
```rust
pub fn generate_list_contents_builtin(&mut self, args: &[IrId]) -> Result<(), CompilerError> {
    let container_operand = self.resolve_ir_id_to_operand(args[0])?;

    // Generate iteration loop using get_child + get_sibling pattern
    let loop_start_label = self.next_string_id;
    self.next_string_id += 1;
    let loop_end_label = self.next_string_id;
    self.next_string_id += 1;

    // Step 1: get_child(container) → Variable 7 (temp var)
    let layout = self.emit_instruction_typed(
        Opcode::Op1(Op1::GetChild),
        &[container_operand],
        Some(7), // Store current object in Variable 7
        Some(0x7FFF), // Branch if no child (empty container)
    )?;

    // Register branch to loop_end if no children
    if let Some(branch_location) = layout.branch_location {
        self.reference_context.unresolved_refs.push(UnresolvedReference {
            reference_type: LegacyReferenceType::Branch,
            location: branch_location,
            target_id: loop_end_label,
            is_packed_address: false,
            offset_size: 2,
            location_space: MemorySpace::Code,
        });
    }

    // Step 2: Loop start label
    self.label_addresses.insert(loop_start_label, self.code_address);

    // Step 3: Check if current_obj == 0 (end of iteration)
    let layout = self.emit_instruction_typed(
        Opcode::Op2(Op2::Je),
        &[Operand::Variable(7), Operand::SmallConstant(0)],
        None,
        Some(0x7FFF), // Branch to loop_end if current_obj == 0
    )?;

    // Register branch to loop_end
    if let Some(branch_location) = layout.branch_location {
        self.reference_context.unresolved_refs.push(UnresolvedReference {
            reference_type: LegacyReferenceType::Branch,
            location: branch_location,
            target_id: loop_end_label,
            is_packed_address: false,
            offset_size: 2,
            location_space: MemorySpace::Code,
        });
    }

    // Step 4: Print current object (loop body)
    // Print "  " (indent)
    self.emit_print_string("  ")?;

    // Get object name property (property 1)
    self.emit_instruction_typed(
        Opcode::Op2(Op2::GetProp),
        &[Operand::Variable(7), Operand::SmallConstant(1)], // property 1 = name
        Some(8), // Store in Variable 8
        None,
    )?;

    // Print object name
    self.emit_instruction_typed(
        Opcode::Op1(Op1::PrintPaddr),
        &[Operand::Variable(8)],
        None,
        None,
    )?;

    // Print newline
    self.emit_instruction_typed(
        Opcode::Op0(Op0::NewLine),
        &[],
        None,
        None,
    )?;

    // Step 5: get_sibling(current_obj) → Variable 7
    self.emit_instruction_typed(
        Opcode::Op1(Op1::GetSibling),
        &[Operand::Variable(7)],
        Some(7), // Update Variable 7 with next sibling
        None,
    )?;

    // Step 6: Jump back to loop start
    self.translate_jump(loop_start_label)?;

    // Step 7: Loop end label
    self.label_addresses.insert(loop_end_label, self.code_address);

    Ok(())
}
```

**Expected Z-Machine bytecode flow:**
```
1. get_child(container) → Variable 7 (or branch to end if empty)
2. LOOP_START:
3.   je Variable_7, 0 → branch to LOOP_END if done
4.   print "  "
5.   get_prop Variable_7, 1 → Variable 8 (get name)
6.   print_paddr Variable_8
7.   new_line
8.   get_sibling Variable_7 → Variable 7 (next object)
9.   jump LOOP_START
10. LOOP_END:
```

### Phase 2: Fix `get_object_contents` for Grue `for` loops

**File**: `src/grue_compiler/codegen_builtins.rs:1187-1285`

**Current broken implementation:**
- Only returns first child using `get_child`
- Comment says "BROKEN PLACEHOLDER IMPLEMENTATION - CAUSES OBJECT ITERATION BUG"

**Solution Options:**
- **Option A**: Generate inline iteration code (like `list_contents`)
- **Option B**: Return iterator object (complex, needs state management)

**Recommended: Option A** - When compiler sees `for obj in container.contents()`, generate get_child/get_sibling loop inline.

### Phase 3: Update IR generation for `for` loops

**File**: `src/grue_compiler/ir.rs` - find where `for` loops are processed

**Current broken pattern:**
```grue
for obj in container.contents() {
    // loop body
}
```
Compiles to:
1. Call `get_object_contents(container)` → "array" (broken)
2. Iterate over "array" (infinite loop)

**Fixed pattern:**
Generate inline get_child/get_sibling iteration:
1. `get_child(container)` → current_obj
2. Loop while current_obj != 0:
   - Execute loop body with current_obj
   - `get_sibling(current_obj)` → current_obj

## Testing Strategy

### Immediate Test (Phase 1)
```bash
# Should show "leaflet" once, not infinite loop
echo "open mailbox" | ./target/debug/gruesome tests/mini_zork_gameplay_test.z3
```

**Expected output:**
```
The a small mailbox contains:
  leaflet
```

### Complete Gameplay Test
Run full Mini Zork test protocol after all phases complete:
```bash
echo -e "open mailbox\ntake leaflet\nread leaflet\nnorth\nnorth\nclimb tree\ntake egg\ndown\nscore\ninventory\nquit\ny" | ./target/debug/gruesome tests/mini_zork_gameplay_test.z3
```

## Key Architectural Benefits

### Aligns with Z-Machine Design
- ✅ **Low Memory**: No arrays, only single object references
- ✅ **Iterator Pattern**: Memory efficient traversal
- ✅ **Dynamic Bounds**: No hardcoded object limits
- ✅ **Native Instructions**: Uses `get_child`/`get_sibling` as intended

### Fixes Multiple Issues
- ✅ **Infinite Loop**: Proper termination when sibling = 0
- ✅ **Container Listing**: Shows all contents, not just first
- ✅ **Memory Efficiency**: No temporary arrays or excessive allocations
- ✅ **Specification Compliance**: Uses Z-Machine object tree correctly

## Implementation Priority

1. **Phase 1** (Critical): Fix `list_contents` infinite loop
2. **Phase 2** (Important): Fix `get_object_contents` for iterator protocol
3. **Phase 3** (Enhancement): Optimize IR generation for `for` loops

## Files to Modify

- `src/grue_compiler/codegen_builtins.rs:1068-1104` - Replace `list_contents` placeholder
- `src/grue_compiler/codegen_builtins.rs:1187-1285` - Fix `get_object_contents` implementation
- `src/grue_compiler/ir.rs` - Update `for` loop IR generation (Phase 3)

## Success Criteria

- ✅ "open mailbox" shows contents once, no infinite loop
- ✅ All container operations work correctly
- ✅ Full Mini Zork gameplay test passes
- ✅ No hardcoded object bounds anywhere in iteration code
- ✅ Memory usage remains minimal (no arrays for object contents)