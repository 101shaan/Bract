//! Pattern parsing for Prism programming language
//!
//! This module handles parsing of all pattern types according to EBNF grammar:
//! - Wildcard patterns (_)
//! - Literal patterns (42, "hello", true)
//! - Identifier patterns (variable bindings)
//! - Reference patterns (&pattern, &mut pattern)
//! - Struct patterns (Point { x, y })
//! - Enum patterns (Option::Some(value))
//! - Tuple patterns ((a, b, c))
//! - Array patterns ([a, b, c])
//! - Or patterns (a | b | c)
//! - Range patterns (1..10)

use crate::lexer::TokenType;
use crate::ast::*;
use super::parser::Parser;
use super::error::{ParseError, ParseResult};

impl<'a> Parser<'a> {
    /// Parse a pattern according to EBNF grammar
    /// Pattern ::= OrPattern
    pub fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        self.parse_or_pattern()
    }
    
    /// Parse an or-pattern: pattern | pattern | ...
    /// OrPattern ::= RangePattern { "|" RangePattern }
    fn parse_or_pattern(&mut self) -> ParseResult<Pattern> {
        let start_pos = self.current_position();
        let mut patterns = vec![self.parse_range_pattern()?];
        
        while self.match_token(&TokenType::Or) {
            patterns.push(self.parse_range_pattern()?);
        }
        
        if patterns.len() == 1 {
            Ok(patterns.into_iter().next().unwrap())
        } else {
            let end_pos = self.current_position();
            Ok(Pattern::Or {
                patterns,
                span: Span::new(start_pos, end_pos),
            })
        }
    }
    
    /// Parse a range pattern: 1..10, 'a'..'z', etc.
    /// RangePattern ::= PrimaryPattern [ ".." PrimaryPattern ]
    fn parse_range_pattern(&mut self) -> ParseResult<Pattern> {
        let start_pos = self.current_position();
        let start_pattern = self.parse_primary_pattern()?;
        
        if self.match_token(&TokenType::DotDot) {
            let end_pattern = self.parse_primary_pattern()?;
            let span_end = self.current_position();
            
            Ok(Pattern::Range {
                start: Box::new(start_pattern),
                end: Box::new(end_pattern),
                span: Span::new(start_pos, span_end),
            })
        } else {
            Ok(start_pattern)
        }
    }
    
    /// Parse primary pattern types
    /// PrimaryPattern ::= "_" | literal | identifier | "&" ["mut"] Pattern
    ///                  | StructPattern | EnumPattern | TuplePattern | ArrayPattern
    fn parse_primary_pattern(&mut self) -> ParseResult<Pattern> {
        if let Some(token) = &self.current_token {
            let start_pos = self.current_position();
            
            match &token.token_type {
                // Wildcard pattern: _
                TokenType::Identifier(name) if name == "_" => {
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Pattern::Wildcard {
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                // Identifier pattern: variable binding
                TokenType::Identifier(name) => {
                    let identifier = self.interner.intern(name);
                    self.advance()?;
                    
                    // Check if this is a struct pattern or enum pattern
                    if self.check(&TokenType::LeftBrace) {
                        // Struct pattern: Identifier { fields }
                        self.parse_struct_pattern_body(identifier, start_pos)
                    } else if self.check(&TokenType::DoubleColon) {
                        // Path pattern: Module::Identifier or enum variant
                        self.parse_path_pattern(identifier, start_pos)
                    } else {
                        // Simple identifier pattern
                        let end_pos = self.current_position();
                        Ok(Pattern::Identifier {
                            name: identifier,
                            span: Span::new(start_pos, end_pos),
                        })
                    }
                }
                
                // Literal patterns
                TokenType::Integer { value, .. } => {
                    let literal = Literal::Integer {
                        value: value.clone(),
                        suffix: None,
                        span: Span::new(start_pos, start_pos),
                    };
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Pattern::Literal {
                        literal,
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                TokenType::Float { value, .. } => {
                    let literal = Literal::Float {
                        value: value.clone(),
                        suffix: None,
                        span: Span::new(start_pos, start_pos),
                    };
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Pattern::Literal {
                        literal,
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                TokenType::String { value, .. } => {
                    let literal = Literal::String {
                        value: value.clone(),
                        span: Span::new(start_pos, start_pos),
                    };
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Pattern::Literal {
                        literal,
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                TokenType::Char(ch) => {
                    let literal = Literal::Char {
                        value: *ch,
                        span: Span::new(start_pos, start_pos),
                    };
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Pattern::Literal {
                        literal,
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                TokenType::True => {
                    let literal = Literal::Bool {
                        value: true,
                        span: Span::new(start_pos, start_pos),
                    };
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Pattern::Literal {
                        literal,
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                TokenType::False => {
                    let literal = Literal::Bool {
                        value: false,
                        span: Span::new(start_pos, start_pos),
                    };
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Pattern::Literal {
                        literal,
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                TokenType::Null => {
                    let literal = Literal::Null {
                        span: Span::new(start_pos, start_pos),
                    };
                    self.advance()?;
                    let end_pos = self.current_position();
                    Ok(Pattern::Literal {
                        literal,
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                // Reference patterns: &pattern, &mut pattern
                TokenType::And => {
                    self.advance()?;
                    let is_mutable = self.match_token(&TokenType::Mut);
                    let pattern = Box::new(self.parse_pattern()?);
                    let end_pos = self.current_position();
                    
                    Ok(Pattern::Reference {
                        pattern,
                        is_mutable,
                        span: Span::new(start_pos, end_pos),
                    })
                }
                
                // Tuple patterns: (pattern, pattern, ...)
                TokenType::LeftParen => {
                    self.parse_tuple_pattern(start_pos)
                }
                
                // Array patterns: [pattern, pattern, ...]
                TokenType::LeftBracket => {
                    self.parse_array_pattern(start_pos)
                }
                
                _ => {
                    Err(ParseError::UnexpectedToken {
                        expected: vec!["pattern".to_string()],
                        found: token.token_type.clone(),
                        position: start_pos,
                    })
                }
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["pattern".to_string()],
                position: self.current_position(),
            })
        }
    }
    
    /// Parse a path pattern (enum variants, module paths)
    /// PathPattern ::= identifier { "::" identifier } [ "(" PatternList? ")" ]
    fn parse_path_pattern(&mut self, first_segment: InternedString, start_pos: Position) -> ParseResult<Pattern> {
        let mut segments = vec![first_segment];
        
        // Parse additional path segments
        while self.match_token(&TokenType::DoubleColon) {
            if let Some(token) = &self.current_token {
                if let TokenType::Identifier(name) = &token.token_type {
                    segments.push(self.interner.intern(name));
                    self.advance()?;
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: vec!["identifier".to_string()],
                        found: token.token_type.clone(),
                        position: self.current_position(),
                    });
                }
            } else {
                return Err(ParseError::UnexpectedEof {
                    expected: vec!["identifier".to_string()],
                    position: self.current_position(),
                });
            }
        }
        
        // Check for enum variant with tuple data
        if self.check(&TokenType::LeftParen) {
            self.advance()?; // consume '('
            
            let mut patterns = Vec::new();
            
            if !self.check(&TokenType::RightParen) {
                patterns.push(self.parse_pattern()?);
                
                while self.match_token(&TokenType::Comma) {
                    if self.check(&TokenType::RightParen) {
                        break; // trailing comma
                    }
                    patterns.push(self.parse_pattern()?);
                }
            }
            
            self.expect(TokenType::RightParen, "enum pattern")?;
            let end_pos = self.current_position();
            
            Ok(Pattern::Enum {
                path: segments,
                patterns: Some(patterns),
                span: Span::new(start_pos, end_pos),
            })
        } else {
            // Simple path or unit enum variant
            let end_pos = self.current_position();
            Ok(Pattern::Enum {
                path: segments,
                patterns: None,
                span: Span::new(start_pos, end_pos),
            })
        }
    }
    
    /// Parse struct pattern body: { field: pattern, field, .. }
    fn parse_struct_pattern_body(&mut self, struct_name: InternedString, start_pos: Position) -> ParseResult<Pattern> {
        self.expect(TokenType::LeftBrace, "struct pattern")?;
        
        let mut fields = Vec::new();
        let mut has_rest = false;
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Check for rest pattern: ..
            if self.match_token(&TokenType::DotDot) {
                has_rest = true;
                break;
            }
            
            // Parse field pattern
            if let Some(token) = &self.current_token {
                if let TokenType::Identifier(field_name) = &token.token_type {
                    let field_start = self.current_position();
                    let field_name_interned = self.interner.intern(field_name);
                    self.advance()?;
                    
                    let pattern = if self.match_token(&TokenType::Colon) {
                        // field: pattern
                        self.parse_pattern()?
                    } else {
                        // field (shorthand for field: field)
                        let field_end = self.current_position();
                        Pattern::Identifier {
                            name: field_name_interned,
                            span: Span::new(field_start, field_end),
                        }
                    };
                    
                    fields.push(StructPatternField {
                        name: field_name_interned,
                        pattern,
                        span: Span::new(field_start, self.current_position()),
                    });
                    
                    // Optional comma
                    if !self.check(&TokenType::RightBrace) {
                        if !self.match_token(&TokenType::Comma) {
                            break;
                        }
                    }
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: vec!["field name or ..".to_string()],
                        found: token.token_type.clone(),
                        position: self.current_position(),
                    });
                }
            } else {
                return Err(ParseError::UnexpectedEof {
                    expected: vec!["field name or ..".to_string()],
                    position: self.current_position(),
                });
            }
        }
        
        self.expect(TokenType::RightBrace, "struct pattern")?;
        let end_pos = self.current_position();
        
        Ok(Pattern::Struct {
            path: vec![struct_name],
            fields,
            has_rest,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse tuple pattern: (pattern, pattern, ...)
    fn parse_tuple_pattern(&mut self, start_pos: Position) -> ParseResult<Pattern> {
        self.expect(TokenType::LeftParen, "tuple pattern")?;
        
        let mut patterns = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            patterns.push(self.parse_pattern()?);
            
            while self.match_token(&TokenType::Comma) {
                if self.check(&TokenType::RightParen) {
                    break; // trailing comma
                }
                patterns.push(self.parse_pattern()?);
            }
        }
        
        self.expect(TokenType::RightParen, "tuple pattern")?;
        let end_pos = self.current_position();
        
        Ok(Pattern::Tuple {
            patterns,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse array pattern: [pattern, pattern, ...]
    fn parse_array_pattern(&mut self, start_pos: Position) -> ParseResult<Pattern> {
        self.expect(TokenType::LeftBracket, "array pattern")?;
        
        let mut patterns = Vec::new();
        
        if !self.check(&TokenType::RightBracket) {
            patterns.push(self.parse_pattern()?);
            
            while self.match_token(&TokenType::Comma) {
                if self.check(&TokenType::RightBracket) {
                    break; // trailing comma
                }
                patterns.push(self.parse_pattern()?);
            }
        }
        
        self.expect(TokenType::RightBracket, "array pattern")?;
        let end_pos = self.current_position();
        
        Ok(Pattern::Array {
            patterns,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Check if the current token can start a pattern
    pub fn is_pattern_start(&self) -> bool {
        if let Some(token) = &self.current_token {
            matches!(token.token_type,
                TokenType::Identifier(_) |
                TokenType::Integer { .. } | TokenType::Float { .. } |
                TokenType::String { .. } | TokenType::Char(_) |
                TokenType::True | TokenType::False | TokenType::Null |
                TokenType::And | TokenType::LeftParen | TokenType::LeftBracket
            )
        } else {
            false
        }
    }
} 