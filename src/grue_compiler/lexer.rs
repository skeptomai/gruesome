// Grue Language Lexer
// Tokenizes Grue source code into a stream of tokens

use crate::grue_compiler::error::CompilerError;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub position: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    StringLiteral(String),
    IntegerLiteral(i16),
    BooleanLiteral(bool),
    Identifier(String),

    // Keywords
    World,
    Room,
    Object,
    Grammar,
    Verb,
    Function,
    Init,
    Contains,
    Names,
    Desc,
    Properties,
    Exits,
    OnEnter,
    OnExit,
    OnLook,
    If,
    Else,
    While,
    For,
    Return,
    Let,
    Var,
    True,
    False,
    Print,
    Move,
    Player,
    Location,

    // Symbols
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    LeftParen,    // (
    RightParen,   // )
    Semicolon,    // ;
    Colon,        // :
    Comma,        // ,
    Dot,          // .
    Arrow,        // =>

    // Operators
    Equal,        // =
    EqualEqual,   // ==
    NotEqual,     // !=
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=
    And,          // &&
    Or,           // ||
    Not,          // !
    Question,     // ?

    // Parameters
    Parameter(String), // $noun, $2, etc.

    // Special
    Newline,
    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    current_char: Option<char>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = chars.first().copied();

        Lexer {
            input: chars,
            position: 0,
            line: 1,
            column: 1,
            current_char,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, CompilerError> {
        let mut tokens = Vec::new();

        while let Some(token) = self.next_token()? {
            if token.kind != TokenKind::EOF {
                tokens.push(token);
            } else {
                tokens.push(token);
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Option<Token>, CompilerError> {
        self.skip_whitespace();

        let start_pos = self.position;
        let start_line = self.line;
        let start_column = self.column;

        match self.current_char {
            None => Ok(Some(Token {
                kind: TokenKind::EOF,
                position: start_pos,
                line: start_line,
                column: start_column,
            })),
            Some(ch) => {
                let token_kind = match ch {
                    // Single character tokens
                    '{' => {
                        self.advance();
                        TokenKind::LeftBrace
                    }
                    '}' => {
                        self.advance();
                        TokenKind::RightBrace
                    }
                    '[' => {
                        self.advance();
                        TokenKind::LeftBracket
                    }
                    ']' => {
                        self.advance();
                        TokenKind::RightBracket
                    }
                    '(' => {
                        self.advance();
                        TokenKind::LeftParen
                    }
                    ')' => {
                        self.advance();
                        TokenKind::RightParen
                    }
                    ';' => {
                        self.advance();
                        TokenKind::Semicolon
                    }
                    ':' => {
                        self.advance();
                        TokenKind::Colon
                    }
                    ',' => {
                        self.advance();
                        TokenKind::Comma
                    }
                    '.' => {
                        self.advance();
                        TokenKind::Dot
                    }
                    '+' => {
                        self.advance();
                        TokenKind::Plus
                    }
                    '-' => {
                        self.advance();
                        TokenKind::Minus
                    }
                    '*' => {
                        self.advance();
                        TokenKind::Star
                    }
                    '/' => {
                        self.advance();
                        if self.current_char == Some('/') {
                            // Single-line comment
                            self.skip_line_comment();
                            return self.next_token();
                        }
                        TokenKind::Slash
                    }
                    '%' => {
                        self.advance();
                        TokenKind::Percent
                    }
                    '?' => {
                        self.advance();
                        TokenKind::Question
                    }
                    '\n' => {
                        self.advance();
                        // Skip multiple newlines
                        while self.current_char == Some('\n') {
                            self.advance();
                        }
                        TokenKind::Newline
                    }

                    // Multi-character operators
                    '=' => {
                        self.advance();
                        if self.current_char == Some('=') {
                            self.advance();
                            TokenKind::EqualEqual
                        } else if self.current_char == Some('>') {
                            self.advance();
                            TokenKind::Arrow
                        } else {
                            TokenKind::Equal
                        }
                    }
                    '!' => {
                        self.advance();
                        if self.current_char == Some('=') {
                            self.advance();
                            TokenKind::NotEqual
                        } else {
                            TokenKind::Not
                        }
                    }
                    '<' => {
                        self.advance();
                        if self.current_char == Some('=') {
                            self.advance();
                            TokenKind::LessEqual
                        } else {
                            TokenKind::Less
                        }
                    }
                    '>' => {
                        self.advance();
                        if self.current_char == Some('=') {
                            self.advance();
                            TokenKind::GreaterEqual
                        } else {
                            TokenKind::Greater
                        }
                    }
                    '&' => {
                        self.advance();
                        if self.current_char == Some('&') {
                            self.advance();
                            TokenKind::And
                        } else {
                            return Err(CompilerError::UnexpectedCharacter('&', start_pos));
                        }
                    }
                    '|' => {
                        self.advance();
                        if self.current_char == Some('|') {
                            self.advance();
                            TokenKind::Or
                        } else {
                            return Err(CompilerError::UnexpectedCharacter('|', start_pos));
                        }
                    }

                    // String literals
                    '"' => {
                        self.advance();
                        let string_value = self.read_string(start_pos)?;
                        TokenKind::StringLiteral(string_value)
                    }

                    // Numbers
                    ch if ch.is_ascii_digit() => {
                        let number = self.read_number()?;
                        TokenKind::IntegerLiteral(number)
                    }

                    // Parameters
                    '$' => {
                        self.advance();
                        let param = self.read_parameter()?;
                        TokenKind::Parameter(param)
                    }

                    // Identifiers and keywords
                    ch if ch.is_alphabetic() || ch == '_' => {
                        let identifier = self.read_identifier();
                        self.keyword_or_identifier(identifier)
                    }

                    // Unexpected character
                    ch => {
                        return Err(CompilerError::UnexpectedCharacter(ch, start_pos));
                    }
                };

                Ok(Some(Token {
                    kind: token_kind,
                    position: start_pos,
                    line: start_line,
                    column: start_column,
                }))
            }
        }
    }

    fn advance(&mut self) {
        if let Some('\n') = self.current_char {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.position += 1;
        self.current_char = self.input.get(self.position).copied();
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.current_char {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn read_string(&mut self, start_pos: usize) -> Result<String, CompilerError> {
        let mut value = String::new();

        while let Some(ch) = self.current_char {
            match ch {
                '"' => {
                    self.advance();
                    return Ok(value);
                }
                '\\' => {
                    self.advance();
                    match self.current_char {
                        Some('n') => {
                            value.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            value.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            value.push('\r');
                            self.advance();
                        }
                        Some('\\') => {
                            value.push('\\');
                            self.advance();
                        }
                        Some('"') => {
                            value.push('"');
                            self.advance();
                        }
                        Some(ch) => {
                            value.push(ch);
                            self.advance();
                        }
                        None => return Err(CompilerError::UnterminatedString(start_pos)),
                    }
                }
                ch => {
                    value.push(ch);
                    self.advance();
                }
            }
        }

        Err(CompilerError::UnterminatedString(start_pos))
    }

    fn read_number(&mut self) -> Result<i16, CompilerError> {
        let mut value = String::new();

        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        value
            .parse::<i16>()
            .map_err(|_| CompilerError::LexicalError("Invalid number".to_string(), self.position))
    }

    fn read_identifier(&mut self) -> String {
        let mut value = String::new();

        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        value
    }

    fn read_parameter(&mut self) -> Result<String, CompilerError> {
        let mut value = String::new();

        // Parameter can be identifier (like $noun) or number (like $2)
        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if value.is_empty() {
            return Err(CompilerError::LexicalError(
                "Empty parameter name after '$'".to_string(),
                self.position,
            ));
        }

        Ok(value)
    }

    fn keyword_or_identifier(&self, identifier: String) -> TokenKind {
        match identifier.as_str() {
            "world" => TokenKind::World,
            "room" => TokenKind::Room,
            "object" => TokenKind::Object,
            "grammar" => TokenKind::Grammar,
            "verb" => TokenKind::Verb,
            "fn" => TokenKind::Function,
            "init" => TokenKind::Init,
            "contains" => TokenKind::Contains,
            "names" => TokenKind::Names,
            "desc" => TokenKind::Desc,
            "properties" => TokenKind::Properties,
            "exits" => TokenKind::Exits,
            "on_enter" => TokenKind::OnEnter,
            "on_exit" => TokenKind::OnExit,
            "on_look" => TokenKind::OnLook,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "return" => TokenKind::Return,
            "let" => TokenKind::Let,
            "var" => TokenKind::Var,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "print" => TokenKind::Print,
            "move" => TokenKind::Move,
            "player" => TokenKind::Player,
            "location" => TokenKind::Location,
            _ => TokenKind::Identifier(identifier),
        }
    }
}

#[cfg(test)]
#[path = "lexer_tests.rs"]
mod tests;
