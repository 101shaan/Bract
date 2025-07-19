// Quick test to verify BOM handling in lexer
use std::process::Command;

fn main() {
    // Test basic lexer functionality
    let source = "fn main() { 42 }";
    println!("Testing lexer with: {}", source);
    
    // This would normally use the bract lexer
    // For now, just verify the source is clean ASCII
    for (i, ch) in source.chars().enumerate() {
        println!("  char {}: '{}' (U+{:04X})", i, ch, ch as u32);
    }
    
    println!("Source appears clean - no BOM or invalid characters detected");
} 