//! Parser integration tests
//! 
//! These tests validate the parser's ability to generate ASTs from real Prism code
//! and handle complex parsing scenarios beyond unit tests.

use prism::{Parser, ast::*};

/// Test parsing a complete Prism program
#[test]
fn test_parse_complete_program() {
    let source = r#"
        fn factorial(n: i32) -> i32 {
            if n <= 1 {
                return 1;
            }
            return n * factorial(n - 1);
        }
        
        fn main() -> i32 {
            let result = factorial(5);
            return result;
        }
    "#;
    
    let mut parser = Parser::new(source, 0).expect("Failed to create parser");
    let module = parser.parse_module().expect("Failed to parse module");
    
    // Verify we got the expected structure
    assert_eq!(module.items.len(), 2); // factorial and main functions
    
    // Check the first function (factorial)
    if let Item::Function { name, params, return_type, body, .. } = &module.items[0] {
        assert_eq!(name.id, 1); // Assuming interned string IDs start at 1
        assert_eq!(params.len(), 1);
        assert!(return_type.is_some());
        assert!(body.is_some());
        
        // Check function body structure
        if let Some(Expr::Block { statements, .. }) = body {
            assert!(!statements.is_empty());
        }
    } else {
        panic!("Expected function item");
    }
    
    // Check the second function (main)
            if let Item::Function { name: _, params, return_type, body, .. } = &module.items[1] {
        assert_eq!(params.len(), 0);
        assert!(return_type.is_some());
        assert!(body.is_some());
    } else {
        panic!("Expected function item");
    }
}

/// Test parsing complex expressions
#[test]
fn test_parse_complex_expressions() {
    let source = r#"
        fn test_expressions() -> i32 {
            let arithmetic = (a + b) * (c - d) / (e % f);
            let logical = (x && y) || (z != w);
            let comparison = (a < b) && (c >= d) && (e == f);
            let function_call = some_function(arg1, arg2, arg3);
            let method_call = object.method(param);
            let array_access = array[index];
            let field_access = struct_instance.field;
            return 0;
        }
    "#;
    
    let mut parser = Parser::new(source, 0).expect("Failed to create parser");
    let module = parser.parse_module().expect("Failed to parse module");
    
    assert_eq!(module.items.len(), 1);
    
    if let Item::Function { body: Some(Expr::Block { statements, .. }), .. } = &module.items[0] {
        // Should have multiple let statements
        assert!(statements.len() >= 7);
        
        // Verify we can parse all these complex expressions without errors
        for stmt in statements {
            if let Stmt::Let { initializer: Some(expr), .. } = stmt {
                // Just verify the expression exists and has a valid span
                assert!(expr.span().start.offset < expr.span().end.offset || 
                       expr.span().start == expr.span().end);
            }
        }
    } else {
        panic!("Expected function with block body");
    }
}

/// Test parsing struct definitions
#[test]
fn test_parse_struct_definitions() {
    let source = r#"
        struct Point {
            x: f64,
            y: f64,
        }
        
        struct Color(u8, u8, u8);
        
        struct Unit;
        
        struct Generic<T> {
            value: T,
        }
    "#;
    
    let mut parser = Parser::new(source, 0).expect("Failed to create parser");
    let module = parser.parse_module().expect("Failed to parse module");
    
    assert_eq!(module.items.len(), 4);
    
    // Check Point struct (named fields)
    if let Item::Struct { name: _, fields: StructFields::Named(fields), .. } = &module.items[0] {
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name.id, 2); // x field
        assert_eq!(fields[1].name.id, 3); // y field
    } else {
        panic!("Expected named struct");
    }
    
    // Check Color struct (tuple fields)
    if let Item::Struct { fields: StructFields::Tuple(types), .. } = &module.items[1] {
        assert_eq!(types.len(), 3);
    } else {
        panic!("Expected tuple struct");
    }
    
    // Check Unit struct
    if let Item::Struct { fields: StructFields::Unit, .. } = &module.items[2] {
        // Unit struct has no fields
    } else {
        panic!("Expected unit struct");
    }
    
    // Check Generic struct
    if let Item::Struct { generics, .. } = &module.items[3] {
        assert_eq!(generics.len(), 1);
    } else {
        panic!("Expected generic struct");
    }
}

/// Test parsing enum definitions
#[test]
fn test_parse_enum_definitions() {
    let source = r#"
        enum Option<T> {
            Some(T),
            None,
        }
        
        enum Result<T, E> {
            Ok(T),
            Err(E),
        }
        
        enum Message {
            Quit,
            Move { x: i32, y: i32 },
            Write(String),
            ChangeColor(i32, i32, i32),
        }
    "#;
    
    let mut parser = Parser::new(source, 0).expect("Failed to create parser");
    let module = parser.parse_module().expect("Failed to parse module");
    
    assert_eq!(module.items.len(), 3);
    
    // Check Option enum
    if let Item::Enum { variants, generics, .. } = &module.items[0] {
        assert_eq!(variants.len(), 2);
        assert_eq!(generics.len(), 1);
        
        // Some variant should have tuple fields
        match &variants[0].fields {
            StructFields::Tuple(types) => assert_eq!(types.len(), 1),
            _ => panic!("Expected tuple variant"),
        }
        
        // None variant should be unit
        match &variants[1].fields {
            StructFields::Unit => {},
            _ => panic!("Expected unit variant"),
        }
    } else {
        panic!("Expected enum");
    }
    
    // Check Message enum with mixed variants
    if let Item::Enum { variants, .. } = &module.items[2] {
        assert_eq!(variants.len(), 4);
        
        // Quit is unit
        match &variants[0].fields {
            StructFields::Unit => {},
            _ => panic!("Expected unit variant"),
        }
        
        // Move has named fields
        match &variants[1].fields {
            StructFields::Named(fields) => assert_eq!(fields.len(), 2),
            _ => panic!("Expected named variant"),
        }
        
        // Write and ChangeColor have tuple fields
        match &variants[2].fields {
            StructFields::Tuple(types) => assert_eq!(types.len(), 1),
            _ => panic!("Expected tuple variant"),
        }
        
        match &variants[3].fields {
            StructFields::Tuple(types) => assert_eq!(types.len(), 3),
            _ => panic!("Expected tuple variant"),
        }
    } else {
        panic!("Expected enum");
    }
}

/// Test parsing control flow statements
#[test]
fn test_parse_control_flow() {
    let source = r#"
        fn control_flow_test(x: i32) -> i32 {
            if x > 0 {
                return x;
            } else if x < 0 {
                return -x;
            } else {
                return 0;
            }
            
            let mut i = 0;
            while i < 10 {
                i = i + 1;
            }
            
            for item in collection {
                process(item);
            }
            
            loop {
                if condition {
                    break;
                }
                continue;
            }
            
            match value {
                1 => first_case(),
                2 | 3 => second_case(),
                _ => default_case(),
            }
        }
    "#;
    
    let mut parser = Parser::new(source, 0).expect("Failed to create parser");
    let module = parser.parse_module().expect("Failed to parse module");
    
    assert_eq!(module.items.len(), 1);
    
    if let Item::Function { body: Some(Expr::Block { statements, .. }), .. } = &module.items[0] {
        // Should have multiple control flow statements
        assert!(statements.len() >= 5);
        
        // Find the if statement
        let has_if = statements.iter().any(|stmt| {
            matches!(stmt, Stmt::If { .. })
        });
        assert!(has_if, "Should have if statement");
        
        // Find the while statement
        let has_while = statements.iter().any(|stmt| {
            matches!(stmt, Stmt::While { .. })
        });
        assert!(has_while, "Should have while statement");
        
        // Find the for statement
        let has_for = statements.iter().any(|stmt| {
            matches!(stmt, Stmt::For { .. })
        });
        assert!(has_for, "Should have for statement");
        
        // Find the loop statement
        let has_loop = statements.iter().any(|stmt| {
            matches!(stmt, Stmt::Loop { .. })
        });
        assert!(has_loop, "Should have loop statement");
        
        // Find the match statement
        let has_match = statements.iter().any(|stmt| {
            matches!(stmt, Stmt::Match { .. })
        });
        assert!(has_match, "Should have match statement");
    } else {
        panic!("Expected function with block body");
    }
}

/// Test parsing pattern matching
#[test]
fn test_parse_pattern_matching() {
    let source = r#"
        fn pattern_test(value: Option<i32>) -> i32 {
            match value {
                Some(x) => x,
                None => 0,
            }
            
            let (a, b) = tuple_value;
            let Point { x, y } = point;
            let [first, second, rest @ ..] = array;
            
            match complex_value {
                Pattern::Variant { field1, field2 } => process(field1, field2),
                Pattern::Other(x) if x > 0 => positive_case(x),
                Pattern::Other(_) => default_case(),
                _ => unreachable(),
            }
        }
    "#;
    
    let mut parser = Parser::new(source, 0).expect("Failed to create parser");
    let module = parser.parse_module().expect("Failed to parse module");
    
    assert_eq!(module.items.len(), 1);
    
    if let Item::Function { body: Some(Expr::Block { statements, .. }), .. } = &module.items[0] {
        // Should have multiple pattern matching constructs
        assert!(!statements.is_empty());
        
        // Find let statements with destructuring patterns
        let destructuring_lets = statements.iter().filter(|stmt| {
            matches!(stmt, Stmt::Let { pattern: Pattern::Tuple { .. }, .. } |
                           Stmt::Let { pattern: Pattern::Struct { .. }, .. } |
                           Stmt::Let { pattern: Pattern::Array { .. }, .. })
        }).count();
        
        assert!(destructuring_lets >= 3, "Should have destructuring let statements");
        
        // Find match statements
        let match_count = statements.iter().filter(|stmt| {
            matches!(stmt, Stmt::Match { .. })
        }).count();
        
        assert!(match_count >= 2, "Should have match statements");
    } else {
        panic!("Expected function with block body");
    }
}

/// Test parser performance with large input
#[test]
fn test_parse_performance() {
    use std::time::Instant;
    
    // Generate a large Prism program
    let mut source = String::new();
    source.push_str("fn main() -> i32 {\n");
    
    // Add many statements
    for i in 0..1000 {
        source.push_str(&format!("    let var_{} = {} + {} * {};\n", i, i, i+1, i+2));
    }
    
    source.push_str("    return 0;\n}\n");
    
    let start = Instant::now();
    let mut parser = Parser::new(&source, 0).expect("Failed to create parser");
    let module = parser.parse_module().expect("Failed to parse module");
    let elapsed = start.elapsed();
    
    println!("Parsed large program in {:?}", elapsed);
    
    // Verify the parsed structure
    assert_eq!(module.items.len(), 1);
    if let Item::Function { body: Some(Expr::Block { statements, .. }), .. } = &module.items[0] {
        assert!(statements.len() >= 1000);
    }
    
    // Performance assertion: should be able to parse quickly
    // This validates the "blazingly fast" claim for parsing
    assert!(elapsed.as_millis() < 100, "Parser performance too slow: {:?}", elapsed);
}

/// Test parser error recovery
#[test]
fn test_parse_error_recovery() {
    let invalid_sources = vec![
        "fn incomplete_function(",
        "struct MissingBrace {",
        "let x = ;", // Missing expression
        "fn func() { return }", // Missing semicolon
        "if condition", // Missing body
    ];
    
    for source in invalid_sources {
        let mut parser = Parser::new(source, 0).expect("Failed to create parser");
        let result = parser.parse_module();
        
        // Should either fail or recover with errors
        if let Ok(_module) = result {
            let errors = parser.errors();
            assert!(!errors.is_empty(), "Expected errors for invalid source: {}", source);
        }
        // If it fails, that's also acceptable for these invalid inputs
    }
}

/// Test parsing with complex nested structures
#[test]
fn test_parse_nested_structures() {
    let source = r#"
        fn nested_test() -> i32 {
            let complex = if condition {
                match inner_value {
                    Some(x) => {
                        if x > 0 {
                            loop {
                                if break_condition {
                                    break x;
                                }
                                x = x - 1;
                            }
                        } else {
                            0
                        }
                    },
                    None => -1,
                }
            } else {
                42
            };
            
            return complex;
        }
    "#;
    
    let mut parser = Parser::new(source, 0).expect("Failed to create parser");
    let module = parser.parse_module().expect("Failed to parse module");
    
    assert_eq!(module.items.len(), 1);
    
    if let Item::Function { body: Some(Expr::Block { statements, .. }), .. } = &module.items[0] {
        // Should have parsed the complex nested structure
        assert!(!statements.is_empty());
        
        // Find the let statement with the complex nested expression
        let has_complex_let = statements.iter().any(|stmt| {
            if let Stmt::Let { initializer: Some(expr), .. } = stmt {
                // The expression should be deeply nested
                matches!(expr, Expr::If { .. })
            } else {
                false
            }
        });
        
        assert!(has_complex_let, "Should have complex nested let statement");
    } else {
        panic!("Expected function with block body");
    }
} 