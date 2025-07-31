//! Native code generation for Bract language
//!
//! Direct compilation to native machine code using Cranelift:
//! - AST → Native Machine Code (no intermediate steps)
//! - Revolutionary memory management integration
//! - Zero external compiler dependencies

pub mod cranelift;

use crate::ast::Module;
use crate::semantic::symbols::SymbolTable;
use crate::parser::StringInterner;

/// Native code generation pipeline using Cranelift
pub struct CodegenPipeline {
    /// Cranelift code generator - direct native machine code
    cranelift_generator: cranelift::CraneliftCodeGenerator,
}

impl CodegenPipeline {
    /// Create a new native code generation pipeline
    pub fn new(symbol_table: SymbolTable, interner: StringInterner) -> Result<Self, String> {
        let cranelift_generator = cranelift::CraneliftCodeGenerator::new(symbol_table, interner)
            .map_err(|e| format!("Failed to create Cranelift generator: {:?}", e))?;
        
        Ok(Self {
            cranelift_generator,
        })
    }
    
    /// Compile a module directly to native machine code
    pub fn compile_module(&mut self, module: &Module) -> Result<Vec<u8>, String> {
        // Direct native compilation using Cranelift
        self.cranelift_generator.generate(module)
            .map_err(|e| format!("Native compilation error: {:?}", e))
    }
}

/// Result type for code generation operations
pub type CodegenResult<T> = Result<T, CodegenError>;

/// Errors that can occur during native code generation
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
    /// Native compilation error
    NativeCompilation(String),
    /// IO error during code generation
    IoError(String),
    /// Internal compiler error
    InternalError(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::UnsupportedFeature(msg) => write!(f, "Unsupported feature: {}", msg),
            CodegenError::TypeConversion(msg) => write!(f, "Type conversion error: {}", msg),
            CodegenError::SymbolResolution(msg) => write!(f, "Symbol resolution error: {}", msg),
            CodegenError::MemoryManagement(msg) => write!(f, "Memory management error: {}", msg),
            CodegenError::NativeCompilation(msg) => write!(f, "Native compilation error: {}", msg),
            CodegenError::IoError(msg) => write!(f, "IO error: {}", msg),
            CodegenError::InternalError(msg) => write!(f, "Internal compiler error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}