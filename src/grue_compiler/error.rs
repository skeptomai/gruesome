// Compiler Error Handling

use std::fmt;

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub line: usize,
    pub column: usize,
    pub source_file: Option<String>,
    pub source_line: Option<String>, // The actual source line for context
}

#[derive(Debug, Clone)]
pub enum CompilerError {
    // Lexical errors
    LexicalError(String, usize), // message, position
    UnexpectedCharacter(char, usize),
    UnterminatedString(usize),

    // Parse errors
    ParseError(String, usize),
    UnexpectedToken(String, usize),
    ExpectedToken(String, String, usize), // expected, found, position

    // Semantic errors
    SemanticError(String, usize),
    UndefinedSymbol(String, usize),
    DuplicateSymbol(String, usize),
    TypeMismatch(String, String, usize), // expected, found, position

    // Code generation errors
    CodeGenError(String),
    AddressOverflow,
    TooManyObjects,
    StringTooLong(String),
    InvalidBytecode(String),
    UnresolvedReference(String),

    // Runtime/execution errors
    ExecutionError(String),
    StackOverflow,
    StackUnderflow,
    InvalidOpcode(u8, usize), // opcode, address

    // IO errors
    IOError(String),
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompilerError::LexicalError(msg, pos) => {
                write!(f, "Lexical error at position {}: {}", pos, msg)
            }
            CompilerError::UnexpectedCharacter(ch, pos) => {
                write!(f, "Unexpected character '{}' at position {}", ch, pos)
            }
            CompilerError::UnterminatedString(pos) => {
                write!(f, "Unterminated string starting at position {}", pos)
            }
            CompilerError::ParseError(msg, pos) => {
                write!(f, "Parse error at position {}: {}", pos, msg)
            }
            CompilerError::UnexpectedToken(token, pos) => {
                write!(f, "Unexpected token '{}' at position {}", token, pos)
            }
            CompilerError::ExpectedToken(expected, found, pos) => {
                write!(
                    f,
                    "Expected '{}' but found '{}' at position {}",
                    expected, found, pos
                )
            }
            CompilerError::SemanticError(msg, pos) => {
                write!(f, "Semantic error at position {}: {}", pos, msg)
            }
            CompilerError::UndefinedSymbol(symbol, pos) => {
                write!(f, "Undefined symbol '{}' at position {}", symbol, pos)
            }
            CompilerError::DuplicateSymbol(symbol, pos) => {
                write!(f, "Duplicate symbol '{}' at position {}", symbol, pos)
            }
            CompilerError::TypeMismatch(expected, found, pos) => {
                write!(
                    f,
                    "Type mismatch at position {}: expected {}, found {}",
                    pos, expected, found
                )
            }
            CompilerError::CodeGenError(msg) => {
                write!(f, "Code generation error: {}", msg)
            }
            CompilerError::AddressOverflow => {
                write!(f, "Address space overflow - program too large")
            }
            CompilerError::TooManyObjects => {
                write!(f, "Too many objects for target Z-Machine version")
            }
            CompilerError::StringTooLong(s) => {
                write!(f, "String too long for Z-Machine encoding: '{}'", s)
            }
            CompilerError::InvalidBytecode(msg) => {
                write!(f, "Invalid bytecode generated: {}", msg)
            }
            CompilerError::UnresolvedReference(msg) => {
                write!(f, "Unresolved reference: {}", msg)
            }
            CompilerError::ExecutionError(msg) => {
                write!(f, "Runtime execution error: {}", msg)
            }
            CompilerError::StackOverflow => {
                write!(
                    f,
                    "Stack overflow - too many nested function calls or expressions"
                )
            }
            CompilerError::StackUnderflow => {
                write!(f, "Stack underflow - attempted to pop from empty stack")
            }
            CompilerError::InvalidOpcode(opcode, addr) => {
                write!(
                    f,
                    "Invalid opcode 0x{:02x} at address 0x{:04x}",
                    opcode, addr
                )
            }
            CompilerError::IOError(msg) => {
                write!(f, "IO error: {}", msg)
            }
        }
    }
}

impl CompilerError {
    /// Get a suggestion for how to fix this error
    pub fn suggestion(&self) -> Option<String> {
        match self {
            CompilerError::UndefinedSymbol(symbol, _) => {
                Some(format!("Did you mean to declare '{}' as a variable or function?", symbol))
            }
            CompilerError::DuplicateSymbol(symbol, _) => {
                Some(format!("Symbol '{}' is already defined. Use a different name or remove the duplicate.", symbol))
            }
            CompilerError::TypeMismatch(expected, found, _) => {
                Some(format!("Try converting {} to {} or check your expression.", found, expected))
            }
            CompilerError::InvalidOpcode(_, _) => {
                Some("This may be caused by a compiler bug in bytecode generation. Try simplifying the code.".to_string())
            }
            CompilerError::UnresolvedReference(msg) => {
                Some(format!("Check that all referenced functions and variables are properly declared: {}", msg))
            }
            CompilerError::StringTooLong(_) => {
                Some("Z-Machine strings are limited to 767 characters. Consider breaking the string into smaller parts.".to_string())
            }
            _ => None,
        }
    }

    /// Check if this error is recoverable (compilation can continue)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CompilerError::UndefinedSymbol(_, _)
                | CompilerError::DuplicateSymbol(_, _)
                | CompilerError::TypeMismatch(_, _, _)
                | CompilerError::StringTooLong(_)
        )
    }
}

impl std::error::Error for CompilerError {}
