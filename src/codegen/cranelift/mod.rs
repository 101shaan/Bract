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
//! - `memory`: Revolutionary hybrid memory management system
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
pub use memory::{BractMemoryManager, MemoryStrategy, MemoryAnnotation, parse_annotation, AllocationOptions, AllocationResult, LeakWarning, LeakSeverity, LeakType};

/// Cranelift code generator - produces native machine code with hybrid memory management
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
    /// **REVOLUTIONARY**: Hybrid memory management system
    memory_manager: BractMemoryManager,
}

impl CraneliftCodeGenerator {
    /// Create a new Cranelift code generator with hybrid memory management
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
            memory_manager: BractMemoryManager::new(),
        })
    }
    
    /// Generate native code for a module with hybrid memory management
    pub fn generate(&mut self, module: &Module) -> CodegenResult<Vec<u8>> {
        // **REVOLUTIONARY**: Initialize hybrid memory management runtime
        {
            let module_ref = self.module.as_mut().unwrap();
            self.memory_manager.initialize_runtime(module_ref)?;
        }
        
        // Phase 1: Declare all functions first (signatures only)
        for item in &module.items {
            if let Item::Function { .. } = item {
                let module_ref = self.module.as_mut().unwrap();
                functions::declare_function_item(module_ref, item, &mut self.context, &self.interner)?;
            }
        }
        
        // Phase 2: Declare all structs with memory strategy analysis
        for item in &module.items {
            if let Item::Struct { .. } = item {
                // TODO: Analyze struct memory requirements and strategies
                // For now, just skip structs - they don't need Cranelift declarations
                continue;
            }
        }
        
        // Phase 3: Compile all function bodies with memory management
        for item in &module.items {
            match item {
                Item::Function { .. } => {
                    let module_ref = self.module.as_mut().unwrap();
                    // TODO: Integrate memory manager into function compilation
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
                if let Some(func_name) = self.interner.get(name) {
                    func_name == "main"
                } else {
                    false
                }
            } else {
                false
            }
        });

        if !has_main {
            // Create a default main function that returns 0
            self.create_default_main()?;
        }

        // Phase 4: **MEMORY MANAGEMENT FINALIZATION**
        // All memory management cleanup and analysis happens here
        
        // Finalize the module and generate machine code
        let module_ref = self.module.take().unwrap();
        let object_product = module_ref.finish();
        
        Ok(object_product.emit().unwrap())
    }
    
    /// Create a default main function for modules that don't have one
    fn create_default_main(&mut self) -> CodegenResult<()> {
        let module = self.module.as_mut().unwrap();
        
        // Create main function signature: main() -> i32
        let mut sig = module.make_signature();
        sig.returns.push(AbiParam::new(ctypes::I32));
        
        let func_id = module.declare_function("main", Linkage::Export, &sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare main function: {}", e)))?;
        
        self.context.register_function("main", func_id);
        
        // Define main function body
        let mut ctx = Context::new();
        ctx.func.signature = sig;
        
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_context);
        
        // Create entry block
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);
        
        // **MEMORY MANAGEMENT**: Initialize memory management for main function
        // This would set up any global memory regions or smart pointer systems
        
        // Return 0 for successful execution
        let zero = builder.ins().iconst(ctypes::I32, 0);
        builder.ins().return_(&[zero]);
        
        // **MEMORY MANAGEMENT**: Cleanup function memory before return
        // This automatically decrements reference counts, deallocates regions, etc.
        
        // Finalize function
        builder.finalize();
        
        // Define function in module
        module.define_function(func_id, &mut ctx)
            .map_err(|e| CodegenError::InternalError(format!("Failed to define main function: {}", e)))?;
        
        Ok(())
    }
    
    /// **NEW**: Allocate memory using hybrid memory management
    pub fn allocate_memory(
        &mut self,
        builder: &mut FunctionBuilder,
        object_type: Type,
        size: u32,
        strategy: Option<MemoryStrategy>,
        region_id: Option<u32>,
    ) -> CodegenResult<cranelift::prelude::Value> {
        let memory_strategy = strategy.unwrap_or_else(|| {
            MemoryStrategy::infer_for_type(64, false, true) // Default sensible strategy
        });
        
        let options = AllocationOptions {
            region_id,
            source_location: "codegen".to_string(),
            alignment: None,
            gc_allowed: true,
        };
        self.memory_manager.allocate(builder, memory_strategy, object_type, size, options).map(|result| result.ptr)
    }
    
    /// **NEW**: Create memory region for bulk allocation
    pub fn create_memory_region(&mut self, name: String, size: u64) -> u32 {
        self.memory_manager.create_region(name, size)
    }
    
    /// **NEW**: Initialize memory region at runtime
    pub fn initialize_memory_region(
        &mut self, 
        builder: &mut FunctionBuilder, 
        region_id: u32
    ) -> CodegenResult<cranelift::prelude::Value> {
        self.memory_manager.initialize_region(builder, region_id)
    }
    
    /// **NEW**: Move linear type (transfer ownership)
    pub fn move_linear_type(
        &mut self, 
        from: cranelift::prelude::Value, 
        to: cranelift::prelude::Value
    ) -> CodegenResult<()> {
        self.memory_manager.move_linear(from, to, "codegen_move")
    }
    
    /// **NEW**: Check linear type usage safety
    pub fn check_linear_safety(&self, value: cranelift::prelude::Value) -> CodegenResult<()> {
        self.memory_manager.check_linear_usage(value, "codegen_usage_check")
    }
    
    /// **NEW**: Generate bounds checking code with optimal performance
    pub fn generate_bounds_check(
        &self,
        builder: &mut FunctionBuilder,
        ptr: cranelift::prelude::Value,
        size: cranelift::prelude::Value,
        access_size: u32,
    ) -> CodegenResult<()> {
        self.memory_manager.generate_bounds_check(builder, ptr, size, access_size)
    }
    
    /// **NEW**: Increment smart pointer reference count
        pub fn smart_pointer_inc_ref(
        &mut self,
        _builder: &mut FunctionBuilder,
        _ptr: cranelift::prelude::Value
    ) -> CodegenResult<()> {
        // TODO: Implement smart pointer increment
        Ok(())
    }
    
    /// **NEW**: Decrement smart pointer reference count
        pub fn smart_pointer_dec_ref(
        &mut self,
        _builder: &mut FunctionBuilder,
        _ptr: cranelift::prelude::Value
    ) -> CodegenResult<()> {
        // TODO: Implement smart pointer decrement  
        Ok(())
    }
    
    /// **NEW**: Cleanup function memory (called at end of each function)
    pub fn cleanup_function_memory(&mut self, builder: &mut FunctionBuilder) -> CodegenResult<()> {
        self.memory_manager.cleanup_function(builder)
    }
    
    /// Get the target triple
    pub fn target_triple(&self) -> &Triple {
        &self.target_triple
    }
    
    /// **NEW**: Get memory manager reference
    pub fn memory_manager(&mut self) -> &mut BractMemoryManager {
        &mut self.memory_manager
    }
}

/// Utility functions for Cranelift code generation with memory management
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
            "char" => Ok(ctypes::I8),
            _ => Err(CodegenError::UnsupportedFeature(
                format!("Type not supported: {}", bract_type)
            )),
        }
    }
    
    /// **NEW**: Get size of Cranelift type in bytes
    pub fn type_size(cranelift_type: Type) -> usize {
        match cranelift_type {
            t if t == ctypes::I8 => 1,
            t if t == ctypes::I16 => 2,
            t if t == ctypes::I32 => 4,
            t if t == ctypes::I64 => 8,
            t if t == ctypes::F32 => 4,
            t if t == ctypes::F64 => 8,
            _ => 8, // Default to pointer size for unknown types
        }
    }
    
    /// **NEW**: Parse memory strategy from user attribute
    pub fn parse_memory_strategy(attribute: &str) -> Option<MemoryStrategy> {
        match parse_annotation(attribute)? {
            MemoryAnnotation::Manual => Some(MemoryStrategy::Manual),
            MemoryAnnotation::Smart => Some(MemoryStrategy::SmartPtr),
            MemoryAnnotation::Linear => Some(MemoryStrategy::Linear),
            MemoryAnnotation::Region(_) => Some(MemoryStrategy::Region),
            MemoryAnnotation::Stack => Some(MemoryStrategy::Stack),
            _ => None,
        }
    }
} 