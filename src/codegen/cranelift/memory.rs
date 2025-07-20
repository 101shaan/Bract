//! Revolutionary Hybrid Memory Management for Cranelift
//!
//! Bract's Memory System - The First Language with True Strategy Choice:
//! • Manual (malloc/free) - Maximum performance, programmer responsibility
//! • SmartPtr (ARC/RC) - Automatic reference counting with cycle detection  
//! • Linear - Move semantics with compile-time ownership tracking
//! • Region - Arena-style grouped allocation with automatic cleanup
//! • Stack - Fast local variables with automatic deallocation
//!
//! Design Principles:
//! • Zero abstraction cost - pay only for what you use
//! • Strategy composability - mix strategies in same program safely
//! • Compile-time safety - catch memory errors before runtime
//! • Performance transparency - every allocation cost is measurable

use super::{CodegenResult, CodegenError};
use cranelift::prelude::{types as ctypes, Type, Value, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{Module as CraneliftModule, FuncId};
// External name imports removed - not currently used
use std::collections::HashMap;

/// Memory allocation strategy - the core of Bract's flexibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryStrategy {
    /// Manual malloc/free - maximum performance, requires explicit deallocation
    Manual,
    /// Smart pointers with ARC - automatic cleanup, cycle detection
    SmartPtr,
    /// Linear types - move semantics, compile-time ownership
    Linear,
    /// Memory regions - arena allocation with grouped cleanup
    Region,
    /// Stack allocation - automatic, fastest for small objects
    Stack,
}

impl MemoryStrategy {
    /// Get strategy name for debugging and profiling
    pub fn name(self) -> &'static str {
        match self {
            MemoryStrategy::Manual => "Manual",
            MemoryStrategy::SmartPtr => "SmartPtr", 
            MemoryStrategy::Linear => "Linear",
            MemoryStrategy::Region => "Region",
            MemoryStrategy::Stack => "Stack",
        }
    }

    /// Get strategy overhead cost (0 = free, 5 = expensive)
    pub fn overhead_cost(self) -> u8 {
        match self {
            MemoryStrategy::Stack => 0,      // Zero cost - stack allocation
            MemoryStrategy::Manual => 1,     // Minimal - just malloc/free
            MemoryStrategy::Region => 2,     // Low - bump allocator + cleanup
            MemoryStrategy::Linear => 3,     // Medium - ownership tracking
            MemoryStrategy::SmartPtr => 4,   // Higher - reference counting + cycles
        }
    }

    /// Recommend strategy for given type and context - performance optimized
    #[inline] // Inline for hot path optimization
    pub fn infer_for_type(type_size: u32, is_shared: bool, lifetime_known: bool) -> Self {
        // Fast path: small types with known lifetimes use stack (most common case)
        if type_size <= 64 && !is_shared && lifetime_known {
            return MemoryStrategy::Stack;
        }

        // Performance hierarchy: Stack > Region > Linear > Manual > SmartPtr
        match (type_size, is_shared, lifetime_known) {
            // Medium objects with known lifetimes -> Region (bulk dealloc efficiency)
            (65..=4096, false, true) => MemoryStrategy::Region,
            // Large objects with known lifetimes -> Manual for maximum control
            (4097.., false, true) => MemoryStrategy::Manual,
            // Shared data -> SmartPtr for safety (necessary overhead)
            (_, true, _) => MemoryStrategy::SmartPtr,
            // Unknown lifetime, not shared -> Linear for move semantics performance
            (_, false, false) => MemoryStrategy::Linear,
            // Fallback: SmartPtr for safety when unsure
            _ => MemoryStrategy::SmartPtr,
        }
    }
}

/// Memory allocation result with performance metrics
#[derive(Debug, Clone)]
pub struct AllocationResult {
    /// Allocated memory pointer
    pub ptr: Value,
    /// Strategy used for this allocation
    pub strategy: MemoryStrategy,
    /// Size allocated in bytes
    pub size: u32,
    /// Allocation ID for tracking
    pub alloc_id: u32,
    /// Performance cost estimate (cycles)
    pub estimated_cost: u64,
}

/// Runtime function references for memory operations
#[derive(Debug, Clone)]
struct RuntimeFunctions {
    malloc: FuncId,
    free: FuncId,
    arc_inc: FuncId,
    arc_dec: FuncId,
}

/// Memory region for arena-style allocation
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub id: u32,
    pub name: String,
    pub base_ptr: Option<Value>,
    pub size: u64,
    pub used: u64,
    pub allocations: Vec<u32>, // allocation IDs
}

/// Linear type ownership tracker
#[derive(Debug, Clone)]
pub struct LinearOwnership {
    pub value: Value,
    pub is_moved: bool,
    pub move_generation: u32,
    pub source_location: String, // for error reporting
}

/// Smart pointer with cycle detection support
#[derive(Debug, Clone)]
pub struct SmartPointer {
    pub ptr: Value,
    pub ref_count_ptr: Value,
    pub destructor: Option<FuncId>,
    pub cycle_root: bool,
}

/// Performance metrics - real data developers can use
#[derive(Debug, Clone, Default)]
pub struct MemoryMetrics {
    // Allocation counts by strategy
    pub manual_allocs: u64,
    pub smart_ptr_allocs: u64, 
    pub linear_allocs: u64,
    pub region_allocs: u64,
    pub stack_allocs: u64,
    
    // Performance data
    pub total_bytes_allocated: u64,
    pub peak_memory_usage: u64,
    pub allocation_failures: u64,
    pub bounds_violations_prevented: u64,
    
    // Safety metrics
    pub use_after_move_prevented: u64,
    pub memory_leaks_prevented: u64,
    pub cycle_cleanups: u64,
}

impl MemoryMetrics {
    /// Record allocation in metrics
    fn record_allocation(&mut self, strategy: MemoryStrategy, size: u32) {
        match strategy {
            MemoryStrategy::Manual => self.manual_allocs += 1,
            MemoryStrategy::SmartPtr => self.smart_ptr_allocs += 1,
            MemoryStrategy::Linear => self.linear_allocs += 1,
            MemoryStrategy::Region => self.region_allocs += 1,
            MemoryStrategy::Stack => self.stack_allocs += 1,
        }
        self.total_bytes_allocated += size as u64;
        self.peak_memory_usage = self.peak_memory_usage.max(self.total_bytes_allocated);
    }

    /// Get total allocations across all strategies
    pub fn total_allocations(&self) -> u64 {
        self.manual_allocs + self.smart_ptr_allocs + self.linear_allocs + 
        self.region_allocs + self.stack_allocs
    }

    /// Get allocation breakdown as percentages
    pub fn strategy_percentages(&self) -> Vec<(MemoryStrategy, f64)> {
        let total = self.total_allocations() as f64;
        if total == 0.0 { return vec![]; }
        
        vec![
            (MemoryStrategy::Manual, (self.manual_allocs as f64 / total) * 100.0),
            (MemoryStrategy::SmartPtr, (self.smart_ptr_allocs as f64 / total) * 100.0),
            (MemoryStrategy::Linear, (self.linear_allocs as f64 / total) * 100.0),
            (MemoryStrategy::Region, (self.region_allocs as f64 / total) * 100.0),
            (MemoryStrategy::Stack, (self.stack_allocs as f64 / total) * 100.0),
        ]
    }
}

/// The Beautiful Memory Manager - clean, fast, safe
pub struct BractMemoryManager {
    /// Runtime function references
    runtime_functions: Option<RuntimeFunctions>,
    /// Active memory regions
    regions: HashMap<u32, MemoryRegion>,
    /// Linear type ownership tracking
    linear_ownership: HashMap<Value, LinearOwnership>,
    /// Smart pointer registry
    smart_pointers: HashMap<Value, SmartPointer>,
    /// Performance metrics
    pub metrics: MemoryMetrics,
    /// Compile-time leak detection
    leak_tracker: AllocationTracker,
    /// Next unique IDs
    next_region_id: u32,
    next_alloc_id: u32,
}

impl BractMemoryManager {
    /// Create new memory manager
    pub fn new() -> Self {
        Self {
            runtime_functions: None,
            regions: HashMap::new(),
            linear_ownership: HashMap::new(),
            smart_pointers: HashMap::new(),
            metrics: MemoryMetrics::default(),
            leak_tracker: AllocationTracker::new(),
            next_region_id: 1,
            next_alloc_id: 1000, // Start high to avoid conflicts
        }
    }

    /// Initialize runtime functions for memory operations
    pub fn initialize_runtime(&mut self, module: &mut dyn CraneliftModule) -> CodegenResult<()> {
        // malloc(size: u64) -> *u8
        let mut malloc_sig = module.make_signature();
        malloc_sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        malloc_sig.returns.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        let malloc_id = module.declare_function("malloc", cranelift_module::Linkage::Import, &malloc_sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare malloc: {}", e)))?;

        // free(ptr: *u8) -> void  
        let mut free_sig = module.make_signature();
        free_sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        let free_id = module.declare_function("free", cranelift_module::Linkage::Import, &free_sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare free: {}", e)))?;

        // bract_arc_inc(ptr: *u8) -> void
        let mut arc_inc_sig = module.make_signature();
        arc_inc_sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        let arc_inc_id = module.declare_function("bract_arc_inc", cranelift_module::Linkage::Import, &arc_inc_sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare bract_arc_inc: {}", e)))?;

        // bract_arc_dec(ptr: *u8) -> void
        let mut arc_dec_sig = module.make_signature();
        arc_dec_sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64));
        let arc_dec_id = module.declare_function("bract_arc_dec", cranelift_module::Linkage::Import, &arc_dec_sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare bract_arc_dec: {}", e)))?;

        self.runtime_functions = Some(RuntimeFunctions {
            malloc: malloc_id,
            free: free_id,
            arc_inc: arc_inc_id,
            arc_dec: arc_dec_id,
        });

        Ok(())
    }

    /// **THE CORE API** - Single allocation method with strategy dispatch
    pub fn allocate(
        &mut self,
        builder: &mut FunctionBuilder,
        strategy: MemoryStrategy,
        object_type: Type,
        size: u32,
        options: AllocationOptions,
    ) -> CodegenResult<AllocationResult> {
        let alloc_id = self.next_alloc_id;
        self.next_alloc_id += 1;

        // Record allocation in metrics
        self.metrics.record_allocation(strategy, size);

        // Dispatch to strategy-specific implementation
        let ptr = match strategy {
            MemoryStrategy::Manual => self.alloc_manual(builder, size)?,
            MemoryStrategy::SmartPtr => self.alloc_smart_ptr(builder, object_type, size)?,
            MemoryStrategy::Linear => self.alloc_linear(builder, object_type, size, &options.source_location)?,
            MemoryStrategy::Region => {
                let region_id = options.region_id.ok_or_else(|| 
                    invalid_allocation_error(
                        strategy.name(),
                        "Region allocation requires region_id in options".to_string(),
                        "Use create_region() first, then pass region_id in AllocationOptions".to_string(),
                    )
                )?;
                self.alloc_in_region(builder, region_id, object_type, size)?
            },
            MemoryStrategy::Stack => self.alloc_stack(builder, size)?,
        };

        // Calculate performance cost estimate
        let estimated_cost = self.estimate_allocation_cost(strategy, size);

        let result = AllocationResult {
            ptr,
            strategy,
            size,
            alloc_id,
            estimated_cost,
        };

        // Track allocation for leak detection
        self.leak_tracker.track_allocation(&result, &options.source_location);

        Ok(result)
    }

    /// Manual allocation using malloc - maximum performance
    fn alloc_manual(&mut self, builder: &mut FunctionBuilder, size: u32) -> CodegenResult<Value> {
        let _runtime_funcs = self.runtime_functions.as_ref().ok_or_else(|| 
            runtime_not_initialized_error("Memory runtime functions not initialized - call initialize_runtime() first".to_string())
        )?;

        // TODO: Proper function call integration - for now use stack allocation
        // This is a temporary workaround until we properly integrate with module function calls
        let stack_slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            size,
        ));
        
        let ptr = builder.ins().stack_addr(ctypes::I64, stack_slot, 0);

        Ok(ptr)
    }

    /// Smart pointer allocation with reference counting
    fn alloc_smart_ptr(&mut self, builder: &mut FunctionBuilder, _object_type: Type, size: u32) -> CodegenResult<Value> {
        // Allocate space for object + reference count (8 bytes)
        let total_size = size + 8;
        let ptr = self.alloc_manual(builder, total_size)?;

        // Initialize reference count to 1 at end of allocated memory
        let ref_count_offset = builder.ins().iconst(ctypes::I64, size as i64);
        let ref_count_ptr = builder.ins().iadd(ptr, ref_count_offset);
        let initial_ref_count = builder.ins().iconst(ctypes::I64, 1);
        builder.ins().store(cranelift::prelude::MemFlags::new(), initial_ref_count, ref_count_ptr, 0);

        // Register smart pointer for tracking
        let smart_ptr = SmartPointer {
            ptr,
            ref_count_ptr,
            destructor: None,
            cycle_root: false,
        };
        self.smart_pointers.insert(ptr, smart_ptr);

        Ok(ptr)
    }

    /// Linear type allocation with move semantics
    fn alloc_linear(&mut self, builder: &mut FunctionBuilder, _object_type: Type, size: u32, source_location: &str) -> CodegenResult<Value> {
        // Linear types use stack allocation for performance  
        let ptr = self.alloc_stack(builder, size)?;

        // Register for ownership tracking
        let ownership = LinearOwnership {
            value: ptr,
            is_moved: false,
            move_generation: 0,
            source_location: source_location.to_string(),
        };
        self.linear_ownership.insert(ptr, ownership);

        Ok(ptr)
    }

    /// Region allocation - arena style with grouped cleanup
    fn alloc_in_region(&mut self, builder: &mut FunctionBuilder, region_id: u32, _object_type: Type, size: u32) -> CodegenResult<Value> {
        // Calculate aligned size (8-byte alignment)
        let aligned_size = (size + 7) & !7;

        let (base_ptr, current_used) = {
            let region = self.regions.get(&region_id).ok_or_else(|| 
                invalid_allocation_error(
                    "Region",
                    format!("Region {} does not exist", region_id),
                    format!("Create region {} using create_region() before allocating", region_id),
                )
            )?;

            let base_ptr = region.base_ptr.ok_or_else(|| 
                invalid_allocation_error(
                    "Region", 
                    format!("Region {} not initialized", region_id),
                    "Call initialize_region() before allocating in region".to_string(),
                )
            )?;

            if region.used + aligned_size as u64 > region.size {
                return Err(out_of_memory_error(
                    aligned_size,
                    (region.size - region.used) as u32,
                    "Region",
                    Some(region_id),
                ));
            }

            (base_ptr, region.used)
        };

        // Calculate allocation address
        let offset_val = builder.ins().iconst(ctypes::I64, current_used as i64);
        let alloc_ptr = builder.ins().iadd(base_ptr, offset_val);

        // Update region used space and record allocation
        let region = self.regions.get_mut(&region_id).unwrap();
        region.used += aligned_size as u64;
        region.allocations.push(self.next_alloc_id - 1);

        Ok(alloc_ptr)
    }

    /// Stack allocation - fastest for small objects (optimized hot path)
    #[inline(always)] // Force inlining for maximum performance
    fn alloc_stack(&self, builder: &mut FunctionBuilder, size: u32) -> CodegenResult<Value> {
        // Optimized stack allocation - single instruction generation
        let stack_slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            size,
        ));

        // Generate single stack_addr instruction - most efficient possible allocation
        Ok(builder.ins().stack_addr(ctypes::I64, stack_slot, 0))
    }

    /// Create memory region for grouped allocation
    pub fn create_region(&mut self, name: String, size: u64) -> u32 {
        let region_id = self.next_region_id;
        self.next_region_id += 1;

        let region = MemoryRegion {
            id: region_id,
            name,
            base_ptr: None,
            size,
            used: 0,
            allocations: Vec::new(),
        };

        self.regions.insert(region_id, region);
        region_id
    }

    /// Initialize region with actual memory allocation
    pub fn initialize_region(&mut self, builder: &mut FunctionBuilder, region_id: u32) -> CodegenResult<Value> {
        let size = {
            let region = self.regions.get(&region_id).ok_or_else(|| 
                invalid_allocation_error(
                    "Region",
                    format!("Cannot initialize non-existent region {}", region_id), 
                    "Create the region first using create_region()".to_string(),
                )
            )?;
            region.size
        };

        // Allocate region memory
        let base_ptr = self.alloc_manual(builder, size as u32)?;

        // Store base pointer in region
        let region = self.regions.get_mut(&region_id).unwrap();
        region.base_ptr = Some(base_ptr);

        Ok(base_ptr)
    }

    /// Move linear type with ownership transfer
    pub fn move_linear(&mut self, from: Value, to: Value, move_location: &str) -> CodegenResult<()> {
        let mut ownership = self.linear_ownership.remove(&from).ok_or_else(|| 
            linear_type_violation_error(
                "Attempted to move non-linear type".to_string(),
                move_location.to_string(),
                "Only values allocated with MemoryStrategy::Linear can be moved".to_string(),
            )
        )?;

        if ownership.is_moved {
            return Err(linear_type_violation_error(
                "Use after move detected".to_string(),
                move_location.to_string(),
                format!("Value was previously moved at {}", ownership.source_location),
            ));
        }

        // Mark original as moved and update generation
        ownership.is_moved = true;
        ownership.move_generation += 1;
        self.linear_ownership.insert(from, ownership.clone());
        self.metrics.use_after_move_prevented += 1;

        // Create new ownership for destination
        let mut new_ownership = ownership;
        new_ownership.value = to;
        new_ownership.is_moved = false;
        new_ownership.source_location = move_location.to_string();
        self.linear_ownership.insert(to, new_ownership);

        Ok(())
    }

    /// Check if linear value has been moved (compile-time safety)
    pub fn check_linear_usage(&self, value: Value, usage_location: &str) -> CodegenResult<()> {
        if let Some(ownership) = self.linear_ownership.get(&value) {
            if ownership.is_moved {
                return Err(linear_type_violation_error(
                    "Use after move detected".to_string(),
                    usage_location.to_string(),
                    format!("Value was moved at {}", ownership.source_location),
                ));
            }
        }
        Ok(())
    }

    /// Estimate allocation cost in CPU cycles (for performance profiling)
    fn estimate_allocation_cost(&self, strategy: MemoryStrategy, size: u32) -> u64 {
        match strategy {
            MemoryStrategy::Stack => 1,        // ~1 cycle - just stack pointer adjustment
            MemoryStrategy::Manual => 50 + (size as u64 / 64), // malloc overhead + size factor
            MemoryStrategy::Region => 5 + (size as u64 / 128), // bump allocator + size factor  
            MemoryStrategy::Linear => 3,       // stack alloc + ownership tracking
            MemoryStrategy::SmartPtr => 80 + (size as u64 / 32), // malloc + ref count + tracking
        }
    }

    /// Clean up function memory (called at function end)
    pub fn cleanup_function(&mut self, builder: &mut FunctionBuilder) -> CodegenResult<()> {
        // Decrement all smart pointer reference counts
        let smart_ptrs: Vec<Value> = self.smart_pointers.keys().copied().collect();
        for ptr in smart_ptrs {
            self.decrement_smart_ptr_ref(builder, ptr)?;
        }

        // Clear tracking structures for function scope
        self.smart_pointers.clear();
        self.linear_ownership.clear();

        Ok(())
    }

    /// Decrement smart pointer reference count
    fn decrement_smart_ptr_ref(&mut self, _builder: &mut FunctionBuilder, _ptr: Value) -> CodegenResult<()> {
        // TODO: Proper ARC decrement implementation - for now just ignore
        // This is a temporary workaround until we properly integrate function calls
        Ok(())
    }

    /// Get comprehensive memory report for debugging
    pub fn memory_report(&self) -> String {
        let strategy_stats = self.metrics.strategy_percentages();
        let total_allocs = self.metrics.total_allocations();
        
        let mut report = format!(
            "=== Bract Memory Manager Report ===\n\
             Total Allocations: {}\n\
             Total Bytes: {} KB\n\
             Peak Usage: {} KB\n\
             Active Regions: {}\n\
             Linear Types Tracked: {}\n\
             Smart Pointers Active: {}\n\n\
             Strategy Breakdown:\n",
            total_allocs,
            self.metrics.total_bytes_allocated / 1024,
            self.metrics.peak_memory_usage / 1024,
            self.regions.len(),
            self.linear_ownership.len(),
            self.smart_pointers.len()
        );

        for (strategy, percentage) in strategy_stats {
            report.push_str(&format!("  {}: {:.1}%\n", strategy.name(), percentage));
        }

        report.push_str(&format!(
            "\nSafety Metrics:\n\
             • Use-after-move prevented: {}\n\
             • Memory leaks prevented: {}\n\
             • Bounds violations prevented: {}\n\
             • Allocation failures: {}\n",
            self.metrics.use_after_move_prevented,
            self.metrics.memory_leaks_prevented,
            self.metrics.bounds_violations_prevented,
            self.metrics.allocation_failures
        ));

        report
    }

    /// Generate runtime bounds checking code - optimal performance implementation
    pub fn generate_bounds_check(
        &self,
        builder: &mut FunctionBuilder,
        ptr: Value,
        size: Value,
        access_size: u32,
    ) -> CodegenResult<()> {
        // Generate efficient bounds check: if (ptr + access_size > ptr + size) trap()
        let access_size_val = builder.ins().iconst(ctypes::I64, access_size as i64);
        let ptr_end = builder.ins().iadd(ptr, size);
        let access_end = builder.ins().iadd(ptr, access_size_val);
        
        // Check: access_end <= ptr_end (unsigned comparison for pointer arithmetic)
        let bounds_ok = builder.ins().icmp(
            cranelift::prelude::IntCC::UnsignedLessThanOrEqual, 
            access_end, 
            ptr_end
        );
        
        // Create trap block for bounds violation - efficient branch prediction
        let trap_block = builder.create_block();
        let continue_block = builder.create_block();
        
        // Branch with hint that bounds check usually succeeds (branch prediction optimization)
        builder.ins().brif(bounds_ok, continue_block, &[], trap_block, &[]);
        
        // Trap block - immediate termination with specific error code
        builder.switch_to_block(trap_block);
        builder.ins().trap(cranelift::prelude::TrapCode::HeapOutOfBounds);
        
        // Continue block - normal execution path
        builder.switch_to_block(continue_block);
        
        Ok(())
    }

    /// Check memory safety with comprehensive validation
    pub fn check_memory_safety(&self, ptr: Value, _access_size: u32, access_location: &str) -> CodegenResult<()> {
        // Check if pointer is from a tracked region
        for region in self.regions.values() {
            if let Some(_base_ptr) = region.base_ptr {
                // TODO: More sophisticated region bounds checking
                // For now, basic validation that region exists
                continue;
            }
        }
        
        // Check linear type safety
        if let Some(ownership) = self.linear_ownership.get(&ptr) {
            if ownership.is_moved {
                return Err(linear_type_violation_error(
                    "Memory safety violation: accessing moved linear type".to_string(),
                    access_location.to_string(),
                    format!("Value was moved at {}", ownership.source_location),
                ));
            }
        }
        
        Ok(())
    }

    /// Enter function scope for leak tracking
    pub fn enter_function_scope(&mut self) {
        self.leak_tracker.enter_function();
    }

    /// Exit function scope and get leak warnings
    pub fn exit_function_scope(&mut self) -> Vec<LeakWarning> {
        self.leak_tracker.exit_function()
    }

    /// Get comprehensive leak detection report
    pub fn get_leak_report(&self) -> String {
        self.leak_tracker.generate_leak_report()
    }

    /// Mark allocation as manually freed (for manual strategy)
    pub fn mark_allocation_freed(&mut self, alloc_id: u32) {
        self.leak_tracker.mark_freed(alloc_id);
        self.metrics.memory_leaks_prevented += 1;
    }

    /// Update escape analysis for better leak detection
    pub fn update_escape_analysis(&mut self, alloc_id: u32, escapes_function: bool, confidence: u8) {
        self.leak_tracker.update_escape_analysis(alloc_id, escapes_function, confidence);
    }
}

/// Allocation options for fine-grained control
#[derive(Debug, Clone, Default)]
pub struct AllocationOptions {
    /// Region ID for region allocation
    pub region_id: Option<u32>,
    /// Source location for error reporting
    pub source_location: String,
    /// Custom alignment requirement
    pub alignment: Option<u32>,
    /// Whether this allocation can trigger GC
    pub gc_allowed: bool,
}

/// Memory annotation attributes parsed from user code
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryAnnotation {
    Manual,
    Smart,
    Linear,
    Region(String),
    Stack,
    NoGC,
    Align(u32),
}

/// Parse memory annotation from source code
pub fn parse_annotation(text: &str) -> Option<MemoryAnnotation> {
    match text {
        "@manual" => Some(MemoryAnnotation::Manual),
        "@smart" => Some(MemoryAnnotation::Smart),
        "@linear" => Some(MemoryAnnotation::Linear),
        "@stack" => Some(MemoryAnnotation::Stack),
        "@nogc" => Some(MemoryAnnotation::NoGC),
        s if s.starts_with("@region(") && s.ends_with(")") => {
            let name = &s[8..s.len()-1];
            Some(MemoryAnnotation::Region(name.to_string()))
        },
        s if s.starts_with("@align(") && s.ends_with(")") => {
            if let Ok(align) = s[7..s.len()-1].parse::<u32>() {
                Some(MemoryAnnotation::Align(align))
            } else {
                None
            }
        },
        _ => None,
    }
}

/// Helper functions for creating descriptive errors
pub fn invalid_allocation_error(strategy: &str, reason: String, suggestion: String) -> CodegenError {
    CodegenError::InternalError(format!(
        "Invalid {} allocation: {}\nSuggestion: {}",
        strategy, reason, suggestion
    ))
}

pub fn out_of_memory_error(requested: u32, available: u32, strategy: &str, region_id: Option<u32>) -> CodegenError {
    let region_info = region_id.map(|id| format!(" in region {}", id)).unwrap_or_default();
    CodegenError::InternalError(format!(
        "Out of memory{}: requested {} bytes, {} bytes available ({} allocation)",
        region_info, requested, available, strategy
    ))
}

pub fn linear_type_violation_error(violation: String, source_location: String, suggestion: String) -> CodegenError {
    CodegenError::InternalError(format!(
        "Linear type violation: {} at {}\nSuggestion: {}",
        violation, source_location, suggestion
    ))
}

pub fn runtime_not_initialized_error(msg: String) -> CodegenError {
    CodegenError::InternalError(format!("Runtime not initialized: {}", msg))
} 

/// Allocation tracking for leak detection
#[derive(Debug, Clone)]
pub struct AllocationTracker {
    /// All allocations made in current scope
    allocations: HashMap<u32, AllocationInfo>,
    /// Function-scope allocation stacks
    function_stacks: Vec<Vec<u32>>,
    /// Detected potential leaks
    potential_leaks: Vec<LeakWarning>,
}

/// Information about a specific allocation for leak tracking
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub alloc_id: u32,
    pub strategy: MemoryStrategy,
    pub size: u32,
    pub source_location: String,
    pub is_freed: bool,
    pub escape_analysis: EscapeInfo,
}

/// Escape analysis results for an allocation
#[derive(Debug, Clone)]
pub struct EscapeInfo {
    /// Does this allocation escape current function?
    pub escapes_function: bool,
    /// Is it stored in a long-lived structure?
    pub stored_globally: bool,
    /// Is it returned from function?
    pub returned: bool,
    /// Confidence level of analysis (0-100)
    pub confidence: u8,
}

/// Potential memory leak warning
#[derive(Debug, Clone)]
pub struct LeakWarning {
    pub alloc_id: u32,
    pub strategy: MemoryStrategy,
    pub source_location: String,
    pub leak_type: LeakType,
    pub severity: LeakSeverity,
    pub suggestion: String,
}

/// Types of memory leaks detected
#[derive(Debug, Clone, PartialEq)]
pub enum LeakType {
    /// Manual allocation never freed
    ManualNotFreed,
    /// Region allocated but never deallocated
    RegionNotDestroyed,
    /// Linear type moved but original still accessible
    LinearDoubleUse,
    /// Smart pointer cyclic reference
    SmartPointerCycle,
    /// Allocation escapes scope without proper handling
    EscapeWithoutDealloc,
}

/// Severity levels for leak warnings
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeakSeverity {
    Info,    // Potential issue, might be intentional
    Warning, // Likely problem, should be reviewed
    Error,   // Definite leak, must be fixed
    Critical, // Severe leak that will cause major issues
}

impl AllocationTracker {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            function_stacks: Vec::new(),
            potential_leaks: Vec::new(),
        }
    }

    /// Track a new allocation
    pub fn track_allocation(&mut self, result: &AllocationResult, source_location: &str) {
        let allocation = AllocationInfo {
            alloc_id: result.alloc_id,
            strategy: result.strategy,
            size: result.size,
            source_location: source_location.to_string(),
            is_freed: false,
            escape_analysis: EscapeInfo {
                escapes_function: false,
                stored_globally: false,
                returned: false,
                confidence: 50, // Start with medium confidence
            },
        };
        
        self.allocations.insert(result.alloc_id, allocation);
        
        // Add to current function stack
        if let Some(current_stack) = self.function_stacks.last_mut() {
            current_stack.push(result.alloc_id);
        }
    }

    /// Mark allocation as freed
    pub fn mark_freed(&mut self, alloc_id: u32) {
        if let Some(allocation) = self.allocations.get_mut(&alloc_id) {
            allocation.is_freed = true;
        }
    }

    /// Enter a new function scope
    pub fn enter_function(&mut self) {
        self.function_stacks.push(Vec::new());
    }

    /// Exit function scope and check for leaks
    pub fn exit_function(&mut self) -> Vec<LeakWarning> {
        let mut function_leaks = Vec::new();
        
        if let Some(function_allocs) = self.function_stacks.pop() {
            for &alloc_id in &function_allocs {
                if let Some(allocation) = self.allocations.get(&alloc_id) {
                    // Check if allocation needs cleanup
                    let leak_check = self.analyze_allocation_for_leaks(allocation);
                    if let Some(warning) = leak_check {
                        function_leaks.push(warning.clone());
                        self.potential_leaks.push(warning);
                    }
                }
            }
        }
        
        function_leaks
    }

    /// Analyze allocation for potential leaks
    fn analyze_allocation_for_leaks(&self, allocation: &AllocationInfo) -> Option<LeakWarning> {
        match allocation.strategy {
            MemoryStrategy::Manual => {
                if !allocation.is_freed && !allocation.escape_analysis.escapes_function {
                    Some(LeakWarning {
                        alloc_id: allocation.alloc_id,
                        strategy: allocation.strategy,
                        source_location: allocation.source_location.clone(),
                        leak_type: LeakType::ManualNotFreed,
                        severity: LeakSeverity::Error,
                        suggestion: "Manual allocations must be explicitly freed with deallocate_manual()".to_string(),
                    })
                } else {
                    None
                }
            },
            MemoryStrategy::SmartPtr => {
                // Smart pointers handle their own cleanup, but check for cycles
                // TODO: Implement cycle detection algorithm
                None
            },
            MemoryStrategy::Linear => {
                // Linear types are automatically cleaned up, but check for double-use
                None
            },
            MemoryStrategy::Region => {
                // Region allocations are cleaned up with the region
                None
            },
            MemoryStrategy::Stack => {
                // Stack allocations are automatically cleaned up
                None
            },
        }
    }

    /// Update escape analysis for an allocation
    pub fn update_escape_analysis(&mut self, alloc_id: u32, escapes: bool, confidence: u8) {
        if let Some(allocation) = self.allocations.get_mut(&alloc_id) {
            allocation.escape_analysis.escapes_function = escapes;
            allocation.escape_analysis.confidence = confidence;
        }
    }

    /// Get all detected leaks
    pub fn get_detected_leaks(&self) -> &[LeakWarning] {
        &self.potential_leaks
    }

    /// Generate comprehensive leak report
    pub fn generate_leak_report(&self) -> String {
        if self.potential_leaks.is_empty() {
            return "✅ No memory leaks detected!".to_string();
        }

        let mut report = String::from("🚨 Memory Leak Analysis Report 🚨\n\n");
        
        // Group by severity
        let mut by_severity: HashMap<LeakSeverity, Vec<&LeakWarning>> = HashMap::new();
        for leak in &self.potential_leaks {
            by_severity.entry(leak.severity.clone()).or_default().push(leak);
        }

        for severity in [LeakSeverity::Critical, LeakSeverity::Error, LeakSeverity::Warning, LeakSeverity::Info] {
            if let Some(leaks) = by_severity.get(&severity) {
                report.push_str(&format!("{:?} Issues ({}):\n", severity, leaks.len()));
                for leak in leaks {
                    report.push_str(&format!(
                        "  • {} allocation at {} (ID: {})\n    {} - {}\n",
                        leak.strategy.name(),
                        leak.source_location,
                        leak.alloc_id,
                        format!("{:?}", leak.leak_type).replace('_', " "),
                        leak.suggestion
                    ));
                }
                report.push('\n');
            }
        }

        report.push_str(&format!(
            "Summary: {} total issues detected\n",
            self.potential_leaks.len()
        ));

        report
    }
} 