//! Memory Strategy Syntax Parser
//!
//! This module extends the Bract parser to support explicit memory strategy annotations
//! and polymorphic memory strategy parameters. It enables developers to:
//! - Explicitly specify memory strategies for variables and functions
//! - Use polymorphic memory strategy parameters in generic functions
//! - Control memory allocation behavior at the language level
//! - Express performance contracts with memory constraints

use crate::ast::{Type, Expr, Span, InternedString, MemoryStrategy, TypeBound, TypeConstraint};
use crate::lexer::{Token, TokenType};
use crate::parser::{Parser, ParseResult, ParseError};

/// Memory strategy annotation syntax parser
impl<'a> Parser<'a> {
    /// Parse memory strategy annotations
    /// Syntax: `@memory(strategy = "stack" | "linear" | "smartptr" | "region" | "manual")`
    pub fn parse_memory_annotation(&mut self) -> ParseResult<MemoryAnnotation> {
        let start_pos = self.current_position();
        
        // Expect @memory
        self.expect(TokenType::At, "memory annotation")?;
        if !self.match_identifier("memory") {
            return Err(ParseError::InvalidSyntax {
                message: "Expected 'memory' after '@'".to_string(),
                position: self.current_position(),
            });
        }
        
        self.expect(TokenType::LeftParen, "memory annotation parameters")?;
        
        let mut strategy = None;
        let mut size_hint = None;
        let mut alignment = None;
        let mut region_id = None;
        
        // Parse memory annotation parameters
        while !self.check(&TokenType::RightParen) && !self.is_at_end() {
            if self.match_identifier("strategy") {
                self.expect(TokenType::Equal, "memory strategy assignment")?;
                strategy = Some(self.parse_memory_strategy_value()?);
            } else if self.match_identifier("size_hint") {
                self.expect(TokenType::Equal, "size hint assignment")?;
                size_hint = Some(self.parse_size_value()?);
            } else if self.match_identifier("alignment") {
                self.expect(TokenType::Equal, "alignment assignment")?;
                alignment = Some(self.parse_alignment_value()?);
            } else if self.match_identifier("region") {
                self.expect(TokenType::Equal, "region assignment")?;
                region_id = Some(self.parse_region_identifier()?);
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: vec!["strategy".to_string(), "size_hint".to_string(), 
                                  "alignment".to_string(), "region".to_string()],
                    found: self.current_token().unwrap().token_type.clone(),
                    position: self.current_position(),
                });
            }
            
            if !self.match_token(&TokenType::Comma) {
                break;
            }
        }
        
        self.expect(TokenType::RightParen, "memory annotation")?;
        
        let end_pos = self.current_position();
        Ok(MemoryAnnotation {
            strategy: strategy.unwrap_or(MemoryStrategy::Inferred),
            size_hint,
            alignment,
            region_id,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse performance contract annotations
    /// Syntax: `@performance(max_cost = 1000, max_memory = 1024, strategy = "linear")`
    pub fn parse_performance_annotation(&mut self) -> ParseResult<PerformanceAnnotation> {
        let start_pos = self.current_position();
        
        // Expect @performance
        self.expect(TokenType::At, "performance annotation")?;
        if !self.match_identifier("performance") {
            return Err(ParseError::InvalidSyntax {
                message: "Expected 'performance' after '@'".to_string(),
                position: self.current_position(),
            });
        }
        
        self.expect(TokenType::LeftParen, "performance annotation parameters")?;
        
        let mut max_cost = None;
        let mut max_memory = None;
        let mut max_allocations = None;
        let mut max_stack = None;
        let mut required_strategy = None;
        let mut deterministic = false;
        
        // Parse performance contract parameters
        while !self.check(&TokenType::RightParen) && !self.is_at_end() {
            if self.match_identifier("max_cost") {
                self.expect(TokenType::Equal, "max cost assignment")?;
                max_cost = Some(self.parse_numeric_literal()?);
            } else if self.match_identifier("max_memory") {
                self.expect(TokenType::Equal, "max memory assignment")?;
                max_memory = Some(self.parse_size_value()?);
            } else if self.match_identifier("max_allocations") {
                self.expect(TokenType::Equal, "max allocations assignment")?;
                max_allocations = Some(self.parse_numeric_literal()?);
            } else if self.match_identifier("max_stack") {
                self.expect(TokenType::Equal, "max stack assignment")?;
                max_stack = Some(self.parse_size_value()?);
            } else if self.match_identifier("strategy") {
                self.expect(TokenType::Equal, "required strategy assignment")?;
                required_strategy = Some(self.parse_memory_strategy_value()?);
            } else if self.match_identifier("deterministic") {
                self.expect(TokenType::Equal, "deterministic assignment")?;
                deterministic = self.parse_boolean_literal()?;
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: vec!["max_cost".to_string(), "max_memory".to_string(), 
                                  "max_allocations".to_string(), "strategy".to_string()],
                    found: self.current_token().unwrap().token_type.clone(),
                    position: self.current_position(),
                });
            }
            
            if !self.match_token(&TokenType::Comma) {
                break;
            }
        }
        
        self.expect(TokenType::RightParen, "performance annotation")?;
        
        let end_pos = self.current_position();
        Ok(PerformanceAnnotation {
            max_cost,
            max_memory,
            max_allocations,
            max_stack,
            required_strategy,
            deterministic,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse memory strategy value from string literal
    /// Recognizes: "stack", "linear", "smartptr", "region", "manual"
    fn parse_memory_strategy_value(&mut self) -> ParseResult<MemoryStrategy> {
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::String { value, .. } => {
                    let strategy = match value.as_str() {
                        "stack" => MemoryStrategy::Stack,
                        "linear" => MemoryStrategy::Linear,
                        "smartptr" => MemoryStrategy::SmartPtr,
                        "region" => MemoryStrategy::Region,
                        "manual" => MemoryStrategy::Manual,
                        _ => return Err(ParseError::InvalidSyntax {
                            message: format!("Unknown memory strategy: '{}'", value),
                            position: token.position,
                        }),
                    };
                    
                    self.advance()?;
                    Ok(strategy)
                }
                _ => Err(ParseError::UnexpectedToken {
                    expected: vec!["string literal".to_string()],
                    found: token.token_type.clone(),
                    position: token.position,
                }),
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["memory strategy string".to_string()],
                position: self.current_position(),
            })
        }
    }
    
    /// Parse explicit memory strategy type annotations
    /// Syntax: `LinearPtr<T>`, `SmartPtr<T>`, `ManualPtr<T>`, `RegionPtr<T>`
    pub fn parse_strategy_wrapper_type(&mut self) -> ParseResult<Option<Type>> {
        if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                let strategy = match name.as_str() {
                    "LinearPtr" => Some(MemoryStrategy::Linear),
                    "SmartPtr" => Some(MemoryStrategy::SmartPtr),
                    "ManualPtr" => Some(MemoryStrategy::Manual),
                    "RegionPtr" => Some(MemoryStrategy::Region),
                    "StackPtr" => Some(MemoryStrategy::Stack), // Rarely used, mostly for consistency
                    _ => None,
                };
                
                if let Some(memory_strategy) = strategy {
                    let start_pos = self.current_position();
                    self.advance()?; // consume wrapper name
                    
                    // Parse generic parameter: Ptr<T>
                    self.expect(TokenType::Less, "generic parameter")?;
                    let inner_type = self.parse_type()?;
                    self.expect(TokenType::Greater, "generic parameter")?;
                    
                    let end_pos = self.current_position();
                    return Ok(Some(Type::Pointer {
                        is_mutable: false, // TODO: Handle mutable variants
                        target_type: Box::new(inner_type),
                        memory_strategy,
                        span: Span::new(start_pos, end_pos),
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Parse polymorphic memory strategy parameters
    /// Syntax: `fn process<T, S: MemoryStrategy>(data: S<T>) -> S<ProcessedT>`
    pub fn parse_memory_strategy_bound(&mut self) -> ParseResult<TypeBound> {
        if self.match_identifier("MemoryStrategy") {
            // Basic memory strategy bound
            return Ok(TypeBound::MemoryStrategy(MemoryStrategy::Inferred));
        }
        
        // Specific strategy constraints
        if self.match_identifier("Stack") {
            return Ok(TypeBound::MemoryStrategy(MemoryStrategy::Stack));
        } else if self.match_identifier("Linear") {
            return Ok(TypeBound::MemoryStrategy(MemoryStrategy::Linear));
        } else if self.match_identifier("SmartPtr") {
            return Ok(TypeBound::MemoryStrategy(MemoryStrategy::SmartPtr));
        } else if self.match_identifier("Region") {
            return Ok(TypeBound::MemoryStrategy(MemoryStrategy::Region));
        } else if self.match_identifier("Manual") {
            return Ok(TypeBound::MemoryStrategy(MemoryStrategy::Manual));
        }
        
        Err(ParseError::InvalidSyntax {
            message: "Expected memory strategy bound".to_string(),
            position: self.current_position(),
        })
    }
    
    /// Parse region-scoped allocation syntax
    /// Syntax: `region region_name { ... }`
    pub fn parse_region_block(&mut self) -> ParseResult<RegionBlock> {
        let start_pos = self.current_position();
        
        // Expect 'region' keyword
        if !self.match_identifier("region") {
            return Err(ParseError::UnexpectedToken {
                expected: vec!["region".to_string()],
                found: self.current_token().unwrap().token_type.clone(),
                position: self.current_position(),
            });
        }
        
        // Parse region name
        let region_name = if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                let name = self.interner.intern(name);
                self.advance()?;
                Some(name)
            } else {
                None
            }
        } else {
            None
        };
        
        // Parse region body
        let body = self.parse_block_expression()?;
        
        let end_pos = self.current_position();
        Ok(RegionBlock {
            name: region_name,
            body,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse variable declaration with memory strategy
    /// Syntax: `let var: StrategyPtr<Type> = value;`
    /// Syntax: `let var: Type @stack = value;`
    pub fn parse_variable_with_memory_strategy(&mut self) -> ParseResult<VariableDeclaration> {
        let start_pos = self.current_position();
        
        // Parse basic let declaration
        self.expect(TokenType::Let, "variable declaration")?;
        
        let is_mutable = self.match_token(&TokenType::Mut);
        
        // Parse variable name
        let name = if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name_str) = &token.token_type {
                let name = self.interner.intern(name_str);
                self.advance()?;
                name
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: vec!["identifier".to_string()],
                    found: token.token_type.clone(),
                    position: token.position,
                });
            }
        } else {
            return Err(ParseError::UnexpectedEof {
                expected: vec!["variable name".to_string()],
                position: self.current_position(),
            });
        };
        
        // Parse type annotation with memory strategy
        let (type_annotation, memory_strategy) = if self.match_token(&TokenType::Colon) {
            // Try to parse strategy wrapper first
            if let Some(wrapper_type) = self.parse_strategy_wrapper_type()? {
                (Some(wrapper_type), MemoryStrategy::Inferred) // Strategy embedded in type
            } else {
                // Parse regular type
                let base_type = self.parse_type()?;
                
                // Check for explicit strategy annotation: Type @stack
                let strategy = if self.match_token(&TokenType::At) {
                    self.parse_inline_memory_strategy()?
                } else {
                    MemoryStrategy::Inferred
                };
                
                (Some(base_type), strategy)
            }
        } else {
            (None, MemoryStrategy::Inferred)
        };
        
        // Parse initializer
        let initializer = if self.match_token(&TokenType::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "variable declaration")?;
        
        let end_pos = self.current_position();
        Ok(VariableDeclaration {
            name,
            is_mutable,
            type_annotation,
            memory_strategy,
            initializer,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse inline memory strategy annotation
    /// Syntax: `@stack`, `@linear`, `@smartptr`, `@region`, `@manual`
    fn parse_inline_memory_strategy(&mut self) -> ParseResult<MemoryStrategy> {
        if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                let strategy = match name.as_str() {
                    "stack" => MemoryStrategy::Stack,
                    "linear" => MemoryStrategy::Linear,
                    "smartptr" => MemoryStrategy::SmartPtr,
                    "region" => MemoryStrategy::Region,
                    "manual" => MemoryStrategy::Manual,
                    _ => return Err(ParseError::InvalidSyntax {
                        message: format!("Unknown memory strategy: @{}", name),
                        position: token.position,
                    }),
                };
                
                self.advance()?;
                Ok(strategy)
            } else {
                Err(ParseError::UnexpectedToken {
                    expected: vec!["memory strategy identifier".to_string()],
                    found: token.token_type.clone(),
                    position: token.position,
                })
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["memory strategy".to_string()],
                position: self.current_position(),
            })
        }
    }
    
    /// Helper: Check if current token is a specific identifier
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
    
    /// Parse numeric literal for performance constraints
    fn parse_numeric_literal(&mut self) -> ParseResult<u64> {
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::Integer { value, .. } => {
                    let parsed_value = value.parse::<u64>()
                        .map_err(|_| ParseError::InvalidSyntax {
                            message: format!("Invalid numeric literal: {}", value),
                            position: token.position,
                        })?;
                    self.advance()?;
                    Ok(parsed_value)
                }
                _ => Err(ParseError::UnexpectedToken {
                    expected: vec!["numeric literal".to_string()],
                    found: token.token_type.clone(),
                    position: token.position,
                }),
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["numeric literal".to_string()],
                position: self.current_position(),
            })
        }
    }
    
    /// Parse size value with units (e.g., "1024", "4KB", "2MB")
    fn parse_size_value(&mut self) -> ParseResult<u64> {
        let base_value = self.parse_numeric_literal()?;
        
        // Check for size unit suffix
        if let Some(token) = &self.current_token {
            if let TokenType::Identifier(unit) = &token.token_type {
                let multiplier = match unit.to_uppercase().as_str() {
                    "B" | "BYTES" => 1,
                    "KB" | "KILOBYTES" => 1024,
                    "MB" | "MEGABYTES" => 1024 * 1024,
                    "GB" | "GIGABYTES" => 1024 * 1024 * 1024,
                    _ => return Ok(base_value), // No unit, return base value
                };
                
                self.advance()?;
                Ok(base_value * multiplier)
            } else {
                Ok(base_value)
            }
        } else {
            Ok(base_value)
        }
    }
    
    /// Parse alignment value (power of 2)
    fn parse_alignment_value(&mut self) -> ParseResult<u8> {
        let value = self.parse_numeric_literal()?;
        
        // Validate that alignment is power of 2
        if value == 0 || (value & (value - 1)) != 0 {
            return Err(ParseError::InvalidSyntax {
                message: format!("Alignment must be power of 2, got {}", value),
                position: self.current_position(),
            });
        }
        
        Ok(value as u8)
    }
    
    /// Parse region identifier for region-specific allocation
    fn parse_region_identifier(&mut self) -> ParseResult<InternedString> {
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::Identifier(name) => {
                    let region_name = self.interner.intern(name);
                    self.advance()?;
                    Ok(region_name)
                }
                TokenType::String { value, .. } => {
                    let region_name = self.interner.intern(value);
                    self.advance()?;
                    Ok(region_name)
                }
                _ => Err(ParseError::UnexpectedToken {
                    expected: vec!["region identifier".to_string()],
                    found: token.token_type.clone(),
                    position: token.position,
                }),
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["region identifier".to_string()],
                position: self.current_position(),
            })
        }
    }
    
    /// Parse boolean literal
    fn parse_boolean_literal(&mut self) -> ParseResult<bool> {
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::Bool(value) => {
                    let result = *value;
                    self.advance()?;
                    Ok(result)
                }
                TokenType::True => {
                    self.advance()?;
                    Ok(true)
                }
                TokenType::False => {
                    self.advance()?;
                    Ok(false)
                }
                TokenType::Identifier(name) if name == "true" => {
                    self.advance()?;
                    Ok(true)
                }
                TokenType::Identifier(name) if name == "false" => {
                    self.advance()?;
                    Ok(false)
                }
                _ => Err(ParseError::UnexpectedToken {
                    expected: vec!["boolean literal".to_string()],
                    found: token.token_type.clone(),
                    position: token.position,
                }),
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["boolean literal".to_string()],
                position: self.current_position(),
            })
        }
    }
}

/// Memory strategy annotation structure
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAnnotation {
    pub strategy: MemoryStrategy,
    pub size_hint: Option<u64>,
    pub alignment: Option<u8>,
    pub region_id: Option<InternedString>,
    pub span: Span,
}

/// Performance contract annotation structure
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceAnnotation {
    pub max_cost: Option<u64>,
    pub max_memory: Option<u64>,
    pub max_allocations: Option<u64>,
    pub max_stack: Option<u64>,
    pub required_strategy: Option<MemoryStrategy>,
    pub deterministic: bool,
    pub span: Span,
}

/// Region block for scoped allocation
#[derive(Debug, Clone, PartialEq)]
pub struct RegionBlock {
    pub name: Option<InternedString>,
    pub body: Expr,
    pub span: Span,
}

/// Variable declaration with memory strategy
#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclaration {
    pub name: InternedString,
    pub is_mutable: bool,
    pub type_annotation: Option<Type>,
    pub memory_strategy: MemoryStrategy,
    pub initializer: Option<Expr>,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::semantic::symbols::SymbolTable;
    
    fn create_test_parser(input: &str) -> Parser {
        Parser::new(input, 0).unwrap()
    }
    
    #[test]
    fn test_memory_annotation_parsing() {
        let mut parser = create_test_parser(r#"@memory(strategy = "stack", size_hint = 1024)"#);
        let annotation = parser.parse_memory_annotation().unwrap();
        
        assert_eq!(annotation.strategy, MemoryStrategy::Stack);
        assert_eq!(annotation.size_hint, Some(1024));
    }
    
    #[test]
    fn test_performance_annotation_parsing() {
        let mut parser = create_test_parser(r#"@performance(max_cost = 1000, max_memory = 2048, deterministic = true)"#);
        let annotation = parser.parse_performance_annotation().unwrap();
        
        assert_eq!(annotation.max_cost, Some(1000));
        assert_eq!(annotation.max_memory, Some(2048));
        assert_eq!(annotation.deterministic, true);
    }
    
    #[test]
    fn test_strategy_wrapper_parsing() {
        let mut parser = create_test_parser("LinearPtr<Buffer>");
        let wrapper_type = parser.parse_strategy_wrapper_type().unwrap();
        
        assert!(wrapper_type.is_some());
        if let Some(Type::Pointer { memory_strategy, .. }) = wrapper_type {
            assert_eq!(memory_strategy, MemoryStrategy::Linear);
        } else {
            panic!("Expected pointer type with linear strategy");
        }
    }
    
    #[test]
    fn test_region_block_parsing() {
        let mut parser = create_test_parser("region temp_data { let x = 42; }");
        let region_block = parser.parse_region_block().unwrap();
        
        assert!(region_block.name.is_some());
    }
} 