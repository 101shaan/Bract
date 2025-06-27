use crate::lexer::position::Position;
use crate::lexer::token::{Token, TokenType};
use crate::lexer::error::LexerError;

/// The Lexer is responsible for converting source code into tokens
pub struct Lexer<'a> {
    /// The input source code
    input: &'a str,
    /// The characters of the input
    chars: std::str::Chars<'a>,
    /// The current position in the input
    current_pos: usize,
    /// The current character
    current_char: Option<char>,
    /// Position tracking for error reporting
    position: Position,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from input source code
    pub fn new(input: &'a str, file_id: usize) -> Self {
        let mut chars = input.chars();
        let current_char = chars.next();
        
        Self {
            input,
            chars,
            current_pos: 0,
            current_char,
            position: Position::start(file_id),
        }
    }
    
    /// Advance to the next character
    pub fn advance(&mut self) {
        if let Some(ch) = self.current_char {
            // Update position tracking
            self.position.advance(ch);
            
            // Move to next character
            self.current_pos += ch.len_utf8();
            self.current_char = self.chars.next();
        }
    }
    
    /// Look at the next character without advancing
    pub fn peek(&self) -> Option<char> {
        let mut chars_clone = self.chars.clone();
        chars_clone.next()
    }
    
    /// Get the current character
    pub fn current_char(&self) -> Option<char> {
        self.current_char
    }
    
    /// Check if we've reached the end of input
    pub fn is_at_end(&self) -> bool {
        self.current_char.is_none()
    }
    
    /// Skip whitespace characters
    pub fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    /// Get the current position
    pub fn get_position(&self) -> Position {
        self.position
    }
    
    /// Get the next token from the input
    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        // Skip any whitespace
        self.skip_whitespace();
        
        // If we're at the end, return EOF token
        if self.is_at_end() {
            return Ok(Token::new(TokenType::Eof, self.position));
        }
        
        // Get the current character
        let ch = self.current_char.unwrap();
        let position = self.position;
        
        // Create the token based on the character
        let token_type = match ch {
            // Single-char operators
            '+' => TokenType::Plus,
            '-' => TokenType::Minus,
            '*' => TokenType::Star,
            '/' => TokenType::Slash,
            '=' => TokenType::Equal,
            '<' => TokenType::Less,
            '>' => TokenType::Greater,
            '!' => TokenType::Not,
            '&' => TokenType::And,
            '|' => TokenType::Or,
            '^' => TokenType::Caret,
            '%' => TokenType::Percent,
            '~' => TokenType::Tilde,
            '?' => TokenType::Question,
            ':' => TokenType::Colon,
            
            // Punctuation
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            '{' => TokenType::LeftBrace,
            '}' => TokenType::RightBrace,
            '[' => TokenType::LeftBracket,
            ']' => TokenType::RightBracket,
            ';' => TokenType::Semicolon,
            ',' => TokenType::Comma,
            '.' => TokenType::Dot,
            
            // Unknown character
            _ => {
                self.advance(); // Consume the invalid character
                return Err(LexerError::InvalidCharacter(ch, position));
            }
        };
        
        // Advance to the next character
        self.advance();
        
        // Return the token
        Ok(Token::new(token_type, position))
    }
} 