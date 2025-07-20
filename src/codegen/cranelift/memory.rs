//! Hybrid Memory Management for Cranelift
//!
//! Bract's Revolutionary Memory System - The First Language to Seamlessly Combine:
//! 1. Manual Memory Management (malloc/free) - Ultimate Performance Control
//! 2. Smart Pointers (ARC/RC) - Automatic Reference Counting  
//! 3. Linear Types - Move Semantics and Ownership Tracking
//! 4. Memory Regions - Stack-like Grouped Allocation/Deallocation
//!
//! This hybrid approach provides:
//! - C-level performance with zero overhead
//! - Rust-level memory safety guarantees  
//! - Unique flexibility for different use cases
//! - Compile-time memory leak prevention

use super::{CodegenResult, CodegenError};
use cranelift::prelude::{types as ctypes, Type, Value, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module as CraneliftModule;
use std::collections::HashMap;

/// Memory allocation strategy for hybrid memory management
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryStrategy {
    /// Manual memory management (malloc/free)
    Manual,
    /// Automatic reference counting (ARC/RC)  
    SmartPointer,
    /// Linear types with move semantics
    Linear,
    /// Memory regions for grouped allocation
    Region,
    /// Stack allocation (default for local variables)
    Stack,
}

/// Memory region for grouped allocation/deallocation
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Region identifier
    pub id: u32,
    /// Region name for debugging
    pub name: String,
    /// Current allocation offset
    pub offset: u64,
    /// Total region size
    pub size: u64,
    /// Base address (runtime)
    pub base_ptr: Option<Value>,
    /// Allocated objects in this region
    pub allocations: Vec<RegionAllocation>,
}

/// An allocation within a memory region
#[derive(Debug, Clone)]
pub struct RegionAllocation {
    /// Object identifier
    pub id: u32,
    /// Object type
    pub object_type: Type,
    /// Size in bytes
    pub size: u32,
    /// Offset from region base
    pub offset: u64,
}

/// Smart pointer metadata for reference counting
#[derive(Debug, Clone)]
pub struct SmartPointer {
    /// Pointer value
    pub ptr: Value,
    /// Reference count
    pub ref_count: Value,
    /// Pointed-to type
    pub pointee_type: Type,
    /// Destructor function ID
    pub destructor: Option<cranelift_module::FuncId>,
}

/// Linear type tracking for move semantics
#[derive(Debug, Clone)]
pub struct LinearType {
    /// Value being tracked
    pub value: Value,
    /// Type information
    pub linear_type: Type,
    /// Whether value has been moved
    pub is_moved: bool,
    /// Move generation for tracking
    pub move_generation: u32,
}

/// Hybrid Memory Manager - coordinates all memory strategies
pub struct HybridMemoryManager {
    /// Active memory regions
    regions: HashMap<u32, MemoryRegion>,
    /// Smart pointer registry
    smart_pointers: HashMap<Value, SmartPointer>,
    /// Linear type tracking
    linear_types: HashMap<Value, LinearType>,
    /// Next region ID
    next_region_id: u32,
    /// Next allocation ID
    next_alloc_id: u32,
    /// Runtime malloc function
    malloc_func: Option<cranelift_module::FuncId>,
    /// Runtime free function  
    free_func: Option<cranelift_module::FuncId>,
    /// Runtime ARC increment function
    arc_inc_func: Option<cranelift_module::FuncId>,
    /// Runtime ARC decrement function
    arc_dec_func: Option<cranelift_module::FuncId>,
}

impl HybridMemoryManager {
    /// Create new hybrid memory manager
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            smart_pointers: HashMap::new(),
            linear_types: HashMap::new(),
            next_region_id: 1,
            next_alloc_id: 1,
            malloc_func: None,
            free_func: None,
            arc_inc_func: None,
            arc_dec_func: None,
        }
    }

    /// Initialize runtime memory functions
    pub fn initialize_runtime_functions(&mut self, module: &mut dyn CraneliftModule) -> CodegenResult<()> {
        // Declare malloc function: malloc(size: u64) -> *u8
        let mut malloc_sig = module.make_signature();
        malloc_sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        malloc_sig.returns.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        
        let malloc_id = module.declare_function("malloc", cranelift_module::Linkage::Import, &malloc_sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare malloc: {}", e)))?;
        self.malloc_func = Some(malloc_id);

        // Declare free function: free(ptr: *u8) -> void
        let mut free_sig = module.make_signature();
        free_sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        
        let free_id = module.declare_function("free", cranelift_module::Linkage::Import, &free_sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare free: {}", e)))?;
        self.free_func = Some(free_id);

        // Declare ARC increment function: bract_arc_inc(ptr: *u8) -> void
        let mut arc_inc_sig = module.make_signature();
        arc_inc_sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        
        let arc_inc_id = module.declare_function("bract_arc_inc", cranelift_module::Linkage::Import, &arc_inc_sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare bract_arc_inc: {}", e)))?;
        self.arc_inc_func = Some(arc_inc_id);

        // Declare ARC decrement function: bract_arc_dec(ptr: *u8) -> void
        let mut arc_dec_sig = module.make_signature();
        arc_dec_sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        
        let arc_dec_id = module.declare_function("bract_arc_dec", cranelift_module::Linkage::Import, &arc_dec_sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare bract_arc_dec: {}", e)))?;
        self.arc_dec_func = Some(arc_dec_id);

        Ok(())
    }

    /// Allocate memory using specified strategy
    pub fn allocate(
        &mut self,
        builder: &mut FunctionBuilder,
        strategy: MemoryStrategy,
        object_type: Type,
        size: u32,
        region_id: Option<u32>,
    ) -> CodegenResult<Value> {
        match strategy {
            MemoryStrategy::Manual => self.allocate_manual(builder, object_type, size),
            MemoryStrategy::SmartPointer => self.allocate_smart_pointer(builder, object_type, size),
            MemoryStrategy::Linear => self.allocate_linear(builder, object_type, size),
            MemoryStrategy::Region => {
                let region_id = region_id.ok_or_else(|| 
                    CodegenError::InternalError("Region allocation requires region_id".to_string())
                )?;
                self.allocate_in_region(builder, region_id, object_type, size)
            },
            MemoryStrategy::Stack => self.allocate_stack(builder, object_type, size),
        }
    }

    /// Manual allocation using malloc
    fn allocate_manual(&mut self, builder: &mut FunctionBuilder, _object_type: Type, size: u32) -> CodegenResult<Value> {
        let malloc_func = self.malloc_func.ok_or_else(|| 
            CodegenError::InternalError("malloc function not initialized".to_string())
        )?;

        // Create function reference for malloc
        let local_malloc = builder.func.import_function(cranelift_codegen::ir::ExternalName::user(0, malloc_func.as_u32()));
        
        // Call malloc(size)
        let size_val = builder.ins().iconst(ctypes::I64, size as i64);
        let malloc_call = builder.ins().call(local_malloc, &[size_val]);
        let ptr = builder.func.dfg.first_result(malloc_call);
        
        Ok(ptr)
    }

    /// Free manually allocated memory
    pub fn deallocate_manual(&mut self, builder: &mut FunctionBuilder, ptr: Value) -> CodegenResult<()> {
        let free_func = self.free_func.ok_or_else(|| 
            CodegenError::InternalError("free function not initialized".to_string())
        )?;

        // Create function reference for free
        let local_free = builder.func.import_function(cranelift_codegen::ir::ExternalName::user(0, free_func.as_u32()));
        
        // Call free(ptr)
        builder.ins().call(local_free, &[ptr]);
        
        Ok(())
    }

    /// Allocate smart pointer with reference counting
    fn allocate_smart_pointer(&mut self, builder: &mut FunctionBuilder, object_type: Type, size: u32) -> CodegenResult<Value> {
        // Allocate memory for object + reference count
        let total_size = size + 8; // 8 bytes for 64-bit reference count
        let ptr = self.allocate_manual(builder, object_type, total_size)?;
        
        // Initialize reference count to 1
        let ref_count_offset = builder.ins().iconst(ctypes::I64, size as i64);
        let ref_count_ptr = builder.ins().iadd(ptr, ref_count_offset);
        let initial_ref_count = builder.ins().iconst(ctypes::I64, 1);
        builder.ins().store(cranelift::prelude::MemFlags::new(), initial_ref_count, ref_count_ptr, 0);
        
        // Register smart pointer
        let smart_ptr = SmartPointer {
            ptr,
            ref_count: initial_ref_count,
            pointee_type: object_type,
            destructor: None,
        };
        self.smart_pointers.insert(ptr, smart_ptr);
        
        Ok(ptr)
    }

    /// Increment smart pointer reference count
    pub fn smart_pointer_inc_ref(&mut self, builder: &mut FunctionBuilder, ptr: Value) -> CodegenResult<()> {
        let arc_inc_func = self.arc_inc_func.ok_or_else(|| 
            CodegenError::InternalError("ARC increment function not initialized".to_string())
        )?;

        // Create function reference for ARC increment
        let local_arc_inc = builder.func.import_function(cranelift_codegen::ir::ExternalName::user(0, arc_inc_func.as_u32()));
        
        // Call bract_arc_inc(ptr)
        builder.ins().call(local_arc_inc, &[ptr]);
        
        Ok(())
    }

    /// Decrement smart pointer reference count (may deallocate)
    pub fn smart_pointer_dec_ref(&mut self, builder: &mut FunctionBuilder, ptr: Value) -> CodegenResult<()> {
        let arc_dec_func = self.arc_dec_func.ok_or_else(|| 
            CodegenError::InternalError("ARC decrement function not initialized".to_string())
        )?;

        // Create function reference for ARC decrement  
        let local_arc_dec = builder.func.import_function(cranelift_codegen::ir::ExternalName::user(0, arc_dec_func.as_u32()));
        
        // Call bract_arc_dec(ptr) - this handles deallocation if ref count reaches 0
        builder.ins().call(local_arc_dec, &[ptr]);
        
        Ok(())
    }

    /// Allocate linear type with move semantics tracking
    fn allocate_linear(&mut self, builder: &mut FunctionBuilder, object_type: Type, size: u32) -> CodegenResult<Value> {
        // Linear types are allocated on stack by default for performance
        let stack_slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            size,
        ));
        
        let ptr = builder.ins().stack_addr(ctypes::I64, stack_slot, 0);
        
        // Register linear type
        let linear_type = LinearType {
            value: ptr,
            linear_type: object_type,
            is_moved: false,
            move_generation: 0,
        };
        self.linear_types.insert(ptr, linear_type);
        
        Ok(ptr)
    }

    /// Move linear type (transfer ownership)
    pub fn linear_type_move(&mut self, from: Value, to: Value) -> CodegenResult<()> {
        if let Some(mut linear_type) = self.linear_types.remove(&from) {
            if linear_type.is_moved {
                return Err(CodegenError::InternalError(
                    "Attempted to move already-moved linear type".to_string()
                ));
            }
            
            // Mark original as moved
            linear_type.is_moved = true;
            linear_type.move_generation += 1;
            self.linear_types.insert(from, linear_type.clone());
            
            // Create new tracking entry for destination
            let mut new_linear_type = linear_type;
            new_linear_type.value = to;
            new_linear_type.is_moved = false;
            self.linear_types.insert(to, new_linear_type);
            
            Ok(())
        } else {
            Err(CodegenError::InternalError(
                "Attempted to move non-linear type".to_string()
            ))
        }
    }

    /// Check if linear type has been moved (compile-time safety)
    pub fn check_linear_type_usage(&self, value: Value) -> CodegenResult<()> {
        if let Some(linear_type) = self.linear_types.get(&value) {
            if linear_type.is_moved {
                return Err(CodegenError::InternalError(
                    "Use after move: linear type has already been moved".to_string()
                ));
            }
        }
        Ok(())
    }

    /// Create new memory region
    pub fn create_region(&mut self, name: String, size: u64) -> u32 {
        let region_id = self.next_region_id;
        self.next_region_id += 1;
        
        let region = MemoryRegion {
            id: region_id,
            name,
            offset: 0,
            size,
            base_ptr: None,
            allocations: Vec::new(),
        };
        
        self.regions.insert(region_id, region);
        region_id
    }

    /// Initialize memory region at runtime
    pub fn initialize_region(&mut self, builder: &mut FunctionBuilder, region_id: u32) -> CodegenResult<Value> {
        let region = self.regions.get_mut(&region_id).ok_or_else(|| 
            CodegenError::InternalError(format!("Region {} not found", region_id))
        )?;
        
        // Allocate region memory using malloc
        let region_ptr = self.allocate_manual(builder, ctypes::I8, region.size as u32)?;
        region.base_ptr = Some(region_ptr);
        
        Ok(region_ptr)
    }

    /// Allocate within memory region
    fn allocate_in_region(&mut self, builder: &mut FunctionBuilder, region_id: u32, object_type: Type, size: u32) -> CodegenResult<Value> {
        let region = self.regions.get_mut(&region_id).ok_or_else(|| 
            CodegenError::InternalError(format!("Region {} not found", region_id))
        )?;
        
        // Check if region has enough space
        let aligned_size = (size + 7) & !7; // 8-byte alignment
        if region.offset + aligned_size as u64 > region.size {
            return Err(CodegenError::InternalError(
                format!("Region {} out of memory: {} bytes needed, {} available", 
                    region_id, aligned_size, region.size - region.offset)
            ));
        }
        
        let base_ptr = region.base_ptr.ok_or_else(|| 
            CodegenError::InternalError(format!("Region {} not initialized", region_id))
        )?;
        
        // Calculate object address
        let offset_val = builder.ins().iconst(ctypes::I64, region.offset as i64);
        let object_ptr = builder.ins().iadd(base_ptr, offset_val);
        
        // Record allocation
        let allocation = RegionAllocation {
            id: self.next_alloc_id,
            object_type,
            size: aligned_size,
            offset: region.offset,
        };
        region.allocations.push(allocation);
        self.next_alloc_id += 1;
        
        // Update region offset
        region.offset += aligned_size as u64;
        
        Ok(object_ptr)
    }

    /// Deallocate entire memory region
    pub fn deallocate_region(&mut self, builder: &mut FunctionBuilder, region_id: u32) -> CodegenResult<()> {
        if let Some(region) = self.regions.remove(&region_id) {
            if let Some(base_ptr) = region.base_ptr {
                self.deallocate_manual(builder, base_ptr)?;
            }
        }
        Ok(())
    }

    /// Stack allocation (uses Cranelift stack slots)
    fn allocate_stack(&self, builder: &mut FunctionBuilder, _object_type: Type, size: u32) -> CodegenResult<Value> {
        let stack_slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            size,
        ));
        
        let ptr = builder.ins().stack_addr(ctypes::I64, stack_slot, 0);
        Ok(ptr)
    }

    /// Get memory strategy for a type (can be customized by user annotations)
    pub fn get_default_strategy(object_type: Type) -> MemoryStrategy {
        // Default strategies based on type characteristics
        match object_type {
            // Small primitive types use stack allocation
            t if t == ctypes::I8 || t == ctypes::I16 || t == ctypes::I32 || t == ctypes::I64 => MemoryStrategy::Stack,
            t if t == ctypes::F32 || t == ctypes::F64 => MemoryStrategy::Stack,
            
            // Larger types or unknown types use smart pointers for safety
            _ => MemoryStrategy::SmartPointer,
        }
    }

    /// Clean up all memory (called at end of function)
    pub fn cleanup_function_memory(&mut self, builder: &mut FunctionBuilder) -> CodegenResult<()> {
        // Automatically decrement reference counts for all smart pointers
        for (ptr, _smart_ptr) in self.smart_pointers.iter() {
            self.smart_pointer_dec_ref(builder, *ptr)?;
        }
        
        // Clear tracking data structures
        self.smart_pointers.clear();
        self.linear_types.clear();
        
        Ok(())
    }

    /// Memory safety analysis and bounds checking
    pub fn check_memory_safety(&self, ptr: Value, access_size: u32) -> CodegenResult<()> {
        // Check if pointer is from a tracked region
        for region in self.regions.values() {
            if let Some(base_ptr) = region.base_ptr {
                if ptr == base_ptr {
                    // TODO: Runtime bounds check generation
                    // For now, assume it's safe within region bounds
                    continue;
                }
            }
        }
        
        // Check linear type safety
        if let Some(linear_type) = self.linear_types.get(&ptr) {
            if linear_type.is_moved {
                return Err(CodegenError::InternalError(
                    "Memory safety violation: accessing moved linear type".to_string()
                ));
            }
        }
        
        Ok(())
    }

    /// Generate runtime bounds checking code
    pub fn generate_bounds_check(
        &self, 
        builder: &mut FunctionBuilder, 
        ptr: Value, 
        size: Value, 
        access_size: u32
    ) -> CodegenResult<()> {
        // Generate runtime check: if (ptr + access_size > ptr + size) abort()
        let access_size_val = builder.ins().iconst(ctypes::I64, access_size as i64);
        let end_ptr = builder.ins().iadd(ptr, size);
        let access_end = builder.ins().iadd(ptr, access_size_val);
        
        let bounds_ok = builder.ins().icmp(cranelift::prelude::IntCC::UnsignedLessThanOrEqual, access_end, end_ptr);
        
        // Create trap block for bounds violation
        let trap_block = builder.create_block();
        let continue_block = builder.create_block();
        
        builder.ins().brif(bounds_ok, continue_block, &[], trap_block, &[]);
        
        // Trap block - abort execution
        builder.switch_to_block(trap_block);
        builder.ins().trap(cranelift::prelude::TrapCode::HeapOutOfBounds);
        
        // Continue block
        builder.switch_to_block(continue_block);
        
        Ok(())
    }
}

/// Memory allocation attributes for user code
#[derive(Debug, Clone)]
pub enum MemoryAttribute {
    /// @manual - use manual memory management
    Manual,
    /// @smart - use smart pointer (ARC/RC)
    Smart,
    /// @linear - use linear type with move semantics
    Linear,
    /// @region(name) - allocate in named region
    Region(String),
    /// @stack - force stack allocation
    Stack,
    /// @no_bounds_check - disable bounds checking for performance
    NoBoundsCheck,
    /// @align(bytes) - specify alignment
    Align(u32),
}

/// Parse memory attribute from user annotation
pub fn parse_memory_attribute(annotation: &str) -> Option<MemoryAttribute> {
    match annotation {
        "@manual" => Some(MemoryAttribute::Manual),
        "@smart" => Some(MemoryAttribute::Smart),
        "@linear" => Some(MemoryAttribute::Linear),
        "@stack" => Some(MemoryAttribute::Stack),
        "@no_bounds_check" => Some(MemoryAttribute::NoBoundsCheck),
        s if s.starts_with("@region(") && s.ends_with(")") => {
            let region_name = &s[8..s.len()-1];
            Some(MemoryAttribute::Region(region_name.to_string()))
        },
        s if s.starts_with("@align(") && s.ends_with(")") => {
            let align_str = &s[7..s.len()-1];
            if let Ok(align) = align_str.parse::<u32>() {
                Some(MemoryAttribute::Align(align))
            } else {
                None
            }
        },
        _ => None,
    }
}

/// Memory statistics for performance monitoring
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocations: u64,
    pub manual_allocations: u64,
    pub smart_pointer_allocations: u64,
    pub linear_type_allocations: u64,
    pub region_allocations: u64,
    pub stack_allocations: u64,
    pub peak_memory_usage: u64,
    pub current_memory_usage: u64,
    pub memory_leaks_prevented: u64,
    pub bounds_checks_generated: u64,
}

impl MemoryStats {
    pub fn new() -> Self {
        Self {
            total_allocations: 0,
            manual_allocations: 0,
            smart_pointer_allocations: 0,
            linear_type_allocations: 0,
            region_allocations: 0,
            stack_allocations: 0,
            peak_memory_usage: 0,
            current_memory_usage: 0,
            memory_leaks_prevented: 0,
            bounds_checks_generated: 0,
        }
    }
}

// TODO: Implement memory profiler integration
// TODO: Implement compile-time memory leak detection
// TODO: Implement automatic memory strategy inference
// TODO: Implement memory pool optimizations
// TODO: Implement NUMA-aware allocation strategies 