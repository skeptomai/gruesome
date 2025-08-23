# Grue Compiler Built-in Functions Reference

This document catalogs all built-in functions and methods supported by the Grue compiler for Z-Machine interactive fiction development.

## Overview

The Grue compiler implements a **Phase 1 built-in method recognition system** that provides hardcoded implementations for common interactive fiction operations. This approach delivers optimal performance and clean semantics while supporting the core vocabulary needed for text adventure games.

See `METHOD_CALL_ARCHITECTURE.md` for architectural details and future enhancement plans.

## Built-in Functions

These functions can be called directly from Grue code using standard function call syntax: `function_name(args)`.

### Core Game Functions
- `print(text)` - Output text to the player
- `move(object, destination)` - Move objects between locations  
- `get_location(object)` - Get an object's current location

### Z-Machine Object System
- `get_child(object)` - Get first child object
- `get_sibling(object)` - Get next sibling object
- `get_prop(object, property)` - Get object property value
- `test_attr(object, attribute)` - Test object attribute
- `set_attr(object, attribute)` - Set object attribute
- `clear_attr(object, attribute)` - Clear object attribute

### String Utilities
- `to_string(value)` - Convert values to strings
- `indexOf(string, substring)` - Find substring position
- `slice(string, start)` - Extract string portion from start
- `substring(string, start, end)` - Extract string between indices
- `toLowerCase(string)` - Convert to lowercase
- `toUpperCase(string)` - Convert to uppercase
- `trim(string)` - Remove whitespace
- `charAt(string, index)` - Get character at index
- `split(string, delimiter)` - Split string into array
- `replace(string, old, new)` - Replace substrings
- `startsWith(string, prefix)` - Check string prefix
- `endsWith(string, suffix)` - Check string suffix

### Math Functions
- `random(max)` - Generate random numbers
- `abs(number)` - Absolute value
- `min(a, b)` - Minimum of values
- `max(a, b)` - Maximum of values
- `round(number)` - Round to nearest integer
- `floor(number)` - Round down
- `ceil(number)` - Round up

### Type Checking
- `is_string(value)` - Check if value is string
- `is_int(value)` - Check if value is integer
- `is_bool(value)` - Check if value is boolean
- `is_array(value)` - Check if value is array
- `is_object(value)` - Check if value is object
- `typeof(value)` - Get type name

## Built-in Object Methods

These methods can be called on objects using dot notation: `object.method(args)`.

### Container/Room Methods
- `contents()` - Get collection of contained objects
- `empty()` - Check if container is empty
- `none()` - Check if collection is empty (alias for empty)
- `add(object)` - Add object to container

### Event Handler Methods
- `on_look()` - Called when examining location
- `on_exit()` - Called when leaving location  
- `on_enter()` - Called when entering location

## Built-in Array Methods

These methods can be called on arrays using dot notation: `array.method(args)`.

### Basic Operations
- `add(value)` / `push(value)` - Add element to end
- `remove(index)` / `removeAt(index)` - Remove element by index
- `length()` / `size()` - Get array length
- `empty()` / `isEmpty()` - Check if array is empty
- `contains(value)` - Check if array contains value

### Advanced Operations
- `filter(predicate)` - Filter elements by function
- `map(transform)` - Transform each element with function
- `forEach(callback)` - Execute function for each element
- `find(predicate)` - Find first matching element
- `indexOf(value)` - Find index of value
- `join(separator)` - Join elements into string
- `reverse()` - Reverse array order
- `sort(comparator?)` - Sort elements (optional comparator function)

## Usage Examples

### Basic Game Operations
```grue
// Move player to a new location
move(player, west_of_house);

// Print a message
print("Welcome to Zork!");

// Check object properties
if test_attr(door, "locked") {
    print("The door is locked.");
}
```

### Object Methods
```grue
// Get contents of a location
let items = player.location.contents();

// Check if container is empty
if !mailbox.empty() {
    print("The mailbox contains something.");
}

// Event handlers
room kitchen {
    on_enter: {
        print("You smell fresh bread baking.");
    }
}
```

### Array Operations
```grue
let inventory = [];
inventory.add("sword");
inventory.add("key");

if inventory.contains("key") {
    print("You have a key!");
}

let itemNames = inventory.map(item => item.name);
print("Items: " + itemNames.join(", "));
```

### String Processing
```grue
let description = "The ANCIENT door is LOCKED.";
let normalized = toLowerCase(description);

if startsWith(normalized, "the") {
    print("Description starts with 'the'");
}

let words = split(description, " ");
print("Word count: " + words.length());
```

## Implementation Notes

### Performance Characteristics
- **Direct Code Generation**: Built-in functions generate optimized Z-Machine bytecode
- **No Function Call Overhead**: Methods are inlined during compilation
- **Type-Aware Operations**: Each function knows its parameter and return types

### Extensibility Limitations
- **Fixed Vocabulary**: New functions require compiler modification
- **No User-Defined Methods**: Custom methods cannot be added in Grue source
- **No Method Inheritance**: No polymorphic method dispatch

### Future Enhancements
Phase 2 development will add:
- Custom method definition syntax
- Runtime method resolution
- Property-based method storage
- Method inheritance and polymorphism

See `METHOD_CALL_ARCHITECTURE.md` for detailed enhancement roadmap.

## Source Code Locations

- **Function Recognition**: `src/grue_compiler/ir.rs:is_builtin_function()`
- **Method Recognition**: `src/grue_compiler/ir.rs:is_builtin_object_method()`
- **IR Generation**: `src/grue_compiler/ir.rs:generate_builtin_function_call()`
- **Code Generation**: `src/grue_compiler/codegen.rs:generate_builtin_function_call()`
- **Array Methods**: `src/grue_compiler/ir.rs:generate_array_method_call()`

---

*Document created: August 22, 2025*  
*Status: Phase 1 implementation complete*  
*Next Review: When adding new built-in functions or beginning Phase 2 development*