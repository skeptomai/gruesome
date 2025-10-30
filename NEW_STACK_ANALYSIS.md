# Z-Machine Stack Imbalance Analysis: "look" Command Investigation

## Stack Operation Count During "look" Command

**Discovered Pattern:**
- "look" command: 6 push operations, 0 pop operations (net +6)
- "open mailbox" direct: 4 push operations, 0 pop operations (net +4)
- "open mailbox" after "look": Works correctly due to +6 stack compensation

## Root Cause: Systematic Stack Discipline Violation

The "look" command is accidentally masking systematic stack imbalance throughout the compiler by leaving 6 extra values on the stack. This explains the state-dependent behavior:

1. **Functions systematically consume more values than they produce**
2. **"look" provides unintentional stack repair (+6 values)**
3. **Subsequent commands work because stack has been "pre-loaded"**
4. **Direct commands fail due to stack underflow**

## Critical Stack vs Locals Usage Guidance (Z-Machine Specification)

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

## The 6 Push Operations in "look" Command

During "look" execution, these push operations occur:
1. `push 1` at PC 0x14b9
2. `push 1` at PC 0x14c7
3. `push 10` at PC 0x14f3
4. `push 10` at PC 0x14b9 (second iteration)
5. `push 10` at PC 0x14c7 (second iteration)
6. `push 1` at PC 0x14f3 (second iteration)

**Problem**: These pushes are never balanced with corresponding pops, violating Z-Machine stack discipline that requires stack to be empty at routine end.

## Function Call Architecture Issues

The logs show multiple function calls with `return_store=Some(0)`, indicating functions are supposed to return values to the stack, but the systematic imbalance suggests:

1. **Function calls are pushing arguments but not properly consuming them**
2. **Return values are being left on stack instead of being consumed**
3. **Local variable usage is bypassed in favor of stack operations**
4. **Stack is being treated as persistent storage rather than temporary evaluation space**

## Required Fixes

1. **Identify the 6 specific push operations in "look" command**
2. **Determine which pushes should have corresponding pops**
3. **Fix function call argument/return value discipline**
4. **Ensure stack is empty at end of each routine per Z-Machine spec**
5. **Use local variables (1-15) for persistent storage, not stack**

## Z-Machine Stack Discipline Compliance

**STACK (Variable 0) MUST be used for:**
1. Function call return values (immediate consumption)
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

## Next Steps

Debug the "look" command specifically to:
1. Find the exact 6 push locations and their purpose
2. Determine proper stack balancing for each push
3. Fix the systematic stack discipline violation
4. Verify stack is empty at routine completion per Z-Machine specification