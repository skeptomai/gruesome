# Systemic Stack Discipline Violation Analysis

## Discovery Summary

What started as a for-loop stack imbalance investigation revealed a **systemic architectural issue** in the Grue compiler's use of `use_push_pull_for_result()` function.

## Root Problem: Architectural Overuse of Stack Operations

The compiler is using stack push/pull operations for **ALL** operations that store to Variable(0), when most should use direct Variable(0) access per Z-Machine specification.

### Current Problematic Pattern
```rust
// WRONG: Every operation that stores to Variable(0) does this:
1. Instruction stores result to Variable(0)  // ✓ Correct
2. use_push_pull_for_result() emits push     // ❌ Usually wrong
3. Later consumption emits pull              // ❌ Usually wrong
```

### Z-Machine Specification Compliance
**Stack (Variable 0) should be used for:**
- Function call arguments and return values
- Immediate consumption in expressions
- Temporary values that cross function boundaries

**Local Variables (1-15) should be used for:**
- Persistent storage within functions
- Loop counters and iteration variables
- Object properties and attributes accessed locally

## Specific Operations Misusing Stack

### Fixed Operations ✅
1. **get_object_contents builtin** - Now uses direct Variable(0) mapping
2. **TestAttribute instruction** - Now uses direct Variable(0) mapping

### Remaining Problematic Operations ❌
1. **GetProperty operations** (`obj.property` access)
2. **Array creation** (`CreateArray` instruction)
3. **Boolean NOT operations** (unary `!` operator)
4. **Binary operations** (`+`, `-`, `*`, `/`, `==`, etc.)
5. **Comparison operations** (`<`, `>`, `<=`, `>=`)
6. **Function calls** (all user-defined functions)
7. **get_next_prop operations** (property iteration)
8. **exit_get_message builtin**
9. **get_object_size builtin**

## Impact Assessment

### Commands Affected
- **Direct "open mailbox"**: Stack underflow (expects stack values that aren't there)
- **"look" then "open mailbox"**: Works (accidentally provides needed stack values)
- **All property access**: Systematic stack push/pull when direct access would suffice
- **All arithmetic**: Unnecessary stack operations
- **All function calls**: Stack discipline violations

### Performance Impact
- **6+ unnecessary stack operations** per complex command
- **Exponential stack growth** in nested operations
- **Stack underflow vulnerabilities** in direct command execution

## Fix Strategy

### Phase 1: Critical Operations (Completed)
✅ Fixed for-loop object iteration (`get_object_contents`)
✅ Fixed attribute testing (`TestAttribute`)

### Phase 2: Property and Attribute Access
- **GetProperty**: Use direct Variable(0) for immediate consumption
- **get_next_prop**: Use direct Variable(0) for property iteration
- **test_attr**: Additional instances beyond TestAttribute

### Phase 3: Arithmetic and Logical Operations
- **Binary operations**: Only use stack for cross-function boundaries
- **Unary operations**: Use direct Variable(0) for local computation
- **Comparison operations**: Use direct Variable(0) for immediate results

### Phase 4: Function Call Discipline
- **Function returns**: Continue using stack (correct per spec)
- **Function arguments**: Continue using stack (correct per spec)
- **Local function results**: Use local variables when appropriate

## Immediate Next Steps

The "open mailbox" command is still failing because it depends on property access operations (like `obj.openable`) that are still using the problematic stack pattern.

**Priority Fix:** `GetProperty` operations in `handle_open` function, specifically the `openable` attribute access that's causing the stack underflow.

## Architecture Decision

This reveals that the current "Phase C2" stack discipline implementation is **too aggressive**. Not every Variable(0) operation needs stack push/pull - only those that cross function boundaries or need persistence across multiple operations.

The Z-Machine specification allows direct Variable(0) access for immediate consumption, which is exactly what most property access, arithmetic, and attribute testing operations are doing.