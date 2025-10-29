# Z-Machine Specification Analysis: Stack vs Locals for Store Instruction

## Variable Number Ranges (sect06.html, sect04.html)
- **$00**: Stack pointer - reading pulls value off, writing pushes value on
- **$01 to $0F**: Local variables (1-15) - routine-specific, preserved across calls
- **$10 to $FF**: Global variables (16-255) - stored in dynamic memory table

## Store Instruction Definition (sect15.html)
```
2OP:13   D     store (variable) value
Set the variable referenced by the operand to value.
```

**Key insight**: Store is an "indirect variable reference" instruction - it takes the target variable NUMBER as an operand, not a fixed variable slot.

## Critical Stack vs Locals Usage Guidance (sect06.html)

### Stack ($00) - Section 6.3:
- "Writing to the stack pointer pushes a value onto the stack; reading from it pulls a value off"
- "The stack is considered as empty at the start of each routine"
- "The stack is left empty at the end of each routine: when a return occurs, any values pushed during the routine are thrown away"
- **Stack discipline**: Values are temporary within routine scope

### Local Variables ($01-$0F) - Section 6.4:
- "Routine calls preserve local variables"
- "When a routine is called, its local variables are created with initial values"
- "Arguments are written into the local variables (argument 1 into local 1 and so on)"
- **Persistent scope**: Locals survive across the entire routine call

### Special Stack Behavior for Store - Section 6.3.4:
- "In the seven opcodes that take indirect variable references (inc, dec, inc_chk, dec_chk, load, **store**, pull), an indirect reference to the stack pointer does not push or pull the top item of the stack - it is read or written in place."

## Root Cause Analysis for Our Stack Underflow

### Problem:
Our compiler is using `emit_instruction(0x0D, [SmallConstant(0)], Some(239), None)` which:
1. Stores Variable(239) **TO** Variable(0) (stack pointer)
2. This writes **in place** to stack top (doesn't push new value)
3. But Variable(239) isn't necessarily on the stack to begin with!

### Z-Machine Store semantics:
`store (variable) value` means:
- **First operand** = target variable NUMBER to store TO
- **Second operand** = source value to store FROM
- Our code: `store 0 239` = "store Variable(239)'s value into Variable(0)"

### The Correct Approach:
Instead of `store 0 239` (storing TO stack), we should be using:
- `store 239 0` = "store Variable(0)'s value into Variable(239)"
- OR better: `store 1 0` = "store Variable(0)'s value into Local Variable 1"

### Why Zork I doesn't have Store instructions:
Zork I was hand-optimized Z-Machine code that directly manipulated local variables and globals without intermediate storage to stack. Our compiler generates intermediate stack operations that then need to be moved to persistent storage - a pattern that requires Store instructions.

### The Real Fix:
Our get_exit builtin should store the loop counter in a LOCAL variable (1-15), not try to store TO the stack. The current approach violates Z-Machine stack discipline by treating the stack as persistent storage rather than temporary expression evaluation space.

## Z-Machine Stack vs Local Variable Specification Compliance

**STACK (Variable 0) MUST be used for:**
1. Function call return values
2. Function call arguments (before moving to locals)
3. Immediate consumption values
4. Expression evaluation
5. **NEVER for persistent storage across multiple operations**

**LOCAL VARIABLES (1-15) are for:**
1. Function parameters (after moved from stack)
2. Persistent variables within routine scope
3. User-declared variables
4. Loop counters
5. **Intermediate values that need to persist across multiple Z-Machine instructions**

## Critical Discovery: Store Instruction Operand Order

Our current implementation has the operands **backwards**:
- Current: `store target_var source_var`
- Z-Machine spec: `store (variable) value` = first operand is target variable NUMBER

**Example of correct Store usage:**
```
store 1 0     ; Store stack top (Variable 0) into Local 1 (Variable 1)
store 17 5    ; Store constant 5 into Global G01 (Variable 17)
store local_var_num stack_value  ; General pattern
```

**Our bug**: We're doing `store 0 239` which stores Global Variable 239 into the stack top in-place, violating stack discipline by not actually pushing/pulling values.

## Action Required

1. **Fix Store instruction operand order** in get_exit builtin
2. **Use local variables (1-15) for persistent storage** instead of trying to store TO stack
3. **Reserve stack (Variable 0) only for immediate expression evaluation** per Z-Machine specification
4. **Update all builtin functions** that violate stack discipline by using stack for persistent storage