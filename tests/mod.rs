//! Integration tests for the Bract compiler
//! 
//! This module contains comprehensive end-to-end tests that validate
//! the entire compiler pipeline from source code to native code (Cranelift).

pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod integration;
pub mod examples;

/// Common test utilities and helpers
pub mod common {
    use bract::{Parser, semantic::SemanticAnalyzer, codegen::cranelift::CraneliftCodeGenerator};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    
    /// Test helper to create a temporary directory
    pub fn create_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp directory")
    }
    
    /// Test helper to write a test file
    pub fn write_test_file(dir: &Path, filename: &str, content: &str) -> std::path::PathBuf {
        let file_path = dir.join(filename);
        fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }
    
    /// Test helper to run the complete compilation pipeline to object bytes
    pub fn compile_bract_source(source: &str) -> Result<Vec<u8>, String> {
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
        
        // Code generation (Cranelift -> object code bytes)
        let interner = parser.take_interner();
        let mut generator = CraneliftCodeGenerator::new(analysis_result.symbol_table, interner)
            .map_err(|e| format!("Failed to create code generator: {:?}", e))?;
        let object_bytes = generator.generate(&ast)
            .map_err(|e| format!("Code generation failed: {:?}", e))?;
        Ok(object_bytes)
    }
    
    /// Basic sanity check for produced object code
    pub fn validate_object_bytes(obj: &[u8]) {
        assert!(!obj.is_empty(), "object bytes should not be empty");
    }
    
    /// Test helper to count lines in generated code
    pub fn count_lines(code: &str) -> usize {
        code.lines().count()
    }
} 
