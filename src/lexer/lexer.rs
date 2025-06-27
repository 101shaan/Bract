use crate::lexer::position::Position;
use crate::lexer::token::{Token, TokenType, NumberBase};
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
    
    /// Look at the character after next without advancing
    pub fn peek_next(&self) -> Option<char> {
        let mut chars_clone = self.chars.clone();
        chars_clone.next(); // Skip the next character
        chars_clone.next() // Get the character after next
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
    
    /// Check if a character is valid for an identifier
    fn is_identifier_char(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }
    
    /// Tokenize a number literal (integer or float)
    fn tokenize_number(&mut self) -> Result<TokenType, LexerError> {
        let start_pos = self.position;
        let mut value = String::new();
        let mut base = NumberBase::Decimal;
        let mut is_float = false;
        let mut has_exponent = false;
        let mut suffix = None;
        
        // Check for hex, octal, or binary prefix
        if self.current_char == Some('0') {
            value.push('0');
            self.advance();
            
            match self.current_char {
                Some('x') | Some('X') => {
                    value.push('x');
                    self.advance();
                    base = NumberBase::Hexadecimal;
                    
                    // Consume hexadecimal digits
                    let mut has_digits = false;
                    while let Some(ch) = self.current_char {
                        if ch.is_ascii_hexdigit() {
                            value.push(ch);
                            has_digits = true;
                            self.advance();
                        } else if ch == '_' {
                            // Skip underscores in numbers
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    if !has_digits {
                        return Err(LexerError::InvalidNumber(value, start_pos));
                    }
                },
                Some('o') | Some('O') => {
                    value.push('o');
                    self.advance();
                    base = NumberBase::Octal;
                    
                    // Consume octal digits
                    let mut has_digits = false;
                    while let Some(ch) = self.current_char {
                        if ch >= '0' && ch <= '7' {
                            value.push(ch);
                            has_digits = true;
                            self.advance();
                        } else if ch == '_' {
                            // Skip underscores in numbers
                            self.advance();
                        } else if ch.is_ascii_digit() {
                            return Err(LexerError::InvalidOctalDigit(ch, self.position));
                        } else {
                            break;
                        }
                    }
                    
                    if !has_digits {
                        return Err(LexerError::InvalidNumber(value, start_pos));
                    }
                },
                Some('b') | Some('B') => {
                    value.push('b');
                    self.advance();
                    base = NumberBase::Binary;
                    
                    // Consume binary digits
                    let mut has_digits = false;
                    while let Some(ch) = self.current_char {
                        if ch == '0' || ch == '1' {
                            value.push(ch);
                            has_digits = true;
                            self.advance();
                        } else if ch == '_' {
                            // Skip underscores in numbers
                            self.advance();
                        } else if ch.is_ascii_digit() {
                            return Err(LexerError::InvalidBinaryDigit(ch, self.position));
                        } else {
                            break;
                        }
                    }
                    
                    if !has_digits {
                        return Err(LexerError::InvalidNumber(value, start_pos));
                    }
                },
                _ => {
                    // Just a regular decimal number starting with 0
                    // Continue with the normal decimal number handling below
                }
            }
        }
        
        // If we didn't process a special base (or it's a decimal starting with 0)
        if base == NumberBase::Decimal {
            // Consume integer part
            while let Some(ch) = self.current_char {
                if ch.is_ascii_digit() {
                    value.push(ch);
                    self.advance();
                } else if ch == '_' {
                    // Skip underscores in numbers
                    self.advance();
                } else {
                    break;
                }
            }
            
            // Check for decimal point
            if self.current_char == Some('.') {
                // Look ahead to ensure it's not the start of a range operator (..)
                if self.peek() != Some('.') {
                    value.push('.');
                    self.advance();
                    is_float = true;
                    
                    // Consume fractional part
                    let mut has_fraction_digits = false;
                    while let Some(ch) = self.current_char {
                        if ch.is_ascii_digit() {
                            value.push(ch);
                            has_fraction_digits = true;
                            self.advance();
                        } else if ch == '_' {
                            // Skip underscores in numbers
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    // A decimal point must be followed by at least one digit
                    if !has_fraction_digits {
                        return Err(LexerError::InvalidNumber(value, start_pos));
                    }
                }
            }
            
            // Check for exponent
            if let Some('e') | Some('E') = self.current_char {
                value.push(self.current_char.unwrap());
                self.advance();
                is_float = true;
                has_exponent = true;
                
                // Check for exponent sign
                if let Some('+') | Some('-') = self.current_char {
                    value.push(self.current_char.unwrap());
                    self.advance();
                }
                
                // Consume exponent digits
                let mut has_exponent_digits = false;
                while let Some(ch) = self.current_char {
                    if ch.is_ascii_digit() {
                        value.push(ch);
                        has_exponent_digits = true;
                        self.advance();
                    } else if ch == '_' {
                        // Skip underscores in numbers
                        self.advance();
                    } else {
                        break;
                    }
                }
                
                // An exponent must be followed by at least one digit
                if !has_exponent_digits {
                    return Err(LexerError::InvalidNumber(value, start_pos));
                }
            }
        }
        
        // Check for numeric suffix
        if let Some(ch) = self.current_char {
            if Self::is_identifier_char(ch) {
                let mut suffix_str = String::new();
                
                while let Some(ch) = self.current_char {
                    if Self::is_identifier_char(ch) {
                        suffix_str.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                
                // Validate suffix
                // For now, we'll just store it and let the parser validate it
                suffix = Some(suffix_str);
            }
        }
        
        // Create the appropriate token type
        if is_float {
            Ok(TokenType::Float { value, suffix })
        } else {
            Ok(TokenType::Integer { value, base, suffix })
        }
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
        
        // Check for number literals
        if ch.is_ascii_digit() {
            return match self.tokenize_number() {
                Ok(token_type) => Ok(Token::new(token_type, position)),
                Err(err) => Err(err),
            };
        }
        
        // Create the token based on the character
        let token_type = match ch {
            // Multi-character operators
            '=' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::Eq
                } else {
                    TokenType::Equal
                }
            },
            '!' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::NotEq
                } else {
                    TokenType::Not
                }
            },
            '<' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::LessEq
                } else if let Some('<') = self.peek() {
                    self.advance(); // Consume the '<'
                    if let Some('=') = self.peek() {
                        self.advance(); // Consume the '='
                        TokenType::LeftShiftEq
                    } else {
                        TokenType::LeftShift
                    }
                } else {
                    TokenType::Less
                }
            },
            '>' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::GreaterEq
                } else if let Some('>') = self.peek() {
                    self.advance(); // Consume the '>'
                    if let Some('=') = self.peek() {
                        self.advance(); // Consume the '='
                        TokenType::RightShiftEq
                    } else {
                        TokenType::RightShift
                    }
                } else {
                    TokenType::Greater
                }
            },
            '&' => {
                if let Some('&') = self.peek() {
                    self.advance(); // Consume the '&'
                    TokenType::LogicalAnd
                } else if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::AndEq
                } else {
                    TokenType::And
                }
            },
            '|' => {
                if let Some('|') = self.peek() {
                    self.advance(); // Consume the '|'
                    TokenType::LogicalOr
                } else if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::OrEq
                } else {
                    TokenType::Or
                }
            },
            '-' => {
                if let Some('>') = self.peek() {
                    self.advance(); // Consume the '>'
                    TokenType::Arrow
                } else if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::MinusEq
                } else {
                    TokenType::Minus
                }
            },
            ':' => {
                if let Some(':') = self.peek() {
                    self.advance(); // Consume the ':'
                    TokenType::DoubleColon
                } else {
                    TokenType::Colon
                }
            },
            '+' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::PlusEq
                } else {
                    TokenType::Plus
                }
            },
            '*' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::StarEq
                } else {
                    TokenType::Star
                }
            },
            '/' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::SlashEq
                } else {
                    TokenType::Slash
                }
            },
            '%' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::PercentEq
                } else {
                    TokenType::Percent
                }
            },
            '^' => {
                if let Some('=') = self.peek() {
                    self.advance(); // Consume the '='
                    TokenType::CaretEq
                } else {
                    TokenType::Caret
                }
            },
            '.' => {
                if let Some('.') = self.peek() {
                    self.advance(); // Consume the '.'
                    TokenType::DotDot
                } else {
                    TokenType::Dot
                }
            },
            
            // Single-char operators and punctuation
            '~' => TokenType::Tilde,
            '?' => TokenType::Question,
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            '{' => TokenType::LeftBrace,
            '}' => TokenType::RightBrace,
            '[' => TokenType::LeftBracket,
            ']' => TokenType::RightBracket,
            ';' => TokenType::Semicolon,
            ',' => TokenType::Comma,
            
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