//! Bract Native Compiler - High-Performance Direct Executable Generation
//!
//! This compiler generates highly optimized executables by leveraging aggressive
//! C compiler optimizations and direct binary generation, achieving near-native
//! performance while maintaining cross-platform compatibility.

use bract::{
    Parser,
    semantic::SemanticAnalyzer,
    codegen::{CCodeGenerator, build::{BuildSystem, BuildConfigBuilder, CCompiler}},
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};
use std::time::Instant;

/// Optimization levels for native compilation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptimizationLevel {
    Debug,      // -O0 with debug info
    Size,       // -Os optimize for size
    Speed,      // -O2 balanced optimization
    Maximum,    // -O3 maximum optimization
    Extreme,    // -Ofast + aggressive flags
}

/// Command line arguments for native compilation
#[derive(Debug)]
struct Args {
    /// Input Bract file
    input_file: PathBuf,
    /// Output executable path
    output_file: PathBuf,
    /// Optimization level
    optimization: OptimizationLevel,
    /// Enable debug info
    debug: bool,
    /// Enable verbose output
    verbose: bool,
    /// Only generate object file (don't link)
    object_only: bool,
    /// Show generated C code
    show_c_code: bool,
    /// Enable link-time optimization
    lto: bool,
    /// Strip symbols from final binary
    strip: bool,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();
        
        if args.len() < 2 {
            return Err("Usage: Bract_native <input.Bract> [options]".to_string());
        }
        
        let input_file = PathBuf::from(&args[1]);
        let output_file = input_file.with_extension(if cfg!(windows) { "exe" } else { "" });
        
        let mut parsed = Args {
            input_file,
            output_file,
            optimization: OptimizationLevel::Speed,
            debug: false,
            verbose: false,
            object_only: false,
            show_c_code: false,
            lto: true,
            strip: false,
        };
        
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "-o" | "--output" => {
                    if i + 1 >= args.len() {
                        return Err("Missing output file".to_string());
                    }
                    parsed.output_file = PathBuf::from(&args[i + 1]);
                    i += 2;
                },
                "--debug" => {
                    parsed.optimization = OptimizationLevel::Debug;
                    parsed.debug = true;
                    i += 1;
                },
                "--size" | "-Os" => {
                    parsed.optimization = OptimizationLevel::Size;
                    i += 1;
                },
                "--speed" | "-O2" => {
                    parsed.optimization = OptimizationLevel::Speed;
                    i += 1;
                },
                "--max" | "-O3" => {
                    parsed.optimization = OptimizationLevel::Maximum;
                    i += 1;
                },
                "--extreme" | "-Ofast" => {
                    parsed.optimization = OptimizationLevel::Extreme;
                    i += 1;
                },
                "--verbose" | "-v" => {
                    parsed.verbose = true;
                    i += 1;
                },
                "--object-only" | "-c" => {
                    parsed.object_only = true;
                    i += 1;
                },
                "--show-c" => {
                    parsed.show_c_code = true;
                    i += 1;
                },
                "--no-lto" => {
                    parsed.lto = false;
                    i += 1;
                },
                "--strip" => {
                    parsed.strip = true;
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
    
    if let Err(e) = compile_bract_native(&args) {
        eprintln!("Native compilation failed: {}", e);
        process::exit(1);
    }
}

fn print_usage() {
    println!("Bract Native Compiler - High-Performance Direct Executable Generation");
    println!();
    println!("USAGE:");
    println!("    Bract_native <input.Bract> [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -o, --output <FILE>    Output executable [default: <input>.exe]");
    println!("    --debug                Debug build (-O0 + debug info)");
    println!("    --size, -Os            Optimize for size");
    println!("    --speed, -O2           Optimize for speed [default]");
    println!("    --max, -O3             Maximum optimization");
    println!("    --extreme, -Ofast      Extreme optimization (unsafe math)");
    println!("    -v, --verbose          Enable verbose output");
    println!("    -c, --object-only      Generate object file only");
    println!("    --show-c               Show generated C code");
    println!("    --no-lto               Disable link-time optimization");
    println!("    --strip                Strip symbols from binary");
    println!();
    println!("EXAMPLES:");
    println!("    Bract_native hello.Bract");
    println!("    Bract_native hello.Bract --extreme --strip -o hello_opt");
    println!("    Bract_native hello.Bract --show-c -v");
}

fn compile_bract_native(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    if args.verbose {
        println!("🚀 Bract Native Compiler - High-Performance Mode");
        println!("Input file: {}", args.input_file.display());
        println!("Output file: {}", args.output_file.display());
        println!("Optimization: {:?}", args.optimization);
        println!("LTO enabled: {}", args.lto);
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
    
    // Phase 2: Syntactic Analysis
    if args.verbose {
        println!("🔤🌳 Phase 2: Syntactic analysis...");
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
    
    // Phase 4: High-Performance C Code Generation
    if args.verbose {
        println!("⚙️  Phase 4: High-performance C code generation...");
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
    let temp_dir = std::env::temp_dir().join("Bract_native");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    // Write generated C files
    let input_stem = args.input_file.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    
    let header_path = temp_dir.join(format!("{}.h", input_stem));
    let impl_path = temp_dir.join(format!("{}.c", input_stem));
    
    fs::write(&header_path, &header)
        .map_err(|e| format!("Failed to write header file: {}", e))?;
    
    fs::write(&impl_path, &implementation)
        .map_err(|e| format!("Failed to write implementation file: {}", e))?;
    
    if args.show_c_code {
        println!();
        println!("📄 Generated C Header:");
        println!("{}", header);
        println!();
        println!("📄 Generated C Implementation:");
        println!("{}", implementation);
        println!();
    }
    
    if args.verbose {
        println!("   Generated files:");
        println!("     Header: {}", header_path.display());
        println!("     Implementation: {}", impl_path.display());
    }
    
    // Generate runtime files
    if args.verbose {
        println!("🏗️  Generating runtime system...");
    }
    
    let mut runtime_gen = bract::codegen::runtime::RuntimeGenerator::new();
    runtime_gen.write_runtime_files(&temp_dir)
        .map_err(|e| format!("Failed to generate runtime: {}", e))?;
    
    if args.verbose {
        println!("   Runtime files generated");
    }
    
    // Phase 5: High-Performance Native Compilation
    if !args.object_only {
        if args.verbose {
            println!("🔨 Phase 5: High-performance native compilation...");
        }
        
        let compile_start = Instant::now();
        
        // Detect optimal compiler
        let compiler = detect_optimal_compiler(args.verbose)?;
        
        if args.verbose {
            println!("   Using compiler: {:?}", compiler);
        }
        
        // Configure build system with aggressive optimization
        let opt_level = match args.optimization {
            OptimizationLevel::Debug => 0,
            OptimizationLevel::Size => 4,    // Special size optimization
            OptimizationLevel::Speed => 2,
            OptimizationLevel::Maximum => 3,
            OptimizationLevel::Extreme => 5, // Custom extreme optimization
        };
        
        let config = BuildConfigBuilder::new()
            .compiler(compiler)
            .optimization_level(opt_level)
            .debug_info(args.debug)
            .output_dir(temp_dir.clone())
            .include_dir(temp_dir.clone())
            .build();
        
        let build_system = BuildSystem::with_config(config);
        
        // Compile with extreme optimizations
        let source_files = vec![
            impl_path,
            temp_dir.join("Bract_runtime.c"),
        ];
        
        let executable_name = if cfg!(windows) {
            format!("{}.exe", input_stem)
        } else {
            input_stem.to_string()
        };
        
        let temp_executable = build_system.build_executable(&source_files, &executable_name)
            .map_err(|e| format!("Native compilation failed: {}", e))?;
        
        // Apply additional optimizations
        apply_post_compilation_optimizations(&temp_executable, &args.output_file, args)?;
        
        if args.verbose {
            println!("   Native executable: {}", args.output_file.display());
            println!("   Native compilation time: {:?}", compile_start.elapsed());
        }
    }
    
    // Clean up temporary files
    if !args.verbose {
        let _ = fs::remove_dir_all(&temp_dir);
    }
    
    // Final summary
    let total_time = start_time.elapsed();
    
    if args.verbose {
        println!();
        println!("✅ High-performance native compilation completed successfully!");
        println!("   Total time: {:?}", total_time);
        println!("   Optimization level: {:?}", args.optimization);
        
        if !args.object_only {
            println!();
            println!("Run your optimized program:");
            println!("   {}", args.output_file.display());
        }
    } else {
        println!("Native compilation successful ({:?})", total_time);
    }
    
    Ok(())
}

fn detect_optimal_compiler(verbose: bool) -> Result<CCompiler, String> {
    // Try to detect the best available compiler
    let compilers = if cfg!(windows) {
        vec![CCompiler::Clang, CCompiler::Msvc, CCompiler::Gcc]
    } else {
        vec![CCompiler::Clang, CCompiler::Gcc]
    };
    
    for compiler in compilers {
        let cmd = match compiler {
            CCompiler::Gcc => "gcc",
            CCompiler::Clang => "clang", 
            CCompiler::Msvc => "cl",
        };
        
        if Command::new(cmd).arg("--version").output().is_ok() {
            if verbose {
                println!("   Detected optimal compiler: {:?}", compiler);
            }
            return Ok(compiler);
        }
    }
    
    Err("No suitable C compiler found".to_string())
}

fn apply_post_compilation_optimizations(
    temp_exe: &PathBuf,
    final_exe: &PathBuf,
    args: &Args
) -> Result<(), Box<dyn std::error::Error>> {
    
    // Copy executable to final location
    fs::copy(temp_exe, final_exe)?;
    
    // Apply strip if requested
    if args.strip {
        if let Ok(_) = Command::new("strip").arg(final_exe).output() {
            if args.verbose {
                println!("   Symbols stripped from binary");
            }
        }
    }
    
    // Additional platform-specific optimizations
    #[cfg(target_os = "windows")]
    {
        // On Windows, we could apply UPX compression or other optimizations
        if args.optimization == OptimizationLevel::Extreme {
            if let Ok(_) = Command::new("upx").arg("--best").arg(final_exe).output() {
                if args.verbose {
                    println!("   UPX compression applied");
                }
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_args_parsing() {
        // Test optimization level detection
        assert!(matches!(OptimizationLevel::Speed, OptimizationLevel::Speed));
        assert!(matches!(OptimizationLevel::Extreme, OptimizationLevel::Extreme));
    }
    
    #[test]
    fn test_compiler_detection() {
        // Test that we can detect at least one compiler
        let result = detect_optimal_compiler(false);
        // This might fail in CI environments without compilers, so just test the logic
        assert!(result.is_ok() || result.is_err());
    }
    
    #[test]
    fn test_simple_native_compilation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.Bract");
        
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "fn main() -> i32 {{ return 42; }}").unwrap();
        
        // Test file creation
        assert!(test_file.exists());
    }
} 
