# Grue Method Call Architecture Analysis

## Problem Statement

The Grue compiler has a fundamental architectural issue with method calls. The `contents()` method call syntax (`object.method()`) is correctly parsed as a MethodCall AST node, but the IR generation is incomplete, leading to runtime crashes when trying to call invalid function addresses.

## Root Cause Analysis

### Parser Behavior (Correct)
- `object.property` → PropertyAccess AST node
- `object.method()` → MethodCall AST node

### IR Generation Problem
Located in `src/grue_compiler/ir.rs` lines 1954-2027:

```rust
Expr::MethodCall { object, method, arguments } => {
    // ... generates GetProperty instruction
    // ... creates conditional branching logic
    // Line 2008: TODO: Implement indirect function call via property value
    // Currently just returns 0 instead of making the call
}
```

### Runtime Failure
1. Property `contents` (property #4) contains 0 or invalid addresses
2. When runtime tries to call these as functions, jumps to null memory (0x0068→0x0069)
3. Results in "Invalid Long form opcode 0x00" crash at address 0x0069

## Architectural Solutions Considered

### Option 1: Make it PropertyAccess
**Approach**: Change `location.contents()` to `location.contents`
- **Pros**: Simple, consistent with Z-Machine property model
- **Cons**: Breaks intuitive `()` syntax for computed collections

### Option 2: Complete MethodCall Implementation
**Approach**: Implement proper indirect function calls via property values
- **Pros**: Maintains intuitive syntax, supports true methods
- **Cons**: Complex, requires Z-Machine function pointer support

### Option 3: Built-in Method Recognition (CHOSEN)
**Approach**: Recognize common methods as built-in operations
- **Pros**: Clean semantics, optimal performance, matches existing array method pattern
- **Cons**: Limited extensibility (see detailed analysis below)

## Current Implementation Decision

**Chosen Solution**: Option 3 - Built-in Method Recognition

This approach extends the existing pattern in `ir.rs` where array methods like `add()` and `length()` are recognized and generate specialized IR instructions.

### Implementation Pattern
```rust
// Extend existing code in ir.rs around line 1966
if is_array {
    return self.generate_array_method_call(object_temp, &method, &arguments, block);
}

// NEW: Add built-in object method recognition
if self.is_builtin_object_method(&method) {
    return self.generate_builtin_object_method_call(
        object_temp, &method, &arguments, block
    );
}
```

### Built-in Methods to Implement
- `contents()` - Return collection of contained objects
- `empty()` - Check if container is empty
- `add(object)` - Add object to container (for containers)
- `remove(object)` - Remove object from container

## Extensibility Limitations

### 1. Hard-coded Method Set
Every new method must be explicitly coded into the compiler. Users can't define custom methods in Grue source code.

```grue
// This would be impossible:
object MyContainer {
    fn custom_search(criteria) -> Object {
        // Custom logic here
    }
}
```

### 2. No User-Defined Methods
All methods must be anticipated by compiler developers. No dynamic method definition.

### 3. No Method Inheritance/Polymorphism
```grue
// This kind of polymorphic behavior is impossible:
trait Searchable {
    fn find(criteria) -> Object;
}

// Polymorphic call - impossible with built-in recognition:
fn search_location(searchable: Searchable, criteria) {
    return searchable.find(criteria); // Can't dispatch dynamically
}
```

### 4. Fixed Method Signatures
Built-in methods have rigid signatures with no overloading or optional parameters.

### 5. Type-Specific Limitations
Hard to customize behavior per object type without complex type checking:
```grue
room.contents()      // Should return objects in room
container.contents() // Should return objects in container  
player.contents()    // Should return inventory items
```

### 6. No Method Composition
Method chaining becomes impossible:
```grue
// This kind of method chaining becomes impossible:
player.location
    .contents()
    .filter(valuable: true)
    .sort_by("weight")
    .first()
```

## Future Enhancement Plan

**Phase 2**: Implement Custom Method Dispatch

Once the built-in method system is stable and the immediate crash is resolved, we will implement a full method dispatch system:

1. **Property-Based Method Storage**: Store method function addresses in object properties
2. **Runtime Method Resolution**: Implement the TODO at ir.rs line 2008
3. **Method Definition Syntax**: Allow user-defined methods in Grue source
4. **Type System Integration**: Support method inheritance and polymorphism
5. **Performance Optimization**: JIT compilation or method caching for frequently-called methods

### Implementation Roadmap
- [ ] Phase 1: Built-in method recognition (current)
  - [ ] Implement `contents()` method
  - [ ] Implement `empty()` method
  - [ ] Add method type checking
- [ ] Phase 2: Custom method dispatch
  - [ ] Complete indirect function call implementation
  - [ ] Add method definition syntax
  - [ ] Implement method property storage
  - [ ] Add runtime method resolution

## Why This Approach?

Despite extensibility limitations, built-in method recognition is chosen because:

1. **Immediate Problem Resolution**: Fixes current crash without architectural overhaul
2. **Performance**: Direct code generation is more efficient than function pointer dispatch
3. **Simplicity**: Easier to implement, debug, and maintain
4. **Sufficient for Current Needs**: IF games need a curated set of operations
5. **Incremental Path**: Can evolve to support user methods later

For interactive fiction games, a well-designed set of built-in collection and object methods may be more valuable than full OOP extensibility.

## Files Modified

- `src/grue_compiler/ir.rs` - Method call IR generation
- `src/grue_compiler/codegen.rs` - Method call bytecode generation
- `examples/mini_zork.grue` - Test cases using `contents()` method

## Related Issues

- Original crash: "Invalid Long form opcode 0x00 at address 0069"
- Addresses involved: 0x0068 (call target), 0x0069 (invalid instruction)
- Function context: `list_objects` function in mini_zork
- Property number: `contents` assigned property #4

---

*Document created: August 21, 2025*  
*Status: Phase 1 implementation in progress*