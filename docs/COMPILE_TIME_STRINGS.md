# Compile-Time String Operations

The Grue language supports string manipulation functions, but with an important limitation: **they only work at compile-time with string literals and constants**.

## Why This Limitation?

The Z-Machine (the target platform for Grue games) has no built-in opcodes for string manipulation. The Z-Machine supports:
- Text output (`print`, `print_ret`)
- Text input (`read`)
- Dictionary lookup for parsing
- **No string slicing, concatenation, case conversion, etc.**

## Supported Functions

All these functions work with string literals at compile-time:

### String Search
- `indexOf(string, substring)` → `int` - Find position of substring (-1 if not found)
- `startsWith(string, prefix)` → `bool` - Check if string starts with prefix
- `endsWith(string, suffix)` → `bool` - Check if string ends with suffix

### String Extraction
- `slice(string, start)` → `string` - Extract from index to end
- `substring(string, start, end)` → `string` - Extract between indices
- `charAt(string, index)` → `string` - Get character at index

### String Transformation
- `toLowerCase(string)` → `string` - Convert to lowercase
- `toUpperCase(string)` → `string` - Convert to uppercase  
- `trim(string)` → `string` - Remove leading/trailing whitespace
- `replace(string, search, replacement)` → `string` - Replace occurrences

## Usage Examples

### ✅ Works (Compile-time constants)
```grue
// With string literals
let pos = indexOf("Hello World", "World");        // pos = 6
let sliced = slice("Hello World", 6);             // sliced = "World"
let upper = toUpperCase("hello");                 // upper = "HELLO"

// With compile-time constants
let MESSAGE = "Hello World";
let greeting = slice(MESSAGE, 0, 5);              // greeting = "Hello"
```

### ❌ Doesn't Work (Runtime variables)
```grue
// With runtime variables - these will return default values
let userInput = readLine();                       // Runtime value
let result = slice(userInput, 0, 5);             // Returns empty string
let pos = indexOf(userInput, "test");             // Returns -1

// With function parameters
fn processText(text) {
    let upper = toUpperCase(text);                // Returns empty string
    return upper;
}
```

## Implementation Details

- **Zero bytecode**: String operations generate no Z-Machine instructions
- **Compile-time evaluation**: Results are computed during compilation
- **Fallback behavior**: Runtime operations return safe defaults (empty string, -1, false)
- **Warning messages**: Compiler warns when runtime operations are attempted

## Debug Output

When compiling with `RUST_LOG=debug`, you'll see:
```
StringSlice: compile-time result slice('Hello World', 6) = 'World'
ZERO BYTES: IR instruction generated no bytecode
```

This confirms the operation happened at compile-time and produced no runtime code.

## Alternative Approaches

For runtime string manipulation in Z-Machine games:

1. **Pre-computed strings**: Generate all needed variations at compile-time
2. **Character arrays**: Manipulate text as arrays of characters using Z-Machine opcodes
3. **Dictionary-based**: Use the Z-Machine's built-in dictionary system
4. **Custom routines**: Write Z-Machine assembly for specific operations

## Future Enhancements

Potential improvements while maintaining Z-Machine compatibility:
- More sophisticated compile-time evaluation
- Better error messages for runtime usage
- Template-based string generation
- Integration with Z-Machine dictionary system