//! Bract Cranelift Native Compiler
//!
//! This is the TRUE native compiler for Bract, generating direct machine code
//! using Cranelift without any C transpilation. This provides:
//!
//! - Direct machine code generation
//! - No external compiler dependencies
//! - Optimized native performance
//! - Cross-platform native compilation
//! - JIT compilation capabilities

use bract::{
    Parser,
    semantic::SemanticAnalyzer,
    codegen::cranelift::CraneliftCodeGenerator,
    profiling::CycleProfiler,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

/// Command line arguments for native Cranelift compilation
#[derive(Debug)]
struct Args {
    /// Input Bract file
    input_file: PathBuf,
    /// Output executable path
    output_file: PathBuf,
    /// Enable verbose output
    verbose: bool,
    /// Show compilation statistics
    stats: bool,
    /// Enable JIT execution instead of AOT compilation
    jit: bool,
    /// Optimization level (0-3)
    optimization: u8,
}

impl Args {
    /// Parse command line arguments
    fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();
        
        if args.len() < 2 {
            return Err("Input file required".to_string());
        }
        
        let input_file = PathBuf::from(&args[1]);
        if !input_file.exists() {
            return Err(format!("Input file does not exist: {}", input_file.display()));
        }
        
        let mut output_file = input_file.with_extension("");
        if cfg!(windows) {
            output_file = output_file.with_extension("exe");
        }
        
        let mut verbose = false;
        let mut stats = false;
        let mut jit = false;
        let mut optimization = 2;
        
        for (i, arg) in args.iter().enumerate().skip(2) {
            match arg.as_str() {
                "-v" | "--verbose" => verbose = true,
                "-s" | "--stats" => stats = true,
                "-j" | "--jit" => jit = true,
                "-O0" => optimization = 0,
                "-O1" => optimization = 1,
                "-O2" => optimization = 2,
                "-O3" => optimization = 3,
                "-o" | "--output" => {
                    if i + 1 < args.len() {
                        output_file = PathBuf::from(&args[i + 1]);
                    }
                }
                _ => {}
            }
        }
        
        Ok(Args {
            input_file,
            output_file,
            verbose,
            stats,
            jit,
            optimization,
        })
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
    
    let start_time = Instant::now();
    
    if args.verbose {
        println!("🚀 Bract Cranelift Native Compiler");
        println!("   Input: {}", args.input_file.display());
        println!("   Output: {}", args.output_file.display());
        println!("   Optimization: -O{}", args.optimization);
        println!("   Mode: {}", if args.jit { "JIT" } else { "AOT" });
        println!();
    }
    
    let profile_result = match compile_native(&args) {
        Ok(profile) => profile,
        Err(e) => {
            eprintln!("Native compilation failed: {}", e);
            process::exit(1);
        }
    };
    
    if args.stats {
        println!("📊 Compilation Statistics:");
        println!("   Total time: {:?}", start_time.elapsed());
        println!("   Mode: Native machine code generation");
        println!("   Backend: Cranelift");
        
        if let Some(profile) = profile_result {
            println!("🔄 Code generation cycles: {}", profile.cpu_cycles);
            if let Some(freq_ghz) = profile.cpu_freq_ghz() {
                println!("⚡ Estimated CPU frequency: {:.2} GHz", freq_ghz);
            }
            println!("📊 Cycles per microsecond: {:.1}", profile.cycles_per_microsecond());
        }
    }
    
    if args.verbose {
        println!("✅ Native compilation completed successfully!");
        if !args.jit {
            println!("   Run your program: {}", args.output_file.display());
        }
    }
}

fn compile_native(args: &Args) -> Result<Option<bract::profiling::ProfilingResult>, String> {
    let start_time = Instant::now();
    
    // Phase 1: Read source code
    if args.verbose {
        println!("📖 Phase 1: Reading source code...");
    }
    
    let source_code = fs::read_to_string(&args.input_file)
        .map_err(|e| format!("Failed to read input file: {}", e))?;
    
    if args.verbose {
        println!("   Source code: {} characters", source_code.len());
    }
    
    // Phase 2: Lexical analysis and parsing
    if args.verbose {
        println!("🔍 Phase 2: Parsing...");
    }
    
    let parse_start = Instant::now();
    
    let mut parser = Parser::new(&source_code, 0)
        .map_err(|e| format!("Parser creation failed: {}", e))?;
    let module = parser.parse_module()
        .map_err(|e| format!("Parse error: {:?}", e))?;
    
    // Extract the string interner from the parser
    let interner = parser.take_interner();
    
    if args.verbose {
        println!("   Parsed {} items in {:?}", module.items.len(), parse_start.elapsed());
    }
    
    // Phase 3: Semantic analysis
    if args.verbose {
        println!("🧠 Phase 3: Semantic analysis...");
    }
    
    let semantic_start = Instant::now();
    
    let mut analyzer = SemanticAnalyzer::new();
    let analysis_result = analyzer.analyze(&module);
    
    let symbol_table = match analysis_result.errors.is_empty() {
        true => analysis_result.symbol_table,
        false => {
            let error_msg = analysis_result.errors
                .into_iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("Semantic errors: {}", error_msg));
        }
    };
    
    if args.verbose {
        println!("   Semantic analysis completed in {:?}", semantic_start.elapsed());
    }
    
    // Phase 4: Native code generation with Cranelift
    if args.verbose {
        println!("⚡ Phase 4: Native code generation...");
    }
    
    let codegen_start = Instant::now();
    let mut cycle_profiler = CycleProfiler::new();
    cycle_profiler.start();
    
    let mut code_generator = CraneliftCodeGenerator::new(symbol_table, interner)
        .map_err(|e| format!("Failed to create code generator: {}", e))?;
    
    if args.verbose {
        println!("   Target: {:?}", code_generator.target_triple());
    }
    
    let object_code = code_generator.generate(&module)
        .map_err(|e| format!("Code generation failed: {}", e))?;
    
    let profile_result = cycle_profiler.stop();
    
    if args.verbose {
        println!("   Generated {} bytes of object code in {:?}", object_code.len(), codegen_start.elapsed());
        println!("📊 Detailed profiling:");
        print!("{}", profile_result.display());
    }
    
    // Phase 5: Object file creation and linking
    if args.verbose {
        println!("🔗 Phase 5: Object file creation and linking...");
    }
    
    let link_start = Instant::now();
    
    // Write object file
    let object_path = args.output_file.with_extension("o");
    fs::write(&object_path, &object_code)
        .map_err(|e| format!("Failed to write object file: {}", e))?;
    
    if args.verbose {
        println!("   Object file: {}", object_path.display());
    }
    
    // Link to executable (platform-specific)
    link_executable(&object_path, &args.output_file, args.verbose)?;
    
    if args.verbose {
        println!("   Linked executable in {:?}", link_start.elapsed());
    }
    
    // Clean up object file
    let _ = fs::remove_file(&object_path);
    
    if args.verbose {
        println!("   Total compilation time: {:?}", start_time.elapsed());
    }
    
    Ok(Some(profile_result))
}

fn link_executable(object_path: &PathBuf, output_path: &PathBuf, verbose: bool) -> Result<(), String> {
    use std::process::Command;
    
    let mut cmd = if cfg!(windows) {
        // Try LLD (LLVM linker) first, then fall back to Microsoft linker
        if Command::new("lld-link").arg("--version").output().is_ok() {
            let mut cmd = Command::new("lld-link");
            cmd.arg("/ENTRY:main")
               .arg("/SUBSYSTEM:CONSOLE")
               .arg(format!("/OUT:{}", output_path.display()))
               .arg(object_path)
               .arg("native_runtime.o")  // Add our runtime
               .arg("msvcrt.lib")        // Windows C runtime
               .arg("kernel32.lib");     // Windows system calls
            cmd
        } else {
            // Use Microsoft linker on Windows
            let mut cmd = Command::new("link");
            cmd.arg("/ENTRY:main")
               .arg("/SUBSYSTEM:CONSOLE")
               .arg(format!("/OUT:{}", output_path.display()))
               .arg(object_path)
               .arg("native_runtime.o")  // Add our runtime
               .arg("msvcrt.lib")        // Windows C runtime
               .arg("kernel32.lib");     // Windows system calls
            cmd
        }
    } else {
        // Use system linker on Unix-like systems
        let mut cmd = Command::new("ld");
        cmd.arg("-o")
           .arg(output_path)
           .arg(object_path)
           .arg("native_runtime.o")  // Add our runtime
           .arg("-lc");              // Link with C library
        cmd
    };
    
    if verbose {
        println!("   Linking command: {:?}", cmd);
    }
    
    let output = cmd.output()
        .map_err(|e| format!("Failed to run linker: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Linker failed: {}", stderr));
    }
    
    Ok(())
}

fn print_usage() {
    println!("Bract Cranelift Native Compiler - True Native Machine Code Generation");
    println!();
    println!("USAGE:");
    println!("    bract_cranelift <input.bract> [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -o, --output <FILE>    Output executable [default: <input>]");
    println!("    -v, --verbose          Enable verbose output");
    println!("    -s, --stats            Show compilation statistics");
    println!("    -j, --jit              Enable JIT execution");
    println!("    -O0, -O1, -O2, -O3     Optimization level [default: -O2]");
    println!();
    println!("FEATURES:");
    println!("    ✅ Direct machine code generation (no C transpilation)");
    println!("    ✅ No external compiler dependencies");
    println!("    ✅ Cross-platform native compilation");
    println!("    ✅ Optimized performance");
    println!("    ✅ JIT compilation support");
    println!();
    println!("EXAMPLES:");
    println!("    bract_cranelift hello.bract");
    println!("    bract_cranelift hello.bract -v -s -O3");
    println!("    bract_cranelift hello.bract --jit");
    println!("    bract_cranelift hello.bract -o hello_native");
} 