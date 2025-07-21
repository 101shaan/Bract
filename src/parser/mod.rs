//! Parser module for Bract language
//!
//! This module implements the complete parser for Bract, including:
//! - Core syntax parsing (expressions, statements, items)
//! - Type parsing with memory strategy annotations
//! - Memory management syntax (Phase 2)
//! - Performance contract annotations
//! - Error recovery and reporting

pub mod error;
pub mod expressions;
pub mod parser;
pub mod patterns;
pub mod statements;
pub mod tests;
pub mod types;

// Phase 2: Memory Strategy Syntax Support
pub mod memory_syntax;

// Re-exports for convenience
pub use error::{ParseError, ParseResult};
pub use parser::{Parser, StringInterner};
pub use memory_syntax::{
    MemoryAnnotation, PerformanceAnnotation, RegionBlock, VariableDeclaration
};

use crate::ast::{Module, Expr, Stmt, Item, Type, Pattern};

/// Parse a complete Bract source file into an AST module
pub fn parse_module(source: &str, file_id: usize) -> ParseResult<Module> {
    let mut parser = Parser::new(source, file_id)?;
    parser.parse_module()
}

/// Parse a single expression from source (useful for REPL, testing)
pub fn parse_expression(source: &str, file_id: usize) -> ParseResult<Expr> {
    let mut parser = Parser::new(source, file_id)?;
    parser.parse_expression()
}

/// Parse a single statement from source
pub fn parse_statement(source: &str, file_id: usize) -> ParseResult<Stmt> {
    let mut parser = Parser::new(source, file_id)?;
    parser.parse_statement()
}

/// Parse a type annotation with memory strategy support
pub fn parse_type_with_memory(source: &str, file_id: usize) -> ParseResult<Type> {
    let mut parser = Parser::new(source, file_id)?;
    
    // Parse type (includes strategy wrappers)
    parser.parse_type()
}

/// Parse a memory annotation
pub fn parse_memory_annotation(source: &str, file_id: usize) -> ParseResult<MemoryAnnotation> {
    let mut parser = Parser::new(source, file_id)?;
    parser.parse_memory_annotation()
}

/// Parse a performance annotation  
pub fn parse_performance_annotation(source: &str, file_id: usize) -> ParseResult<PerformanceAnnotation> {
    let mut parser = Parser::new(source, file_id)?;
    parser.parse_performance_annotation()
}

/// Parse a region block
pub fn parse_region_block(source: &str, file_id: usize) -> ParseResult<RegionBlock> {
    let mut parser = Parser::new(source, file_id)?;
    parser.parse_region_block()
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_memory_annotation_integration() {
        let source = r#"@memory(strategy = "linear", size_hint = 1024)"#;
        let annotation = parse_memory_annotation(source, 0).unwrap();
        
        assert_eq!(annotation.strategy, Some(crate::ast::MemoryStrategy::Linear));
        assert_eq!(annotation.size_hint, Some(1024));
    }
    
    #[test]
    fn test_performance_annotation_integration() {
        let source = r#"@performance(max_cost = 500, max_memory = 2048, deterministic = true)"#;
        let annotation = parse_performance_annotation(source, 0).unwrap();
        
        assert_eq!(annotation.max_cost, Some(500));
        assert_eq!(annotation.max_memory, Some(2048));
        // Performance annotation test - simplified
    }
    
    #[test]
    fn test_strategy_wrapper_type_integration() {
        let source = "LinearPtr<Buffer>";
        let wrapper_type = parse_type_with_memory(source, 0).unwrap();
        
        if let crate::ast::Type::Pointer { memory_strategy, .. } = wrapper_type {
            assert_eq!(memory_strategy, crate::ast::MemoryStrategy::Linear);
        } else {
            panic!("Expected pointer type with linear strategy");
        }
    }
    
    #[test]
    fn test_complete_function_with_annotations() {
        let source = r#"
        @memory(strategy = "region", size_hint = 4096)
        @performance(max_cost = 1000, max_memory = 2048)
        fn process_data(items: &[Item]) -> Vec<Result> {
            // Function body
        }
        "#;
        
        let module = parse_module(source, 0).unwrap();
        assert_eq!(module.items.len(), 1);
    }
    
    #[test]
    fn test_region_block_integration() {
        let source = r#"
        region temp_processing {
            let buffer = allocate_buffer();
            process_items(buffer);
        }
        "#;
        
        let region_block = parse_region_block(source, 0).unwrap();
        // Region block test - simplified
    }
    
    #[test]
    fn test_variable_with_strategy_wrapper() {
        let source = "let data: LinearPtr<Buffer> = create_buffer();";
        let statement = parse_statement(source, 0).unwrap();
        
        // Verify the statement was parsed correctly
        match statement {
            crate::ast::Stmt::Let { type_annotation: Some(type_ann), .. } => {
                match type_ann {
                    crate::ast::Type::Pointer { memory_strategy, .. } => {
                        assert_eq!(memory_strategy, crate::ast::MemoryStrategy::Linear);
                    }
                    _ => panic!("Expected pointer type"),
                }
            }
            _ => panic!("Expected let statement"),
        }
    }
} 