# IR Variable Mapping Analysis - Critical Bug Investigation

## Problem Statement
The Grue compiler has a fundamental mapping issue where IR variable IDs cannot be resolved to proper Z-Machine operands, causing all unknown variables to default to player object (ID 1). This creates semantically incorrect behavior where different objects/variables all resolve to the same value.

## Root Cause Analysis

### What Currently Works
- **Direct object mappings**: Named objects from Grue source code are properly mapped
  ```
  IR ID 28 ('west_of_house') -> Object #2
  IR ID 29 ('mailbox') -> Object #15
  IR ID 30 ('leaflet') -> Object #15
  ```
  Total: 13 direct object mappings created successfully

### What's Broken
- **Intermediate variable mappings**: Variables created by assignments, method calls, and temporary calculations
  ```
  LoadVar requests: IR IDs 83, 105, 162, 196, 243, 262...
  Actual mappings: NONE FOUND -> fallback to player object (1)
  ```

### The Mapping Gap
The codegen tracks two separate types of IR IDs:

1. **Direct Object References** (IR IDs 28-60): Work correctly
   - Source: `ir.symbol_ids` and `ir.object_numbers` from IR generation
   - Storage: `self.ir_id_to_object_number` HashMap
   - Usage: Direct object names in Grue code

2. **Intermediate Variables** (IR IDs 83+): COMPLETELY MISSING
   - Source: StoreVar/LoadVar instructions during execution
   - Storage: NO TRACKING MECHANISM EXISTS
   - Usage: Method results, temporary assignments, calculations

### Evidence of the Problem

#### StoreVar Creates Mappings:
```
StoreVar: copying from IR source 265 to IR var_id 262
StoreVar: copying from IR source 285 to IR var_id 284
```
This shows variables are being created and assigned values.

#### LoadVar Cannot Find Them:
```
LoadVar: IR var_id 262 -> [NOT FOUND] -> fallback to player
LoadVar: IR var_id 284 -> [NOT FOUND] -> fallback to player
```
This shows the assignments are not being tracked for later lookup.

#### Current Incorrect Fallback:
```rust
// In resolve_ir_id_to_operand()
log::debug!(
    "resolve_ir_id_to_operand: IR ID {} assumed to be player object -> LargeConstant(1) [Direct object reference]",
    ir_id
);
Ok(Operand::LargeConstant(1)) // WRONG: Everything becomes player object
```

## Semantic Impact
This bug causes game logic to be completely broken:
- `room.contents()` returns player object instead of room contents
- `container.empty()` returns player object instead of boolean
- Variable assignments are ignored
- All intermediate calculations resolve to the same incorrect value

## Fix Plan

### Phase 1: Track Variable Contents During StoreVar
Modify StoreVar instruction to record what value each variable contains:
```rust
IrInstruction::StoreVar { var_id, source } => {
    let source_operand = self.resolve_ir_id_to_operand(*source)?;
    
    // CRITICAL: Record what this variable now contains
    match source_operand {
        Operand::LargeConstant(value) => {
            self.ir_id_to_integer.insert(*var_id, value as i16);
        }
        Operand::Variable(var_num) => {
            self.ir_id_to_stack_var.insert(*var_id, var_num);
        }
        // Handle other operand types...
    }
    
    // Then execute the actual store instruction...
}
```

### Phase 2: Fix Method Call Result Tracking
Method calls like `room.contents()` need to properly record their results in the appropriate mapping tables.

### Phase 3: Remove Dangerous Fallback
Replace the fallback with compilation errors to force explicit handling:
```rust
// REMOVE: Ok(Operand::LargeConstant(1))
// REPLACE WITH:
Err(CompilerError::CodeGenError(format!(
    "Cannot resolve IR ID {} - no mapping found", ir_id
)))
```

## Expected Outcome
After this fix:
- All IR variable IDs will have explicit mappings
- Method calls will return proper typed results (collections, booleans, objects)
- Variable assignments will be preserved and retrievable
- Compilation errors will catch unmapped variables instead of hiding them
- Game logic will execute with correct semantic values

## Current Status
- **Analysis**: Complete
- **Root cause**: Identified (missing intermediate variable tracking)  
- **Fix plan**: Documented
- **Implementation**: Ready to begin

Date: August 24, 2025