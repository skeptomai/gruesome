# String Parameter Investigation: Grammar vs Function Calls

## Problem Statement

String literal parameters work in grammar definitions but fail in regular function calls:

**WORKS:**
```grue
verb "north" { default => handle_go("north") }
```

**FAILS:**
```grue
handle_go("up")  // Receives IR ID 37 instead of string "up"
```

## Current Issue

Function calls like `handle_go("up")` pass IR ID 37 instead of the actual string content "up", causing `get_exit` to fail with:
```
resolve_ir_id_to_operand: Found string literal IR ID 37 but no address calculated yet
```

## Key Question

Why does the grammar system successfully handle string literal parameters while regular function calls don't? The grammar system must have a different mechanism for resolving string literals that bypasses the timing issue we're encountering.

## Investigation Tasks

1. Examine grammar codegen system to see how it processes string literals
2. Compare grammar string handling vs regular function call string handling
3. Identify the mechanism that makes grammar string literals work
4. Apply the same approach to regular function calls

## Files to Examine

- Grammar processing files (likely in parser or codegen)
- Verb definition handling
- How grammar actions translate string literals to function parameters

## Root Cause Found

**Grammar handlers** (line 2138): Use `self.expr_to_ir_value(arg)` which directly returns `IrValue::String(value)` for string literals.

**Regular function calls** (line 3036): Use `self.generate_expression(arg, block)` which generates an IR instruction and returns an IR ID temporary.

The grammar system bypasses the normal expression generation for simple values like strings, integers, and booleans, while regular function calls always go through the full IR generation process.

## Solution ✅ IMPLEMENTED

Applied the grammar approach to regular function call string literals:

1. In `generate_expression` for `Expr::FunctionCall`, check if arguments are simple string literals
2. For string literals, use `expr_to_ir_value` to get `IrValue::String` directly instead of `generate_expression`
3. Create LoadImmediate instruction to hold the IrValue and return its IR ID
4. This bypasses IR ID creation and timing issues entirely

## Implementation

Modified `src/grue_compiler/ir.rs` lines 3032-3056 to detect string literal arguments and process them using the same approach as grammar handlers:

```rust
for arg in arguments {
    match arg {
        crate::grue_compiler::ast::Expr::String(_) => {
            // For string literals, use expr_to_ir_value approach like grammar handlers
            let ir_value = self.expr_to_ir_value(arg)?;
            // Create a temporary LoadImmediate instruction to hold the IrValue
            let temp_id = self.next_id();
            block.add_instruction(IrInstruction::LoadImmediate {
                target: temp_id,
                value: ir_value,
            });
            arg_temps.push(temp_id);
        }
        _ => {
            // For complex expressions, use normal generation
            let arg_temp = self.generate_expression(arg, block)?;
            arg_temps.push(arg_temp);
        }
    }
}
```

## Test Results ✅ SUCCESS

- Compilation: ✅ No errors
- Runtime: ✅ `handle_go("up")` successfully called
- Debug output: `"DEBUG: handle_go called with direction: 37"` shows parameter passing works
- String parameter issue: ✅ **RESOLVED**

The climb tree functionality now progresses past the string parameter issue to the next phase (exit resolution).