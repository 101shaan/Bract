//! Type parsing for Bract programming language
//!
//! This module handles parsing of all type expressions according to EBNF grammar:
//! - Primitive types (i32, f64, bool, char, str)
//! - Path types (user-defined types, generics)
//! - Array types [T; N]
//! - Slice types &[T]
//! - Tuple types (T1, T2, T3)
//! - Function types fn(T1, T2) -> T3
//! - Reference types &T, &mut T
//! - Pointer types *const T, *mut T
//! - Generic types T
//! - Inferred types _

use crate::lexer::{TokenType, Position};
use crate::ast::{Type, PrimitiveType, Span, MemoryStrategy, Ownership};
use super::parser::Parser;
use super::error::{ParseError, ParseResult, ParseContext, ExpectedToken, Suggestion, SuggestionCategory};

impl<'a> Parser<'a> {
    /// Parse a type according to EBNF grammar
    /// Type ::= FunctionType | SliceType | ArrayType | PointerType | ReferenceType | TupleType | PathType
    pub fn parse_type(&mut self) -> ParseResult<Type> {
        if let Some(token) = &self.current_token {
            let start_pos = self.current_position();
            
            match &token.token_type {
                // Function types: fn(params) -> return_type
                TokenType::Fn => {
                    self.parse_function_type(start_pos)
                }
                
                // Reference types: &T, &mut T
                TokenType::And => {
                    self.parse_reference_type(start_pos)
                }
                
                // Pointer types: *const T, *mut T
                TokenType::Star => {
                    self.parse_pointer_type(start_pos)
                }
                
                // Tuple types: (T1, T2, T3)
                TokenType::LeftParen => {
                    self.parse_tuple_type(start_pos)
                }
                
                // Array/slice types: [T; N] or &[T]
                TokenType::LeftBracket => {
                    self.parse_array_or_slice_type(start_pos)
                }
                
                // Inferred type: _
                TokenType::Identifier(name) if name == "_" => {
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Type::Inferred {
                        constraints: Vec::new(),
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                // Never type: !
                TokenType::Not => {
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Type::Never {
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                // Path types or primitive types
                TokenType::Identifier(name) => {
                    let name_clone = name.clone();
                    self.parse_path_or_primitive_type(&name_clone, start_pos)
                }
                
                _ => {
                    Err(ParseError::UnexpectedToken {
                        expected: vec![ExpectedToken::new("type", "primitive, reference, pointer, tuple, array, or path type")],
                        found: token.token_type.clone(),
                        position: start_pos,
                        context: ParseContext::TypeAnnotation,
                        suggestions: vec![
                            Suggestion::new("Use a valid type", start_pos)
                                .with_category(SuggestionCategory::Type)
                        ],
                        help: Some("Types can be primitives (i32, bool), references (&T), pointers (*T), tuples ((T, U)), arrays ([T; N]), or paths (MyStruct)".to_string()),
                    })
                }
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec![ExpectedToken::new("type", "primitive, reference, pointer, tuple, array, or path type")],
                position: self.current_position(),
                context: ParseContext::TypeAnnotation,
                unclosed_delimiters: Vec::new(),
                suggestions: vec![
                    Suggestion::new("Add a type annotation", self.current_position())
                        .with_category(SuggestionCategory::Type)
                ],
            })
        }
    }
    
    /// Parse function type: fn(param_types) -> return_type
    fn parse_function_type(&mut self, start_pos: Position) -> ParseResult<Type> {
        self.expect(TokenType::Fn, "function type")?;
        self.expect(TokenType::LeftParen, "function parameters")?;
        
        let mut params = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            params.push(self.parse_type()?);
            
            while self.match_token(&TokenType::Comma) {
                if self.check(&TokenType::RightParen) {
                    break; // trailing comma
                }
                params.push(self.parse_type()?);
            }
        }
        
        self.expect(TokenType::RightParen, "function parameters")?;
        
        let return_type = if self.match_token(&TokenType::Arrow) {
            Box::new(self.parse_type()?)
        } else {
            // Default to unit type
            Box::new(Type::Tuple {
                types: Vec::new(),
                memory_strategy: MemoryStrategy::Inferred,
                span: Span::new(start_pos, start_pos),
            })
        };
        
        let end_pos = self.current_position();
        Ok(Type::Function {
            params,
            return_type,
            is_variadic: false, // TODO: Add variadic support
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse reference type: &T or &mut T
    fn parse_reference_type(&mut self, start_pos: Position) -> ParseResult<Type> {
        self.expect(TokenType::And, "reference type")?;
        let is_mutable = self.match_token(&TokenType::Mut);
        let target_type = Box::new(self.parse_type()?);
        let end_pos = self.current_position();
        
        Ok(Type::Reference {
            is_mutable,
            target_type,
            lifetime: None,
            ownership: Ownership::Borrowed,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse pointer type: *const T or *mut T
    fn parse_pointer_type(&mut self, start_pos: Position) -> ParseResult<Type> {
        self.expect(TokenType::Star, "pointer type")?;
        
        let is_mutable = if self.match_token(&TokenType::Mut) {
            true
        } else if self.match_token(&TokenType::Const) {
            false
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: vec![ExpectedToken::new("const or mut", "pointer mutability specifier")],
                found: self.current_token.as_ref().unwrap().token_type.clone(),
                position: self.current_position(),
                context: ParseContext::TypeAnnotation,
                suggestions: vec![
                    Suggestion::new("Use 'const' for immutable pointer or 'mut' for mutable", self.current_position())
                        .with_category(SuggestionCategory::Type)
                ],
                help: Some("Pointer types require explicit mutability: *const T or *mut T".to_string()),
            });
        };
        
        let target_type = Box::new(self.parse_type()?);
        let end_pos = self.current_position();
        
        Ok(Type::Pointer {
            is_mutable,
            target_type,
            memory_strategy: MemoryStrategy::Manual,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse tuple type: (T1, T2, T3) or ()
    fn parse_tuple_type(&mut self, start_pos: Position) -> ParseResult<Type> {
        self.expect(TokenType::LeftParen, "tuple type")?;
        
        let mut types = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            types.push(self.parse_type()?);
            
            while self.match_token(&TokenType::Comma) {
                if self.check(&TokenType::RightParen) {
                    break; // trailing comma
                }
                types.push(self.parse_type()?);
            }
        }
        
        self.expect(TokenType::RightParen, "tuple type")?;
        let end_pos = self.current_position();
        
        Ok(Type::Tuple {
            types,
            memory_strategy: MemoryStrategy::Inferred,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse array type [T; N] or determine if it's a slice
    fn parse_array_or_slice_type(&mut self, start_pos: Position) -> ParseResult<Type> {
        self.expect(TokenType::LeftBracket, "array or slice type")?;
        
        let element_type = Box::new(self.parse_type()?);
        
        if self.match_token(&TokenType::Semicolon) {
            // Array type: [T; N]
            let size = Box::new(self.parse_expression()?);
            self.expect(TokenType::RightBracket, "array type")?;
            let end_pos = self.current_position();
            
            Ok(Type::Array {
                element_type,
                size,
                memory_strategy: MemoryStrategy::Inferred,
                span: Span::new(start_pos, end_pos),
            })
        } else {
            // This would be a slice type &[T], but we need the & to be parsed first
            // So this is an error - slices must be written as &[T]
            Err(ParseError::UnexpectedToken {
                expected: vec![ExpectedToken::new("semicolon for array size", "array size separator ';'")],
                found: self.current_token.as_ref().unwrap().token_type.clone(),
                position: self.current_position(),
                context: ParseContext::TypeAnnotation,
                suggestions: vec![
                    Suggestion::new("Use semicolon to specify array size: [T; size]", self.current_position())
                        .with_replacement("; size")
                        .with_category(SuggestionCategory::Type)
                ],
                help: Some("Arrays require explicit size: [T; N]. Use &[T] for slices".to_string()),
            })
        }
    }
    
    /// Parse path type or primitive type
    fn parse_path_or_primitive_type(&mut self, name: &str, start_pos: Position) -> ParseResult<Type> {
        // Check if it's a primitive type
        let primitive = match name {
            "i8" => Some(PrimitiveType::I8),
            "i16" => Some(PrimitiveType::I16),
            "i32" => Some(PrimitiveType::I32),
            "i64" => Some(PrimitiveType::I64),
            "i128" => Some(PrimitiveType::I128),
            "isize" => Some(PrimitiveType::ISize),
            "u8" => Some(PrimitiveType::U8),
            "u16" => Some(PrimitiveType::U16),
            "u32" => Some(PrimitiveType::U32),
            "u64" => Some(PrimitiveType::U64),
            "u128" => Some(PrimitiveType::U128),
            "usize" => Some(PrimitiveType::USize),
            "f32" => Some(PrimitiveType::F32),
            "f64" => Some(PrimitiveType::F64),
            "bool" => Some(PrimitiveType::Bool),
            "char" => Some(PrimitiveType::Char),
            "str" => Some(PrimitiveType::Str),
            _ => None,
        };
        
        if let Some(prim) = primitive {
            self.advance()?;
            let end_pos = self.current_position();
            Ok(Type::Primitive {
                kind: prim,
                memory_strategy: MemoryStrategy::Inferred,
                span: Span::new(start_pos, end_pos),
            })
        } else {
            // Path type
            self.parse_path_type(start_pos)
        }
    }
    
    /// Parse path type: Identifier::Identifier<Generics>
    fn parse_path_type(&mut self, start_pos: Position) -> ParseResult<Type> {
        let mut segments = Vec::new();
        
        // Parse first segment
        if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                segments.push(self.interner.intern(name));
                self.advance()?;
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: vec![ExpectedToken::new("identifier", "type name or path segment")],
                    found: token.token_type.clone(),
                    position: self.current_position(),
                    context: ParseContext::TypeAnnotation,
                    suggestions: vec![
                        Suggestion::new("Use a valid identifier for the type", self.current_position())
                            .with_category(SuggestionCategory::Type)
                    ],
                    help: Some("Type names must be valid identifiers, optionally with module paths".to_string()),
                })
            }
        } else {
            return Err(ParseError::UnexpectedEof {
                expected: vec![ExpectedToken::new("identifier", "type name")],
                position: self.current_position(),
                context: ParseContext::TypeAnnotation,
                unclosed_delimiters: Vec::new(),
                suggestions: vec![
                    Suggestion::new("Add a type name", self.current_position())
                        .with_category(SuggestionCategory::Type)
                ],
            })
        }
        
        // Parse additional path segments
        while self.match_token(&TokenType::DoubleColon) {
            if let Some(token) = &self.current_token {
                if let TokenType::Identifier(name) = &token.token_type {
                    segments.push(self.interner.intern(name));
                    self.advance()?;
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: vec![ExpectedToken::new("identifier", "type name or path segment")],
                        found: token.token_type.clone(),
                        position: self.current_position(),
                        context: ParseContext::TypeAnnotation,
                        suggestions: vec![
                            Suggestion::new("Use a valid identifier for the type", self.current_position())
                                .with_category(SuggestionCategory::Type)
                        ],
                        help: Some("Type names must be valid identifiers, optionally with module paths".to_string()),
                    })
                }
            } else {
                return Err(ParseError::UnexpectedEof {
                    expected: vec![ExpectedToken::new("identifier", "type name")],
                    position: self.current_position(),
                    context: ParseContext::TypeAnnotation,
                    unclosed_delimiters: Vec::new(),
                    suggestions: vec![
                        Suggestion::new("Add a type name", self.current_position())
                            .with_category(SuggestionCategory::Type)
                    ],
                })
            }
        }
        
        // Parse generic arguments if present
        let generics = if self.match_token(&TokenType::Less) {
            let mut generic_args = Vec::new();
            
            if !self.check(&TokenType::Greater) {
                generic_args.push(self.parse_type()?);
                
                while self.match_token(&TokenType::Comma) {
                    if self.check(&TokenType::Greater) {
                        break; // trailing comma
                    }
                    generic_args.push(self.parse_type()?);
                }
            }
            
            self.expect(TokenType::Greater, "generic arguments")?;
            generic_args
        } else {
            Vec::new()
        };
        
        let end_pos = self.current_position();
        Ok(Type::Path {
            segments,
            generics,
            memory_strategy: MemoryStrategy::Inferred,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Check if the current token can start a type
    pub fn is_type_start(&self) -> bool {
        if let Some(token) = &self.current_token {
            matches!(token.token_type,
                TokenType::Fn | TokenType::And | TokenType::Star |
                TokenType::LeftParen | TokenType::LeftBracket |
                TokenType::Not | TokenType::Identifier(_)
            )
        } else {
            false
        }
    }
} 
