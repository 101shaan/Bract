//! Main parser implementation for the Prism programming language

use crate::lexer::{Lexer, Token, TokenType, Position};
use crate::ast::*;
use super::error::{ParseError, ParseResult};
use std::collections::HashMap;

/// String interner for efficient string storage
pub struct StringInterner {
    strings: Vec<String>,
    map: HashMap<String, u32>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            map: HashMap::new(),
        }
    }
    
    pub fn intern(&mut self, s: &str) -> InternedString {
        if let Some(&id) = self.map.get(s) {
            InternedString::new(id)
        } else {
            let id = self.strings.len() as u32;
            self.strings.push(s.to_string());
            self.map.insert(s.to_string(), id);
            InternedString::new(id)
        }
    }
    
    pub fn get(&self, interned: &InternedString) -> Option<&str> {
        self.strings.get(interned.id as usize).map(|s| s.as_str())
    }
}

/// The main parser struct that converts tokens to AST
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Option<Token>,
    interner: StringInterner,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    /// Create a new parser from source code
    pub fn new(input: &'a str, file_id: usize) -> ParseResult<Self> {
        let mut lexer = Lexer::new(input, file_id);
        let current_token = match lexer.next_token() {
            Ok(token) => Some(token),
            Err(err) => return Err(ParseError::from(err)),
        };
        
        Ok(Parser {
            lexer,
            current_token,
            interner: StringInterner::new(),
            errors: Vec::new(),
        })
    }
    
    /// Get the current token without consuming it
    pub fn current_token(&self) -> Option<&Token> {
        self.current_token.as_ref()
    }
    
    /// Advance to the next token
    pub fn advance(&mut self) -> ParseResult<()> {
        match self.lexer.next_token() {
            Ok(token) => {
                self.current_token = Some(token);
                Ok(())
            }
            Err(err) => Err(ParseError::from(err))
        }
    }
    
    /// Check if current token matches the expected type
    pub fn check(&self, token_type: &TokenType) -> bool {
        self.current_token
            .as_ref()
            .map(|t| std::mem::discriminant(&t.token_type) == std::mem::discriminant(token_type))
            .unwrap_or(false)
    }
    
    /// Check if we've reached the end of input
    pub fn is_at_end(&self) -> bool {
        self.current_token
            .as_ref()
            .map(|t| matches!(t.token_type, TokenType::Eof))
            .unwrap_or(true)
    }
    
    /// Consume a token if it matches the expected type
    pub fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance().unwrap_or(());
            true
        } else {
            false
        }
    }
    
    /// Expect a specific token type and consume it, or return an error
    pub fn expect(&mut self, expected: TokenType, context: &str) -> ParseResult<Token> {
        if let Some(token) = &self.current_token {
            if std::mem::discriminant(&token.token_type) == std::mem::discriminant(&expected) {
                let token = token.clone();
                self.advance()?;
                Ok(token)
            } else {
                Err(ParseError::UnexpectedToken {
                    expected: vec![format!("{:?}", expected)],
                    found: token.token_type.clone(),
                    position: token.position,
                })
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec![format!("{:?}", expected)],
                position: self.lexer.get_position(),
            })
        }
    }
    
    /// Get current position for error reporting
    pub fn current_position(&self) -> Position {
        self.current_token
            .as_ref()
            .map(|t| t.position)
            .unwrap_or_else(|| self.lexer.get_position())
    }
    
    /// Add an error to the error list but continue parsing
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }
    
    /// Get all accumulated errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
    
    /// Synchronize parser state after an error (error recovery)
    pub fn synchronize(&mut self) {
        while !self.is_at_end() {
            if let Some(token) = &self.current_token {
                match &token.token_type {
                    TokenType::Semicolon => {
                        self.advance().unwrap_or(());
                        return;
                    }
                    TokenType::Fn | TokenType::Struct | TokenType::Enum | 
                    TokenType::Let | TokenType::If | TokenType::While | 
                    TokenType::For | TokenType::Return => return,
                    _ => {
                        self.advance().unwrap_or(());
                    }
                }
            } else {
                break;
            }
        }
    }
    
    /// Parse a complete module (top-level entry point)
    pub fn parse_module(&mut self) -> ParseResult<Module> {
        let start_pos = self.current_position();
        let mut items = Vec::new();
        
        while !self.is_at_end() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(err) => {
                    self.add_error(err);
                    self.synchronize();
                }
            }
        }
        
        let end_pos = self.current_position();
        Ok(Module {
            items,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a top-level item (function, struct, etc.)
    pub fn parse_item(&mut self) -> ParseResult<Item> {
        let start_pos = self.current_position();
        
        // Parse visibility modifier
        let visibility = if self.match_token(&TokenType::Pub) {
            Visibility::Public
        } else {
            Visibility::Private
        };
        
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::Fn => self.parse_function(visibility, start_pos),
                TokenType::Struct => self.parse_struct(visibility, start_pos),
                TokenType::Enum => self.parse_enum(visibility, start_pos),
                TokenType::Type => self.parse_type_alias(visibility, start_pos),
                TokenType::Const => self.parse_const(visibility, start_pos),
                TokenType::Mod => self.parse_module_decl(visibility, start_pos),
                TokenType::Impl => self.parse_impl_block(start_pos),
                TokenType::Use => self.parse_use_decl(start_pos),
                _ => Err(ParseError::UnexpectedToken {
                    expected: vec!["item declaration".to_string()],
                    found: token.token_type.clone(),
                    position: token.position,
                }),
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["item declaration".to_string()],
                position: self.current_position(),
            })
        }
    }
    
    /// Parse a function declaration
    fn parse_function(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Fn, "function declaration")?;
        
        // Function name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "function name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected function name".to_string(),
                position: name_token.position,
            });
        };
        
        // Generic parameters (TODO: implement later)
        let generics = Vec::new();
        
        // Parameters
        self.expect(TokenType::LeftParen, "function parameters")?;
        let mut params = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                let param_start = self.current_position();
                let pattern = self.parse_pattern()?;
                
                let type_annotation = if self.match_token(&TokenType::Colon) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                
                params.push(Parameter {
                    pattern,
                    type_annotation,
                    span: Span::new(param_start, self.current_position()),
                });
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.expect(TokenType::RightParen, "function parameters")?;
        
        // Return type
        let return_type = if self.match_token(&TokenType::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        // Function body
        let body = if self.check(&TokenType::LeftBrace) {
            Some(self.parse_block_expression()?)
        } else {
            self.expect(TokenType::Semicolon, "function body or semicolon")?;
            None
        };
        
        let end_pos = self.current_position();
        Ok(Item::Function {
            visibility,
            name,
            generics,
            params,
            return_type,
            body,
            is_extern: false, // TODO: handle extern functions
            span: Span::new(start_pos, end_pos),
        })
    }
    
    // Placeholder methods for now - will be implemented in separate modules
    fn parse_struct(&mut self, _visibility: Visibility, _start_pos: Position) -> ParseResult<Item> {
        todo!("Struct parsing will be implemented")
    }
    
    fn parse_enum(&mut self, _visibility: Visibility, _start_pos: Position) -> ParseResult<Item> {
        todo!("Enum parsing will be implemented")
    }
    
    fn parse_type_alias(&mut self, _visibility: Visibility, _start_pos: Position) -> ParseResult<Item> {
        todo!("Type alias parsing will be implemented")
    }
    
    fn parse_const(&mut self, _visibility: Visibility, _start_pos: Position) -> ParseResult<Item> {
        todo!("Const parsing will be implemented")
    }
    
    fn parse_module_decl(&mut self, _visibility: Visibility, _start_pos: Position) -> ParseResult<Item> {
        todo!("Module declaration parsing will be implemented")
    }
    
    fn parse_impl_block(&mut self, _start_pos: Position) -> ParseResult<Item> {
        todo!("Impl block parsing will be implemented")
    }
    
    fn parse_use_decl(&mut self, _start_pos: Position) -> ParseResult<Item> {
        todo!("Use declaration parsing will be implemented")
    }
    
    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        todo!("Pattern parsing will be implemented")
    }
    
    fn parse_type(&mut self) -> ParseResult<Type> {
        todo!("Type parsing will be implemented")
    }
    
    fn parse_block_expression(&mut self) -> ParseResult<Expr> {
        todo!("Block expression parsing will be implemented")
    }
} 