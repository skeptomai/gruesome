# Z-Machine Stack vs Local Variable Usage: Specification Analysis

**Date**: August 31, 2025  
**Session**: Mini_zork stack underflow debugging  
**Status**: ✅ SPECIFICATION COMPLIANCE ESTABLISHED  

## Executive Summary

Analysis of mini_zork's "Stack underflow" runtime error revealed systematic misuse of stack vs local variables in the Grue compiler. By examining the Z-Machine specification and actual Zork I disassembly, we established the correct usage patterns and identified the root cause of the stack management crisis.

## Problem Analysis

### Runtime Error Pattern
```
Error during execution: Stack underflow
[2025-08-31T13:45:40Z DEBUG gruesome::opcodes_math] and 0 0
```

**Root Cause**: Excessive stack usage where local variables should be used, leading to stack underflow during complex operations in mini_zork's initialization.

### Compiler Pattern Analysis
Every instruction in mini_zork compilation showed:
```rust
self.emit_instruction(opcode, &operands, Some(0), None)?;  // ❌ Always using stack
self.ir_id_to_stack_var.insert(target, self.stack_depth as u8);  // ❌ Stack mapping
```

This generated excessive stack operations:
- `GetProperty` operations: `store_var: 0` (stack)
- `BinaryOp` operations: `store_var: 0` (stack) 
- Function calls: `store_var: 0` (stack)
- Property operations: `store_var: 0` (stack)

**Impact**: 328 IR operations in mini_zork, nearly all pushing to stack with minimal pops, causing stack underflow.

## Z-Machine Specification Analysis

### Section 6.3: The Stack
> "Writing to the stack pointer (variable number **$00**) pushes a value onto the stack; reading from it pulls a value off."

> "The stack is considered as empty at the start of each routine: it is illegal to pull values from it unless values have first been pushed on."

> "The stack is left empty at the end of each routine: when a return occurs, any values pushed during the routine are thrown away."

### Section 6.4: Routine Calls  
> "Routine calls preserve local variables and the stack (except when the return value is stored in a local variable or onto the top of the stack)."

> "When a routine is called, its local variables are created with initial values... Next, the arguments are written into the local variables (argument 1 into local 1 and so on)."

### Variable Number Encoding
- **Variable 0**: Stack pointer (push/pop mechanism)
- **Variables 1-15**: Local variables (persistent within routine)
- **Variables 16-255**: Global variables (persistent across game)

## Real Zork I Usage Patterns

### Disassembly Evidence
```
Routine R0004, 6 locals (0000, 0000, 0000, 0000, 0000, 0000)
       LOADW           L00,#00 -> L01          ; ✅ Local var for persistent storage
       RANDOM          #64 -> -(SP)            ; ✅ Stack for immediate use  
       JG              L00,(SP)+ [TRUE] RTRUE  ; ✅ Stack consumed immediately
       GET_PROP        (SP)+,#05 -> L03        ; ✅ Property result → LOCAL VARIABLE
```

### Key Patterns in Zork I

**Stack Usage (-> -(SP), (SP)+):**
- `RANDOM #64 -> -(SP)` - Immediate random value
- `JG L00,(SP)+ [TRUE]` - Immediate consumption of stack value
- `ADD L00,#02 -> -(SP)` followed by immediate use
- Function return values used in next instruction

**Local Variable Usage (-> L01, L02, etc.):**
- `LOADW L00,#00 -> L01` - Persistent value storage
- `GET_PROP (SP)+,#05 -> L03` - **Property results stored in locals**
- Function parameters mapped to L00, L01, L02...
- Loop counters and control variables

## Specification-Compliant Usage Rules

### ✅ Stack (Variable 0) - Use For:
1. **Function call return values** consumed immediately
2. **Temporary expression evaluation** (arithmetic, comparisons)  
3. **Immediate consumption values** used by the very next instruction
4. **Push/pop sequences** where value doesn't persist

### ✅ Local Variables (1-15) - Use For:
1. **Function parameters** (per Z-Machine spec 6.4.4)
2. **Property access results** (confirmed by Zork I disassembly)
3. **Object references** that persist across instructions
4. **Values used multiple times** within the same routine
5. **Loop counters and control variables**

### ❌ Never Use Stack For:
- Property access results that persist
- Object references used across multiple operations
- Values that need to survive across instruction boundaries
- Complex expressions where intermediate results are reused

## Evidence from Zork I Disassembly

**Property Operations in Zork I:**
```
GET_PROP        (SP)+,#05 -> L03        ; Property result → Local variable L03
GET_PROP        G76,#07 -> -(SP)        ; Only when used immediately  
GET_PARENT      G6f -> -(SP)            ; Immediate parent lookup
```

**Pattern**: Property results go to **local variables** when they need to persist, **stack** only when consumed immediately.

**Object Operations:**
```
LOADW           L00,#00 -> L01          ; Object reference → Local variable
STORE           L02,G00                 ; Object stored in local for reuse
INSERT_OBJ      G6f,G76                 ; Objects referenced from locals/globals
```

**Arithmetic Operations:**
```
ADD             L00,#02 -> -(SP)        ; Stack when result used immediately
ADD             L00,(SP)+ -> L05        ; Result → Local when persistent
MUL             L02,#02 -> -(SP)        ; Stack for immediate consumption
```

## Compiler Architecture Implications

### Current Broken Pattern
```rust
// ❌ WRONG: Always using stack
self.emit_instruction(0x11, &[obj_operand, prop_operand], Some(0), None)?;
self.ir_id_to_stack_var.insert(target, self.stack_depth as u8);
```

### Specification-Compliant Pattern  
```rust  
// ✅ CORRECT: Use local variables for persistent results
let local_var = self.allocate_local_variable();
self.emit_instruction(0x11, &[obj_operand, prop_operand], Some(local_var), None)?;
self.ir_id_to_local_var.insert(target, local_var);
```

### Required Changes

**High Priority - Property Operations:**
- `translate_get_property()` - Use local variables
- `translate_get_property_by_number()` - Use local variables  
- `translate_set_property()` - Operand resolution may need locals

**Medium Priority - Object Operations:**
- Object reference loading - Use locals for persistent references
- Binary operations with persistent results - Use locals
- Function calls with stored results - Use locals appropriately

**Low Priority - Expression Evaluation:**
- Complex expressions - Mixed usage based on consumption pattern
- Temporary calculations - Keep stack for immediate use

## Testing Strategy

### Validation Approach
1. **Simple Tests**: Verify basic operations still work (test_01_basic.z3 ✅)
2. **Property Tests**: Test property access with local variables
3. **Mini_zork**: Full complex game with proper stack/local balance
4. **Stack Monitoring**: Add logging to track stack depth during execution

### Success Criteria
- ✅ mini_zork compiles without placeholder errors
- ✅ mini_zork runs without "Stack underflow" errors
- ✅ Property operations work correctly  
- ✅ Object manipulation functions properly
- ✅ Stack remains balanced throughout execution

## Implementation Status

**Analysis Phase**: ✅ COMPLETE
- Stack underflow root cause identified
- Z-Machine specification consulted
- Zork I disassembly patterns analyzed
- Specification-compliant rules established

**Implementation Phase**: 🔄 READY TO PROCEED
- Property operation fixes prepared
- Local variable allocation system available
- Target: Fix GetProperty, GetPropertyByNumber, and related operations

**Verification Phase**: ⏳ PENDING
- Test mini_zork execution after fixes
- Validate stack balance in complex scenarios

## Reference Documentation

- **Z-Machine Specification**: `/Users/cb/Projects/Z-Machine-Standard/sect06.html`
- **Zork I Disassembly**: Generated via `gruedasm-txd` showing real usage patterns
- **CLAUDE.md**: Contains previous architectural guidance on stack vs locals
- **Current Session**: Complete analysis of mini_zork stack underflow issue

## Conclusion

The stack underflow issue in mini_zork is a **specification compliance problem**. Our compiler was using stack variables for persistent storage when the Z-Machine specification and real-world evidence (Zork I) clearly show that property access results and persistent values should use local variables.

The fix is straightforward: modify property operations and other persistent value instructions to use `allocate_local_variable()` and `ir_id_to_local_var` instead of stack mappings. This aligns our compiler with both the specification and proven working patterns from actual Infocom games.