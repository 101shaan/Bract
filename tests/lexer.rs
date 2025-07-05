//! Lexer integration tests
//! 
//! These tests validate the lexer's ability to tokenize real Prism code
//! and handle complex scenarios that go beyond unit tests.

use prism::lexer::{Lexer, TokenType};

/// Test lexing a complete Prism program
#[test]
fn test_lex_complete_program() {
    let source = r#"
        // Simple function with all major token types
        fn fibonacci(n: i32) -> i32 {
            if n <= 1 {
                return n;
            }
            return fibonacci(n - 1) + fibonacci(n - 2);
        }
        
        fn main() -> i32 {
            let result = fibonacci(10);
            println!("Fibonacci(10) = {}", result);
            return 0;
        }
    "#;
    
    let mut lexer = Lexer::new(source, 0);
    let mut tokens = Vec::new();
    
    // Collect all tokens
    loop {
        match lexer.next_token() {
            Ok(token) => {
                let is_eof = matches!(token.token_type, TokenType::Eof);
                tokens.push(token.token_type);
                if is_eof {
                    break;
                }
            }
            Err(e) => panic!("Lexer error: {}", e),
        }
    }
    
    // Verify we got the expected tokens
    assert!(!tokens.is_empty());
    assert!(matches!(tokens[0], TokenType::Fn));
    assert!(matches!(tokens.last(), Some(TokenType::Eof)));
    
    // Count major token types
    let fn_count = tokens.iter().filter(|t| matches!(t, TokenType::Fn)).count();
    let if_count = tokens.iter().filter(|t| matches!(t, TokenType::If)).count();
    let return_count = tokens.iter().filter(|t| matches!(t, TokenType::Return)).count();
    let let_count = tokens.iter().filter(|t| matches!(t, TokenType::Let)).count();
    
    assert_eq!(fn_count, 2); // fibonacci and main
    assert_eq!(if_count, 1); // if condition
    assert_eq!(return_count, 3); // three return statements
    assert_eq!(let_count, 1); // one let binding
}

/// Test lexing complex expressions
#[test]
fn test_lex_complex_expressions() {
    let source = r#"
        let complex = (a + b) * (c - d) / (e % f);
        let bitwise = (x & y) | (z ^ w) << 2 >> 1;
        let logical = (a && b) || (c != d) && (e >= f);
        let assignment = x += y *= z /= w %= q;
    "#;
    
    let mut lexer = Lexer::new(source, 0);
    let mut tokens = Vec::new();
    
    // Collect all tokens
    loop {
        match lexer.next_token() {
            Ok(token) => {
                let is_eof = matches!(token.token_type, TokenType::Eof);
                tokens.push(token.token_type);
                if is_eof {
                    break;
                }
            }
            Err(e) => panic!("Lexer error: {}", e),
        }
    }
    
    // Verify complex operators are tokenized correctly
    assert!(tokens.iter().any(|t| matches!(t, TokenType::LogicalAnd)));
    assert!(tokens.iter().any(|t| matches!(t, TokenType::LogicalOr)));
    assert!(tokens.iter().any(|t| matches!(t, TokenType::LeftShift)));
    assert!(tokens.iter().any(|t| matches!(t, TokenType::RightShift)));
    assert!(tokens.iter().any(|t| matches!(t, TokenType::PlusEq)));
    assert!(tokens.iter().any(|t| matches!(t, TokenType::StarEq)));
}

/// Test lexing string literals with various escape sequences
#[test]
fn test_lex_string_literals() {
    let source = r#"
        let simple = "Hello, world!";
        let escaped = "Line 1\nLine 2\tTabbed";
        let quoted = "She said \"Hello!\"";
        let raw = r"Raw string with \n no escapes";
    "#;
    
    let mut lexer = Lexer::new(source, 0);
    let mut string_tokens = Vec::new();
    
    // Collect all string tokens
    loop {
        match lexer.next_token() {
            Ok(token) => {
                let is_eof = matches!(token.token_type, TokenType::Eof);
                if let TokenType::String { value, raw, .. } = token.token_type {
                    string_tokens.push((value, raw));
                }
                if is_eof {
                    break;
                }
            }
            Err(e) => panic!("Lexer error: {}", e),
        }
    }
    
    assert_eq!(string_tokens.len(), 4);
    assert_eq!(string_tokens[0].0, "Hello, world!");
    assert!(!string_tokens[1].0.is_empty());
    assert!(!string_tokens[2].0.is_empty());
    assert_eq!(string_tokens[3].1, true); // Raw string
}

/// Test lexing numeric literals
#[test]
fn test_lex_numeric_literals() {
    let source = r#"
        let decimal = 42;
        let hex = 0xFF;
        let binary = 0b1010;
        let octal = 0o777;
        let float = 3.14;
        let scientific = 1.5e-10;
    "#;
    
    let mut lexer = Lexer::new(source, 0);
    let mut numeric_tokens = Vec::new();
    
    // Collect all numeric tokens
    loop {
        match lexer.next_token() {
            Ok(token) => {
                let is_eof = matches!(token.token_type, TokenType::Eof);
                match token.token_type {
                    TokenType::Integer { value, base, suffix } => {
                        numeric_tokens.push(format!("int:{}:{:?}:{:?}", value, base, suffix));
                    }
                    TokenType::Float { value, suffix } => {
                        numeric_tokens.push(format!("float:{}:{:?}", value, suffix));
                    }
                    _ => {}
                }
                if is_eof {
                    break;
                }
            }
            Err(e) => panic!("Lexer error: {}", e),
        }
    }
    
    assert_eq!(numeric_tokens.len(), 6);
    assert!(numeric_tokens[0].contains("int:42:"));
    assert!(numeric_tokens[1].contains("int:0xFF:"));
    assert!(numeric_tokens[2].contains("int:0b1010:"));
    assert!(numeric_tokens[3].contains("int:0o777:"));
    assert!(numeric_tokens[4].contains("float:3.14:"));
    assert!(numeric_tokens[5].contains("float:1.5e-10:"));
}

/// Test lexer performance with large input
#[test]
fn test_lex_performance() {
    use std::time::Instant;
    
    // Generate a large Prism program
    let mut functions = Vec::new();
    for i in 0..1000 {
        let func_name = format!("func_{}", i);
        let param_name = format!("param_{}", i);
        let function_text = format!("fn {}({}: i32) -> i32 {{ return {} * 2; }}", func_name, param_name, param_name);
        functions.push(function_text);
    }
    let source = functions.join("\n");
    
    let start = Instant::now();
    let mut lexer = Lexer::new(&source, 0);
    let mut token_count = 0;
    
    // Tokenize the entire large program
    loop {
        match lexer.next_token() {
            Ok(token) => {
                token_count += 1;
                if matches!(token.token_type, TokenType::Eof) {
                    break;
                }
            }
            Err(e) => panic!("Lexer error: {}", e),
        }
    }
    
    let elapsed = start.elapsed();
    
    println!("Lexed {} tokens in {:?}", token_count, elapsed);
    let tokens_per_ms = token_count as f64 / elapsed.as_millis() as f64;
    
    // Use format! to avoid prefix issues
    let perf_msg = format!("Performance: {:.2} tokens per millisecond", tokens_per_ms);
    println!("{}", perf_msg);
    
    // Verify we got a reasonable number of tokens
    assert!(token_count > 10000);
    
    // Performance assertion
    let error_msg = format!("Lexer performance too slow: {:.2} tokens per millisecond", tokens_per_ms);
    assert!(tokens_per_ms > 1000.0, "{}", error_msg);
}

/// Test lexer error handling
#[test]
fn test_lex_error_handling() {
    let invalid_sources = vec![
        ("@", "invalid character"),
        ("\"unterminated", "unterminated string"),
        ("'unterminated", "unterminated character"), 
        ("/* unterminated", "unterminated comment"),
    ];
    
    for (source, expected_error) in invalid_sources {
        let mut lexer = Lexer::new(source, 0);
        let mut found_error = false;
        
        loop {
            match lexer.next_token() {
                Ok(token) => {
                    if matches!(token.token_type, TokenType::Eof) {
                        break;
                    }
                }
                Err(_) => {
                    found_error = true;
                    break;
                }
            }
        }
        
        assert!(found_error, "Expected error for '{}' containing '{}'", source, expected_error);
    }
}

/// Test lexer with comments
#[test]
fn test_lex_with_comments() {
    let source = r#"
        // Line comment
        fn test() {
            /* Block comment */
            let x = 42; // End of line comment
            /* Multi-line
               block comment */
            return x;
        }
        
        /// Doc comment
        /** Doc block comment */
        fn documented() {}
    "#;
    
    let mut lexer = Lexer::new_with_comments(source, 0);
    let mut comment_tokens = Vec::new();
    
    // Collect all comment tokens
    loop {
        match lexer.next_token() {
            Ok(token) => {
                match token.token_type {
                    TokenType::LineComment(ref content) => {
                        comment_tokens.push(format!("line:{}", content));
                    }
                    TokenType::BlockComment(ref content) => {
                        comment_tokens.push(format!("block:{}", content));
                    }
                    TokenType::DocLineComment(ref content) => {
                        comment_tokens.push(format!("doc_line:{}", content));
                    }
                    TokenType::DocBlockComment(ref content) => {
                        comment_tokens.push(format!("doc_block:{}", content));
                    }
                    _ => {}
                }
                if matches!(token.token_type, TokenType::Eof) {
                    break;
                }
            }
            Err(e) => panic!("Lexer error: {}", e),
        }
    }
    
    assert_eq!(comment_tokens.len(), 6);
    assert!(comment_tokens[0].starts_with("line:"));
    assert!(comment_tokens[1].starts_with("block:"));
    assert!(comment_tokens[2].starts_with("line:"));
    assert!(comment_tokens[3].contains("Multi-line"));
    assert!(comment_tokens[4].starts_with("doc_line:"));
    assert!(comment_tokens[5].starts_with("doc_block:"));
} 