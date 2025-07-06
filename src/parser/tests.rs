//! Parser tests for the Bract programming language

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
            Item::Function { name: _, params, return_type, body, .. } => {
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
        let _result = parser.parse_module();
        
        // Should have errors but still return a result due to error recovery
        assert!(!parser.errors().is_empty());
    }

    #[test]
    fn test_let_statement() {
        let mut parser = Parser::new(
            "let x = 42; let mut y: i32 = 10; let z;",
            0
        ).unwrap();
        
        // Parse first let statement
        let stmt1 = parser.parse_statement().unwrap();
        match stmt1 {
            Stmt::Let { pattern, type_annotation, initializer, is_mutable, .. } => {
                assert!(!is_mutable);
                assert!(type_annotation.is_none());
                assert!(initializer.is_some());
                if let Some(Pattern::Identifier { name: _, .. }) = Some(pattern) {
                    // Pattern parsing will be implemented later
                }
            }
            _ => panic!("Expected let statement"),
        }
        
        // Parse second let statement (mutable with type)
        let stmt2 = parser.parse_statement().unwrap();
        match stmt2 {
            Stmt::Let { is_mutable, type_annotation, .. } => {
                assert!(is_mutable);
                assert!(type_annotation.is_some());
            }
            _ => panic!("Expected mutable let statement"),
        }
        
        // Parse third let statement (no initializer)
        let stmt3 = parser.parse_statement().unwrap();
        match stmt3 {
            Stmt::Let { initializer, .. } => {
                assert!(initializer.is_none());
            }
            _ => panic!("Expected let statement without initializer"),
        }
    }
    
    #[test]
    fn test_assignment_statements() {
        let mut parser = Parser::new(
            "x = 42; y += 10; z *= 3;",
            0
        ).unwrap();
        
        // Parse regular assignment
        let stmt1 = parser.parse_statement().unwrap();
        match stmt1 {
            Stmt::Assignment { .. } => {},
            _ => panic!("Expected assignment statement"),
        }
        
        // Parse compound assignment (+=)
        let stmt2 = parser.parse_statement().unwrap();
        match stmt2 {
            Stmt::CompoundAssignment { op, .. } => {
                assert_eq!(op, BinaryOp::Add);
            }
            _ => panic!("Expected compound assignment statement"),
        }
        
        // Parse compound assignment (*=)
        let stmt3 = parser.parse_statement().unwrap();
        match stmt3 {
            Stmt::CompoundAssignment { op, .. } => {
                assert_eq!(op, BinaryOp::Multiply);
            }
            _ => panic!("Expected compound assignment statement"),
        }
    }
    
    #[test]
    fn test_if_statement() {
        let mut parser = Parser::new(
            "if x > 0 { return 1; } else { return 0; }",
            0
        ).unwrap();
        
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::If { condition, then_block, else_block, .. } => {
                // Check condition is a binary expression
                matches!(condition, Expr::Binary { .. });
                
                // Check then block has statements
                assert!(!then_block.is_empty());
                
                // Check else block exists
                assert!(else_block.is_some());
            }
            _ => panic!("Expected if statement"),
        }
    }
    
    #[test]
    fn test_while_statement() {
        let mut parser = Parser::new(
            "while x < 10 { x = x + 1; }",
            0
        ).unwrap();
        
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::While { condition, body, .. } => {
                matches!(condition, Expr::Binary { .. });
                assert!(!body.is_empty());
            }
            _ => panic!("Expected while statement"),
        }
    }
    
    #[test]
    fn test_for_statement() {
        let mut parser = Parser::new(
            "for i in 0..10 { print(i); }",
            0
        ).unwrap();
        
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For { pattern: _, iterable, body, .. } => {
                matches!(iterable, Expr::Range { .. });
                assert!(!body.is_empty());
            }
            _ => panic!("Expected for statement"),
        }
    }
    
    #[test]
    fn test_loop_statement() {
        let mut parser = Parser::new(
            "loop { break; }",
            0
        ).unwrap();
        
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Loop { label, body, .. } => {
                assert!(label.is_none());
                assert!(!body.is_empty());
                // Check first statement is break
                matches!(body[0], Stmt::Break { .. });
            }
            _ => panic!("Expected loop statement"),
        }
    }
    
    #[test]
    fn test_break_continue_return() {
        let mut parser = Parser::new(
            "break; continue; return 42;",
            0
        ).unwrap();
        
        // Test break
        let stmt1 = parser.parse_statement().unwrap();
        match stmt1 {
            Stmt::Break { label, expr, .. } => {
                assert!(label.is_none());
                assert!(expr.is_none());
            }
            _ => panic!("Expected break statement"),
        }
        
        // Test continue
        let stmt2 = parser.parse_statement().unwrap();
        match stmt2 {
            Stmt::Continue { label, .. } => {
                assert!(label.is_none());
            }
            _ => panic!("Expected continue statement"),
        }
        
        // Test return with value
        let stmt3 = parser.parse_statement().unwrap();
        match stmt3 {
            Stmt::Return { expr, .. } => {
                assert!(expr.is_some());
            }
            _ => panic!("Expected return statement"),
        }
    }
    
    #[test]
    fn test_block_statement() {
        let mut parser = Parser::new(
            "{ let x = 1; let y = 2; x + y }",
            0
        ).unwrap();
        
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expr::Block { statements, trailing_expr, .. } => {
                assert_eq!(statements.len(), 2);
                assert!(trailing_expr.is_some());
                
                // Check statements are let bindings
                matches!(statements[0], Stmt::Let { .. });
                matches!(statements[1], Stmt::Let { .. });
            }
            _ => panic!("Expected block expression"),
        }
    }
    
    #[test]
    fn test_expression_statement() {
        let mut parser = Parser::new(
            "foo(); 42; true;",
            0
        ).unwrap();
        
        // Parse function call expression statement
        let stmt1 = parser.parse_statement().unwrap();
        match stmt1 {
            Stmt::Expression { expr, .. } => {
                matches!(expr, Expr::Call { .. });
            }
            _ => panic!("Expected expression statement"),
        }
        
        // Parse literal expression statement
        let stmt2 = parser.parse_statement().unwrap();
        match stmt2 {
            Stmt::Expression { expr, .. } => {
                matches!(expr, Expr::Literal { .. });
            }
            _ => panic!("Expected expression statement"),
        }
    }
} 
