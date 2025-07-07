//! Code Generation Module for Bract
//!
//! This module handles the translation of Bract AST to various target languages.
//! Initially targeting C for rapid development and bootstrapping.
//!
//! Architecture:
//! - CodegenContext: Manages symbol tables, type mapping, and generation state
//! - CCodeBuilder: Efficient C code generation with proper formatting
//! - Target-specific generators: expressions, statements, items
//! - Runtime integration: memory management, error handling, standard library

use crate::ast::*;
use crate::semantic::SymbolTable;
use std::collections::HashMap;
// Removed unused import

pub mod c_gen;
pub mod expressions;
pub mod statements;  
pub mod items;
pub mod runtime;
pub mod build;
pub mod cranelift;  // NEW: Native code generation

pub use c_gen::CCodeGenerator;

/// Result type for code generation operations
pub type CodegenResult<T> = Result<T, CodegenError>;

/// Errors that can occur during code generation
#[derive(Debug, Clone, PartialEq)]
pub enum CodegenError {
    /// Unsupported language feature
    UnsupportedFeature(String),
    /// Type conversion error
    TypeConversion(String),
    /// Symbol resolution error
    SymbolResolution(String),
    /// Memory management error
    MemoryManagement(String),
    /// Target language limitation
    TargetLimitation(String),
    /// IO error during code generation
    IoError(String),
    /// Internal compiler error
    InternalError(String),
    /// LLVM-specific error
    LlvmError(String),
    /// Linker error
    LinkerError(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::UnsupportedFeature(msg) => write!(f, "Unsupported feature: {}", msg),
            CodegenError::TypeConversion(msg) => write!(f, "Type conversion error: {}", msg),
            CodegenError::SymbolResolution(msg) => write!(f, "Symbol resolution error: {}", msg),
            CodegenError::MemoryManagement(msg) => write!(f, "Memory management error: {}", msg),
            CodegenError::TargetLimitation(msg) => write!(f, "Target language limitation: {}", msg),
            CodegenError::IoError(msg) => write!(f, "IO error: {}", msg),
            CodegenError::InternalError(msg) => write!(f, "Internal compiler error: {}", msg),
            CodegenError::LlvmError(msg) => write!(f, "LLVM error: {}", msg),
            CodegenError::LinkerError(msg) => write!(f, "Linker error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

/// Code generation context - tracks state during generation
#[derive(Debug)]
pub struct CodegenContext {
    /// Symbol table from semantic analysis
    pub symbol_table: SymbolTable,
    /// Current scope depth
    pub scope_depth: usize,
    /// Generated symbol names (for name mangling)
    pub symbol_names: HashMap<SymbolId, String>,
    /// Type mappings (Bract type -> target type)
    pub type_mappings: HashMap<String, String>,
    /// Current function context (for returns, etc.)
    pub current_function: Option<String>,
    /// Loop contexts (for break/continue)
    pub loop_contexts: Vec<LoopContext>,
    /// Temporary variable counter
    pub temp_counter: usize,
    /// Generated includes
    pub includes: Vec<String>,
    /// Forward declarations needed
    pub forward_decls: Vec<String>,
}

/// Loop context for break/continue handling
#[derive(Debug, Clone)]
pub struct LoopContext {
    pub loop_label: Option<String>,
    pub break_label: String,
    pub continue_label: String,
}

impl CodegenContext {
    /// Create a new context with semantic analysis results
    pub fn new(symbol_table: SymbolTable) -> Self {
        let mut ctx = Self {
            symbol_table,
            scope_depth: 0,
            symbol_names: HashMap::new(),
            type_mappings: HashMap::new(),
            current_function: None,
            loop_contexts: Vec::new(),
            temp_counter: 0,
            includes: Vec::new(),
            forward_decls: Vec::new(),
        };
        
        // Initialize standard type mappings
        ctx.init_type_mappings();
        ctx.init_standard_includes();
        
        ctx
    }
    
    /// Initialize type mappings from Bract to C
    fn init_type_mappings(&mut self) {
        // Integer types
        self.type_mappings.insert("i8".to_string(), "int8_t".to_string());
        self.type_mappings.insert("i16".to_string(), "int16_t".to_string());
        self.type_mappings.insert("i32".to_string(), "int32_t".to_string());
        self.type_mappings.insert("i64".to_string(), "int64_t".to_string());
        self.type_mappings.insert("i128".to_string(), "int128_t".to_string());
        self.type_mappings.insert("isize".to_string(), "intptr_t".to_string());
        
        self.type_mappings.insert("u8".to_string(), "uint8_t".to_string());
        self.type_mappings.insert("u16".to_string(), "uint16_t".to_string());
        self.type_mappings.insert("u32".to_string(), "uint32_t".to_string());
        self.type_mappings.insert("u64".to_string(), "uint64_t".to_string());
        self.type_mappings.insert("u128".to_string(), "uint128_t".to_string());
        self.type_mappings.insert("usize".to_string(), "uintptr_t".to_string());
        
        // Float types
        self.type_mappings.insert("f32".to_string(), "float".to_string());
        self.type_mappings.insert("f64".to_string(), "double".to_string());
        
        // Other primitives
        self.type_mappings.insert("bool".to_string(), "bool".to_string());
        self.type_mappings.insert("char".to_string(), "char32_t".to_string()); // UTF-32
        self.type_mappings.insert("str".to_string(), "Bract_str_t".to_string());
        self.type_mappings.insert("()".to_string(), "void".to_string());
    }
    
    /// Initialize standard includes
    fn init_standard_includes(&mut self) {
        self.includes.push("#include <stdint.h>".to_string());
        self.includes.push("#include <stdbool.h>".to_string());
        self.includes.push("#include <stddef.h>".to_string());
        self.includes.push("#include <stdio.h>".to_string());
        self.includes.push("#include <stdlib.h>".to_string());
        self.includes.push("#include <string.h>".to_string());
        self.includes.push("#include \"Bract_runtime.h\"".to_string());
    }
    
    /// Generate a mangled symbol name
    pub fn mangle_symbol(&mut self, symbol_id: SymbolId, base_name: &str) -> String {
        if let Some(existing) = self.symbol_names.get(&symbol_id) {
            return existing.clone();
        }
        
        let mangled = format!("Bract_{}_{}", base_name, symbol_id);
        self.symbol_names.insert(symbol_id, mangled.clone());
        mangled
    }
    
    /// Generate a temporary variable name
    pub fn temp_var(&mut self) -> String {
        let name = format!("tmp_{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }
    
    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }
    
    /// Exit current scope
    pub fn exit_scope(&mut self) {
        self.scope_depth = self.scope_depth.saturating_sub(1);
    }
    
    /// Enter a loop context
    pub fn enter_loop(&mut self, label: Option<String>) {
        let loop_id = self.loop_contexts.len();
        let break_label = format!("loop_break_{}", loop_id);
        let continue_label = format!("loop_continue_{}", loop_id);
        
        self.loop_contexts.push(LoopContext {
            loop_label: label,
            break_label,
            continue_label,
        });
    }
    
    /// Exit current loop context
    pub fn exit_loop(&mut self) {
        self.loop_contexts.pop();
    }
    
    /// Get current loop context
    pub fn current_loop(&self) -> Option<&LoopContext> {
        self.loop_contexts.last()
    }
    
    /// Map Bract type to C type
    pub fn map_type(&self, bract_type: &str) -> String {
        self.type_mappings.get(bract_type)
            .cloned()
            .unwrap_or_else(|| {
                // Default mapping for user-defined types
                format!("struct {}", bract_type)
            })
    }
}

/// Efficient C code builder with proper formatting
#[derive(Debug, Clone)]
pub struct CCodeBuilder {
    /// Current indentation level
    indent_level: usize,
    /// Generated code buffer
    code: String,
    /// Header buffer (for declarations)
    header: String,
    /// Whether we're in a header context
    in_header: bool,
}

impl CCodeBuilder {
    /// Create a new code builder
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            code: String::new(),
            header: String::new(),
            in_header: false,
        }
    }
    
    /// Create builder with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            indent_level: 0,
            code: String::with_capacity(capacity),
            header: String::with_capacity(capacity / 4),
            in_header: false,
        }
    }
    
    /// Switch to header context
    pub fn header_context(&mut self) {
        self.in_header = true;
    }
    
    /// Switch to code context
    pub fn code_context(&mut self) {
        self.in_header = false;
    }
    
    /// Add a line with proper indentation
    pub fn line(&mut self, text: &str) {
        self.indent();
        self.push_str(text);
        self.newline();
    }
    
    /// Add text without newline
    pub fn push_str(&mut self, text: &str) {
        if self.in_header {
            self.header.push_str(text);
        } else {
            self.code.push_str(text);
        }
    }
    
    /// Add indentation
    pub fn indent(&mut self) {
        let spaces = "    ".repeat(self.indent_level);
        self.push_str(&spaces);
    }
    
    /// Add newline
    pub fn newline(&mut self) {
        self.push_str("\n");
    }
    
    /// Increase indentation
    pub fn indent_inc(&mut self) {
        self.indent_level += 1;
    }
    
    /// Decrease indentation
    pub fn indent_dec(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }
    
    /// Add a block with automatic indentation
    pub fn block<F>(&mut self, f: F) 
    where
        F: FnOnce(&mut Self)
    {
        self.line("{");
        self.indent_inc();
        f(self);
        self.indent_dec();
        self.line("}");
    }
    
    /// Add opening brace and increase indentation
    pub fn open_block(&mut self) {
        self.line("{");
        self.indent_inc();
    }
    
    /// Add closing brace and decrease indentation
    pub fn close_block(&mut self) {
        self.indent_dec();
        self.line("}");
    }
    
    /// Add a function definition
    pub fn function(&mut self, signature: &str, body: impl FnOnce(&mut Self)) {
        self.line(signature);
        self.block(body);
    }
    
    /// Add a comment
    pub fn comment(&mut self, text: &str) {
        self.line(&format!("// {}", text));
    }
    
    /// Add a multi-line comment
    pub fn comment_block(&mut self, text: &str) {
        self.line("/*");
        for line in text.lines() {
            self.line(&format!(" * {}", line));
        }
        self.line(" */");
    }
    
    /// Get the generated code
    pub fn code(&self) -> &str {
        &self.code
    }
    
    /// Get the generated header
    pub fn header(&self) -> &str {
        &self.header
    }
    
    /// Get both header and code combined
    pub fn build(self) -> (String, String) {
        (self.header, self.code)
    }
    
    /// Clear the builder
    pub fn clear(&mut self) {
        self.code.clear();
        self.header.clear();
        self.indent_level = 0;
        self.in_header = false;
    }
}

/// Format a C identifier (escape keywords if needed)
pub fn format_c_identifier(name: &str) -> String {
    // C keywords that need escaping
    const C_KEYWORDS: &[&str] = &[
        "auto", "break", "case", "char", "const", "continue", "default", "do",
        "double", "else", "enum", "extern", "float", "for", "goto", "if",
        "inline", "int", "long", "register", "restrict", "return", "short",
        "signed", "sizeof", "static", "struct", "switch", "typedef", "union",
        "unsigned", "void", "volatile", "while", "_Bool", "_Complex", "_Imaginary",
        "_Alignas", "_Alignof", "_Atomic", "_Generic", "_Noreturn",
        "_Static_assert", "_Thread_local"
    ];
    
    if C_KEYWORDS.contains(&name) {
        format!("Bract_{}", name)
    } else {
        name.to_string()
    }
}

/// Performance metrics for code generation
#[derive(Debug, Default)]
pub struct CodegenMetrics {
    pub nodes_processed: usize,
    pub lines_generated: usize,
    pub compilation_time_ms: u128,
    pub memory_usage_bytes: usize,
}

impl CodegenMetrics {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_node(&mut self) {
        self.nodes_processed += 1;
    }
    
    pub fn record_lines(&mut self, count: usize) {
        self.lines_generated += count;
    }
    
    pub fn nodes_per_second(&self) -> f64 {
        if self.compilation_time_ms == 0 {
            0.0
        } else {
            (self.nodes_processed as f64) / (self.compilation_time_ms as f64 / 1000.0)
        }
    }
    
    pub fn lines_per_second(&self) -> f64 {
        if self.compilation_time_ms == 0 {
            0.0
        } else {
            (self.lines_generated as f64) / (self.compilation_time_ms as f64 / 1000.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::SymbolTable;
    
    #[test]
    fn test_codegen_context_creation() {
        let symbol_table = SymbolTable::new();
        let ctx = CodegenContext::new(symbol_table);
        
        assert_eq!(ctx.scope_depth, 0);
        assert!(ctx.includes.contains(&"#include <stdint.h>".to_string()));
        assert_eq!(ctx.map_type("i32"), "int32_t");
        assert_eq!(ctx.map_type("f64"), "double");
    }
    
    #[test]
    fn test_symbol_mangling() {
        let symbol_table = SymbolTable::new();
        let mut ctx = CodegenContext::new(symbol_table);
        
        let mangled = ctx.mangle_symbol(42, "test_func");
        assert_eq!(mangled, "Bract_test_func_42");
        
        // Should return same name for same symbol
        let mangled2 = ctx.mangle_symbol(42, "test_func");
        assert_eq!(mangled, mangled2);
    }
    
    #[test]
    fn test_temp_var_generation() {
        let symbol_table = SymbolTable::new();
        let mut ctx = CodegenContext::new(symbol_table);
        
        assert_eq!(ctx.temp_var(), "tmp_0");
        assert_eq!(ctx.temp_var(), "tmp_1");
        assert_eq!(ctx.temp_var(), "tmp_2");
    }
    
    #[test]
    fn test_c_code_builder() {
        let mut builder = CCodeBuilder::new();
        
        builder.line("int main() {");
        builder.indent_inc();
        builder.line("printf(\"Hello, World!\\n\");");
        builder.line("return 0;");
        builder.indent_dec();
        builder.line("}");
        
        let expected = "int main() {\n    printf(\"Hello, World!\\n\");\n    return 0;\n}\n";
        assert_eq!(builder.code(), expected);
    }
    
    #[test]
    fn test_c_code_builder_block() {
        let mut builder = CCodeBuilder::new();
        
        builder.function("int main()", |b| {
            b.line("printf(\"Hello, World!\\n\");");
            b.line("return 0;");
        });
        
        let expected = "int main()\n{\n    printf(\"Hello, World!\\n\");\n    return 0;\n}\n";
        assert_eq!(builder.code(), expected);
    }
    
    #[test]
    fn test_c_identifier_formatting() {
        assert_eq!(format_c_identifier("test"), "test");
        assert_eq!(format_c_identifier("int"), "Bract_int");
        assert_eq!(format_c_identifier("return"), "Bract_return");
        assert_eq!(format_c_identifier("my_function"), "my_function");
    }
    
    #[test]
    fn test_loop_context() {
        let symbol_table = SymbolTable::new();
        let mut ctx = CodegenContext::new(symbol_table);
        
        ctx.enter_loop(Some("outer".to_string()));
        assert_eq!(ctx.current_loop().unwrap().loop_label, Some("outer".to_string()));
        
        ctx.enter_loop(None);
        assert_eq!(ctx.current_loop().unwrap().loop_label, None);
        
        ctx.exit_loop();
        assert_eq!(ctx.current_loop().unwrap().loop_label, Some("outer".to_string()));
        
        ctx.exit_loop();
        assert!(ctx.current_loop().is_none());
    }
} 
