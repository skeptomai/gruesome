# DeferredBranchPatch Usage Pattern Analysis

## PURPOSE
Document all DeferredBranchPatch usage patterns across the codebase to prepare for unified patching system implementation.

## STRUCT DEFINITION
Location: `src/grue_compiler/codegen.rs:203-217`

```rust
#[derive(Debug, Clone)]
pub struct DeferredBranchPatch {
    /// Address where the branch instruction starts in code_space
    pub instruction_address: usize,
    /// Exact byte offset in code_space where the branch offset field is located
    pub branch_offset_location: usize,
    /// IR ID of the target label that this branch jumps to
    pub target_label_id: IrId,
    /// Z-Machine branch polarity: true = branch on true, false = branch on false
    pub branch_on_true: bool,
    /// Size of the branch offset field: 1 or 2 bytes
    pub offset_size: u8,
}
```

## STATE MANAGEMENT
Location: `src/grue_compiler/codegen.rs:235`

```rust
pub struct TwoPassState {
    pub enabled: bool,
    pub deferred_branches: Vec<DeferredBranchPatch>,  // ← Storage
    pub label_addresses: IndexMap<IrId, usize>,
}
```

## CREATION PATTERNS

### Pattern 1: Automatic Creation in emit_instruction_typed
**Location**: `src/grue_compiler/codegen_instructions.rs:2038-2061`

**Trigger**: When all conditions met:
- `self.two_pass_state.enabled` is true
- `opcode.branches()` returns true
- `target_label_id_param.is_some()` (forward reference)

**Creation Logic**:
```rust
// Extract polarity from legacy branch_offset encoding
let branch_on_true = if let Some(offset) = branch_offset {
    (offset as u16) & 0x8000 != 0 // Check bit 15
} else {
    true // Default for non-encoded offsets
};

self.two_pass_state.deferred_branches.push(DeferredBranchPatch {
    instruction_address: start_address,
    branch_offset_location: branch_location,  // ← Calculated during instruction layout
    target_label_id,
    branch_on_true,
    offset_size: 2,  // Always 2 bytes initially
});
```

**Key Properties**:
- **Automatic**: No manual creation required
- **Forward references only**: Only when target label not yet defined
- **Memory location tracking**: `branch_offset_location` calculated during instruction layout
- **Legacy compatibility**: Extracts polarity from existing branch_offset encoding

### Pattern 2: Manual Creation in Tests
**Location**: `src/grue_compiler/two_pass_tests.rs` (multiple locations)

**Usage**: Unit tests directly construct DeferredBranchPatch for verification:
```rust
let patch = DeferredBranchPatch {
    instruction_address: 0x1000,
    branch_offset_location: 0x1003,
    target_label_id: 42,
    branch_on_true: true,
    offset_size: 2,
};
codegen.two_pass_state.deferred_branches.push(patch);
```

**Purpose**: Test branch resolution logic with known values

## RESOLUTION PATTERNS

### Pattern 1: Central Resolution Function
**Location**: `src/grue_compiler/codegen.rs` (resolve_deferred_branches method)

**Process**:
1. Iterate through all `deferred_branches`
2. Look up `target_label_id` in `label_addresses`
3. Calculate branch offset: `target_address - (instruction_address + instruction_length)`
4. Write offset bytes to `branch_offset_location` in `code_space`
5. Handle 1-byte vs 2-byte encoding

**Memory Patching**:
- **Direct code_space access**: `self.code_space[branch_offset_location..] = offset_bytes`
- **No bounds checking coordination**: Operates independently of UnresolvedReference

## USAGE LOCATIONS ACROSS CODEBASE

### Core Generation Files
1. **codegen.rs**: 24 mentions
   - Struct definition and TwoPassState
   - Comments about automatic creation
   - Resolution logic

2. **codegen_instructions.rs**: 3 mentions
   - Primary creation site in emit_instruction_typed
   - Comments about automatic creation

3. **codegen_builtins.rs**: 0 direct mentions
   - Uses emit_instruction_typed which creates DeferredBranchPatch automatically

### Test Files
1. **two_pass_tests.rs**: 18 DeferredBranchPatch struct creations
   - Tests for deferred branch resolution
   - Manual construction for verification

2. **push_pull_branch_integration_tests.rs**: 8 mentions
   - Integration testing with push/pull stack operations
   - Manual construction in test scenarios

3. **unresolved_reference_tests.rs**: 6 mentions
   - Comments about avoiding conflicts with DeferredBranchPatch
   - Recognition of collision potential

## SPECIFIC INSTRUCTION TYPES THAT CREATE DEFERRED PATCHES

Based on comments in codegen.rs:

### Control Flow Instructions
1. **Grammar System Branches**: Lines 6931, 7061, 7158, 7213
   - Handler dispatch chains
   - Object matching logic

2. **Property Access Branches**: Lines 7747, 7803, 7869, 7905
   - Property validation
   - Name matching loops

3. **Builtin Function Branches**: Lines 8444, 9626
   - Input validation
   - Error handling

4. **Complex Logic Branches**: Lines 12963, 13038, 13114, 13279, 13350, 13378
   - Multi-step algorithms
   - Loop constructs

### Key Insight: ALL Branch Instructions
ANY Z-Machine instruction with `opcode.branches() == true` will create a DeferredBranchPatch when:
- Two-pass compilation enabled
- Target label provided
- Target not yet resolved

## MEMORY LOCATION TRACKING

### Branch Offset Location Calculation
**Source**: `codegen_instructions.rs:2057`

The `branch_offset_location` field tracks the EXACT byte offset where the branch parameter will be written:

```rust
branch_offset_location: branch_location,  // From instruction layout
```

**Critical Property**: This is the memory address that DeferredBranchPatch will patch during resolution.

## COLLISION POTENTIAL WITH UNRESOLVEDREFERENCE

### Evidence of Collision Risk
**Source**: `unresolved_reference_tests.rs:4`
```rust
//! to prevent "inscrutable class of bugs" due to conflicts with DeferredBranchPatch.
```

### Root Cause of Collision
1. **DeferredBranchPatch** patches `branch_offset_location` (1-2 bytes)
2. **UnresolvedReference** patches placeholder locations (2 bytes typically)
3. **No coordination**: Both systems operate independently
4. **Overlapping ranges**: If a branch instruction immediately followed by a global reference, their patch locations can overlap

### Example Collision Scenario
```
Address 0x17a6: Branch instruction starts
Address 0x17a7: Branch offset field (DeferredBranchPatch target)
Address 0x17a8: Branch offset field continues
Address 0x17a7: Global reference placeholder (UnresolvedReference target)
Address 0x17a8: Global reference placeholder continues
```
Result: Both systems try to patch bytes 0x17a7-0x17a8, creating invalid bytecode.

## CONCLUSION

DeferredBranchPatch is:
1. **Automatically created** by emit_instruction_typed for branch instructions
2. **Stored** in TwoPassState.deferred_branches Vec
3. **Resolved** by writing offset bytes directly to code_space
4. **Collision-prone** with UnresolvedReference due to independent operation
5. **Memory-unsafe** without coordination with other patching systems

The core architectural problem is that DeferredBranchPatch operates directly on code_space memory locations without any awareness of or coordination with UnresolvedReference patches that may target overlapping locations.