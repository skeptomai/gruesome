// Comprehensive lexer tests

#[cfg(test)]
mod tests {
    use crate::grue_compiler::lexer::{Lexer, TokenKind};

    fn tokenize_input(input: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(input);
        lexer
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn test_empty_input() {
        let tokens = tokenize_input("");
        assert_eq!(tokens, vec![TokenKind::EOF]);
    }

    #[test]
    fn test_single_tokens() {
        let tokens = tokenize_input("{ } [ ] ( ) ; : , . + - * / % ? ! < > =");
        assert_eq!(
            tokens,
            vec![
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::Semicolon,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Question,
                TokenKind::Not,
                TokenKind::Less,
                TokenKind::Greater,
                TokenKind::Equal,
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_multi_character_operators() {
        let tokens = tokenize_input("== != <= >= && || =>");
        assert_eq!(
            tokens,
            vec![
                TokenKind::EqualEqual,
                TokenKind::NotEqual,
                TokenKind::LessEqual,
                TokenKind::GreaterEqual,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Arrow,
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let tokens = tokenize_input(
            "world room object grammar verb fn init if else while for return let var true false",
        );
        assert_eq!(
            tokens,
            vec![
                TokenKind::World,
                TokenKind::Room,
                TokenKind::Object,
                TokenKind::Grammar,
                TokenKind::Verb,
                TokenKind::Function,
                TokenKind::Init,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::While,
                TokenKind::For,
                TokenKind::Return,
                TokenKind::Let,
                TokenKind::Var,
                TokenKind::True,
                TokenKind::False,
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_identifiers() {
        let tokens = tokenize_input("hello _world test123 _123abc");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Identifier("hello".to_string()),
                TokenKind::Identifier("_world".to_string()),
                TokenKind::Identifier("test123".to_string()),
                TokenKind::Identifier("_123abc".to_string()),
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_numbers() {
        let tokens = tokenize_input("0 42 1234 32767");
        assert_eq!(
            tokens,
            vec![
                TokenKind::IntegerLiteral(0),
                TokenKind::IntegerLiteral(42),
                TokenKind::IntegerLiteral(1234),
                TokenKind::IntegerLiteral(32767),
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_strings() {
        let tokens =
            tokenize_input(r#""hello" "world with spaces" "with\nnewlines" "with\"quotes""#);
        assert_eq!(
            tokens,
            vec![
                TokenKind::StringLiteral("hello".to_string()),
                TokenKind::StringLiteral("world with spaces".to_string()),
                TokenKind::StringLiteral("with\nnewlines".to_string()),
                TokenKind::StringLiteral("with\"quotes".to_string()),
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_comments() {
        let tokens = tokenize_input("hello // this is a comment\nworldtest");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Identifier("hello".to_string()),
                TokenKind::Newline,
                TokenKind::Identifier("worldtest".to_string()),
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_newlines() {
        let tokens = tokenize_input("hello\n\n\nworldtest");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Identifier("hello".to_string()),
                TokenKind::Newline, // Multiple newlines collapsed into one
                TokenKind::Identifier("worldtest".to_string()),
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_whitespace_handling() {
        let tokens = tokenize_input("   hello   \t  worldtest   ");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Identifier("hello".to_string()),
                TokenKind::Identifier("worldtest".to_string()),
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_position_tracking() {
        let mut lexer = Lexer::new("hello world");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].position, 0); // "hello" starts at 0
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        assert_eq!(tokens[1].position, 6); // "world" starts at 6
        assert_eq!(tokens[1].line, 1);
        assert_eq!(tokens[1].column, 7);
    }

    #[test]
    fn test_multiline_position_tracking() {
        let mut lexer = Lexer::new("hello\nworld\ntest");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].line, 1); // "hello"
        assert_eq!(tokens[0].column, 1);

        assert_eq!(tokens[1].line, 1); // newline
        assert_eq!(tokens[1].column, 6);

        assert_eq!(tokens[2].line, 2); // "world"
        assert_eq!(tokens[2].column, 1);

        assert_eq!(tokens[3].line, 2); // newline
        assert_eq!(tokens[3].column, 6);

        assert_eq!(tokens[4].line, 3); // "test"
        assert_eq!(tokens[4].column, 1);
    }

    #[test]
    fn test_complex_program() {
        let input = r#"
            world {
                room west_house "West of House" {
                    desc: "You are standing in a field."
                    exits: { north: north_house }
                }
            }
            
            grammar {
                verb "look" {
                    default => look_around()
                }
            }
            
            fn look_around() {
                if player.location == west_house {
                    print("Looking around...");
                } else {
                    print("You see nothing special.");
                }
            }
        "#;

        let tokens = tokenize_input(input);

        // Check that we get the expected sequence of tokens
        assert!(tokens.contains(&TokenKind::World));
        assert!(tokens.contains(&TokenKind::Room));
        assert!(tokens.contains(&TokenKind::Grammar));
        assert!(tokens.contains(&TokenKind::Function));
        assert!(tokens.contains(&TokenKind::If));
        assert!(tokens.contains(&TokenKind::StringLiteral("West of House".to_string())));
        assert!(tokens.contains(&TokenKind::StringLiteral(
            "You are standing in a field.".to_string()
        )));
        assert!(tokens.contains(&TokenKind::Identifier("west_house".to_string())));
        assert!(tokens.contains(&TokenKind::Arrow));
        assert!(tokens.last() == Some(&TokenKind::EOF));
    }

    // Error cases
    #[test]
    fn test_unterminated_string() {
        let mut lexer = Lexer::new(r#""hello world"#);
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_unexpected_character() {
        let mut lexer = Lexer::new("hello @ world");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_single_ampersand_error() {
        let mut lexer = Lexer::new("hello & world");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_single_pipe_error() {
        let mut lexer = Lexer::new("hello | world");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_escape_sequences() {
        let tokens = tokenize_input(r#""line1\nline2\ttab\r\n\\backslash""#);
        assert_eq!(
            tokens,
            vec![
                TokenKind::StringLiteral("line1\nline2\ttab\r\n\\backslash".to_string()),
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_keyword_vs_identifier_boundary() {
        // Test that keywords are recognized correctly when adjacent to other tokens
        let tokens = tokenize_input("worldtest roomworld ifelse");
        assert_eq!(
            tokens,
            vec![
                TokenKind::Identifier("worldtest".to_string()),
                TokenKind::Identifier("roomworld".to_string()),
                TokenKind::Identifier("ifelse".to_string()),
                TokenKind::EOF,
            ]
        );
    }

    #[test]
    fn test_number_boundaries() {
        let tokens = tokenize_input("123abc abc123 123.456");
        // Note: 123.456 should tokenize as 123, ., 456 since we don't support floats
        assert_eq!(
            tokens,
            vec![
                TokenKind::IntegerLiteral(123),
                TokenKind::Identifier("abc".to_string()),
                TokenKind::Identifier("abc123".to_string()),
                TokenKind::IntegerLiteral(123),
                TokenKind::Dot,
                TokenKind::IntegerLiteral(456),
                TokenKind::EOF,
            ]
        );
    }
}
