//! Parser tests for the Prism programming language

#[cfg(test)]
mod tests {
    use super::super::{Parser, ParseResult};
    use crate::ast::*;

    /// Helper function to create a parser and parse a module
    fn parse_module(input: &str) -> ParseResult<Module> {
        let mut parser = Parser::new(input, 0)?;
        parser.parse_module()
    }

    /// Helper function to create a parser and parse an expression
    fn parse_expression(input: &str) -> ParseResult<Expr> {
        let mut parser = Parser::new(input, 0)?;
        parser.parse_expression()
    }

    #[test]
    fn test_parser_creation() {
        let result = Parser::new("fn main() {}", 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_module() {
        let result = parse_module("");
        assert!(result.is_ok());
        let module = result.unwrap();
        assert!(module.items.is_empty());
    }

    #[test]
    fn test_simple_function_declaration() {
        let input = "fn test();";
        let result = parse_module(input);
        assert!(result.is_ok());
        
        let module = result.unwrap();
        assert_eq!(module.items.len(), 1);
        
        match &module.items[0] {
            Item::Function { name, params, return_type, body, .. } => {
                // We can't easily test the interned string content without access to interner
                assert!(params.is_empty());
                assert!(return_type.is_none());
                assert!(body.is_none());
            }
            _ => panic!("Expected function item"),
        }
    }

    #[test]
    fn test_simple_expression_parsing() {
        let result = parse_expression("42");
        assert!(result.is_ok());
        
        match result.unwrap() {
            Expr::Literal { literal: Literal::Integer { .. }, .. } => {
                // Success - we parsed an integer literal
            }
            _ => panic!("Expected integer literal"),
        }
    }

    #[test]
    fn test_binary_expression() {
        let result = parse_expression("1 + 2");
        assert!(result.is_ok());
        
        match result.unwrap() {
            Expr::Binary { op: BinaryOp::Add, .. } => {
                // Success - we parsed an addition expression
            }
            _ => panic!("Expected binary addition expression"),
        }
    }

    #[test]
    fn test_parenthesized_expression() {
        let result = parse_expression("(42)");
        assert!(result.is_ok());
        
        match result.unwrap() {
            Expr::Parenthesized { .. } => {
                // Success - we parsed a parenthesized expression
            }
            _ => panic!("Expected parenthesized expression"),
        }
    }

    #[test]
    fn test_identifier_expression() {
        let result = parse_expression("variable");
        assert!(result.is_ok());
        
        match result.unwrap() {
            Expr::Identifier { .. } => {
                // Success - we parsed an identifier
            }
            _ => panic!("Expected identifier expression"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        let result = parse_expression("1 + 2 * 3");
        assert!(result.is_ok());
        
        // Should parse as 1 + (2 * 3) due to precedence
        match result.unwrap() {
            Expr::Binary { 
                op: BinaryOp::Add,
                left,
                right,
                ..
            } => {
                // Left should be literal 1
                match left.as_ref() {
                    Expr::Literal { literal: Literal::Integer { .. }, .. } => (),
                    _ => panic!("Expected integer literal on left"),
                }
                
                // Right should be multiplication
                match right.as_ref() {
                    Expr::Binary { op: BinaryOp::Multiply, .. } => (),
                    _ => panic!("Expected multiplication on right"),
                }
            }
            _ => panic!("Expected addition at top level"),
        }
    }

    #[test]
    fn test_unary_expression() {
        let result = parse_expression("-42");
        assert!(result.is_ok());
        
        match result.unwrap() {
            Expr::Unary { op: UnaryOp::Negate, .. } => {
                // Success - we parsed a negation
            }
            _ => panic!("Expected unary negation expression"),
        }
    }

    #[test]
    fn test_error_handling() {
        let result = parse_expression("1 +");
        assert!(result.is_err());
        
        // Should fail because there's no right operand for +
    }

    #[test]
    fn test_multiple_errors() {
        let input = "fn test( { fn other();";
        let mut parser = Parser::new(input, 0).unwrap();
        let result = parser.parse_module();
        
        // Should have errors but still return a result due to error recovery
        assert!(!parser.errors().is_empty());
    }
} 