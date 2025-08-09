// Compiler Error Handling

use std::fmt;

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
            CompilerError::IOError(msg) => {
                write!(f, "IO error: {}", msg)
            }
        }
    }
}

impl std::error::Error for CompilerError {}
