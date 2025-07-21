//! Memory Strategy and Performance Annotation Parser
//!
//! This module handles parsing of Bract's memory management annotations:
//! - Memory strategy annotations: @memory(strategy = "stack")
//! - Performance contracts: @performance(max_cost = 1000)
//! - Region blocks: region "name" { ... }
//! - Strategy wrapper types: LinearPtr<T>, SmartPtr<T>
//! - Express performance contracts with memory constraints

use crate::ast::{Type, Expr, Span, InternedString, MemoryStrategy, TypeBound};
use crate::lexer::{TokenType};
use super::parser::Parser;
use super::error::{ParseError, ParseResult, ParseContext, ExpectedToken, Suggestion, SuggestionCategory};

/// Memory strategy annotation syntax parser
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAnnotation {
    pub strategy: Option<MemoryStrategy>,
    pub size_hint: Option<u64>,
    pub alignment: Option<u8>,
    pub region: Option<InternedString>,
    pub span: Span,
}

/// Performance contract annotation
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceAnnotation {
    pub max_cost: Option<u64>,
    pub max_memory: Option<u64>,
    pub max_latency_ms: Option<u32>,
    pub span: Span,
}

/// Region block syntax: region "name" { ... }
#[derive(Debug, Clone, PartialEq)]
pub struct RegionBlock {
    pub name: InternedString,
    pub body: Vec<crate::ast::Stmt>,
    pub span: Span,
}

/// Variable declaration with memory strategy
#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclaration {
    pub name: InternedString,
    pub var_type: Type,
    pub strategy: MemoryStrategy,
    pub initializer: Option<Expr>,
    pub span: Span,
}

impl<'a> Parser<'a> {
    /// Parse @memory annotation
    pub fn parse_memory_annotation(&mut self) -> ParseResult<MemoryAnnotation> {
        let start_pos = self.current_position();
        
        // Expect @memory
        self.expect(TokenType::At, "memory annotation")?;
        if !self.match_identifier("memory") {
            return Err(ParseError::invalid_syntax(
                "Expected 'memory' after '@'",
                self.current_position(),
                ParseContext::MemoryAnnotation,
            ));
        }
        
        self.expect(TokenType::LeftParen, "memory annotation parameters")?;
        
        let mut annotation = MemoryAnnotation {
            strategy: None,
            size_hint: None,
            alignment: None,
            region: None,
            span: Span::new(start_pos, self.current_position()),
        };
        
        // Parse parameter list
        while !self.check(&TokenType::RightParen) {
            let param_name = self.expect_identifier("parameter name")?;
            
            if !["strategy", "size_hint", "alignment", "region"].contains(&param_name.as_str()) {
                return Err(ParseError::memory_annotation_error(
                    &format!("Unknown parameter: {}", param_name),
                    self.current_position(),
                    &param_name,
                    vec!["strategy".to_string(), "size_hint".to_string(), "alignment".to_string(), "region".to_string()],
                ));
            }
            
            self.expect(TokenType::Equal, "parameter value")?;
            
            match param_name.as_str() {
                "strategy" => {
                    annotation.strategy = Some(self.parse_memory_strategy_value()?);
                }
                "size_hint" => {
                    annotation.size_hint = Some(self.parse_size_value()?);
                }
                "alignment" => {
                    annotation.alignment = Some(self.parse_alignment_value()?);
                }
                "region" => {
                    annotation.region = Some(self.parse_region_identifier()?);
                }
                _ => unreachable!(),
            }
            
            if !self.match_token(&TokenType::Comma) {
                break;
            }
        }
        
        self.expect(TokenType::RightParen, "memory annotation")?;
        annotation.span = Span::new(start_pos, self.current_position());
        
        Ok(annotation)
    }
    
    /// Parse @performance annotation
    pub fn parse_performance_annotation(&mut self) -> ParseResult<PerformanceAnnotation> {
        let start_pos = self.current_position();
        
        self.expect(TokenType::At, "performance annotation")?;
        if !self.match_identifier("performance") {
            return Err(ParseError::invalid_syntax(
                "Expected 'performance' after '@'",
                self.current_position(),
                ParseContext::PerformanceAnnotation,
            ));
        }
        
        self.expect(TokenType::LeftParen, "performance annotation parameters")?;
        
        let mut annotation = PerformanceAnnotation {
            max_cost: None,
            max_memory: None,
            max_latency_ms: None,
            span: Span::new(start_pos, self.current_position()),
        };
        
        // Parse parameter list
        while !self.check(&TokenType::RightParen) {
            let param_name = self.expect_identifier("parameter name")?;
            
            if !["max_cost", "max_memory", "max_latency_ms"].contains(&param_name.as_str()) {
                return Err(ParseError::memory_annotation_error(
                    &format!("Unknown performance parameter: {}", param_name),
                    self.current_position(),
                    &param_name,
                    vec!["max_cost".to_string(), "max_memory".to_string(), "max_latency_ms".to_string()],
                ));
            }
            
            self.expect(TokenType::Equal, "parameter value")?;
            
            match param_name.as_str() {
                "max_cost" => {
                    annotation.max_cost = Some(self.parse_numeric_literal()?);
                }
                "max_memory" => {
                    annotation.max_memory = Some(self.parse_numeric_literal()?);
                }
                "max_latency_ms" => {
                    annotation.max_latency_ms = Some(self.parse_numeric_literal()? as u32);
                }
                _ => unreachable!(),
            }
            
            if !self.match_token(&TokenType::Comma) {
                break;
            }
        }
        
        self.expect(TokenType::RightParen, "performance annotation")?;
        annotation.span = Span::new(start_pos, self.current_position());
        
        Ok(annotation)
    }
    
    /// Parse memory strategy value from string literal
    pub fn parse_memory_strategy_value(&mut self) -> ParseResult<MemoryStrategy> {
        if let Some(token) = &self.current_token {
            if let TokenType::String { value, .. } = &token.token_type {
                let strategy = match value.as_str() {
                    "stack" => MemoryStrategy::Stack,
                    "linear" => MemoryStrategy::Linear,
                    "smartptr" => MemoryStrategy::SmartPtr,
                    "region" => MemoryStrategy::Region,
                    "manual" => MemoryStrategy::Manual,
                    "inferred" => MemoryStrategy::Inferred,
                    _ => {
                        return Err(ParseError::memory_annotation_error(
                            &format!("Invalid memory strategy: {}", value),
                            token.position,
                            value,
                            vec!["stack".to_string(), "linear".to_string(), "smartptr".to_string(), 
                                "region".to_string(), "manual".to_string(), "inferred".to_string()],
                        ));
                    }
                };
                self.advance()?;
                Ok(strategy)
            } else {
                Err(ParseError::unexpected_token(
                    "string literal",
                    "memory strategy value",
                    token.token_type.clone(),
                    token.position,
                    ParseContext::MemoryAnnotation,
                ))
            }
        } else {
            Err(ParseError::unexpected_eof(
                "memory strategy string",
                "string literal with strategy name",
                self.current_position(),
                ParseContext::MemoryAnnotation,
            ))
        }
    }
    
    /// Parse strategy wrapper type: LinearPtr<T>, SmartPtr<T>, etc.
    pub fn parse_strategy_wrapper_type(&mut self) -> ParseResult<Type> {
        let start_pos = self.current_position();
        
        let wrapper_name = self.expect_identifier("strategy wrapper name")?;
        let strategy = match wrapper_name.as_str() {
            "LinearPtr" => MemoryStrategy::Linear,
            "SmartPtr" => MemoryStrategy::SmartPtr,
            "RegionPtr" => MemoryStrategy::Region,
            "StackPtr" => MemoryStrategy::Stack,
            _ => {
                return Err(ParseError::invalid_syntax(
                    &format!("Unknown strategy wrapper: {}", wrapper_name),
                    self.current_position(),
                    ParseContext::TypeAnnotation,
                ));
            }
        };
        
        self.expect(TokenType::Less, "generic type parameter")?;
        let inner_type = self.parse_type()?;
        self.expect(TokenType::Greater, "generic type parameter")?;
        
        Ok(Type::Pointer {
            is_mutable: false,
            target_type: Box::new(inner_type),
            memory_strategy: strategy,
            span: Span::new(start_pos, self.current_position()),
        })
    }
    
    /// Parse memory strategy bound for generics
    pub fn parse_memory_strategy_bound(&mut self) -> ParseResult<TypeBound> {
        let start_pos = self.current_position();
        
        if !self.match_identifier("memory") {
            return Err(ParseError::invalid_syntax(
                "Expected 'memory' for strategy bound",
                self.current_position(),
                ParseContext::GenericParameters,
            ));
        }
        
        self.expect(TokenType::LeftParen, "memory strategy bound")?;
        let strategy = self.parse_memory_strategy_value()?;
        self.expect(TokenType::RightParen, "memory strategy bound")?;
        
                 Ok(TypeBound::MemoryStrategy(strategy))
    }
    
    /// Parse region block: region "name" { ... }
    pub fn parse_region_block(&mut self) -> ParseResult<RegionBlock> {
        let start_pos = self.current_position();
        
        if !self.match_identifier("region") {
            return Err(ParseError::unexpected_token(
                "region",
                "region block declaration",
                self.current_token().unwrap().token_type.clone(),
                self.current_position(),
                ParseContext::Statement,
            ));
        }
        
        let name = self.parse_region_identifier()?;
        self.expect(TokenType::LeftBrace, "region block body")?;
        
        let mut statements = Vec::new();
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        
        self.expect(TokenType::RightBrace, "region block")?;
        
        Ok(RegionBlock {
            name,
            body: statements,
            span: Span::new(start_pos, self.current_position()),
        })
    }
    
    /// Parse variable declaration with memory strategy
    pub fn parse_variable_with_memory_strategy(&mut self) -> ParseResult<VariableDeclaration> {
        let start_pos = self.current_position();
        
        self.expect(TokenType::Let, "variable declaration")?;
        let name = self.expect_identifier("variable name")?;
        self.expect(TokenType::Colon, "type annotation")?;
        
        let var_type = self.parse_type()?;
        
        // Parse @strategy annotation
        let strategy = if self.check(&TokenType::At) {
            self.advance()?;
            if self.match_identifier("memory") {
                self.expect(TokenType::LeftParen, "memory strategy")?;
                self.expect(TokenType::Identifier("strategy".to_string()), "strategy parameter")?;
                self.expect(TokenType::Equal, "strategy value")?;
                let strategy = self.parse_memory_strategy_value()?;
                self.expect(TokenType::RightParen, "memory strategy")?;
                strategy
            } else {
                MemoryStrategy::Inferred
            }
        } else {
            MemoryStrategy::Inferred
        };
        
        let initializer = if self.match_token(&TokenType::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "variable declaration")?;
        
        Ok(VariableDeclaration {
            name: self.interner.intern(&name),
            var_type,
            strategy,
            initializer,
            span: Span::new(start_pos, self.current_position()),
        })
    }
    
    // Helper methods
    
    /// Expect an identifier token and return its value
    fn expect_identifier(&mut self, description: &str) -> ParseResult<String> {
        if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                let name = name.clone();
                self.advance()?;
                Ok(name)
            } else {
                Err(ParseError::unexpected_token(
                    "identifier",
                    description,
                    token.token_type.clone(),
                    token.position,
                    ParseContext::MemoryAnnotation,
                ))
            }
        } else {
            Err(ParseError::unexpected_eof(
                "identifier",
                description,
                self.current_position(),
                ParseContext::MemoryAnnotation,
            ))
        }
    }
    
    /// Match an identifier with specific value
    fn match_identifier(&mut self, expected: &str) -> bool {
        if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                if name == expected {
                    self.advance().unwrap_or(());
                    return true;
                }
            }
        }
        false
    }
    
    /// Parse a numeric literal
    fn parse_numeric_literal(&mut self) -> ParseResult<u64> {
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::Integer { value, .. } => {
                    let num = value.parse::<u64>()
                        .map_err(|_| ParseError::invalid_syntax(
                            &format!("Invalid numeric literal: {}", value),
                            token.position,
                            ParseContext::MemoryAnnotation,
                        ))?;
                    self.advance()?;
                    Ok(num)
                }
                _ => Err(ParseError::unexpected_token(
                    "numeric literal",
                    "integer value",
                    token.token_type.clone(),
                    token.position,
                    ParseContext::MemoryAnnotation,
                ))
            }
        } else {
            Err(ParseError::unexpected_eof(
                "numeric literal",
                "integer value",
                self.current_position(),
                ParseContext::MemoryAnnotation,
            ))
        }
    }
    
    /// Parse size value
    fn parse_size_value(&mut self) -> ParseResult<u64> {
        self.parse_numeric_literal()
    }
    
    /// Parse alignment value
    fn parse_alignment_value(&mut self) -> ParseResult<u8> {
        let value = self.parse_numeric_literal()?;
        if value > 255 {
            return Err(ParseError::invalid_syntax(
                "Alignment must be <= 255",
                self.current_position(),
                ParseContext::MemoryAnnotation,
            ));
        }
        Ok(value as u8)
    }
    
    /// Parse region identifier
    fn parse_region_identifier(&mut self) -> ParseResult<InternedString> {
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::String { value, .. } => {
                    let interned = self.interner.intern(value);
                    self.advance()?;
                    Ok(interned)
                }
                TokenType::Identifier(name) => {
                    let interned = self.interner.intern(name);
                    self.advance()?;
                    Ok(interned)
                }
                _ => Err(ParseError::unexpected_token(
                    "region identifier",
                    "string literal or identifier",
                    token.token_type.clone(),
                    token.position,
                    ParseContext::MemoryAnnotation,
                ))
            }
        } else {
            Err(ParseError::unexpected_eof(
                "region identifier",
                "string literal or identifier",
                self.current_position(),
                ParseContext::MemoryAnnotation,
            ))
        }
    }
    
    /// Parse boolean literal
    fn parse_boolean_literal(&mut self) -> ParseResult<bool> {
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::True => {
                    self.advance()?;
                    Ok(true)
                }
                TokenType::False => {
                    self.advance()?;
                    Ok(false)
                }
                _ => Err(ParseError::unexpected_token(
                    "boolean literal",
                    "true or false",
                    token.token_type.clone(),
                    token.position,
                    ParseContext::MemoryAnnotation,
                ))
            }
        } else {
            Err(ParseError::unexpected_eof(
                "boolean literal",
                "true or false",
                self.current_position(),
                ParseContext::MemoryAnnotation,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::semantic::symbols::SymbolTable;
    
    fn parse_memory_annotation(input: &str) -> ParseResult<MemoryAnnotation> {
        let mut parser = Parser::new(input, 0).unwrap();
        parser.parse_memory_annotation()
    }
    
    #[test]
    fn test_memory_annotation_basic() {
        let result = parse_memory_annotation("@memory(strategy = \"stack\")");
        assert!(result.is_ok());
        let annotation = result.unwrap();
        assert_eq!(annotation.strategy, Some(MemoryStrategy::Stack));
    }
    
    #[test]
    fn test_memory_annotation_multiple_params() {
        let result = parse_memory_annotation("@memory(strategy = \"linear\", size_hint = 1024)");
        assert!(result.is_ok());
        let annotation = result.unwrap();
        assert_eq!(annotation.strategy, Some(MemoryStrategy::Linear));
        assert_eq!(annotation.size_hint, Some(1024));
    }
    
    #[test]
    fn test_strategy_wrapper_type() {
        let input = "LinearPtr<i32>";
        let mut parser = Parser::new(input, 0).unwrap();
        let result = parser.parse_strategy_wrapper_type();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_invalid_strategy() {
        let result = parse_memory_annotation("@memory(strategy = \"invalid\")");
        assert!(result.is_err());
    }
} 