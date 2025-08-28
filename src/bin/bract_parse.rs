//! CLI tool to parse Bract source code and display the AST
//! 
//! Usage: cargo run --bin bract_parse -- <file.bract>
//! Usage: cargo run --bin bract_parse -- --string "expression or code"

use bract::Parser;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    
    let (input, source_desc) = if args[1] == "--string" {
        if args.len() != 3 {
            eprintln!("Error: --string requires an argument");
            print_usage(&args[0]);
            std::process::exit(1);
        }
        (args[2].clone(), format!("string: {}", args[2]))
    } else {
        let file_path = &args[1];
        if Path::new(file_path).exists() {
            match fs::read_to_string(file_path) {
                Ok(content) => (content, format!("file: {}", file_path)),
                Err(e) => {
                    eprintln!("Error reading file '{}': {}", file_path, e);
                    std::process::exit(1);
                }
            }
        } else {
            // If not a file, treat as inline code (backward compatibility)
            (args[1].clone(), format!("string: {}", args[1]))
        }
    };
    
    println!("Parsing: {}", source_desc);
    println!("{}", "=".repeat(50));
    
    // Try to parse as expression first
    match Parser::new(&input, 0) {
        Ok(mut parser) => {
            match parser.parse_expression() {
                Ok(expr) => {
                    println!("✅ Successfully parsed as expression:");
                    println!("{:#?}", expr);
                }
                Err(_) => {
                    // If expression parsing fails, try as module
                    match Parser::new(&input, 0) {
                        Ok(mut module_parser) => {
                            match module_parser.parse_module() {
                                Ok(module) => {
                                    println!("✅ Successfully parsed as module:");
                                    println!("{:#?}", module);
                                    
                                    // Show any errors that were recovered from
                                    let errors = module_parser.errors();
                                    if !errors.is_empty() {
                                        println!("\n⚠️  Errors encountered (but recovered):");
                                        for error in errors {
                                            println!("  {}", error);
                                        }
                                    }
                                }
                                Err(error) => {
                                    println!("❌ Failed to parse as module:");
                                    println!("  {}", error);
                                }
                            }
                        }
                        Err(error) => {
                            println!("❌ Failed to create parser:");
                            println!("  {}", error);
                        }
                    }
                }
            }
        }
        Err(error) => {
            println!("❌ Failed to create parser:");
            println!("  {}", error);
        }
    }
}

fn print_usage(program_name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <file.bract>              Parse a Bract file", program_name);
    eprintln!("  {} --string \"code\"           Parse inline code", program_name);
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} examples/simple_function.bract", program_name);
    eprintln!("  {} --string \"1 + 2 * 3\"", program_name);
}
