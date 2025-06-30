//! Recursive descent parser for the Prism programming language
//!
//! This parser converts a stream of tokens from the lexer into an Abstract Syntax Tree (AST).
//! It implements a recursive descent parser with operator precedence parsing for expressions.
//!
//! Design principles:
//! - Clean recursive descent following the grammar
//! - Robust error handling with recovery
//! - Efficient operator precedence parsing
//! - Position tracking for excellent error messages

use crate::lexer::{Lexer, Token, TokenType, LexerError, Position};
use crate::ast::*;
use std::fmt;

/// Parser errors with detailed information for user-friendly error messages
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Unexpected token encountered
    UnexpectedToken {
        expected: Vec<String>,
        found: TokenType,
        position: Position,
    },
    /// Unexpected end of file
    UnexpectedEof {
        expected: Vec<String>,
        position: Position,
    },
    /// Lexer error during parsing
    LexerError(LexerError),
    /// Invalid syntax construct
    InvalidSyntax {
        message: String,
        position: Position,
    },
    /// Expression expected but not found
    ExpectedExpression {
        position: Position,
    },
    /// Pattern expected but not found
    ExpectedPattern {
        position: Position,
    },
    /// Type expected but not found
    ExpectedType {
        position: Position,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found, position } => {
                write!(f, "Unexpected token at {}:{}: expected {}, found {:?}", 
                       position.line, position.column, 
                       expected.join(" or "), found)
            }
            ParseError::UnexpectedEof { expected, position } => {
                write!(f, "Unexpected end of file at {}:{}: expected {}", 
                       position.line, position.column, expected.join(" or "))
            }
            ParseError::LexerError(err) => write!(f, "Lexer error: {}", err),
            ParseError::InvalidSyntax { message, position } => {
                write!(f, "Invalid syntax at {}:{}: {}", 
                       position.line, position.column, message)
            }
            ParseError::ExpectedExpression { position } => {
                write!(f, "Expected expression at {}:{}", position.line, position.column)
            }
            ParseError::ExpectedPattern { position } => {
                write!(f, "Expected pattern at {}:{}", position.line, position.column)
            }
            ParseError::ExpectedType { position } => {
                write!(f, "Expected type at {}:{}", position.line, position.column)
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        ParseError::LexerError(err)
    }
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// String interner for efficient string storage
pub struct StringInterner {
    strings: Vec<String>,
    map: std::collections::HashMap<String, u32>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            map: std::collections::HashMap::new(),
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
    
    /// Parse a struct declaration
    fn parse_struct(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Struct, "struct declaration")?;
        
        let name_token = self.expect(TokenType::Identifier("".to_string()), "struct name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected struct name".to_string(),
                position: name_token.position,
            });
        };
        
        let generics = Vec::new(); // TODO: implement generics
        
        let fields = if self.match_token(&TokenType::LeftBrace) {
            // Named fields
            let mut field_list = Vec::new();
            
            while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                let field_start = self.current_position();
                
                // Field visibility
                let field_visibility = if self.match_token(&TokenType::Pub) {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                
                // Field name
                let field_name_token = self.expect(TokenType::Identifier("".to_string()), "field name")?;
                let field_name = if let TokenType::Identifier(name_str) = field_name_token.token_type {
                    self.interner.intern(&name_str)
                } else {
                    return Err(ParseError::InvalidSyntax {
                        message: "Expected field name".to_string(),
                        position: field_name_token.position,
                    });
                };
                
                self.expect(TokenType::Colon, "field type")?;
                let field_type = self.parse_type()?;
                
                field_list.push(StructField {
                    visibility: field_visibility,
                    name: field_name,
                    field_type,
                    span: Span::new(field_start, self.current_position()),
                });
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
            
            self.expect(TokenType::RightBrace, "struct fields")?;
            StructFields::Named(field_list)
        } else if self.match_token(&TokenType::LeftParen) {
            // Tuple struct
            let mut field_types = Vec::new();
            
            while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                field_types.push(self.parse_type()?);
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
            
            self.expect(TokenType::RightParen, "tuple struct fields")?;
            self.expect(TokenType::Semicolon, "tuple struct declaration")?;
            StructFields::Tuple(field_types)
        } else {
            // Unit struct
            self.expect(TokenType::Semicolon, "unit struct declaration")?;
            StructFields::Unit
        };
        
        let end_pos = self.current_position();
        Ok(Item::Struct {
            visibility,
            name,
            generics,
            fields,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse an enum declaration
    fn parse_enum(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Enum, "enum declaration")?;
        
        let name_token = self.expect(TokenType::Identifier("".to_string()), "enum name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected enum name".to_string(),
                position: name_token.position,
            });
        };
        
        let generics = Vec::new(); // TODO: implement generics
        
        self.expect(TokenType::LeftBrace, "enum variants")?;
        let mut variants = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            let variant_start = self.current_position();
            
            let variant_name_token = self.expect(TokenType::Identifier("".to_string()), "variant name")?;
            let variant_name = if let TokenType::Identifier(name_str) = variant_name_token.token_type {
                self.interner.intern(&name_str)
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected variant name".to_string(),
                    position: variant_name_token.position,
                });
            };
            
            let fields = if self.match_token(&TokenType::LeftBrace) {
                // Named variant fields
                let mut field_list = Vec::new();
                
                while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                    let field_start = self.current_position();
                    
                    let field_name_token = self.expect(TokenType::Identifier("".to_string()), "field name")?;
                    let field_name = if let TokenType::Identifier(name_str) = field_name_token.token_type {
                        self.interner.intern(&name_str)
                    } else {
                        return Err(ParseError::InvalidSyntax {
                            message: "Expected field name".to_string(),
                            position: field_name_token.position,
                        });
                    };
                    
                    self.expect(TokenType::Colon, "field type")?;
                    let field_type = self.parse_type()?;
                    
                    field_list.push(StructField {
                        visibility: Visibility::Public, // Enum fields are always public
                        name: field_name,
                        field_type,
                        span: Span::new(field_start, self.current_position()),
                    });
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
                
                self.expect(TokenType::RightBrace, "variant fields")?;
                StructFields::Named(field_list)
            } else if self.match_token(&TokenType::LeftParen) {
                // Tuple variant
                let mut field_types = Vec::new();
                
                while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                    field_types.push(self.parse_type()?);
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
                
                self.expect(TokenType::RightParen, "tuple variant fields")?;
                StructFields::Tuple(field_types)
            } else {
                // Unit variant
                StructFields::Unit
            };
            
            // Optional discriminant (for C-style enums)
            let discriminant = if self.match_token(&TokenType::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            variants.push(EnumVariant {
                name: variant_name,
                fields,
                discriminant,
                span: Span::new(variant_start, self.current_position()),
            });
            
            if !self.match_token(&TokenType::Comma) {
                break;
            }
        }
        
        self.expect(TokenType::RightBrace, "enum variants")?;
        
        let end_pos = self.current_position();
        Ok(Item::Enum {
            visibility,
            name,
            generics,
            variants,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a type alias declaration
    fn parse_type_alias(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Type, "type alias")?;
        
        let name_token = self.expect(TokenType::Identifier("".to_string()), "type alias name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected type alias name".to_string(),
                position: name_token.position,
            });
        };
        
        let generics = Vec::new(); // TODO: implement generics
        
        self.expect(TokenType::Equal, "type alias definition")?;
        let target_type = self.parse_type()?;
        self.expect(TokenType::Semicolon, "type alias declaration")?;
        
        let end_pos = self.current_position();
        Ok(Item::TypeAlias {
            visibility,
            name,
            generics,
            target_type,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a constant declaration
    fn parse_const(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Const, "constant declaration")?;
        
        let name_token = self.expect(TokenType::Identifier("".to_string()), "constant name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected constant name".to_string(),
                position: name_token.position,
            });
        };
        
        self.expect(TokenType::Colon, "constant type")?;
        let type_annotation = self.parse_type()?;
        
        self.expect(TokenType::Equal, "constant value")?;
        let value = self.parse_expression()?;
        
        self.expect(TokenType::Semicolon, "constant declaration")?;
        
        let end_pos = self.current_position();
        Ok(Item::Const {
            visibility,
            name,
            type_annotation,
            value,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a module declaration
    fn parse_module_decl(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Mod, "module declaration")?;
        
        let name_token = self.expect(TokenType::Identifier("".to_string()), "module name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected module name".to_string(),
                position: name_token.position,
            });
        };
        
        let items = if self.match_token(&TokenType::LeftBrace) {
            let mut module_items = Vec::new();
            
            while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                match self.parse_item() {
                    Ok(item) => module_items.push(item),
                    Err(err) => {
                        self.add_error(err);
                        self.synchronize();
                    }
                }
            }
            
            self.expect(TokenType::RightBrace, "module items")?;
            Some(module_items)
        } else {
            self.expect(TokenType::Semicolon, "module declaration")?;
            None
        };
        
        let end_pos = self.current_position();
        Ok(Item::Module {
            visibility,
            name,
            items,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse an impl block
    fn parse_impl_block(&mut self, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Impl, "impl block")?;
        
        let generics = Vec::new(); // TODO: implement generics
        let target_type = self.parse_type()?;
        
        // TODO: handle trait implementations (impl Trait for Type)
        let trait_ref = None;
        
        self.expect(TokenType::LeftBrace, "impl block items")?;
        let mut items = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            let item_start = self.current_position();
            
            // Parse visibility
            let visibility = if self.match_token(&TokenType::Pub) {
                Visibility::Public
            } else {
                Visibility::Private
            };
            
            if let Some(token) = &self.current_token {
                match &token.token_type {
                    TokenType::Fn => {
                        // Parse method
                        let func_item = self.parse_function(visibility, item_start)?;
                        if let Item::Function { name, generics, params, return_type, body, .. } = func_item {
                            items.push(ImplItem::Function {
                                visibility,
                                name,
                                generics,
                                params,
                                return_type,
                                body,
                                span: Span::new(item_start, self.current_position()),
                            });
                        }
                    }
                    TokenType::Type => {
                        // Parse associated type
                        let type_item = self.parse_type_alias(visibility, item_start)?;
                        if let Item::TypeAlias { name, generics, target_type, .. } = type_item {
                            items.push(ImplItem::Type {
                                visibility,
                                name,
                                generics,
                                target_type,
                                span: Span::new(item_start, self.current_position()),
                            });
                        }
                    }
                    TokenType::Const => {
                        // Parse associated constant
                        let const_item = self.parse_const(visibility, item_start)?;
                        if let Item::Const { name, type_annotation, value, .. } = const_item {
                            items.push(ImplItem::Const {
                                visibility,
                                name,
                                type_annotation,
                                value: Some(value),
                                span: Span::new(item_start, self.current_position()),
                            });
                        }
                    }
                    _ => {
                        return Err(ParseError::UnexpectedToken {
                            expected: vec!["impl item".to_string()],
                            found: token.token_type.clone(),
                            position: token.position,
                        });
                    }
                }
            } else {
                break;
            }
        }
        
        self.expect(TokenType::RightBrace, "impl block")?;
        
        let end_pos = self.current_position();
        Ok(Item::Impl {
            generics,
            target_type,
            trait_ref,
            items,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a use declaration
    fn parse_use_decl(&mut self, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Use, "use declaration")?;
        
        let mut path = Vec::new();
        
        // Parse the path
        let first_token = self.expect(TokenType::Identifier("".to_string()), "use path")?;
        if let TokenType::Identifier(name_str) = first_token.token_type {
            path.push(self.interner.intern(&name_str));
        }
        
        while self.match_token(&TokenType::DoubleColon) {
            let name_token = self.expect(TokenType::Identifier("".to_string()), "use path segment")?;
            if let TokenType::Identifier(name_str) = name_token.token_type {
                path.push(self.interner.intern(&name_str));
            }
        }
        
        // Parse optional alias
        let alias = if self.match_token(&TokenType::As) {
            let alias_token = self.expect(TokenType::Identifier("".to_string()), "use alias")?;
            if let TokenType::Identifier(name_str) = alias_token.token_type {
                Some(self.interner.intern(&name_str))
            } else {
                None
            }
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "use declaration")?;
        
        let end_pos = self.current_position();
        Ok(Item::Use {
            path,
            alias,
            span: Span::new(start_pos, end_pos),
        })
    }
} 