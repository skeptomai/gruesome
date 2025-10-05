# Phase 1 Investigation Findings: Redundant Jump Pattern Analysis

## Summary
Identified the root cause of 32 offset-2 jumps in mini_zork.grue compilation. The pattern is caused by IR generation creating redundant jump sequences with empty else branches in if-statements.

## Pattern Identified

### IR Generation Pattern (ir.rs:1905-1940)
```rust
// If statement with no else branch creates:
Branch { condition, true_label: then_label, false_label: else_label }
Label { id: then_label }
[then_branch statements]
Jump { label: end_label }          // Line 1929
Label { id: else_label }           // Line 1933 - EMPTY else branch
Label { id: end_label }            // Line 1940
```

### Problem
When the else branch is empty (line 1934-1936):
1. Jump to `end_label` is emitted at line 1929
2. `else_label` is emitted at line 1933 (no code)
3. `end_label` is emitted immediately at line 1940 (no code)
4. Both labels defer to the next code-emitting instruction
5. Jump ends up with offset=2 (jumping to instruction right after itself)

### Evidence
- 32 fall-through jumps (offset=2) in mini_zork.grue
- Pattern confirmed: Multiple consecutive deferred labels (e.g., labels 72, 73, 74)
- Example: Jump at PC 0x1028 to target 0x102b (offset 2)
  - Label 73 (else_label) - deferred
  - Label 74 (end_label) - deferred
  - Both resolve to same address as next code-emitting instruction

## Root Cause

**The redundant jump problem occurs in two scenarios:**

1. **If-statement with no else branch** (ir.rs:1934-1936)
   - Empty else branch creates unused `else_label`
   - Jump from then-branch to `end_label` is redundant when followed immediately by `end_label`

2. **If-statement with empty else branch**
   - Similar pattern but else_branch is Some(empty block)

## Solution Approach (Option A)

### Fix Location: src/grue_compiler/ir.rs:1905-1940

**Strategy**: Eliminate redundant jump when then-branch is immediately followed by end_label

### Proposed Fix:
```rust
// After then branch
self.generate_statement(*if_stmt.then_branch, block)?;

// DON'T emit jump to end_label if else branch is empty
// The fall-through will naturally reach end_label
if if_stmt.else_branch.is_some() {
    block.add_instruction(IrInstruction::Jump { label: end_label });
}

// Else branch
block.add_instruction(IrInstruction::Label { id: else_label });
if let Some(else_branch) = if_stmt.else_branch {
    self.generate_statement(*else_branch, block)?;
}

// End label
block.add_instruction(IrInstruction::Label { id: end_label });
```

### Expected Impact:
- Eliminates all 32 offset-2 jumps
- Cleaner IR generation
- Matches standard compiler optimization (don't jump to next instruction)
- More efficient Z-code output

## Files Modified During Investigation:
- `src/grue_compiler/codegen.rs:1496-1511` - Temporarily disabled NOP conversion for testing
- (Restored after investigation)

## Next Steps:
1. **Phase 2**: Implement the IR generation fix in ir.rs
2. **Phase 3**: Validate fix eliminates offset-2 jumps
3. **Phase 4**: Consider removing NOP workaround if no longer needed
