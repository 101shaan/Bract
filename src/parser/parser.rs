//! Main parser implementation for the Bract programming language

use crate::lexer::{Lexer, Token, TokenType, Position};
use crate::ast::{Module, Item, Expr, Stmt, Span, Visibility, Parameter, InternedString, Pattern, Type};
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
    pub(super) current_token: Option<Token>,
    pub(super) interner: StringInterner,
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
    pub fn expect(&mut self, expected: TokenType, _context: &str) -> ParseResult<Token> {
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
        
        // Generic parameters
        let generics = if self.match_token(&TokenType::Less) {
            let mut generic_params = Vec::new();
            
            if !self.check(&TokenType::Greater) {
                loop {
                    let param_start = self.current_position();
                    
                    // Parse generic parameter name
                    if let Some(token) = &self.current_token {
                        if let TokenType::Identifier(param_name) = &token.token_type {
                            let name = self.interner.intern(param_name);
                            self.advance()?;
                            
                            // TODO: Parse trait bounds and default values
                            generic_params.push(crate::ast::GenericParam {
                                name,
                                bounds: Vec::new(),
                                default: None,
                                span: Span::new(param_start, self.current_position()),
                            });
                        } else {
                            return Err(ParseError::InvalidSyntax {
                                message: "Expected generic parameter name".to_string(),
                                position: token.position,
                            });
                        }
                    }
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }
            
            self.expect(TokenType::Greater, "generic parameters")?;
            generic_params
        } else {
            Vec::new()
        };
        
        // Parameters
        self.expect(TokenType::LeftParen, "function parameters")?;
        let mut params = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                let param_start = self.current_position();
                
                // Check for special self parameters: self, &self, &mut self
                let (pattern, type_annotation) = if self.is_self_parameter() {
                    self.parse_self_parameter()?
                } else {
                    let pattern = self.parse_pattern()?;
                    let type_annotation = if self.match_token(&TokenType::Colon) {
                        Some(self.parse_type()?)
                    } else {
                        None
                    };
                    (pattern, type_annotation)
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
    
    // Implement struct parsing
    fn parse_struct(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Struct, "struct declaration")?;
        
        // Struct name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "struct name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected struct name".to_string(),
                position: name_token.position,
            });
        };
        
        // Generic parameters
        let generics = if self.match_token(&TokenType::Less) {
            let mut generic_params = Vec::new();
            
            if !self.check(&TokenType::Greater) {
                loop {
                    let param_start = self.current_position();
                    
                    // Parse generic parameter name
                    if let Some(token) = &self.current_token {
                        if let TokenType::Identifier(param_name) = &token.token_type {
                            let name = self.interner.intern(param_name);
                            self.advance()?;
                            
                            // TODO: Parse trait bounds and default values
                            generic_params.push(crate::ast::GenericParam {
                                name,
                                bounds: Vec::new(),
                                default: None,
                                span: Span::new(param_start, self.current_position()),
                            });
                        } else {
                            return Err(ParseError::InvalidSyntax {
                                message: "Expected generic parameter name".to_string(),
                                position: token.position,
                            });
                        }
                    }
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }
            
            self.expect(TokenType::Greater, "generic parameters")?;
            generic_params
        } else {
            Vec::new()
        };
        
        // Parse struct fields
        let fields = if self.match_token(&TokenType::LeftBrace) {
            // Named fields: struct Point { x: i32, y: i32 }
            let mut field_list = Vec::new();
            
            while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                let field_start = self.current_position();
                
                // Field visibility (default private)
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
                
                self.expect(TokenType::Colon, "field type annotation")?;
                let field_type = self.parse_type()?;
                
                field_list.push(crate::ast::StructField {
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
            crate::ast::StructFields::Named(field_list)
        } else if self.match_token(&TokenType::LeftParen) {
            // Tuple struct: struct Point(i32, i32);
            let mut types = Vec::new();
            
            while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                types.push(self.parse_type()?);
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
            
            self.expect(TokenType::RightParen, "tuple struct fields")?;
            self.expect(TokenType::Semicolon, "tuple struct declaration")?;
            crate::ast::StructFields::Tuple(types)
        } else {
            // Unit struct: struct Unit;
            self.expect(TokenType::Semicolon, "unit struct declaration")?;
            crate::ast::StructFields::Unit
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
    
    // Implement enum parsing
    fn parse_enum(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Enum, "enum declaration")?;
        
        // Enum name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "enum name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected enum name".to_string(),
                position: name_token.position,
            });
        };
        
        // Generic parameters
        let generics = if self.match_token(&TokenType::Less) {
            let mut generic_params = Vec::new();
            
            if !self.check(&TokenType::Greater) {
                loop {
                    let param_start = self.current_position();
                    
                    // Parse generic parameter name
                    if let Some(token) = &self.current_token {
                        if let TokenType::Identifier(param_name) = &token.token_type {
                            let name = self.interner.intern(param_name);
                            self.advance()?;
                            
                            // TODO: Parse trait bounds and default values
                            generic_params.push(crate::ast::GenericParam {
                                name,
                                bounds: Vec::new(),
                                default: None,
                                span: Span::new(param_start, self.current_position()),
                            });
                        } else {
                            return Err(ParseError::InvalidSyntax {
                                message: "Expected generic parameter name".to_string(),
                                position: token.position,
                            });
                        }
                    }
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }
            
            self.expect(TokenType::Greater, "generic parameters")?;
            generic_params
        } else {
            Vec::new()
        };
        
        // Parse enum variants
        self.expect(TokenType::LeftBrace, "enum variants")?;
        let mut variants = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            let variant_start = self.current_position();
            
            // Variant name
            let variant_name_token = self.expect(TokenType::Identifier("".to_string()), "variant name")?;
            let variant_name = if let TokenType::Identifier(name_str) = variant_name_token.token_type {
                self.interner.intern(&name_str)
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected variant name".to_string(),
                    position: variant_name_token.position,
                });
            };
            
            // Parse variant fields
            let fields = if self.match_token(&TokenType::LeftBrace) {
                // Named fields: Some { value: T }
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
                    
                    field_list.push(crate::ast::StructField {
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
                crate::ast::StructFields::Named(field_list)
            } else if self.match_token(&TokenType::LeftParen) {
                // Tuple fields: Some(T)
                let mut types = Vec::new();
                
                while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                    types.push(self.parse_type()?);
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
                
                self.expect(TokenType::RightParen, "variant fields")?;
                crate::ast::StructFields::Tuple(types)
            } else {
                // Unit variant: None
                crate::ast::StructFields::Unit
            };
            
            // Optional discriminant value
            let discriminant = if self.match_token(&TokenType::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            variants.push(crate::ast::EnumVariant {
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

    fn parse_type_alias(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Type, "type alias")?;
        
        // Type alias name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "type name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected type name".to_string(),
                position: name_token.position,
            });
        };
        
        // Generic parameters (placeholder for now)
        let generics = Vec::new();
        
        self.expect(TokenType::Equal, "type alias")?;
        let target_type = self.parse_type()?;
        self.expect(TokenType::Semicolon, "type alias")?;
        
        let end_pos = self.current_position();
        Ok(Item::TypeAlias {
            visibility,
            name,
            generics,
            target_type,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    fn parse_const(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Const, "const declaration")?;
        
        // Const name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "const name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected const name".to_string(),
                position: name_token.position,
            });
        };
        
        self.expect(TokenType::Colon, "const type")?;
        let type_annotation = self.parse_type()?;
        self.expect(TokenType::Equal, "const value")?;
        let value = self.parse_expression()?;
        self.expect(TokenType::Semicolon, "const declaration")?;
        
        let end_pos = self.current_position();
        Ok(Item::Const {
            visibility,
            name,
            type_annotation,
            value,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    fn parse_module_decl(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Mod, "module declaration")?;
        
        // Module name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "module name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected module name".to_string(),
                position: name_token.position,
            });
        };
        
        // Parse module body
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
            
            self.expect(TokenType::RightBrace, "module body")?;
            Some(module_items)
        } else {
            // External module: mod foo;
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
    
    fn parse_impl_block(&mut self, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Impl, "impl block")?;
        
        // Generic parameters (placeholder for now)
        let generics = Vec::new();
        
        // Target type
        let target_type = self.parse_type()?;
        
        // Optional trait implementation
        let trait_ref = if self.match_token(&TokenType::For) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        self.expect(TokenType::LeftBrace, "impl block")?;
        let mut items = Vec::new();
        
        // Parse impl items (functions, types, consts)
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            let item_start = self.current_position();
            let item_visibility = if self.match_token(&TokenType::Pub) {
                Visibility::Public
            } else {
                Visibility::Private
            };
            
            if self.check(&TokenType::Fn) {
                // Parse method
                if let Ok(Item::Function { name, generics, params, return_type, body, is_extern: _, .. }) = self.parse_function(item_visibility, item_start) {
                    items.push(crate::ast::ImplItem::Function {
                        visibility: item_visibility,
                        name,
                        generics,
                        params,
                        return_type,
                        body,
                        span: Span::new(item_start, self.current_position()),
                    });
                }
            } else {
                // Skip unknown items for now
                self.synchronize();
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
    
    fn parse_use_decl(&mut self, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Use, "use declaration")?;
        
        // Parse use path
        let mut path = Vec::new();
        
        loop {
            let name_token = self.expect(TokenType::Identifier("".to_string()), "use path")?;
            if let TokenType::Identifier(name_str) = name_token.token_type {
                path.push(self.interner.intern(&name_str));
            }
            
            if !self.match_token(&TokenType::DoubleColon) {
                break;
            }
        }
        
        // TODO: Add alias support when 'as' keyword is implemented
        let alias = None;
        
        self.expect(TokenType::Semicolon, "use declaration")?;
        
        let end_pos = self.current_position();
        Ok(Item::Use {
            path,
            alias,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a block expression: { [statements...] [expr] }
    pub fn parse_block_expression(&mut self) -> ParseResult<Expr> {
        let start_pos = self.current_position();
        self.expect(TokenType::LeftBrace, "block expression")?;
        
        let mut statements = Vec::new();
        let mut trailing_expr = None;
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Try to parse as statement first
            if self.is_statement_start() {
                match self.parse_statement() {
                    Ok(stmt) => statements.push(stmt),
                    Err(err) => {
                        self.add_error(err);
                        self.synchronize();
                    }
                }
            } else {
                // Try to parse as expression
                let expr_start = self.current_position();
                let expr = self.parse_expression()?;
                
                // If there's no semicolon and we're at the end of the block,
                // this is the trailing expression
                if !self.check(&TokenType::Semicolon) && self.check(&TokenType::RightBrace) {
                    trailing_expr = Some(expr);
                    break;
                } else {
                    // It's an expression statement
                    self.expect(TokenType::Semicolon, "expression statement")?;
                    let end_pos = self.current_position();
                    statements.push(Stmt::Expression {
                        expr,
                        span: Span::new(expr_start, end_pos),
                    });
                }
            }
        }
        
        self.expect(TokenType::RightBrace, "block expression")?;
        let end_pos = self.current_position();
        
        Ok(Expr::Block {
            statements,
            trailing_expr: trailing_expr.map(Box::new),
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Check if current token sequence represents a self parameter
    fn is_self_parameter(&self) -> bool {
        match &self.current_token {
            Some(token) => match &token.token_type {
                // Direct self parameter
                TokenType::Identifier(name) if name == "self" => true,
                // &self or &mut self
                TokenType::And => {
                    // We need to look ahead to see if this is &self or &mut self
                    // This is a simplified check - in a full implementation we'd need proper lookahead
                    true
                }
                _ => false,
            },
            None => false,
        }
    }
    
    /// Parse a self parameter and return the pattern and inferred type
    fn parse_self_parameter(&mut self) -> ParseResult<(Pattern, Option<Type>)> {
        let start_pos = self.current_position();
        
        if let Some(token) = &self.current_token {
            match &token.token_type {
                // Direct self parameter: self
                TokenType::Identifier(name) if name == "self" => {
                    self.advance()?;
                    let pattern = Pattern::Identifier {
                        name: self.interner.intern("self"),
                        is_mutable: false,
                        span: Span::new(start_pos, self.current_position()),
                    };
                    // Type will be inferred as the struct type during semantic analysis
                    let self_type = Type::Path {
                        segments: vec![self.interner.intern("Self")],
                        generics: Vec::new(),
                        span: Span::new(start_pos, self.current_position()),
                    };
                    Ok((pattern, Some(self_type)))
                }
                // Reference self parameter: &self or &mut self
                TokenType::And => {
                    self.advance()?; // consume &
                    let is_mutable = self.match_token(&TokenType::Mut);
                    
                    // Expect 'self' identifier
                    if let Some(token) = &self.current_token {
                        if let TokenType::Identifier(name) = &token.token_type {
                            if name == "self" {
                                self.advance()?;
                                let pattern = Pattern::Identifier {
                                    name: self.interner.intern("self"),
                                    is_mutable: false, // The reference itself is not mutable
                                    span: Span::new(start_pos, self.current_position()),
                                };
                                // Create reference type to Self
                                let self_type = Type::Reference {
                                    is_mutable,
                                    target_type: Box::new(Type::Path {
                                        segments: vec![self.interner.intern("Self")],
                                        generics: Vec::new(),
                                        span: Span::new(start_pos, self.current_position()),
                                    }),
                                    span: Span::new(start_pos, self.current_position()),
                                };
                                Ok((pattern, Some(self_type)))
                            } else {
                                Err(ParseError::UnexpectedToken {
                                    expected: vec!["self".to_string()],
                                    found: token.token_type.clone(),
                                    position: token.position,
                                })
                            }
                        } else {
                            Err(ParseError::UnexpectedToken {
                                expected: vec!["self".to_string()],
                                found: token.token_type.clone(),
                                position: token.position,
                            })
                        }
                    } else {
                        Err(ParseError::UnexpectedEof {
                            expected: vec!["self".to_string()],
                            position: self.current_position(),
                        })
                    }
                }
                _ => Err(ParseError::UnexpectedToken {
                    expected: vec!["self parameter".to_string()],
                    found: token.token_type.clone(),
                    position: token.position,
                }),
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["self parameter".to_string()],
                position: self.current_position(),
            })
        }
    }
} 
