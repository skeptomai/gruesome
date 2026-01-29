# Grue Z-Machine Compiler: Complete Architecture Guide

**Version**: 2.0
**Last Updated**: January 2026
**Status**: Production Ready

## Table of Contents

1. [Overview](#overview)
2. [Project Structure](#project-structure)
3. [Complete Compilation Pipeline](#complete-compilation-pipeline)
4. [Phase-by-Phase Architecture](#phase-by-phase-architecture)
5. [Grammar Pattern Processing](#grammar-pattern-processing-complete-flow)
6. [Memory Layout & Execution Model](#memory-layout--execution-model)
7. [Advanced Features](#advanced-features)
8. [Testing & Validation](#testing--validation)
9. [Design Philosophy](#design-philosophy)
10. [References](#references)

---

## Overview

The Grue Z-Machine Compiler is a complete implementation of a domain-specific language compiler that translates Grue source code into executable Z-Machine bytecode. This document provides a comprehensive architectural overview, including detailed explanations of grammar pattern processing and code generation.

### What is the Z-Machine?

The Z-Machine is a virtual machine originally developed by Infocom for their interactive fiction games (like Zork). It provides:
- Platform-independent bytecode execution
- Sophisticated text handling and encoding
- Object-oriented programming features
- Built-in parser and dictionary systems
- Memory management and save/restore capabilities

### Target Audience

This document is for developers who want to:
- Understand the complete compilation pipeline
- Contribute to the compiler implementation
- Debug compilation issues
- Extend the language with new features
- Understand Z-Machine code generation

---

## Project Structure

```
src/grue_compiler/
├── mod.rs              # Module declarations and public API
├── lexer.rs           # Tokenization and lexical analysis
├── parser.rs          # Syntax analysis and AST generation
├── ast.rs             # Abstract Syntax Tree definitions
├── semantic.rs        # Semantic analysis and type checking
├── ir.rs              # Intermediate Representation
├── codegen.rs         # Z-Machine bytecode generation (main)
├── codegen_grammar.rs # Grammar pattern matching code generation
├── codegen_*.rs       # Other specialized code generation modules
├── error.rs           # Error handling and reporting
└── tests/             # Comprehensive test suites
```

---

## Complete Compilation Pipeline

The Grue compiler follows a traditional multi-phase compilation approach:

```
Grue Source Code (.grue)
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

**Key Benefits of Multi-Phase Design**:
- Clear separation of concerns
- Each phase handles one responsibility
- Easy to debug (inspect output at each stage)
- Extensible (add new phases or modify existing ones)
- Follows standard compiler architecture patterns

---

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
pub enum TokenKind {
    // Keywords
    Fn, Let, If, Else, While, For, Return,
    Room, Object, Grammar, Init, Verb,

    // Literals
    StringLiteral(String),
    IntegerLiteral(i32),
    BooleanLiteral(bool),

    // Operators
    Plus, Minus, Star, Slash, Equal, EqualEqual,
    Arrow,  // => for grammar patterns

    // Identifiers and special tokens
    Identifier(String),
    Eof,
}
```

**Example Input/Output**:
```grue
// Source code
verb "look" {
    "at" + noun => examine($2)
}

// Token stream
VERB, STRING("look"), LBRACE,
  STRING("at"), PLUS, IDENTIFIER("noun"), ARROW,
  IDENTIFIER("examine"), LPAREN, DOLLAR, NUMBER(2), RPAREN,
RBRACE
```

---

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
    Grammar { verbs: Vec<VerbDecl> },
    // ... expressions, statements, etc.
}

// Grammar-specific AST nodes
pub struct GrammarDecl {
    pub verbs: Vec<VerbDecl>,
    pub vocabulary: Option<VocabularyDecl>,
}

pub struct VerbDecl {
    pub word: String,              // e.g., "look"
    pub patterns: Vec<VerbPattern>, // All patterns for this verb
}

pub struct VerbPattern {
    pub pattern: Vec<PatternElement>,  // What to match
    pub handler: Handler,               // What to do
}

pub enum PatternElement {
    Default,           // "default" keyword
    Literal(String),   // String literals like "at", "around"
    Noun,             // "noun" keyword
    // Plus advanced elements for future use...
}

pub enum Handler {
    FunctionCall(String, Vec<Expr>),  // Function name + args (as strings/exprs)
    Block(BlockStmt),                  // Inline code block
}
```

**Design Decisions**:
- Recursive descent parsing for clarity and maintainability
- Rich AST nodes with position information for error reporting
- Separate handling of world elements (rooms, objects) vs code elements

---

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

---

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
    LoadVar { target: IrId, var_id: IrId },
    StoreVar { var_id: IrId, source: IrId },

    // Arithmetic and logic
    BinaryOp { target: IrId, op: BinaryOperator, left: IrId, right: IrId },
    UnaryOp { target: IrId, op: UnaryOperator, operand: IrId },

    // Control flow
    Branch { condition: IrId, true_label: u32, false_label: u32 },
    Jump { label: u32 },
    Label { id: u32 },

    // Function calls
    Call { target: Option<IrId>, function: IrId, args: Vec<IrValue> },
    Return { value: Option<IrId> },

    // Object system
    GetProperty { target: IrId, object: IrId, property: String },
    SetProperty { object: IrId, property: String, value: IrId },
}
```

**Grammar Representation in IR**:
```rust
pub struct IrGrammar {
    pub verb: String,              // e.g., "look"
    pub patterns: Vec<IrPattern>,  // All patterns for this verb
}

pub struct IrPattern {
    pub pattern: Vec<IrPatternElement>,  // Pattern structure
    pub handler: IrHandler,               // What to execute
}

pub enum IrPatternElement {
    Default,           // Just the verb alone
    Literal(String),   // A specific word like "around", "at"
    Noun,             // An object reference
    // ... other advanced elements
}

pub enum IrHandler {
    FunctionCall(IrId, Vec<IrValue>),  // Call function with resolved ID + args
    Block(IrBlock),                     // Inline code block
}
```

**Design Benefits**:
- Platform-independent representation
- Easy optimization opportunities
- Clean separation from AST complexity
- Forward reference resolution through two-pass processing
- Function names resolved to numeric IDs (IrId)

---

### Phase 5: Code Generation (`codegen.rs`, `codegen_grammar.rs`)

**Purpose**: Transform IR into executable Z-Machine bytecode.

**Key Components**:
- `ZMachineCodeGen`: Main bytecode emission engine
- Memory layout management (header, globals, objects, dictionary, strings, code)
- Address resolution and reference patching
- Z-Machine version compatibility (v3/v4/v5)

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
    → Z-Machine: PRINT_PADDR opcode (0x8D) with string address
```

**Version Compatibility**:
- **v3**: 6 Z-characters per word, even-aligned strings, 255 objects max
- **v5**: 9 Z-characters per word, 4-byte aligned routines/strings, 65535 objects max

---

## Grammar Pattern Processing: Complete Flow

This section explains the complete transformation of grammar patterns from source code through all compilation stages to runtime execution, with concrete examples.

### Overview: The Grammar System

The grammar system is the text adventure command processing engine. It converts natural language input (like "look at mailbox") into executable game actions.

### Example Pattern: "look at [object]"

Let's trace how the pattern `"at" + noun => examine($2)` transforms through every compilation stage.

---

### Stage 1: Source Code (.grue)

**Input:**
```grue
grammar {
    verb "look" {
        default => look_around(),
        "at" + noun => examine($2),
        "around" => look_around()
    }
}
```

**What it means:**
- `verb "look"` - Define patterns for the verb "look"
- `default =>` - Just "look" alone
- `"at" + noun =>` - "look at [object]"
- `"around" =>` - "look around" (literal word)
- `examine($2)` - Call examine() function with second word (the object)

---

### Stage 2: Tokens (Lexer Output)

**Token stream for `"at" + noun => examine($2)`:**
```
STRING("at"), PLUS, IDENTIFIER("noun"), ARROW,
IDENTIFIER("examine"), LPAREN, DOLLAR, NUMBER(2), RPAREN
```

**Token breakdown:**
- `STRING("at")` - Literal string to match
- `PLUS` - Pattern connector
- `IDENTIFIER("noun")` - Special keyword for object placeholder
- `ARROW` - Pattern/handler separator (=>)
- `IDENTIFIER("examine")` - Function name
- `LPAREN, DOLLAR, NUMBER(2), RPAREN` - Argument `$2` (second parsed word)

---

### Stage 3: AST (Parser Output)

**Parsed structure:**
```rust
GrammarDecl {
    verbs: vec![
        VerbDecl {
            word: "look".to_string(),
            patterns: vec![
                // Pattern 1: default => look_around()
                VerbPattern {
                    pattern: vec![PatternElement::Default],
                    handler: Handler::FunctionCall(
                        "look_around".to_string(),
                        vec![]  // No arguments
                    )
                },

                // Pattern 2: "at" + noun => examine($2)
                VerbPattern {
                    pattern: vec![
                        PatternElement::Literal("at".to_string()),
                        PatternElement::Noun
                    ],
                    handler: Handler::FunctionCall(
                        "examine".to_string(),
                        vec![Expr::SpecialVariable(2)]  // $2
                    )
                },

                // Pattern 3: "around" => look_around()
                VerbPattern {
                    pattern: vec![
                        PatternElement::Literal("around".to_string())
                    ],
                    handler: Handler::FunctionCall(
                        "look_around".to_string(),
                        vec![]
                    )
                }
            ]
        }
    ],
    vocabulary: None
}
```

**Key aspects of AST:**
- Function references are still **strings** (`"examine"`)
- Arguments are **expression trees** (`Expr::SpecialVariable(2)`)
- Pattern elements are **categorized** (Literal, Noun, Default)
- Structure directly represents source syntax

**Parser code** (parser.rs:385-452):
```rust
fn parse_verb_pattern(&mut self) -> Result<VerbPattern, CompilerError> {
    let mut pattern = Vec::new();

    // Parse pattern elements until we hit '=>'
    while !self.check(&TokenKind::Arrow) && !self.is_at_end() {
        match &self.peek().kind {
            TokenKind::StringLiteral(val) => {
                pattern.push(PatternElement::Literal(val.clone()));
                self.advance();
            }
            TokenKind::Identifier(name) if name == "noun" => {
                pattern.push(PatternElement::Noun);
                self.advance();
            }
            TokenKind::Identifier(name) if name == "default" => {
                pattern.push(PatternElement::Default);
                self.advance();
            }
            TokenKind::Plus => {
                self.advance(); // Skip '+' connector
            }
            _ => break,
        }
    }

    self.consume(TokenKind::Arrow, "Expected '=>' after pattern")?;
    let handler = self.parse_handler()?;

    Ok(VerbPattern { pattern, handler })
}
```

---

### Stage 4: IR (Semantic Analysis + IR Generation)

**Transformed structure:**
```rust
IrGrammar {
    verb: "look".to_string(),
    patterns: vec![
        // Pattern 1: default => look_around()
        IrPattern {
            pattern: vec![IrPatternElement::Default],
            handler: IrHandler::FunctionCall(
                IrId(42),  // look_around function ID (resolved)
                vec![]
            )
        },

        // Pattern 2: "at" + noun => examine($2)
        IrPattern {
            pattern: vec![
                IrPatternElement::Literal("at".to_string()),
                IrPatternElement::Noun
            ],
            handler: IrHandler::FunctionCall(
                IrId(57),  // examine function ID (resolved)
                vec![IrValue::Noun(2)]  // Second word as object
            )
        },

        // Pattern 3: "around" => look_around()
        IrPattern {
            pattern: vec![
                IrPatternElement::Literal("around".to_string())
            ],
            handler: IrHandler::FunctionCall(
                IrId(42),  // same function as pattern 1
                vec![]
            )
        }
    ]
}
```

**Key transformations in IR:**
- Function **strings → numeric IDs**: `"examine"` → `IrId(57)`
- Expression trees **resolved**: `Expr::SpecialVariable(2)` → `IrValue::Noun(2)`
- Pattern elements **mirrored**: `PatternElement::Literal` → `IrPatternElement::Literal`
- Symbol table lookups **completed**: All names resolved to IDs

**IR generation code** (ir.rs:2564-2677):
```rust
fn generate_grammar(&mut self, grammar: GrammarDecl) -> Result<Vec<IrGrammar>, CompilerError> {
    let mut ir_grammar = Vec::new();

    for verb in grammar.verbs {
        let mut patterns = Vec::new();

        for pattern in verb.patterns {
            // Convert AST pattern elements to IR pattern elements
            let ir_pattern_elements: Vec<IrPatternElement> = pattern
                .pattern
                .into_iter()
                .map(|elem| match elem {
                    PatternElement::Literal(s) => IrPatternElement::Literal(s),
                    PatternElement::Noun => IrPatternElement::Noun,
                    PatternElement::Default => IrPatternElement::Default,
                    // ... other elements
                })
                .collect();

            // Resolve handler function name to IR ID
            let ir_handler = match pattern.handler {
                Handler::FunctionCall(name, args) => {
                    // Convert arguments to IR values
                    let mut ir_args = Vec::new();
                    for arg in args {
                        let ir_value = self.expr_to_ir_value(arg)?;
                        ir_args.push(ir_value);
                    }

                    // CRITICAL: Look up function ID using symbol table
                    let func_id = if let Some(&id) = self.symbol_ids.get(&name) {
                        id
                    } else {
                        return Err(CompilerError::SemanticError(
                            format!("Function '{}' not found", name),
                            0,
                        ));
                    };

                    IrHandler::FunctionCall(func_id, ir_args)
                }
                Handler::Block(block) => {
                    let ir_block = self.generate_block(block)?;
                    IrHandler::Block(ir_block)
                }
            };

            patterns.push(IrPattern {
                pattern: ir_pattern_elements,
                handler: ir_handler,
            });
        }

        ir_grammar.push(IrGrammar {
            verb: verb.word,
            patterns,
        });
    }

    Ok(ir_grammar)
}
```

---

### Stage 5: Z-Machine Bytecode (Code Generation)

**Generated bytecode for "look" verb patterns:**

The code generator (`codegen_grammar.rs`) produces Z-Machine instructions that:
1. Check which verb was typed
2. Check how many words were entered
3. Match against patterns in order
4. Call the appropriate handler function

**Conceptual bytecode flow:**
```
Address | Bytecode                | Description
--------|-------------------------|----------------------------------
0x1000  | LOADW parse_buf+2 → tmp | Load first word (verb) dict address
0x1005  | JE tmp, "look" → +match | If first word is "look", continue
0x100A  | JUMP main_loop          | Otherwise back to main loop
        |                         |
0x100F  | [match]:                | Verb matched, check patterns
0x100F  | LOADB parse_buf+1 → wc  | Load word count
        |                         |
        | // Pattern matching:    |
0x1012  | JE wc, 1 → +default     | If 1 word, call default handler
        |                         |
0x1017  | LOADW parse_buf+6 → w2  | Load second word dict address
0x101C  | JE w2, "at" → +at_noun  | If "at", check for noun pattern
0x1021  | JE w2, "around" → +lit  | If "around", call literal handler
        |                         |
0x1026  | // Try verb+noun:       |
0x1026  | [resolve_noun]:         |
0x1026  | CALL lookup_object(w2)  | Look up object by dict address
        | → obj_id                |
0x102B  | JE obj_id, 0 → +unknown | If not found, unknown object
0x1030  | CALL examine(obj_id)    | Call examine(object)
0x1035  | JUMP main_loop          |
        |                         |
0x103A  | [at_noun]:              | "at" + noun pattern
0x103A  | LOADW parse_buf+10 → w3 | Load third word
0x103F  | CALL lookup_object(w3)  | Resolve noun
0x1044  | CALL examine(obj_id)    | Call examine(object)
0x1049  | JUMP main_loop          |
        |                         |
0x104E  | [lit]:                  | "around" literal pattern
0x104E  | CALL look_around()      | Call look_around()
0x1053  | JUMP main_loop          |
        |                         |
0x1058  | [default]:              | Default pattern
0x1058  | CALL look_around()      | Call look_around()
0x105D  | JUMP main_loop          |
```

**Code generation entry point** (codegen_grammar.rs:30-80):
```rust
pub fn generate_verb_matching(
    &mut self,
    verb: &str,
    patterns: &[IrPattern],
    main_loop_jump_id: u32,
) -> Result<(), CompilerError> {
    // Main entry point for generating pattern matching code
    // Delegates to specialized handlers:

    // 1. Check verb matches
    // 2. Extract word count from parse buffer
    // 3. Route to pattern handlers based on pattern types:

    self.handle_literal_patterns(patterns, verb)?;
    self.handle_literal_noun_patterns(patterns, verb)?;
    self.handle_verb_noun_patterns(patterns)?;
    self.handle_default_patterns(patterns)?;

    // All patterns jump back to main loop
    self.emit_jump_to_main_loop(main_loop_jump_id)?;

    Ok(())
}
```

**Pattern handler example** (literal+noun patterns):
```rust
fn handle_literal_noun_patterns(
    &mut self,
    patterns: &[IrPattern],
    verb: &str,
) -> Result<(), CompilerError> {
    // Find patterns like "at" + noun
    let lit_noun_patterns: Vec<_> = patterns
        .iter()
        .filter(|p| matches!(
            p.pattern.as_slice(),
            [IrPatternElement::Literal(_), IrPatternElement::Noun]
        ))
        .collect();

    for pattern in lit_noun_patterns {
        if let [IrPatternElement::Literal(literal), IrPatternElement::Noun] =
            pattern.pattern.as_slice()
        {
            // 1. Check word count >= 3 (verb + literal + noun)
            // 2. Check second word matches literal
            // 3. Resolve third word to object
            // 4. Call handler function with object

            // Generate bytecode for these checks...
            self.emit_word_count_check(3)?;
            self.emit_literal_match_check(literal, 2)?;  // Check word 2
            self.emit_object_resolution(3)?;             // Resolve word 3
            self.emit_handler_call(&pattern.handler)?;
        }
    }

    Ok(())
}
```

---

### Stage 6: Runtime Execution

**When player types "look at mailbox":**

1. **Input Processing** (Z-Machine SREAD instruction):
   ```
   Parse Buffer after SREAD:
   [0] = max words (usually 20)
   [1] = actual word count (3)
   [2-3] = word 1 dict address (low, high) → "look"
   [4] = word 1 text position
   [5] = word 1 length
   [6-7] = word 2 dict address → "at"
   [8] = word 2 text position
   [9] = word 2 length
   [10-11] = word 3 dict address → "mailbox"
   [12] = word 3 text position
   [13] = word 3 length
   ```

2. **Pattern Matching Execution**:
   ```
   Step 1: Load word 1 dict address → matches "look" ✓
   Step 2: Load word count → 3 words
   Step 3: Try patterns in order:
           - default (1 word)? NO, have 3 words
           - "around" (2 words)? NO, word 2 is "at" not "around"
           - "at" + noun? YES! ✓
             - Word count >= 3? YES ✓
             - Word 2 == "at"? YES ✓
             - Word 3 resolves to object? Check...
   Step 4: Object lookup:
           - Load word 3 dict address (for "mailbox")
           - Search game objects for matching name
           - Find: mailbox object #5
   Step 5: Call handler:
           - CALL examine(5)  // Call examine with mailbox object
   ```

3. **Handler Execution**:
   ```rust
   fn examine(obj) {  // obj = 5 (mailbox)
       if !player_can_see(obj) {
           println("You can't see any such thing.");
           return;
       }

       // Special handling for mailbox
       if obj == mailbox {
           print("The small mailbox is ");
           if mailbox.open {
               print("open");
           } else {
               print("closed");
           }
           println(".");
       }
   }
   ```

**Runtime flow diagram:**
```
Player Input: "look at mailbox"
        ↓
    [SREAD] Parse into words, look up in dictionary
        ↓
Parse Buffer: ["look", "at", "mailbox"] (dict addresses)
        ↓
    [Pattern Matching Code]
        ↓
Check verb: word 1 == "look"? YES
        ↓
Check patterns for "look":
  1. default (1 word)? NO
  2. "at" + noun (3 words, word 2 = "at")? YES ✓
        ↓
Resolve noun: word 3 ("mailbox") → object #5
        ↓
Call handler: examine(5)
        ↓
Execute examine() function code
        ↓
Print result: "The small mailbox is closed."
        ↓
Return to main loop
```

---

### Key Differences: AST vs IR vs Runtime

| Aspect | AST | IR | Runtime |
|--------|-----|-----|---------|
| **Function refs** | Strings (`"examine"`) | Numeric IDs (`IrId(57)`) | Z-Machine addresses (`0x1530`) |
| **Arguments** | Expr trees (`Expr::SpecialVariable(2)`) | Values (`IrValue::Noun(2)`) | Parse buffer indices |
| **Pattern elements** | `PatternElement::Literal` | `IrPatternElement::Literal` | Bytecode checks |
| **Purpose** | Direct source representation | Compiler-friendly format | Executable instructions |
| **Created by** | Parser | IR generator | Code generator |

---

### Pattern Matching Strategy

The compiler generates code to check patterns in this order:

1. **Literal-only patterns** (`"around"`) - Most specific, checked first
2. **Literal+Noun patterns** (`"at" + noun`) - Moderately specific
3. **Verb+Noun patterns** (`noun`) - Generic object handling
4. **Default pattern** (`default`) - Fallback, checked last

This ordering ensures the most specific pattern matches first, enabling natural command disambiguation.

---

### Grammar System Summary

**Complete transformation:**
```
Source: "at" + noun => examine($2)
    ↓ Lexer
Tokens: STRING("at"), PLUS, IDENTIFIER("noun"), ARROW, ...
    ↓ Parser
AST: VerbPattern {
    pattern: [Literal("at"), Noun],
    handler: FunctionCall("examine", [Expr::SpecialVariable(2)])
}
    ↓ IR Generator
IR: IrPattern {
    pattern: [Literal("at"), Noun],
    handler: FunctionCall(IrId(57), [IrValue::Noun(2)])
}
    ↓ Code Generator
Z-Machine:
    - Check word_count >= 3
    - Check word[2] dict address == "at"
    - Resolve word[3] to object ID
    - CALL examine(object_id)
    ↓ Runtime
Execution:
    - Player types "look at mailbox"
    - SREAD tokenizes input
    - Pattern matching checks each pattern
    - Finds match: "at" + noun
    - Resolves "mailbox" → object #5
    - Calls examine(5)
    - Displays object description
```

---

## Memory Layout & Execution Model

### Z-Machine File Structure

```
┌─────────────────────────────────────────────────────────────┐
│ Z-Machine File Structure                                    │
├─────────────────────────────────────────────────────────────┤
│ Header (0x0000-0x003F)                                      │
│  ├─ Initial PC → Points to Init Block Start                │
│  ├─ Static Memory Base                                     │
│  ├─ Dictionary address                                     │
│  ├─ Object table address                                   │
│  └─ Other standard Z-Machine header fields                 │
├─────────────────────────────────────────────────────────────┤
│ Static Memory (0x0040-...)                                  │
│  ├─ Global Variables (240 words = 480 bytes)               │
│  ├─ Object Tables                                          │
│  ├─ Property Tables                                        │
│  └─ Dictionary                                             │
├─────────────────────────────────────────────────────────────┤
│ Dynamic Memory (... onwards)                                │
│  ├─ String Table (ZSCII-encoded strings)                   │
│  ├─ Init Block (Direct Execution Code)                     │
│  │   ├─ User's init{} code compiled directly              │
│  │   └─ CALL main_loop_routine (packed address)           │
│  ├─ Main Loop Routine                                      │
│  │   ├─ Routine Header (0x00 = 0 locals)                  │
│  │   ├─ Print Prompt ("> ")                               │
│  │   ├─ SREAD instruction (wait for input)                │
│  │   ├─ Grammar pattern matching                          │
│  │   └─ Jump back to main loop start                      │
│  └─ User-defined Functions                                 │
│      ├─ examine()                                          │
│      ├─ look_around()                                      │
│      └─ Other game functions                               │
└─────────────────────────────────────────────────────────────┘
```

### Runtime Execution Sequence

1. **Z-Machine Startup**
   - Interpreter loads game file
   - Sets PC to initial address from header
   - Begins execution at init block start

2. **Init Block Execution**
   ```
   PC → Init Block Start
         ├─ Execute user's init{} code directly
         ├─ Print game banner
         ├─ Set up initial game state
         └─ CALL main_loop_routine (0x20 opcode)
   ```

3. **Main Loop Routine Execution**
   ```
   Main Loop Routine:
         ├─ Routine Header (0x00)
         ├─ Print prompt "> "
         ├─ SREAD (wait for user input)
         ├─ Parse input (tokenize into words)
         ├─ Match against grammar patterns
         ├─ Execute matched command handler
         └─ Jump back to routine start (infinite loop)
   ```

### Memory Layout Example (mini_zork.z3)

```
Address Range | Content
--------------|------------------------------------------
0x0000-0x003F | Z-Machine Header
0x0040-0x05BE | Static Memory (objects, dictionary, globals)
0x05BF-0x0BCD | Dynamic Memory (strings, routines)
0x0BCE        | Init Block Start ← Initial PC points here
              | ├─ User init code (print statements)
              | └─ CALL 0x05F7 (main loop routine)
0x05F7        | Main Loop Routine Start
              | ├─ 0x00 (routine header)
              | ├─ Print prompt
              | ├─ SREAD instruction
              | ├─ Grammar matching (generated code)
              | └─ Jump back to 0x05F8
0x0620        | examine() function
              | ├─ Routine header
              | ├─ Function code
              | └─ Return instruction
0x0680        | look_around() function
              | └─ ...
```

### Key Architectural Decisions

#### 1. Init Block as Direct Execution

**Design Choice:** `init {}` blocks compile to direct execution code, not callable routines.

**Rationale:**
- Matches real Z-Machine games (Zork I starts with direct execution)
- Init code typically runs once at startup
- Simpler than making init callable
- Follows Z-Machine specification for program entry

#### 2. Main Loop as Callable Routine

**Design Choice:** Main game loop is a proper Z-Machine routine with header.

**Rationale:**
- Enables proper calling convention (CALL instruction)
- Allows routine to be called repeatedly
- Matches Z-Machine specification for routines
- Supports packed address resolution

#### 3. CALL vs JUMP for Control Transfer

**Design Choice:** Use CALL instruction to transfer from init to main loop.

**Rationale:**
- Main loop is a proper routine, should be called not jumped to
- Matches real Z-Machine games (Zork I uses CALL for major control transfer)
- Enables proper return semantics (though main loop never returns)
- Uses packed addresses correctly

---

## Advanced Features

### Object Numbering System

The Grue compiler uses a dual numbering system that distinguishes between internal compiler IDs and final Z-Machine object numbers.

#### Two Independent Numbering Systems

**1. IR ID (Internal Compiler Use)**
- **Purpose**: Internal tracking within the IR system
- **Range**: 32-bit unsigned integers (0 to 4,294,967,295)
- **Assignment**: Based on source code order and compiler-generated objects
- **Example**: User objects get IDs 1-100, built-in objects use high IDs (9999+)

**2. Z-Machine Object Number (Final Bytecode)**
- **Purpose**: Actual object references used in Z-Machine bytecode
- **Range**: 8-bit unsigned integers (1 to 255 for v3, 1 to 65535 for v5)
- **Assignment**: Sequential starting from 1, following Z-Machine conventions
- **Example**: Player=1, First room=2, Second room=3, etc.

#### Critical Architectural Decision: Player Object Creation

The compiler automatically creates a player object as **Z-Machine object #1**:

```rust
// Player object gets high IR ID to avoid conflicts
all_objects.push(ObjectData {
    id: 9999u32, // IR ID: arbitrary high number
    name: "player".to_string(),
    short_name: "yourself".to_string(),
    properties: player_properties,
});

// But becomes Z-Machine object #1 through sequential mapping
for (index, object) in all_objects.iter().enumerate() {
    let obj_num = (index + 1) as u8; // Player: index=0 → obj_num=1
    object_id_to_number.insert(object.id, obj_num);
}
```

#### Why This Matters

- **Frotz Compatibility**: External interpreters expect object #1 to exist
- **Z-Machine Convention**: Interactive fiction traditionally uses object #1 for player
- **IR Flexibility**: High IR IDs prevent conflicts with user-defined objects
- **Reference Resolution**: `player.location` resolves to object #1 property access

### String Management

Strings are handled with special care:
- **ZSCII Encoding**: Z-Machine text format (3 chars per 16-bit word)
- **Deduplication**: Identical strings share storage
- **Abbreviation System**: High-frequency strings compressed (650 bytes saved)
- **Packed Addresses**: Efficient referencing system
- **Alignment**: Platform-specific alignment requirements (even in v3)

### Builtin Function System

The compiler provides essential Z-Machine primitives:
- `print(string)`: Text output using PRINT_PADDR
- `println(string)`: Text output with newline
- `move(object, destination)`: Object manipulation using INSERT_OBJ
- `get_location(object)`: Parent lookup using GET_PARENT
- `player_can_see(object)`: Visibility checking
- `get_exit(room, direction)`: Exit resolution

Smart detection distinguishes builtin vs user-defined functions at compile time.

### Two-Pass Compilation

The compiler uses a sophisticated two-pass system:

1. **First Pass**: Generate IR and collect all symbols/addresses
2. **Second Pass**: Resolve forward references and emit final bytecode

This enables:
- Forward function declarations
- Complex control flow with loops and conditionals
- Proper address calculation for strings and routines
- Grammar pattern handlers can reference functions defined later

---

## Testing & Validation

### Test Programs

```grue
// basic_test.grue - Simple program with functions
fn test_function() {
    print("Hello from test function!");
}

init {
    print("Basic Grue Compiler Test");
    test_function();
}

// mini_zork.grue - Game-like program with rooms, objects, and grammar
room west_of_house "West of House" {
    desc: "You are standing in an open field west of a white house."

    object mailbox {
        names: ["mailbox", "box"]
        openable: true
        container: true
    }
}

grammar {
    verb "look" {
        default => look_around(),
        "at" + noun => examine($2)
    }

    verb "open" {
        noun => handle_open($noun)
    }
}

fn examine(obj) {
    if !player_can_see(obj) {
        println("You can't see any such thing.");
        return;
    }
    println(obj.desc);
}
```

### Testing Commands

```bash
# Compile test game
cargo run --bin grue-compiler -- examples/mini_zork.grue

# Run with interpreter
cargo run --bin gruesome examples/mini_zork.z3

# Test with external interpreter (validation)
frotz examples/mini_zork.z3

# Debug compilation
RUST_LOG=debug cargo run --bin grue-compiler -- examples/mini_zork.grue --print-ir
```

---

## Design Philosophy

### Clarity Over Cleverness
- Straightforward recursive descent parsing
- Clear phase separation
- Descriptive naming and documentation
- Explicit rather than implicit behavior

### Robustness
- Comprehensive error handling at every phase
- Extensive test coverage (unit + integration)
- Graceful degradation where possible
- Position tracking for precise error reporting

### Extensibility
- Clean interfaces between phases
- Well-defined data structures
- Module separation (codegen_grammar.rs)
- Clear extension points for new features

### Z-Machine Fidelity
- Accurate implementation of Z-Machine specification
- Proper version compatibility (v3/v4/v5)
- Validation against commercial games (Zork I)
- Compatibility with external interpreters (Frotz)

---

## References

### Internal Documentation
- `docs/COMPILER_BUG_PATTERNS.md` - Critical bug patterns and gotchas
- `docs/GRAMMAR_PATTERN_MATCHING_ANALYSIS.md` - Deep dive into pattern matching
- `docs/DEVELOPER_ARCHITECTURE_GUIDE.md` - Platform-wide overview
- `CLAUDE.md` - Project guidelines and debugging tools

### External Resources
- Z-Machine Standards Document v1.1 at `/Users/cb/Projects/Z-Machine-Standard/`
- Inform Designer's Manual (Graham Nelson)
- Z-Machine specification: `sect07.html` (output), `sect15.html` (opcodes)

### Source Files
- `src/grue_compiler/lexer.rs` - Tokenization (588 lines)
- `src/grue_compiler/parser.rs` - Parsing (1,234 lines)
- `src/grue_compiler/ir.rs` - IR generation (2,677 lines)
- `src/grue_compiler/codegen.rs` - Main code generation (5,800+ lines)
- `src/grue_compiler/codegen_grammar.rs` - Grammar code generation (1,353 lines)

---

## Conclusion

The Grue Z-Machine Compiler represents a complete, production-quality implementation of a domain-specific language compiler. Its architecture balances theoretical computer science principles with practical game development needs.

**Key Achievements:**
- ✅ Complete multi-phase pipeline (Lexer → Parser → Semantic → IR → CodeGen)
- ✅ Comprehensive grammar pattern system with runtime matching
- ✅ Z-Machine compliance validated against commercial games
- ✅ Extensive test coverage and external interpreter compatibility
- ✅ Clear documentation and maintainable code structure

The grammar pattern processing system, detailed in this document, demonstrates sophisticated compilation techniques while maintaining clarity and debuggability. Whether you're interested in language design, virtual machine targets, or interactive fiction development, this codebase provides a solid foundation for exploration and extension.

---

**Document Version**: 2.0
**Last Updated**: January 29, 2026
**Contributors**: Pancho (Claude), Sparky (Project Lead)
