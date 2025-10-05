# Stack Underflow Root Cause Analysis

## Summary
Stack underflow at PC 0x13a7 caused by **SSA violation** in loop variable code generation. The loop counter IR value is consumed by one instruction but reused by a later instruction, violating single-assignment semantics required by stack-based evaluation.

## Symptom
```
Stack underflow at PC 0x13a7
Instruction: 2OP:store var0 → var0 (0x2d 0x00 0x00)
Stack depth: 0 (expected >= 1)
```

## Root Cause

### IR Pattern (Problematic)
```
Label 425 (loop_start)
  LoadVar { target: 428, var_id: 423 }       // Load loop counter → stack
  LoadImmediate { target: 429, value: 1 }
  BinaryOp { target: 430, op: Less, left: 428, right: 429 }
  Branch { condition: 430, true_label: 426, false_label: 427 }

Label 426 (loop_body)
  GetArrayElement { target: 431, array: 412, index: 428 }  // CONSUMES IR 428
  StoreVar { var_id: 422, source: 431 }
  LoadVar { target: 433, var_id: 16 }
  BinaryOp { target: 434, op: NotEqual, left: 422, right: 433 }
  Branch { condition: 434, true_label: 435, false_label: 436 }

Label 435
  GetPropertyByNumber { target: 439, object: 420, property_num: 29 }
  Branch { condition: 439, true_label: 441, false_label: 442 }

Label 441
  Call { target: Some(440), function: 444, args: [420, 422] }
  Jump { label: 443 }    <- offset-2 jump (converted to NOPs)

Label 442
  LoadImmediate { target: 440, value: Integer(1) }

Label 443
Label 436
Label 437
  LoadImmediate { target: 445, value: Integer(1) }
  BinaryOp { target: 446, op: Add, left: 428, right: 445 }  // ERROR: IR 428 consumed!
  StoreVar { var_id: 423, source: 446 }
  Jump { label: 425 }
```

### The SSA Violation
- **IR ID 428** is created once by `LoadVar { target: 428, var_id: 423 }`
- **First use**: `GetArrayElement { index: 428 }` - This **consumes** the stack value
- **Second use**: `BinaryOp Add { left: 428 }` - Expects value still on stack → **UNDERFLOW**

### Why NOPs Revealed This Bug
1. **Original behavior**: Offset-2 jump at label 441 created infinite loop
   - Execution never reached the `BinaryOp Add`
   - Bug was masked by infinite loop
2. **After NOP conversion**: Jump converted to NOPs
   - Execution continues to `BinaryOp Add`
   - Add tries to pop IR 428 from stack
   - Stack is empty → underflow revealed

## Control Flow Paths

### Path 1: Branch false_label → 436 (loop increment path)
```
Label 426
→ GetArrayElement (consumes IR 428)
→ Branch false_label: 436
→ Label 436
→ Label 437
→ BinaryOp Add (expects IR 428) ← UNDERFLOW
```

### Path 2: Jump from label 441 (also reaches increment)
```
Label 426
→ GetArrayElement (consumes IR 428)
→ Branch true_label: 435
→ Call
→ Jump to 443
→ Label 443 → 436 → 437
→ BinaryOp Add (expects IR 428) ← UNDERFLOW
```

## Fix Strategy

### Option 1: Reload Loop Counter Before Add
Insert `LoadVar` before the Add to reload the loop counter:
```rust
Label 443/436/437
  LoadVar { target: NEW_ID, var_id: 423 }  // Reload loop counter
  LoadImmediate { target: 445, value: Integer(1) }
  BinaryOp { target: 446, op: Add, left: NEW_ID, right: 445 }
  StoreVar { var_id: 423, source: 446 }
```

### Option 2: Fix IR Generation to Use Separate IR IDs
Change loop codegen to create new IR ID for each use:
- IR 428 for Less comparison
- IR 428_b for GetArrayElement index
- IR 428_c for Add operation

### Option 3: Store Loop Counter in Local Variable
Use a true local variable instead of stack-temporary for loop counter.

## Recommended Fix
**Option 1** is simplest and most reliable:
- Minimal change to IR generation
- Ensures SSA property: each use gets fresh load
- Matches Z-Machine stack semantics

## Location to Fix
`src/grue_compiler/ir.rs` - For-loop statement generation
- Find where loop increment is generated
- Insert LoadVar before the BinaryOp Add

## Files Involved
- `/tmp/mini_zork_test.z3` - Compiled output showing the bug
- `examples/mini_zork.grue` - Source code with for-loops (lines 292, 368, 375, 383, 392)
- `src/grue_compiler/ir.rs` - IR generation for for-statements
- `src/grue_compiler/codegen.rs` - Code that converts jump offset-2 to NOPs (revealed the bug)
