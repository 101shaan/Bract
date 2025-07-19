//! Cranelift Native Code Generation Backend
//!
//! This module provides direct native machine code generation using Cranelift,
//! a high-performance code generator that produces optimized machine code
//! without requiring external compilers.
//!
//! Architecture:
//! - `context`: Manages Cranelift compilation context and state
//! - `types`: Maps Bract types to Cranelift types
//! - `functions`: Handles function compilation and calling conventions
//! - `expressions`: Compiles expressions to Cranelift IR
//! - `statements`: Compiles statements and control flow
//! - `memory`: Memory management and allocation
//! - `runtime`: Runtime system integration

use crate::ast::{Module, Item};
use crate::semantic::SymbolTable;
use crate::parser::StringInterner;
use super::{CodegenResult, CodegenError};

use cranelift::prelude::{types as ctypes, Type, AbiParam, InstBuilder};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Module as CraneliftModule, Linkage};
use cranelift_object::{ObjectModule, ObjectBuilder};
use target_lexicon::Triple;
use cranelift_codegen::Context;

pub mod context;
pub mod types;
pub mod functions;
pub mod expressions;
pub mod statements;
pub mod memory;
pub mod runtime;

pub use context::CraneliftContext;

/// Cranelift code generator - produces native machine code
pub struct CraneliftCodeGenerator {
    /// Cranelift compilation context
    context: CraneliftContext,
    /// Object module for code generation
    module: Option<ObjectModule>,
    /// Symbol table from semantic analysis
    #[allow(dead_code)] // TODO: Use for symbol resolution when implementing advanced features
    symbol_table: SymbolTable,
    /// String interner for name resolution
    interner: StringInterner,
    /// Target triple
    target_triple: Triple,
    /// Function builder context (reused for performance)
    builder_context: FunctionBuilderContext,
}

impl CraneliftCodeGenerator {
    /// Create a new Cranelift code generator
    pub fn new(symbol_table: SymbolTable, interner: StringInterner) -> CodegenResult<Self> {
        let target_triple = Triple::host();
        
        // Create optimized settings for native code generation
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false")
            .map_err(|e| CodegenError::InternalError(format!("Failed to set compiler flag: {}", e)))?;
        flag_builder.set("is_pic", "false")
            .map_err(|e| CodegenError::InternalError(format!("Failed to set compiler flag: {}", e)))?;
        flag_builder.set("opt_level", "speed")
            .map_err(|e| CodegenError::InternalError(format!("Failed to set compiler flag: {}", e)))?;
        
        let isa_builder = cranelift_codegen::isa::lookup(target_triple.clone())
            .map_err(|e| CodegenError::InternalError(format!("Failed to create ISA: {}", e)))?;
        
        let isa = isa_builder.finish(settings::Flags::new(flag_builder))
            .map_err(|e| CodegenError::InternalError(format!("Failed to finalize ISA: {}", e)))?;
        
        // Create object module
        let object_builder = ObjectBuilder::new(isa, "bract_program", cranelift_module::default_libcall_names())
            .map_err(|e| CodegenError::InternalError(format!("Failed to create object builder: {}", e)))?;
        
        let module = ObjectModule::new(object_builder);
        
        Ok(Self {
            context: CraneliftContext::new(),
            module: Some(module),
            symbol_table,
            interner,
            target_triple,
            builder_context: FunctionBuilderContext::new(),
        })
    }
    
    /// Generate native code for a module
    pub fn generate(&mut self, module: &Module) -> CodegenResult<Vec<u8>> {
        // Phase 1: Declare all functions first (signatures only)
        for item in &module.items {
            if let Item::Function { .. } = item {
                let module_ref = self.module.as_mut().unwrap();
                functions::declare_function_item(module_ref, item, &mut self.context, &self.interner)?;
            }
        }
        
        // Phase 2: Declare all structs  
        for item in &module.items {
            if let Item::Struct { .. } = item {
                // TODO: Implement struct declaration
                // For now, just skip structs - they don't need Cranelift declarations
                continue;
            }
        }
        
        // Phase 3: Compile all function bodies
        for item in &module.items {
            match item {
                Item::Function { .. } => {
                    let module_ref = self.module.as_mut().unwrap();
                    functions::compile_function_item(module_ref, item, &mut self.builder_context, &mut self.context, &self.interner)?;
                }
                _ => {
                    // Skip non-function items for now
                    continue;
                }
            }
        }
        
        // Check if main function exists properly
        let has_main = module.items.iter().any(|item| {
            if let Item::Function { name, .. } = item {
                self.interner.get(name).map_or(false, |n| n == "main")
            } else {
                false
            }
        });
        
        if !has_main {
            return Err(CodegenError::InternalError("No main function found".to_string()));
        }
        
        // Finalize and produce object code
        let module = self.module.take().unwrap();
        let object_product = module.finish();
        
        Ok(object_product.emit()
            .map_err(|e| CodegenError::InternalError(format!("Failed to emit object code: {}", e)))?)
    }
    
    /// Generate a simple main function for testing (fallback)
    #[allow(dead_code)] // TODO: Use as fallback when no main function is provided
    fn generate_simple_main(&mut self) -> CodegenResult<()> {
        let module = self.module.as_mut().unwrap();
        
        // Create main function signature
        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(ctypes::I32));
        
        // Declare main function
        let func_id = module.declare_function("main", Linkage::Export, &sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare main function: {}", e)))?;
        
        // Create function context
        let mut ctx = Context::new();
        ctx.func.signature = sig;
        
        // Create function builder
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_context);
        
        // Create entry block
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);
        
        // Return 0 for successful execution
        let zero = builder.ins().iconst(ctypes::I32, 0);
        builder.ins().return_(&[zero]);
        
        // Finalize function
        builder.finalize();
        
        // Define function in module
        module.define_function(func_id, &mut ctx)
            .map_err(|e| CodegenError::InternalError(format!("Failed to define main function: {}", e)))?;
        
        Ok(())
    }
    
    /// Get the target triple
    pub fn target_triple(&self) -> &Triple {
        &self.target_triple
    }
}

/// Utility functions for Cranelift code generation
pub mod utils {
    use super::*;
    
    /// Convert a Bract type to a Cranelift type
    pub fn bract_to_cranelift_type(bract_type: &str) -> CodegenResult<Type> {
        match bract_type {
            "i8" => Ok(ctypes::I8),
            "i16" => Ok(ctypes::I16),
            "i32" => Ok(ctypes::I32),
            "i64" => Ok(ctypes::I64),
            "u8" => Ok(ctypes::I8),
            "u16" => Ok(ctypes::I16),
            "u32" => Ok(ctypes::I32),
            "u64" => Ok(ctypes::I64),
            "f32" => Ok(ctypes::F32),
            "f64" => Ok(ctypes::F64),
            "bool" => Ok(ctypes::I8),
            _ => Err(CodegenError::UnsupportedFeature(
                format!("Type not supported: {}", bract_type)
            )),
        }
    }
    
    /// Get the size of a Cranelift type in bytes
    pub fn type_size(ty: Type) -> usize {
        match ty {
            ctypes::I8 => 1,
            ctypes::I16 => 2,
            ctypes::I32 => 4,
            ctypes::I64 => 8,
            ctypes::F32 => 4,
            ctypes::F64 => 8,
            _ => 8, // Default to pointer size
        }
    }
} 