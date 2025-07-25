use crate::lexer::position::Position;
use crate::lexer::token::{Token, TokenType, NumberBase};
use crate::lexer::error::LexerError;
use std::char;
use std::collections::HashMap;

/// The Lexer is responsible for converting source code into tokens
pub struct Lexer<'a> {
    /// The input source code
    _input: &'a str,
    /// The characters of the input
    chars: std::str::Chars<'a>,
    /// The current position in the input
    current_pos: usize,
    /// The current character
    current_char: Option<char>,
    /// Position tracking for error reporting
    position: Position,
    /// Keyword lookup table
    keywords: HashMap<String, TokenType>,
    /// Whether to include comments in the token stream
    include_comments: bool,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from input source code
    pub fn new(input: &'a str, file_id: usize) -> Self {
        let mut chars = input.chars();
        let mut current_char = chars.next();
        let mut position = Position::start(file_id);
        let mut current_pos = 0;
        
        // Skip BOM (Byte Order Mark) if present
        if let Some('\u{FEFF}') = current_char {
            current_pos += 3; // BOM is 3 bytes in UTF-8
            position.advance(3); // UTF-8 BOM is 3 bytes
            current_char = chars.next();
        }
        
        Self {
            _input: input,
            chars,
            current_pos,
            current_char,
            position,
            keywords: Self::init_keywords(),
            include_comments: false,
        }
    }
    
    /// Create a new lexer that includes comments in the token stream
    pub fn new_with_comments(input: &'a str, file_id: usize) -> Self {
        let mut lexer = Self::new(input, file_id);
        lexer.include_comments = true;
        lexer
    }
    
    /// Initialize the keyword lookup table
    fn init_keywords() -> HashMap<String, TokenType> {
        let mut keywords = HashMap::new();
        
        // Add all keywords
        keywords.insert("abort".to_string(), TokenType::Abort);
        keywords.insert("break".to_string(), TokenType::Break);
        keywords.insert("box".to_string(), TokenType::Box);
        keywords.insert("const".to_string(), TokenType::Const);
        keywords.insert("continue".to_string(), TokenType::Continue);
        keywords.insert("do".to_string(), TokenType::Do);
        keywords.insert("else".to_string(), TokenType::Else);
        keywords.insert("enum".to_string(), TokenType::Enum);
        keywords.insert("extern".to_string(), TokenType::Extern);
        keywords.insert("false".to_string(), TokenType::False);
        keywords.insert("fn".to_string(), TokenType::Fn);
        keywords.insert("for".to_string(), TokenType::For);
        keywords.insert("if".to_string(), TokenType::If);
        keywords.insert("impl".to_string(), TokenType::Impl);
        keywords.insert("in".to_string(), TokenType::In);
        keywords.insert("let".to_string(), TokenType::Let);
        keywords.insert("loop".to_string(), TokenType::Loop);
        keywords.insert("match".to_string(), TokenType::Match);
        keywords.insert("mod".to_string(), TokenType::Mod);
        keywords.insert("move".to_string(), TokenType::Move);
        keywords.insert("mut".to_string(), TokenType::Mut);
        keywords.insert("pub".to_string(), TokenType::Pub);
        keywords.insert("return".to_string(), TokenType::Return);
        keywords.insert("struct".to_string(), TokenType::Struct);
        keywords.insert("true".to_string(), TokenType::True);
        keywords.insert("type".to_string(), TokenType::Type);
        keywords.insert("use".to_string(), TokenType::Use);
        keywords.insert("while".to_string(), TokenType::While);
        keywords.insert("async".to_string(), TokenType::Async);
        keywords.insert("await".to_string(), TokenType::Await);
        
        // Reserved keywords
        keywords.insert("trait".to_string(), TokenType::Trait);
        keywords.insert("try".to_string(), TokenType::Try);
        
        // Boolean literals and null
        keywords.insert("null".to_string(), TokenType::Null);
        
        keywords
    }
    
    /// Advance to the next character
    pub fn advance(&mut self) {
        if let Some(ch) = self.current_char {
            // Update position tracking
            if ch == '\n' {
                self.position.next_line();
            } else {
                self.position.next_column();
            }
            
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
    
    /// Check if a character can start an identifier
    fn is_identifier_start(ch: char) -> bool {
        ch.is_alphabetic() || ch == '_'
    }
    
    /// Tokenize a character literal
    fn tokenize_char(&mut self) -> Result<TokenType, LexerError> {
        let start_pos = self.position;
        
        // Consume the opening quote
        self.advance();
        
        // Check for empty character literal
        if self.current_char == Some('\'') {
            self.advance(); // Consume the closing quote
            return Err(LexerError::EmptyCharLiteral(start_pos));
        }
        
        let ch = if self.current_char == Some('\\') {
            // Escape sequence
            self.advance(); // Consume the backslash
            self.process_escape_sequence()?
        } else if let Some(c) = self.current_char {
            self.advance(); // Consume the character
            c
        } else {
            return Err(LexerError::UnterminatedChar(start_pos));
        };
        
        // Check for closing quote
        if self.current_char != Some('\'') {
            // If there are more characters before the closing quote, it's a multi-character literal
            if self.current_char.is_some() {
                return Err(LexerError::MultiCharLiteral(start_pos));
            } else {
                return Err(LexerError::UnterminatedChar(start_pos));
            }
        }
        
        // Consume the closing quote
        self.advance();
        
        Ok(TokenType::Char(ch))
    }
    
    /// Skip a line comment and return the comment text if include_comments is true
    fn skip_line_comment(&mut self) -> Option<TokenType> {
        let _start_pos = self.position;
        let mut comment = String::new();
        let is_doc_comment = self.peek() == Some('/');
        
        // Skip the second '/' or third '/' for doc comments
        if is_doc_comment {
            self.advance();
        }
        
        // Skip the current character (which is the second '/')
        self.advance();
        
        // Collect the comment text
        while let Some(ch) = self.current_char {
            if ch == '\n' {
                break;
            }
            comment.push(ch);
            self.advance();
        }
        
        // Return the comment token if include_comments is true
        if self.include_comments {
            if is_doc_comment {
                Some(TokenType::DocLineComment(comment))
            } else {
                Some(TokenType::LineComment(comment))
            }
        } else {
            None
        }
    }
    
    /// Skip a block comment and return the comment text if include_comments is true
    fn skip_block_comment(&mut self) -> Result<Option<TokenType>, LexerError> {
        let start_pos = self.position;
        let mut comment = String::new();
        let is_doc_comment = self.peek() == Some('*');
        
        // Skip the '*' character
        self.advance();
        
        // Check if it's a doc comment (second '*')
        if is_doc_comment {
            // Skip the second '*' for doc comments if present
            if self.current_char == Some('*') && self.peek() != Some('/') {
                self.advance();
            }
        }
        
        // Track nesting level for nested block comments
        let mut nesting_level = 1;
        
        // Collect the comment text
        while let Some(ch) = self.current_char {
            // Check for nested block comment start
            if ch == '/' && self.peek() == Some('*') {
                comment.push(ch);
                self.advance();
                comment.push(self.current_char.unwrap());
                self.advance();
                nesting_level += 1;
                continue;
            }
            
            // Check for block comment end
            if ch == '*' && self.peek() == Some('/') {
                self.advance(); // Skip the '*'
                self.advance(); // Skip the '/'
                nesting_level -= 1;
                
                if nesting_level == 0 {
                    break;
                } else {
                    comment.push('*');
                    comment.push('/');
                    continue;
                }
            }
            
            comment.push(ch);
            self.advance();
        }
        
        // Check if we reached the end of input without closing the comment
        if nesting_level > 0 {
            return Err(LexerError::UnterminatedBlockComment(start_pos));
        }
        
        // Return the comment token if include_comments is true
        if self.include_comments {
            if is_doc_comment {
                Ok(Some(TokenType::DocBlockComment(comment)))
            } else {
                Ok(Some(TokenType::BlockComment(comment)))
            }
        } else {
            Ok(None)
        }
    }
    
    /// Tokenize an identifier or keyword
    fn tokenize_identifier(&mut self) -> TokenType {
        let mut identifier = String::new();
        
        // Consume all identifier characters
        while let Some(ch) = self.current_char {
            if Self::is_identifier_char(ch) {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        // Check if it's a keyword
        if let Some(keyword_type) = self.keywords.get(&identifier) {
            keyword_type.clone()
        } else {
            // It's a regular identifier
            TokenType::Identifier(identifier)
        }
    }
    
    /// Process an escape sequence in a string literal
    fn process_escape_sequence(&mut self) -> Result<char, LexerError> {
        // We've already consumed the backslash
        let escape_pos = self.position;
        
        if let Some(ch) = self.current_char {
            self.advance(); // Consume the escape character
            
            match ch {
                'n' => Ok('\n'),
                'r' => Ok('\r'),
                't' => Ok('\t'),
                '\\' => Ok('\\'),
                '"' => Ok('"'),
                '\'' => Ok('\''),
                '0' => Ok('\0'),
                'u' => self.process_unicode_escape(escape_pos),
                _ => Err(LexerError::InvalidEscapeSequence(format!("\\{}", ch), escape_pos)),
            }
        } else {
            Err(LexerError::InvalidEscapeSequence("\\".to_string(), escape_pos))
        }
    }
    
    /// Process a Unicode escape sequence (\u{XXXX})
    fn process_unicode_escape(&mut self, escape_pos: Position) -> Result<char, LexerError> {
        // We expect a { after \u
        if self.current_char != Some('{') {
            return Err(LexerError::InvalidUnicodeEscape("\\u".to_string(), escape_pos));
        }
        self.advance(); // Consume the '{'
        
        let mut code_point = 0u32;
        let mut digit_count = 0;
        
        // Read hex digits until we find a closing }
        while let Some(ch) = self.current_char {
            if ch == '}' {
                self.advance(); // Consume the '}'
                break;
            }
            
            if let Some(digit) = ch.to_digit(16) {
                code_point = code_point * 16 + digit;
                digit_count += 1;
                
                if digit_count > 6 {
                    // Too many digits for a valid Unicode code point
                    return Err(LexerError::InvalidUnicodeEscape(
                        format!("\\u{{...}} (too many digits)"), 
                        escape_pos
                    ));
                }
                
                self.advance(); // Consume the digit
            } else {
                return Err(LexerError::InvalidUnicodeEscape(
                    format!("\\u{{{}...}}", ch), 
                    escape_pos
                ));
            }
        }
        
        if digit_count == 0 {
            return Err(LexerError::InvalidUnicodeEscape("\\u{}".to_string(), escape_pos));
        }
        
        // Convert the code point to a character
        match char::from_u32(code_point) {
            Some(c) => Ok(c),
            None => Err(LexerError::InvalidUnicodeEscape(
                format!("\\u{{{:x}}}", code_point), 
                escape_pos
            )),
        }
    }
    
    /// Tokenize a string literal
    fn tokenize_string(&mut self) -> Result<TokenType, LexerError> {
        let start_pos = self.position;
        let mut value = String::new();
        
        // Consume the opening quote
        self.advance();
        
        while let Some(ch) = self.current_char {
            if ch == '"' {
                // End of string
                self.advance(); // Consume the closing quote
                return Ok(TokenType::String { 
                    value, 
                    raw: false,
                    raw_delimiter: None,
                });
            } else if ch == '\\' {
                // Escape sequence
                self.advance(); // Consume the backslash
                let escaped_char = self.process_escape_sequence()?;
                value.push(escaped_char);
            } else {
                // Regular character
                value.push(ch);
                self.advance();
            }
        }
        
        // If we get here, the string was not terminated
        Err(LexerError::UnterminatedString(start_pos))
    }
    
    /// Tokenize a number literal (integer or float)
    fn tokenize_number(&mut self) -> Result<TokenType, LexerError> {
        let start_pos = self.position;
        let mut value = String::new();
        let mut base = NumberBase::Decimal;
        let mut is_float = false;
        let mut _has_exponent = false;
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
                _has_exponent = true;
                
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
    
    /// Count the number of hash symbols (#) in a raw string delimiter
    fn count_hashes(&mut self) -> usize {
        let mut count = 0;
        
        while let Some('#') = self.current_char {
            count += 1;
            self.advance();
        }
        
        count
    }
    
    /// Tokenize a raw string literal (r"..." or r#"..."#)
    fn tokenize_raw_string(&mut self) -> Result<TokenType, LexerError> {
        let start_pos = self.position;
        
        // Consume the 'r' character
        self.advance();
        
        // Count the number of hash symbols
        let hash_count = self.count_hashes();
        
        // Check for opening quote
        if self.current_char != Some('"') {
            return Err(LexerError::InvalidRawStringDelimiter(start_pos));
        }
        self.advance(); // Consume the opening quote
        
        let mut value = String::new();
        
        // Collect characters until we find the closing delimiter
        loop {
            if self.current_char.is_none() {
                return Err(LexerError::UnterminatedRawString(start_pos));
            }
            
            // Check for closing quote
            if self.current_char == Some('"') {
                self.advance(); // Consume the quote
                
                // Count the number of hash symbols after the quote
                let mut closing_hash_count = 0;
                while let Some('#') = self.current_char {
                    closing_hash_count += 1;
                    self.advance();
                }
                
                // If the number of hash symbols matches, we've found the end of the string
                if closing_hash_count == hash_count {
                    break;
                }
                
                // Otherwise, add the quote and hash symbols to the string value
                value.push('"');
                for _ in 0..closing_hash_count {
                    value.push('#');
                }
            } else {
                // Add the current character to the string value
                value.push(self.current_char.unwrap());
                self.advance();
            }
        }
        
        Ok(TokenType::String {
            value,
            raw: true,
            raw_delimiter: if hash_count > 0 { Some(hash_count) } else { None },
        })
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
        
        // Check for raw string literals
        if ch == 'r' && (self.peek() == Some('#') || self.peek() == Some('"')) {
            return match self.tokenize_raw_string() {
                Ok(token_type) => Ok(Token::new(token_type, position)),
                Err(err) => Err(err),
            };
        }
        
        // Check for character literals
        if ch == '\'' {
            return match self.tokenize_char() {
                Ok(token_type) => Ok(Token::new(token_type, position)),
                Err(err) => Err(err),
            };
        }
        
        // Check for comments
        if ch == '/' {
            if self.peek() == Some('/') {
                // Line comment
                self.advance(); // Skip the first '/'
                if let Some(comment_token) = self.skip_line_comment() {
                    return Ok(Token::new(comment_token, position));
                }
                return self.next_token(); // Skip the comment and get the next token
            } else if self.peek() == Some('*') {
                // Block comment
                self.advance(); // Skip the '/'
                match self.skip_block_comment() {
                    Ok(Some(comment_token)) => return Ok(Token::new(comment_token, position)),
                    Ok(None) => return self.next_token(), // Skip the comment and get the next token
                    Err(err) => return Err(err),
                }
            }
        }
        
        // Check for identifiers and keywords
        if Self::is_identifier_start(ch) {
            let token_type = self.tokenize_identifier();
            return Ok(Token::new(token_type, position));
        }
        
        // Check for string literals
        if ch == '"' {
            return match self.tokenize_string() {
                Ok(token_type) => Ok(Token::new(token_type, position)),
                Err(err) => Err(err),
            };
        }
        
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
                } else if let Some('>') = self.peek() {
                    self.advance(); // Consume the '>'
                    TokenType::FatArrow
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
            '@' => TokenType::At,  // For memory annotations
            
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