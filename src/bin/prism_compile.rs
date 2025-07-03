//! Prism Compiler CLI Driver
//!
//! This is a complete demonstration of the Prism compiler pipeline:
//! Source Code → Lexing → Parsing → Semantic Analysis → Code Generation → C Compilation

use prism::{
    Lexer, Parser,
    semantic::{SemanticAnalyzer, SymbolTable},
    codegen::{
        CCodeGenerator, CodegenResult, CodegenError,
        build::{BuildSystem, BuildConfigBuilder, CCompiler}
    },
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

/// Command line arguments
#[derive(Debug)]
struct Args {
    /// Input Prism file
    input_file: PathBuf,
    /// Output directory
    output_dir: PathBuf,
    /// Optimization level
    optimization: u8,
    /// Enable debug info
    debug: bool,
    /// Enable verbose output
    verbose: bool,
    /// Only generate C code (don't compile)
    c_only: bool,
    /// Compiler to use
    compiler: Option<CCompiler>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();
        
        if args.len() < 2 {
            return Err("Usage: prism_compile <input.prism> [options]".to_string());
        }
        
        let mut parsed = Args {
            input_file: PathBuf::from(&args[1]),
            output_dir: PathBuf::from("target"),
            optimization: 0,
            debug: true,
            verbose: false,
            c_only: false,
            compiler: None,
        };
        
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "-o" | "--output" => {
                    if i + 1 >= args.len() {
                        return Err("Missing output directory".to_string());
                    }
                    parsed.output_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                },
                "-O" => {
                    if i + 1 >= args.len() {
                        return Err("Missing optimization level".to_string());
                    }
                    parsed.optimization = args[i + 1].parse()
                        .map_err(|_| "Invalid optimization level")?;
                    i += 2;
                },
                "--debug" => {
                    parsed.debug = true;
                    i += 1;
                },
                "--no-debug" => {
                    parsed.debug = false;
                    i += 1;
                },
                "--verbose" | "-v" => {
                    parsed.verbose = true;
                    i += 1;
                },
                "--c-only" => {
                    parsed.c_only = true;
                    i += 1;
                },
                "--gcc" => {
                    parsed.compiler = Some(CCompiler::Gcc);
                    i += 1;
                },
                "--clang" => {
                    parsed.compiler = Some(CCompiler::Clang);
                    i += 1;
                },
                "--msvc" => {
                    parsed.compiler = Some(CCompiler::Msvc);
                    i += 1;
                },
                _ => {
                    return Err(format!("Unknown option: {}", args[i]));
                }
            }
        }
        
        Ok(parsed)
    }
}

fn main() {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            print_usage();
            process::exit(1);
        }
    };
    
    if let Err(e) = compile_prism_file(&args) {
        eprintln!("Compilation failed: {}", e);
        process::exit(1);
    }
}

fn print_usage() {
    println!("Prism Compiler - Phase 5 Code Generation Demo");
    println!();
    println!("USAGE:");
    println!("    prism_compile <input.prism> [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -o, --output <DIR>     Output directory [default: target]");
    println!("    -O <LEVEL>             Optimization level (0-3) [default: 0]");
    println!("    --debug                Enable debug information [default]");
    println!("    --no-debug             Disable debug information");
    println!("    -v, --verbose          Enable verbose output");
    println!("    --c-only               Only generate C code, don't compile");
    println!("    --gcc                  Use GCC compiler");
    println!("    --clang                Use Clang compiler");
    println!("    --msvc                 Use MSVC compiler");
    println!();
    println!("EXAMPLES:");
    println!("    prism_compile hello.prism");
    println!("    prism_compile hello.prism -O 2 --clang");
    println!("    prism_compile hello.prism --c-only -v");
}

fn compile_prism_file(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    if args.verbose {
        println!("🚀 Prism Compiler - Phase 5 Code Generation");
        println!("Input file: {}", args.input_file.display());
        println!("Output directory: {}", args.output_dir.display());
        println!();
    }
    
    // Phase 1: Read source file
    if args.verbose {
        println!("📖 Phase 1: Reading source file...");
    }
    
    let source_code = fs::read_to_string(&args.input_file)
        .map_err(|e| format!("Failed to read input file: {}", e))?;
    
    if args.verbose {
        println!("   Source code: {} bytes", source_code.len());
    }
    
    // Phase 2: Syntactic Analysis (includes lexing)
    if args.verbose {
        println!("🔤🌳 Phase 2: Syntactic analysis (includes lexing)...");
    }
    
    let parser_start = Instant::now();
    let mut parser = Parser::new(&source_code, 0)
        .map_err(|e| format!("Parser initialization failed: {}", e))?;
    let ast = parser.parse_module()
        .map_err(|e| format!("Parsing failed: {}", e))?;
    
    if args.verbose {
        println!("   AST nodes: {} items", ast.items.len());
        println!("   Parsing time: {:?}", parser_start.elapsed());
    }
    
    // Phase 3: Semantic Analysis
    if args.verbose {
        println!("🧠 Phase 3: Semantic analysis...");
    }
    
    let semantic_start = Instant::now();
    let mut analyzer = SemanticAnalyzer::new();
    let analysis_result = analyzer.analyze(&ast);
    
    if !analysis_result.errors.is_empty() {
        for error in &analysis_result.errors {
            eprintln!("Semantic error: {:?}", error);
        }
        return Err("Semantic analysis failed".into());
    }
    
    if args.verbose {
        println!("   Symbols analyzed: {}", analysis_result.stats.symbols_analyzed);
        println!("   Semantic analysis time: {:?}", semantic_start.elapsed());
    }
    
    // Phase 4: Code Generation
    if args.verbose {
        println!("⚙️  Phase 4: C code generation...");
    }
    
    let codegen_start = Instant::now();
    let mut generator = CCodeGenerator::new(analysis_result.symbol_table);
    let (header, implementation) = generator.generate_module(&ast)
        .map_err(|e| format!("Code generation failed: {}", e))?;
    
    if args.verbose {
        let metrics = generator.metrics();
        println!("   Nodes processed: {}", metrics.nodes_processed);
        println!("   Lines generated: {}", metrics.lines_generated);
        println!("   Generation speed: {:.0} nodes/sec", metrics.nodes_per_second());
        println!("   Code generation time: {:?}", codegen_start.elapsed());
    }
    
    // Create output directory
    fs::create_dir_all(&args.output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;
    
    // Write generated C files
    let input_stem = args.input_file.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    
    let header_path = args.output_dir.join(format!("{}.h", input_stem));
    let impl_path = args.output_dir.join(format!("{}.c", input_stem));
    
    fs::write(&header_path, &header)
        .map_err(|e| format!("Failed to write header file: {}", e))?;
    
    fs::write(&impl_path, &implementation)
        .map_err(|e| format!("Failed to write implementation file: {}", e))?;
    
    if args.verbose {
        println!("   Generated files:");
        println!("     Header: {}", header_path.display());
        println!("     Implementation: {}", impl_path.display());
    }
    
    // Generate runtime files
    if args.verbose {
        println!("🏗️  Generating runtime system...");
    }
    
    let mut runtime_gen = prism::codegen::runtime::RuntimeGenerator::new();
    runtime_gen.write_runtime_files(&args.output_dir)
        .map_err(|e| format!("Failed to generate runtime: {}", e))?;
    
    if args.verbose {
        println!("   Runtime files generated");
    }
    
    // Phase 6: C Compilation (if requested)
    if !args.c_only {
        if args.verbose {
            println!("🔨 Phase 6: C compilation...");
        }
        
        let compile_start = Instant::now();
        
        // Detect or use specified compiler
        let compiler = if let Some(compiler) = args.compiler {
            compiler
        } else {
            BuildSystem::detect_compiler()
                .map_err(|e| format!("Compiler detection failed: {}", e))?
        };
        
        if args.verbose {
            println!("   Using compiler: {:?}", compiler);
        }
        
        // Configure build system
        let config = BuildConfigBuilder::new()
            .compiler(compiler)
            .optimization_level(args.optimization)
            .debug_info(args.debug)
            .output_dir(args.output_dir.clone())
            .include_dir(args.output_dir.clone())
            .build();
        
        let build_system = BuildSystem::with_config(config);
        
        // Compile to executable
        let source_files = vec![
            impl_path,
            args.output_dir.join("prism_runtime.c"),
        ];
        
        let executable_name = if cfg!(windows) {
            format!("{}.exe", input_stem)
        } else {
            input_stem.to_string()
        };
        
        let executable_path = build_system.build_executable(&source_files, &executable_name)
            .map_err(|e| format!("C compilation failed: {}", e))?;
        
        if args.verbose {
            println!("   Executable: {}", executable_path.display());
            println!("   C compilation time: {:?}", compile_start.elapsed());
        }
    }
    
    // Final summary
    let total_time = start_time.elapsed();
    
    if args.verbose {
        println!();
        println!("✅ Compilation completed successfully!");
        println!("   Total time: {:?}", total_time);
        
        if !args.c_only {
            println!();
            println!("Run your program:");
            let executable_name = if cfg!(windows) {
                format!("{}.exe", input_stem)
            } else {
                input_stem.to_string()
            };
            println!("   ./{}/{}", args.output_dir.display(), executable_name);
        }
    } else {
        println!("Compilation successful ({:?})", total_time);
    }
    
    Ok(())
}

// Example Prism program for testing
const EXAMPLE_PRISM_CODE: &str = r#"
// Example Prism program demonstrating the compiler

fn main() -> i32 {
    let message = "Hello, Prism!";
    print_string(message);
    
    let x = 42;
    let y = calculate_square(x);
    
    print_int(y);
    return 0;
}

fn calculate_square(n: i32) -> i32 {
    return n * n;
}

fn print_string(s: str) {
    // This would be implemented in the runtime
}

fn print_int(n: i32) {
    // This would be implemented in the runtime
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_args_parsing() {
        // Test basic parsing
        env::set_var("0", "prism_compile");
        env::set_var("1", "test.prism");
        
        // This test would need more work to be fully functional
        // as env::args() gets the actual program arguments
    }
    
    #[test]
    fn test_example_prism_code() {
        // Test that our example code is valid
        assert!(!EXAMPLE_PRISM_CODE.is_empty());
        assert!(EXAMPLE_PRISM_CODE.contains("fn main()"));
        assert!(EXAMPLE_PRISM_CODE.contains("return"));
    }
    
    #[test]
    fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.prism");
        
        // Write test file
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "{}", EXAMPLE_PRISM_CODE).unwrap();
        
        // Read it back
        let content = fs::read_to_string(&test_file).unwrap();
        assert!(content.contains("fn main()"));
    }
} 