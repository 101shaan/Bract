//! CLI tool to parse Bract source code and display the AST
//! 
//! Usage: cargo run --bin Bract_parse -- "expression or code"

use bract::Parser;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <Bract_code>", args[0]);
        eprintln!("Example: {} \"1 + 2 * 3\"", args[0]);
        std::process::exit(1);
    }
    
    let input = &args[1];
    println!("Parsing: {}", input);
    println!("{}", "=".repeat(50));
    
    // Try to parse as expression first
    match Parser::new(input, 0) {
        Ok(mut parser) => {
            match parser.parse_expression() {
                Ok(expr) => {
                    println!("✅ Successfully parsed as expression:");
                    println!("{:#?}", expr);
                }
                Err(_) => {
                    // If expression parsing fails, try as module
                    match Parser::new(input, 0) {
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
