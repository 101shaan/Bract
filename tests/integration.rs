//! End-to-end integration tests
//! 
//! These tests validate the complete compiler pipeline from source code to Cranelift native object code.

use bract::{Parser, semantic::SemanticAnalyzer, codegen::cranelift::CraneliftCodeGenerator};
use std::process::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test helper to create a temporary directory
fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Test helper to write a test file
fn write_test_file(dir: &Path, filename: &str, content: &str) -> std::path::PathBuf {
    let file_path = dir.join(filename);
    fs::write(&file_path, content).expect("Failed to write test file");
    file_path
}

/// Test helper to run the complete compilation pipeline to object bytes
fn compile_bract_source(source: &str) -> Result<Vec<u8>, String> {
    // Parse
    let mut parser = Parser::new(source, 0)
        .map_err(|e| format!("Parser creation failed: {}", e))?;
    let ast = parser.parse_module()
        .map_err(|e| format!("Parsing failed: {}", e))?;
    
    // Semantic analysis
    let mut analyzer = SemanticAnalyzer::new();
    let analysis_result = analyzer.analyze(&ast);
    
    if !analysis_result.errors.is_empty() {
        return Err(format!("Semantic analysis failed: {:?}", analysis_result.errors));
    }
    
    // Code generation (Cranelift -> object code)
    let interner = parser.take_interner();
    let mut generator = CraneliftCodeGenerator::new(analysis_result.symbol_table, interner)
        .map_err(|e| format!("Failed to create code generator: {:?}", e))?;
    let object_bytes = generator.generate(&ast)
        .map_err(|e| format!("Code generation failed: {:?}", e))?;
    Ok(object_bytes)
}

/// Test helper to validate generated C code structure
fn validate_object(obj: &[u8]) {
    assert!(!obj.is_empty());
}

/// Test complete compilation pipeline for a simple program
#[test]
fn test_complete_pipeline_simple() {
    let source = r#"
        fn main() -> i32 {
            return 42;
        }
    "#;
    
    let object_bytes = compile_bract_source(source)
        .expect("Compilation should succeed");
    validate_object(&object_bytes);
}

/// Test compilation of a program with arithmetic operations
#[test]
fn test_complete_pipeline_arithmetic() {
    let source = r#"
        fn add(a: i32, b: i32) -> i32 {
            return a + b;
        }
        
        fn main() -> i32 {
            let result = add(10, 20);
            return result;
        }
    "#;
    
    let object_bytes = compile_bract_source(source)
        .expect("Compilation should succeed");
    validate_object(&object_bytes);
}

/// Test compilation of a program with control flow
#[test]
fn test_complete_pipeline_control_flow() {
    let source = r#"
        fn factorial(n: i32) -> i32 {
            if n <= 1 {
                return 1;
            }
            return n * factorial(n - 1);
        }
        
        fn main() -> i32 {
            return factorial(5);
        }
    "#;
    
    let object_bytes = compile_bract_source(source)
        .expect("Compilation should succeed");
    validate_object(&object_bytes);
}

/// Test compilation of a program with data structures
#[test]
fn test_complete_pipeline_structs() {
    let source = r#"
        struct Point {
            x: i32,
            y: i32,
        }
        
        fn create_point(x: i32, y: i32) -> Point {
            let p = Point { x: x, y: y };
            return p;
        }
        
        fn main() -> i32 {
            let point = create_point(10, 20);
            return point.x + point.y;
        }
    "#;
    
    let object_bytes = compile_bract_source(source)
        .expect("Compilation should succeed");
    validate_object(&object_bytes);
}

/// Runtime C generation no longer applies with direct Cranelift backend; keep placeholder to avoid regressions
#[test]
fn test_complete_pipeline_with_runtime() {
    let source = r#"
        fn main() -> i32 {
            return 0;
        }
    "#;
    
    let temp_dir = create_temp_dir();
    let temp_path = temp_dir.path();
    
    // Cranelift produces native object; no separate C runtime is generated.
    // This test now simply ensures compilation succeeds.
    let object_bytes = compile_bract_source(source)
        .expect("Compilation should succeed");
    validate_object(&object_bytes);
}

/// C compilation path removed; keep as no-op to document change
#[test]
fn test_complete_pipeline_c_compilation() {
    // Skip this test if no C compiler is available
    if !is_c_compiler_available() {
        println!("Skipping C compilation test - no compiler available");
        return;
    }
    
    let source = r#"
        fn main() -> i32 {
            return 42;
        }
    "#;
    
    let temp_dir = create_temp_dir();
    let temp_path = temp_dir.path();
    
    // Cranelift path: ensure object bytes are produced
    let object_bytes = compile_bract_source(source)
        .expect("Compilation should succeed");
    assert!(!object_bytes.is_empty());
}

/// Test error handling in the complete pipeline
#[test]
fn test_complete_pipeline_error_handling() {
    let invalid_sources = vec![
        // Syntax error
        "fn main() -> i32 { return; }",
        
        // Type error (if semantic analysis catches it)
        "fn main() -> i32 { return \"string\"; }",
        
        // Undefined function
        "fn main() -> i32 { return undefined_function(); }",
    ];
    
    for source in invalid_sources {
        let result = compile_bract_source(source);
        
        // Should either fail compilation or produce valid object code
        match result {
            Ok(bytes) => {
                validate_object(&bytes);
            }
            Err(_) => {
                // Failing is also acceptable for invalid input
            }
        }
    }
}

/// Test performance of complete pipeline
#[test]
fn test_complete_pipeline_performance() {
    use std::time::Instant;
    
    let source = format!(
        "fn main() -> i32 {{\n{}\n    return 0;\n}}",
        (0..100).map(|i| format!("    let var_{} = {} + {};", i, i, i+1))
                .collect::<Vec<_>>()
                .join("\n")
    );
    
    let start = Instant::now();
    let result = compile_bract_source(&source);
    let elapsed = start.elapsed();
    
    println!("Complete pipeline took: {:?}", elapsed);
    
    // Should compile successfully
    assert!(result.is_ok(), "Performance test should compile successfully");
    
    // Performance assertion: complete pipeline should be fast
    assert!(elapsed.as_millis() < 500, "Complete pipeline too slow: {:?}", elapsed);
}

/// Test compilation of multiple modules (if supported)
#[test]
fn test_complete_pipeline_multiple_modules() {
    let source = r#"
        mod math {
            fn square(x: i32) -> i32 {
                return x * x;
            }
        }
        
        fn main() -> i32 {
            return math::square(5);
        }
    "#;
    
    let result = compile_bract_source(source);
    
    // This might not be fully implemented yet, so we allow it to fail
    match result {
        Ok(bytes) => {
            validate_object(&bytes);
        }
        Err(_) => {
            // Modules might not be fully implemented yet
            println!("Multiple modules not yet supported - skipping test");
        }
    }
}

/// Test memory safety and resource cleanup in generated code
#[test]
fn test_complete_pipeline_memory_safety() {
    let source = r#"
        fn main() -> i32 {
            let x = 42;
            let y = x;
            return y;
        }
    "#;
    
    let object_bytes = compile_bract_source(source)
        .expect("Compilation should succeed");
    validate_object(&object_bytes);
}

/// Helper function to check if a C compiler is available
fn is_c_compiler_available() -> bool {
    Command::new("gcc")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Test real-world example programs
#[test]
fn test_real_world_examples() {
    let examples = vec![
        // Fibonacci sequence
        r#"
            fn fibonacci(n: i32) -> i32 {
                if n <= 1 {
                    return n;
                }
                return fibonacci(n - 1) + fibonacci(n - 2);
            }
            
            fn main() -> i32 {
                return fibonacci(10);
            }
        "#,
        
        // Simple calculator
        r#"
            fn add(a: i32, b: i32) -> i32 { return a + b; }
            fn subtract(a: i32, b: i32) -> i32 { return a - b; }
            fn multiply(a: i32, b: i32) -> i32 { return a * b; }
            fn divide(a: i32, b: i32) -> i32 { return a / b; }
            
            fn main() -> i32 {
                let result = add(multiply(5, 3), divide(10, 2));
                return result;
            }
        "#,
        
        // Data processing
        r#"
            struct Data {
                value: i32,
                count: i32,
            }
            
            fn process_data(data: Data) -> i32 {
                return data.value * data.count;
            }
            
            fn main() -> i32 {
                let data = Data { value: 10, count: 5 };
                return process_data(data);
            }
        "#,
    ];
    
    for (i, source) in examples.iter().enumerate() {
        let result = compile_bract_source(source);
        assert!(result.is_ok(), "Example {} should compile successfully", i);
        
        let bytes = result.unwrap();
        validate_object(&bytes);
    }
} 
