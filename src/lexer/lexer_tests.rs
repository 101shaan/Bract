#[cfg(test)]
mod tests {
    use crate::lexer::lexer::Lexer;
    use crate::lexer::token::{TokenType, NumberBase};
    
    // Helper function to create a test lexer
    fn create_lexer(input: &str) -> Lexer {
        Lexer::new(input, 0)
    }
    
    // Helper function to get all tokens from a lexer
    fn collect_tokens(lexer: &mut Lexer) -> Vec<TokenType> {
        let mut tokens = Vec::new();
        loop {
            match lexer.next_token() {
                Ok(token) => {
                    let token_type = token.token_type;
                    tokens.push(token_type.clone());
                    if matches!(token_type, TokenType::Eof) {
                        break;
                    }
                },
                Err(e) => {
                    panic!("Lexer error: {}", e);
                }
            }
        }
        tokens
    }
    
    #[test]
    fn test_empty_input() {
        let mut lexer = create_lexer("");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![TokenType::Eof]);
    }
    
    #[test]
    fn test_whitespace() {
        let mut lexer = create_lexer(" \t\n\r");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![TokenType::Eof]);
    }
    
    #[test]
    fn test_single_char_tokens() {
        let mut lexer = create_lexer("+-*/(){}[];,.~?:");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Plus,
            TokenType::Minus,
            TokenType::Star,
            TokenType::Slash,
            TokenType::LeftParen,
            TokenType::RightParen,
            TokenType::LeftBrace,
            TokenType::RightBrace,
            TokenType::LeftBracket,
            TokenType::RightBracket,
            TokenType::Semicolon,
            TokenType::Comma,
            TokenType::Dot,
            TokenType::Tilde,
            TokenType::Question,
            TokenType::Colon,
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_multi_char_operators() {
        let mut lexer = create_lexer("== != <= >= && || -> :: += -= *= /= %= &= |= ^= <<= >>= << >> .. =>");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Eq,
            TokenType::NotEq,
            TokenType::LessEq,
            TokenType::GreaterEq,
            TokenType::LogicalAnd,
            TokenType::LogicalOr,
            TokenType::Arrow,
            TokenType::DoubleColon,
            TokenType::PlusEq,
            TokenType::MinusEq,
            TokenType::StarEq,
            TokenType::SlashEq,
            TokenType::PercentEq,
            TokenType::AndEq,
            TokenType::OrEq,
            TokenType::CaretEq,
            TokenType::LeftShiftEq,
            TokenType::RightShiftEq,
            TokenType::LeftShift,
            TokenType::RightShift,
            TokenType::DotDot,
            TokenType::FatArrow,
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_keywords() {
        let input = "fn let if else while for return struct enum impl trait \
                    mod pub use const mut break continue loop match type \
                    in move box extern abort do async await try";
        let mut lexer = create_lexer(input);
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Fn,
            TokenType::Let,
            TokenType::If,
            TokenType::Else,
            TokenType::While,
            TokenType::For,
            TokenType::Return,
            TokenType::Struct,
            TokenType::Enum,
            TokenType::Impl,
            TokenType::Trait,
            TokenType::Mod,
            TokenType::Pub,
            TokenType::Use,
            TokenType::Const,
            TokenType::Mut,
            TokenType::Break,
            TokenType::Continue,
            TokenType::Loop,
            TokenType::Match,
            TokenType::Type,
            TokenType::In,
            TokenType::Move,
            TokenType::Box,
            TokenType::Extern,
            TokenType::Abort,
            TokenType::Do,
            TokenType::Async,
            TokenType::Await,
            TokenType::Try,
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_boolean_literals() {
        let mut lexer = create_lexer("true false");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::True,
            TokenType::False,
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_null_literal() {
        let mut lexer = create_lexer("null");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Null,
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_identifiers() {
        let mut lexer = create_lexer("foo bar baz _test test123 x1 y2 z3");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Identifier("foo".to_string()),
            TokenType::Identifier("bar".to_string()),
            TokenType::Identifier("baz".to_string()),
            TokenType::Identifier("_test".to_string()),
            TokenType::Identifier("test123".to_string()),
            TokenType::Identifier("x1".to_string()),
            TokenType::Identifier("y2".to_string()),
            TokenType::Identifier("z3".to_string()),
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_integer_literals() {
        let mut lexer = create_lexer("123 0 42 0x1A 0b1010 0o777");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Integer { value: "123".to_string(), base: NumberBase::Decimal, suffix: None },
            TokenType::Integer { value: "0".to_string(), base: NumberBase::Decimal, suffix: None },
            TokenType::Integer { value: "42".to_string(), base: NumberBase::Decimal, suffix: None },
            TokenType::Integer { value: "0x1A".to_string(), base: NumberBase::Hexadecimal, suffix: None },
            TokenType::Integer { value: "0b1010".to_string(), base: NumberBase::Binary, suffix: None },
            TokenType::Integer { value: "0o777".to_string(), base: NumberBase::Octal, suffix: None },
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_float_literals() {
        let mut lexer = create_lexer("3.14 1.0 2e10 1.5e-3 0.5");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Float { value: "3.14".to_string(), suffix: None },
            TokenType::Float { value: "1.0".to_string(), suffix: None },
            TokenType::Float { value: "2e10".to_string(), suffix: None },
            TokenType::Float { value: "1.5e-3".to_string(), suffix: None },
            TokenType::Float { value: "0.5".to_string(), suffix: None },
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_numeric_literals_with_underscores() {
        let mut lexer = create_lexer("1_000_000 1_234.567_89 0xDEAD_BEEF 0b1010_1010");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Integer { value: "1000000".to_string(), base: NumberBase::Decimal, suffix: None },
            TokenType::Float { value: "1234.56789".to_string(), suffix: None },
            TokenType::Integer { value: "0xDEADBEEF".to_string(), base: NumberBase::Hexadecimal, suffix: None },
            TokenType::Integer { value: "0b10101010".to_string(), base: NumberBase::Binary, suffix: None },
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_numeric_literals_with_suffixes() {
        let mut lexer = create_lexer("123u32 3.14f64 0xFFi16");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Integer { value: "123".to_string(), base: NumberBase::Decimal, suffix: Some("u32".to_string()) },
            TokenType::Float { value: "3.14".to_string(), suffix: Some("f64".to_string()) },
            TokenType::Integer { value: "0xFF".to_string(), base: NumberBase::Hexadecimal, suffix: Some("i16".to_string()) },
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_raw_string_literals() {
        // Test basic raw string
        let mut lexer = create_lexer(r#"r"hello world""#);
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::String { value: "hello world".to_string(), raw: true, raw_delimiter: None },
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_raw_string_with_escape_sequences() {
        let mut lexer = create_lexer(r#"r"C:\path\to\file.txt" r"Line 1\nLine 2""#);
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::String { value: r"C:\path\to\file.txt".to_string(), raw: true, raw_delimiter: None },
            TokenType::String { value: r"Line 1\nLine 2".to_string(), raw: true, raw_delimiter: None },
            TokenType::Eof,
        ]);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_invalid_raw_string_delimiter() {
        let mut lexer = create_lexer(r"r#hello#");
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_unterminated_raw_string() {
        let mut lexer = create_lexer(r#"r#"unterminated raw string"#);
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_mismatched_raw_string_delimiters() {
        let mut lexer = create_lexer(r###"r##"mismatched delimiters"#"###);
        collect_tokens(&mut lexer);
    }
    
    #[test]
    fn test_character_literals() {
        let mut lexer = create_lexer("'a' 'Z' '0' '_'");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Char('a'),
            TokenType::Char('Z'),
            TokenType::Char('0'),
            TokenType::Char('_'),
            TokenType::Eof,
        ]);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_empty_char_literal() {
        let mut lexer = create_lexer("''");
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_unterminated_char_literal() {
        let mut lexer = create_lexer("'a");
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_multi_char_literal() {
        let mut lexer = create_lexer("'ab'");
        collect_tokens(&mut lexer);
    }
    
    #[test]
    fn test_string_literals() {
        let mut lexer = create_lexer(r#""hello world" "with \"quotes\"" "with \n \t \r \\ escapes" "empty""#);
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::String { value: "hello world".to_string(), raw: false, raw_delimiter: None },
            TokenType::String { value: "with \"quotes\"".to_string(), raw: false, raw_delimiter: None },
            TokenType::String { value: "with \n \t \r \\ escapes".to_string(), raw: false, raw_delimiter: None },
            TokenType::String { value: "empty".to_string(), raw: false, raw_delimiter: None },
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_unicode_escape_sequences() {
        let mut lexer = create_lexer(r#""Unicode: \u{1F600}""#); // Unicode for 😀
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::String { value: "Unicode: 😀".to_string(), raw: false, raw_delimiter: None },
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_line_comments() {
        let mut lexer = Lexer::new_with_comments("// This is a comment\nlet x = 5; // Another comment", 0);
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::LineComment(" This is a comment".to_string()),
            TokenType::Let,
            TokenType::Identifier("x".to_string()),
            TokenType::Equal,
            TokenType::Integer { value: "5".to_string(), base: NumberBase::Decimal, suffix: None },
            TokenType::Semicolon,
            TokenType::LineComment(" Another comment".to_string()),
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_block_comments() {
        let mut lexer = Lexer::new_with_comments("/* Block comment */\nlet x = 5; /* Another\nblock comment */", 0);
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::BlockComment(" Block comment ".to_string()),
            TokenType::Let,
            TokenType::Identifier("x".to_string()),
            TokenType::Equal,
            TokenType::Integer { value: "5".to_string(), base: NumberBase::Decimal, suffix: None },
            TokenType::Semicolon,
            TokenType::BlockComment(" Another\nblock comment ".to_string()),
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_nested_block_comments() {
        let mut lexer = Lexer::new_with_comments("/* Outer /* Nested */ Comment */", 0);
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::BlockComment(" Outer /* Nested */ Comment ".to_string()),
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_doc_comments() {
        let mut lexer = Lexer::new_with_comments("/// Doc comment\n/** Doc block comment */", 0);
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::DocLineComment(" Doc comment".to_string()),
            TokenType::DocBlockComment(" Doc block comment ".to_string()),
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_mixed_tokens() {
        let input = r#"
            fn main() {
                let x = 42;
                let y = 3.14;
                let message = "Hello, world!";
                let ch = 'a';
                let path = r"C:\path\to\file.txt";
                
                if x > 10 {
                    println!("x is greater than 10");
                } else {
                    println!("x is not greater than 10");
                }
                
                // This is a comment
                /* This is a block comment */
            }
        "#;
        
        let mut lexer = create_lexer(input);
        let tokens = collect_tokens(&mut lexer);
        
        // We'll just check the count and a few key tokens to avoid a massive assertion
        assert_eq!(tokens.len(), 53); // 52 tokens + EOF
        assert!(matches!(tokens[0], TokenType::Fn));
        assert!(matches!(tokens[1], TokenType::Identifier(ref s) if s == "main"));
        // Find the character 'a' token
        let has_char_a = tokens.iter().any(|t| matches!(t, TokenType::Char('a')));
        assert!(has_char_a);
        // Find the raw string token
        let has_raw_string = tokens.iter().any(|t| matches!(t, TokenType::String { raw: true, .. }));
        assert!(has_raw_string);
        assert!(matches!(tokens[52], TokenType::Eof));
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_invalid_character() {
        let mut lexer = create_lexer("§");  // Updated to use § instead of @
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_unterminated_string() {
        let mut lexer = create_lexer("\"unterminated string");
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_invalid_escape_sequence() {
        let mut lexer = create_lexer("\"invalid escape \\z\"");
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_invalid_unicode_escape() {
        let mut lexer = create_lexer("\"invalid unicode \\u{FFFFFFFF}\"");
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_invalid_number_format() {
        let mut lexer = create_lexer("0b2"); // Invalid binary digit
        collect_tokens(&mut lexer);
    }
    
    #[test]
    #[should_panic(expected = "Lexer error")]
    fn test_unterminated_block_comment() {
        let mut lexer = create_lexer("/* unterminated block comment");
        collect_tokens(&mut lexer);
    }
    
    #[test]
    fn test_position_tracking() {
        let input = "let x = 5;\nlet y = 10;";
        let mut lexer = create_lexer(input);
        
        // First token "let" at line 1, column 1
        let token = lexer.next_token().unwrap();
        assert_eq!(token.position.line, 1);
        assert_eq!(token.position.column, 1);
        
        // Skip to the second line - tokens are: let, x, =, 5, ;, let
        for _ in 0..4 {
            lexer.next_token().unwrap();
        }
        
        // First token on second line "let" at line 2, column 1
        let token = lexer.next_token().unwrap();
        assert_eq!(token.position.line, 2);
        assert_eq!(token.position.column, 1);
    }
    
    #[test]
    fn test_edge_case_adjacent_tokens() {
        let mut lexer = create_lexer("x+y-z*a/b");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Identifier("x".to_string()),
            TokenType::Plus,
            TokenType::Identifier("y".to_string()),
            TokenType::Minus,
            TokenType::Identifier("z".to_string()),
            TokenType::Star,
            TokenType::Identifier("a".to_string()),
            TokenType::Slash,
            TokenType::Identifier("b".to_string()),
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_edge_case_dot_vs_dotdot() {
        let mut lexer = create_lexer("x.y..z");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Identifier("x".to_string()),
            TokenType::Dot,
            TokenType::Identifier("y".to_string()),
            TokenType::DotDot,
            TokenType::Identifier("z".to_string()),
            TokenType::Eof,
        ]);
    }
    
    #[test]
    fn test_edge_case_float_vs_range() {
        let mut lexer = create_lexer("1.0..5.0");
        let tokens = collect_tokens(&mut lexer);
        assert_eq!(tokens, vec![
            TokenType::Float { value: "1.0".to_string(), suffix: None },
            TokenType::DotDot,
            TokenType::Float { value: "5.0".to_string(), suffix: None },
            TokenType::Eof,
        ]);
    }
} 