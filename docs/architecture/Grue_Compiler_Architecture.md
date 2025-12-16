# Grue Z-Machine Compiler Architecture Guide

## Overview

The Grue Z-Machine Compiler is a complete implementation of a domain-specific language compiler that translates Grue source code into executable Z-Machine bytecode. This document provides a comprehensive overview of the compiler's architecture, design decisions, and implementation details for newcomers to the project.

## What is the Z-Machine?

The Z-Machine is a virtual machine originally developed by Infocom for their interactive fiction games (like Zork). It provides:
- Platform-independent bytecode execution
- Sophisticated text handling and encoding
- Object-oriented programming features
- Built-in parser and dictionary systems
- Memory management and save/restore capabilities

## Project Structure

```
src/grue_compiler/
├── mod.rs              # Module declarations and public API
├── lexer.rs           # Tokenization and lexical analysis
├── parser.rs          # Syntax analysis and AST generation
├── ast.rs             # Abstract Syntax Tree definitions
├── semantic.rs        # Semantic analysis and type checking
├── ir.rs              # Intermediate Representation
├── codegen.rs         # Z-Machine bytecode generation
├── error.rs           # Error handling and reporting
└── tests/             # Comprehensive test suites
```

## Compilation Pipeline

The Grue compiler follows a traditional multi-phase compilation approach:

```
Grue Source Code
       ↓
   [Lexer] → Tokens
       ↓
   [Parser] → Abstract Syntax Tree (AST)
       ↓
   [Semantic Analyzer] → Validated AST + Symbol Tables
       ↓
   [IR Generator] → Intermediate Representation
       ↓
   [Code Generator] → Z-Machine Bytecode
       ↓
   Z-Machine Story File (.z3/.z4/.z5)
```

## Phase-by-Phase Architecture

### Phase 1: Lexical Analysis (`lexer.rs`)

**Purpose**: Convert raw source text into a stream of meaningful tokens.

**Key Components**:
- `Token` enum: Defines all language tokens (keywords, operators, literals, identifiers)
- `Lexer` struct: State-driven tokenizer with lookahead capabilities
- Position tracking for error reporting

**Design Decisions**:
- Single-character lookahead for operator disambiguation
- Unicode support for string literals
- Comprehensive error recovery

**Example Token Types**:
```rust
pub enum Token {
    // Keywords
    Fn, Let, If, Else, While, For, Return,
    Room, Object, Grammar, Init,
    
    // Literals
    StringLiteral(String),
    IntegerLiteral(i32),
    BooleanLiteral(bool),
    
    // Operators
    Plus, Minus, Star, Slash, Equal, EqualEqual,
    
    // Identifiers and special tokens
    Identifier(String),
    Eof,
}
```

### Phase 2: Syntax Analysis (`parser.rs`, `ast.rs`)

**Purpose**: Parse token stream into a structured Abstract Syntax Tree.

**Key Components**:
- `Parser` struct: Recursive descent parser with error recovery
- `ASTNode` variants: Comprehensive AST representation
- Precedence climbing for expression parsing

**AST Structure**:
```rust
pub enum ASTNode {
    Program { items: Vec<ASTNode> },
    FunctionDef { name: String, params: Vec<Parameter>, body: Box<ASTNode> },
    Room { name: String, description: String, exits: Vec<Exit> },
    Object { names: Vec<String>, properties: Vec<Property> },
    Grammar { patterns: Vec<GrammarPattern> },
    // ... expressions, statements, etc.
}
```

**Design Decisions**:
- Recursive descent parsing for clarity and maintainability
- Rich AST nodes with position information for error reporting
- Separate handling of world elements (rooms, objects) vs code elements

### Phase 3: Semantic Analysis (`semantic.rs`)

**Purpose**: Validate program semantics, perform type checking, and build symbol tables.

**Key Components**:
- `SemanticAnalyzer`: Main analysis engine
- `Scope` hierarchy: Nested scoping (Global → Function → Block → Room)
- `Type` system: Strong typing with selective implicit conversions
- `SymbolTable`: Tracks variables, functions, rooms, objects

**Type System**:
```rust
pub enum Type {
    Any,        // Gradual typing support
    Bool,       // Boolean values
    Int,        // Integer values  
    String,     // Text strings
    Room,       // Game rooms
    Object,     // Game objects
    Array(Box<Type>),  // Homogeneous arrays
}
```

**Analysis Phases**:
1. **Symbol Collection**: Gather all global declarations (functions, rooms, objects)
2. **Reference Validation**: Ensure all identifiers are defined and accessible
3. **Type Checking**: Verify type compatibility in expressions and assignments
4. **Control Flow Analysis**: Validate boolean conditions in if/while/ternary expressions

**Design Philosophy**:
- Pragmatic type system: strict enough to catch errors, flexible enough for game development
- Hierarchical scoping with proper variable shadowing
- Built-in functions and variables automatically available
- Duck typing for object property access

### Phase 4: IR Generation (`ir.rs`)

**Purpose**: Transform validated AST into a lower-level intermediate representation.

**Key Components**:
- `IrInstruction` enum: Platform-independent instruction set
- `IrProgram`: Complete program representation with metadata
- `IrId` system: Unique identifiers for values and targets

**IR Instruction Set**:
```rust
pub enum IrInstruction {
    // Data movement
    LoadImmediate { target: IrId, value: IrValue },
    LoadVar { target: IrId, var_name: String },
    StoreVar { var_name: String, source: IrId },
    
    // Arithmetic and logic
    BinaryOp { target: IrId, op: BinaryOperator, left: IrId, right: IrId },
    UnaryOp { target: IrId, op: UnaryOperator, operand: IrId },
    
    // Control flow
    Branch { condition: IrId, true_label: String, false_label: String },
    Jump { label: String },
    Label { name: String },
    
    // Function calls
    Call { target: Option<IrId>, function: IrId, args: Vec<IrId> },
    Return { value: Option<IrId> },
    
    // Object system
    GetProperty { target: IrId, object: IrId, property: String },
    SetProperty { object: IrId, property: String, value: IrId },
}
```

**Design Benefits**:
- Platform-independent representation
- Easy optimization opportunities
- Clean separation from AST complexity
- Forward reference resolution through two-pass processing

### Phase 5: Code Generation (`codegen.rs`)

**Purpose**: Transform IR into executable Z-Machine bytecode.

**Key Components**:
- `CodeGenerator`: Main bytecode emission engine
- Memory layout management (header, globals, objects, dictionary, strings, code)
- Address resolution and reference patching
- Z-Machine version compatibility (v3/v5)

**Z-Machine Memory Layout**:
```
┌─────────────────┐ 0x0000
│   Header (64B)  │
├─────────────────┤ 0x0040
│  Globals (480B) │ 
├─────────────────┤ 0x0220
│  Object Tables  │
├─────────────────┤ Variable
│   Dictionary    │
├─────────────────┤ Variable
│     Strings     │ (Even-aligned in v3)
├─────────────────┤ Variable  
│      Code       │ (4-byte aligned in v5)
└─────────────────┘
```

**Code Generation Process**:
1. **Memory Layout**: Calculate addresses for all program sections
2. **String Processing**: Encode strings using ZSCII format with deduplication
3. **Instruction Translation**: Convert IR instructions to Z-Machine opcodes
4. **Address Resolution**: Patch forward references and jump targets
5. **Header Generation**: Create proper Z-Machine header with version info

**Z-Machine Instruction Mapping**:
```rust
// Example: Binary addition
IrInstruction::BinaryOp { op: BinaryOperator::Add, .. } 
    → Z-Machine: ADD opcode (0x14) with operand encoding

// Example: Function call  
IrInstruction::Call { function, args, .. }
    → Z-Machine: CALL_VS opcode (0xE0) with packed address

// Example: Print statement
builtin print(string) 
    → Z-Machine: PRINT_PADDR opcode (0xB3) with string address
```

**Version Compatibility**:
- **v3**: 6 Z-characters per word, even-aligned strings, 255 objects max
- **v5**: 9 Z-characters per word, 4-byte aligned routines/strings, 65535 objects max

### Object Numbering System

The Grue compiler uses a dual numbering system that distinguishes between internal compiler IDs and final Z-Machine object numbers. Understanding this distinction is crucial for developers working on object-related features.

#### Two Independent Numbering Systems

**1. IR ID (Internal Compiler Use)**
- **Purpose**: Internal tracking within the IR (Intermediate Representation) system
- **Range**: 32-bit unsigned integers (0 to 4,294,967,295)
- **Assignment**: Based on source code order and compiler-generated objects
- **Example**: User objects get IDs 1-100, built-in objects use high IDs (9999+)

**2. Z-Machine Object Number (Final Bytecode)**
- **Purpose**: Actual object references used in Z-Machine bytecode
- **Range**: 8-bit unsigned integers (1 to 255 for v3, 1 to 65535 for v5)
- **Assignment**: Sequential starting from 1, following Z-Machine conventions
- **Example**: Player=1, First room=2, Second room=3, etc.

#### Critical Architectural Decision: Player Object Creation

The compiler automatically creates a player object as **Z-Machine object #1** to ensure compatibility with Z-Machine conventions and external interpreters like Frotz:

```rust
// Player object gets high IR ID to avoid conflicts
all_objects.push(ObjectData {
    id: 9999u32, // IR ID: arbitrary high number
    name: "player".to_string(),
    short_name: "yourself".to_string(),
    properties: player_properties,
    // ... other fields
});

// But becomes Z-Machine object #1 through sequential mapping
for (index, object) in all_objects.iter().enumerate() {
    let obj_num = (index + 1) as u8; // Player: index=0 → obj_num=1
    object_id_to_number.insert(object.id, obj_num);
}
```

#### Why This Matters

This dual numbering system solves several critical problems:

**1. Frotz Compatibility**: External Z-Machine interpreters expect object #1 to exist when property access occurs. Without a player object, `get_prop` calls fail with "object 0" errors.

**2. Z-Machine Convention**: Interactive fiction traditionally uses object #1 for the player character, object #2+ for rooms and items.

**3. IR Flexibility**: High IR IDs (9999+) for built-in objects prevent conflicts with user-defined objects that get lower IDs based on source code order.

**4. Reference Resolution**: During compilation, `player.location` references are resolved:
   - IR: `player` (IR ID 9999) → property access instruction
   - Z-Machine: Object #1 → `get_prop 1, property_number`

#### Object Creation Sequence

1. **Player Object**: Created first with IR ID 9999 → becomes Z-Machine object #1
2. **Rooms**: Added from IR in order → become Z-Machine objects #2, #3, #4...
3. **Regular Objects**: Added from IR in order → continue sequential numbering
4. **Mapping Table**: Built to translate IR IDs to Z-Machine object numbers
5. **Bytecode Generation**: All object references use final Z-Machine numbers

This architecture ensures both internal compiler flexibility and external Z-Machine compatibility.

## Advanced Features

### Two-Pass Compilation

The compiler uses a sophisticated two-pass system:

1. **First Pass**: Generate IR and collect all symbols/addresses
2. **Second Pass**: Resolve forward references and emit final bytecode

This enables:
- Forward function declarations
- Complex control flow with loops and conditionals
- Proper address calculation for strings and routines

### String Management

Strings are handled with special care:
- **ZSCII Encoding**: Z-Machine text format (3 chars per 16-bit word)
- **Deduplication**: Identical strings share storage
- **Packed Addresses**: Efficient referencing system
- **Alignment**: Platform-specific alignment requirements

### Builtin Function System

The compiler provides essential Z-Machine primitives:
- `print(string)`: Text output using Z-Machine PRINT_PADDR
- `move(object, destination)`: Object manipulation using INSERT_OBJ  
- `get_location(object)`: Parent lookup using GET_PARENT

Smart detection distinguishes builtin vs user-defined functions at compile time.

### Error Handling

Comprehensive error reporting throughout all phases:
- Position tracking for precise error location
- Descriptive error messages with context
- Graceful recovery where possible
- Structured error types for different phases

## Testing Architecture

The compiler includes extensive test coverage:

### Unit Tests by Phase:
- **Lexer**: Token recognition and position tracking
- **Parser**: AST generation and error recovery  
- **Semantic**: Type checking and scope validation
- **IR Generation**: Instruction sequence correctness
- **Code Generation**: Bytecode output validation

### Integration Tests:
- **Golden Files**: Complete compilation of sample programs
- **End-to-End**: Generated bytecode execution in Z-Machine interpreter
- **Compatibility**: Both v3 and v5 Z-Machine formats

### Test Programs:
```grue
// basic_test.grue - Simple program with functions
fn test_function() {
    print("Hello from test function!");
}

init {
    print("Basic Grue Compiler Test");
    test_function();
}

// mini_zork.grue - Game-like program with rooms and objects
room living_room {
    description: "A cozy living room"
    exits: { north: kitchen }
}

object lamp {
    names: ["lamp", "light"]
    location: living_room
}
```

## Performance Considerations

### Compilation Speed:
- Single-pass lexing and parsing
- Efficient symbol table implementation using HashMap
- Minimal memory allocation during IR generation

### Generated Code Quality:
- Direct IR-to-bytecode mapping (no unnecessary optimization overhead)
- Proper Z-Machine instruction selection
- Efficient string and memory layout

### Memory Usage:
- Streaming lexer (doesn't store entire source)
- AST nodes freed after IR generation
- Compact IR representation

## Extension Points

The architecture provides clear extension opportunities:

### Language Features:
- Additional data types (floats, structs)
- Advanced control flow (switch, try/catch)
- Object-oriented features (inheritance, methods)

### Code Generation:
- Optimization passes on IR
- Multiple backend targets (beyond Z-Machine)
- Debug information generation

### Analysis:
- Static analysis warnings
- Dead code elimination
- Unused variable detection

## Design Philosophy

### Clarity Over Cleverness:
- Straightforward recursive descent parsing
- Clear phase separation
- Descriptive naming and documentation

### Robustness:
- Comprehensive error handling
- Extensive test coverage
- Graceful degradation where possible

### Extensibility:
- Clean interfaces between phases
- Pluggable components where feasible
- Well-defined data structures

### Z-Machine Fidelity:
- Accurate implementation of Z-Machine specification
- Proper version compatibility
- Full feature support for interactive fiction development

## Getting Started

For newcomers wanting to contribute:

1. **Start with tests**: Understand expected behavior through test cases
2. **Follow the pipeline**: Trace a simple program through all compilation phases  
3. **Read the Z-Machine spec**: Understanding the target platform is crucial
4. **Experiment**: Modify test programs and observe the generated bytecode
5. **Focus on one phase**: Deep dive into lexing, parsing, or code generation

The codebase is designed to be approachable, with clear separation of concerns and comprehensive documentation throughout.

## Conclusion

The Grue Z-Machine Compiler represents a complete, production-quality implementation of a domain-specific language compiler. Its architecture balances theoretical computer science principles with practical game development needs, resulting in a system that's both educationally valuable and genuinely useful for creating interactive fiction.

The multi-phase design, comprehensive error handling, and extensive test coverage make it an excellent example of how to structure a compiler project. Whether you're interested in language design, virtual machine targets, or interactive fiction development, this codebase provides a solid foundation for exploration and extension.