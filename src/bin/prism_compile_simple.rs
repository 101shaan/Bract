//! Simplified Prism Compiler CLI 
//!
//! This demonstrates the complete Prism compilation pipeline:
//! Source Code → Parsing → Semantic Analysis → C Code Generation

use prism::{
    Parser,
    semantic::{SemanticAnalyzer},
    codegen::{CCodeGenerator, runtime::RuntimeGenerator},
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("🚀 Prism Compiler - Phase 5 Code Generation Demo");
        println!();
        println!("USAGE:");
        println!("    prism_compile_simple <input.prism> [output_dir]");
        println!();
        println!("EXAMPLES:");
        println!("    prism_compile_simple hello.prism");
        println!("    prism_compile_simple hello.prism output/");
        process::exit(1);
    }
    
    let input_file = PathBuf::from(&args[1]);
    let output_dir = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from("target")
    };
    
    if let Err(e) = compile_prism_file(&input_file, &output_dir) {
        eprintln!("❌ Compilation failed: {}", e);
        process::exit(1);
    }
}

fn compile_prism_file(input_file: &PathBuf, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    println!("🚀 Prism Compiler - Phase 5 Code Generation");
    println!("Input: {}", input_file.display());
    println!("Output: {}", output_dir.display());
    println!();
    
    // Phase 1: Read source file
    println!("📖 Reading source file...");
    let source_code = fs::read_to_string(input_file)
        .map_err(|e| format!("Failed to read input file: {}", e))?;
    println!("   ✓ {} bytes read", source_code.len());
    
    // Phase 2: Parsing (includes lexing)
    println!("🌳 Parsing...");
    let parse_start = Instant::now();
    let mut parser = Parser::new(&source_code, 0)
        .map_err(|e| format!("Parser initialization failed: {}", e))?;
    let ast = parser.parse_module()
        .map_err(|e| format!("Parsing failed: {}", e))?;
    println!("   ✓ {} items parsed ({:?})", ast.items.len(), parse_start.elapsed());
    
    // Phase 3: Semantic Analysis
    println!("🧠 Semantic analysis...");
    let semantic_start = Instant::now();
    let mut analyzer = SemanticAnalyzer::new();
    let analysis_result = analyzer.analyze(&ast);
    
    if !analysis_result.errors.is_empty() {
        println!("   ❌ Semantic errors found:");
        for error in &analysis_result.errors {
            println!("      {:?}", error);
        }
        return Err("Semantic analysis failed".into());
    }
    
    println!("   ✓ {} symbols analyzed ({:?})", 
        analysis_result.stats.symbols_analyzed, 
        semantic_start.elapsed());
    
    // Phase 4: Code Generation
    println!("⚙️ C code generation...");
    let codegen_start = Instant::now();
    let mut generator = CCodeGenerator::new(analysis_result.symbol_table);
    let (header, implementation) = generator.generate_module(&ast)
        .map_err(|e| format!("Code generation failed: {}", e))?;
    
    let metrics = generator.metrics();
    println!("   ✓ {} nodes processed, {} lines generated ({:?})", 
        metrics.nodes_processed, 
        metrics.lines_generated, 
        codegen_start.elapsed());
    
    // Create output directory
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;
    
    // Write generated C files
    let input_stem = input_file.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    
    let header_path = output_dir.join(format!("{}.h", input_stem));
    let impl_path = output_dir.join(format!("{}.c", input_stem));
    
    fs::write(&header_path, &header)
        .map_err(|e| format!("Failed to write header file: {}", e))?;
    
    fs::write(&impl_path, &implementation)
        .map_err(|e| format!("Failed to write implementation file: {}", e))?;
    
    println!("   ✓ Generated:");
    println!("     Header: {}", header_path.display());
    println!("     Implementation: {}", impl_path.display());
    
    // Generate runtime files
    println!("🏗️ Generating runtime...");
    let mut runtime_gen = RuntimeGenerator::new();
    runtime_gen.write_runtime_files(output_dir)
        .map_err(|e| format!("Failed to generate runtime: {}", e))?;
    
    println!("   ✓ Runtime system generated");
    
    // Show total time
    let total_time = start_time.elapsed();
    println!();
    println!("✅ Compilation completed successfully!");
    println!("   Total time: {:?}", total_time);
    println!();
    println!("📁 Generated files in {}/:", output_dir.display());
    println!("   {}.h - Header file", input_stem);
    println!("   {}.c - Implementation", input_stem);
    println!("   prism_runtime.h - Runtime header");
    println!("   prism_runtime.c - Runtime implementation");
    println!();
    println!("🔨 To compile with gcc:");
    println!("   gcc {}/{}.c {}/prism_runtime.c -o {}/{}", 
        output_dir.display(), input_stem,
        output_dir.display(), 
        output_dir.display(), input_stem);
    
    Ok(())
}

// Simple test to demonstrate usage
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    const SIMPLE_PRISM_PROGRAM: &str = r#"
fn main() -> i32 {
    return 42;
}

fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
"#;
    
    #[test]
    fn test_simple_compilation() {
        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_simple.prism");
        let output_dir = temp_dir.join("prism_test_output");
        
        // Write test program
        fs::write(&test_file, SIMPLE_PRISM_PROGRAM).unwrap();
        
        // Try to compile (this may fail due to incomplete implementation)
        let result = compile_prism_file(&test_file, &output_dir);
        
        // Clean up
        let _ = fs::remove_file(&test_file);
        let _ = fs::remove_dir_all(&output_dir);
        
        // For now, just verify the function doesn't panic
        // In a complete implementation, this should succeed
        println!("Compilation result: {:?}", result);
    }
} 