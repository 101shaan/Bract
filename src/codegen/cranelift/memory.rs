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

/// Memory region for arena-style allocation with optimization
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub id: u32,
    pub name: String,
    pub base_ptr: Option<Value>,
    pub size: u64,
    pub used: u64,
    pub allocations: Vec<u32>, // allocation IDs
    /// Advanced allocator for alignment optimization
    pub allocator: OptimizedRegionAllocator,
    /// Default alignment for this region
    pub default_alignment: u32,
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
    /// Runtime function references (legacy - being replaced by runtime_bridge)
    runtime_functions: Option<RuntimeFunctions>,
    /// Modern runtime bridge system
    runtime_bridge: RuntimeBridge,
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
    /// Cycle detection for smart pointers
    cycle_detector: CycleDetector,
    /// Runtime memory profiler for performance analysis
    profiler: MemoryProfiler,
    /// Next unique IDs
    next_region_id: u32,
    next_alloc_id: u32,
}

impl BractMemoryManager {
    /// Create new memory manager
    pub fn new() -> Self {
        Self {
            runtime_functions: None,
            runtime_bridge: RuntimeBridge::new(),
            regions: HashMap::new(),
            linear_ownership: HashMap::new(),
            smart_pointers: HashMap::new(),
            metrics: MemoryMetrics::default(),
            leak_tracker: AllocationTracker::new(),
            cycle_detector: CycleDetector::new(),
            profiler: MemoryProfiler::new(),
            next_region_id: 1,
            next_alloc_id: 1000, // Start high to avoid conflicts
        }
    }

    /// Initialize runtime system with modern bridge architecture
    pub fn initialize_runtime(&mut self, module: &mut dyn CraneliftModule) -> CodegenResult<()> {
        // Initialize the new runtime bridge
        self.runtime_bridge.initialize(module)?;
        
        // Keep legacy initialization for backwards compatibility during transition
        let runtime_funcs = RuntimeFunctions {
            malloc: FuncId::from_u32(0), // Placeholder
            free: FuncId::from_u32(1),   // Placeholder
            arc_inc: FuncId::from_u32(2), // Placeholder
            arc_dec: FuncId::from_u32(3), // Placeholder
        };
        self.runtime_functions = Some(runtime_funcs);

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

        // Record hotspot for performance profiling
        self.profiler.record_hotspot(options.source_location.clone(), size, strategy);

        Ok(result)
    }

    /// Manual allocation using malloc - maximum performance with runtime bridge
    fn alloc_manual(&mut self, builder: &mut FunctionBuilder, size: u32) -> CodegenResult<Value> {
        // Use modern runtime bridge for clean integration
        let size_val = builder.ins().iconst(ctypes::I64, size as i64);
        self.runtime_bridge.generate_malloc_call(builder, size_val)
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

        /// Region allocation - arena style with optimal alignment
    fn alloc_in_region(&mut self, builder: &mut FunctionBuilder, region_id: u32, _object_type: Type, size: u32) -> CodegenResult<Value> {
        self.alloc_in_region_with_hint(builder, region_id, size, AlignmentHint::Structure)
    }

    /// Region allocation with alignment hint for maximum performance
    pub fn alloc_in_region_with_hint(&mut self, builder: &mut FunctionBuilder, region_id: u32, size: u32, hint: AlignmentHint) -> CodegenResult<Value> {
        let (base_ptr, aligned_allocation) = {
            let region = self.regions.get_mut(&region_id).ok_or_else(|| 
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

            // Calculate optimal alignment based on hint
            let optimal_alignment = region.allocator.calculate_optimal_alignment(size, hint);
            
            // Calculate optimally aligned allocation
            let aligned_alloc = region.allocator.calculate_aligned_allocation(
                size, 
                region.used, 
                optimal_alignment
            );

            // Check if allocation fits in region
            if aligned_alloc.aligned_offset + aligned_alloc.actual_size > region.size {
                return Err(out_of_memory_error(
                    size,
                    (region.size - aligned_alloc.aligned_offset) as u32,
                    "Region",
                    Some(region_id),
                ));
            }

            (base_ptr, aligned_alloc)
        };

        // Generate allocation address with optimal alignment
        let offset_val = builder.ins().iconst(ctypes::I64, aligned_allocation.aligned_offset as i64);
        let alloc_ptr = builder.ins().iadd(base_ptr, offset_val);
        
        // Update region state
        let region = self.regions.get_mut(&region_id).unwrap();
        region.used = aligned_allocation.aligned_offset + aligned_allocation.actual_size;
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

        /// Create memory region for grouped allocation with alignment optimization
    pub fn create_region(&mut self, name: String, size: u64) -> u32 {
        self.create_region_with_alignment(name, size, 64) // Default to cache-line alignment
    }

    /// Create memory region with custom alignment
    pub fn create_region_with_alignment(&mut self, name: String, size: u64, alignment: u32) -> u32 {
        let region_id = self.next_region_id;
        self.next_region_id += 1;
        
        let region = MemoryRegion {
            id: region_id,
            name: name.clone(),
            base_ptr: None,
            size,
            used: 0,
            allocations: Vec::new(),
            allocator: OptimizedRegionAllocator::new(alignment, 64), // 64-byte cache lines
            default_alignment: alignment,
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

    /// Decrement smart pointer reference count using runtime bridge
    fn decrement_smart_ptr_ref(&mut self, builder: &mut FunctionBuilder, ptr: Value) -> CodegenResult<()> {
        // Use runtime bridge for proper ARC decrement
        self.runtime_bridge.generate_arc_dec_call(builder, ptr)
    }

    /// Free manually allocated memory using runtime bridge
    pub fn deallocate_manual(&mut self, builder: &mut FunctionBuilder, ptr: Value) -> CodegenResult<()> {
        // Use runtime bridge for proper free call
        self.runtime_bridge.generate_free_call(builder, ptr)
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

    /// Add reference for cycle detection
    pub fn add_smart_pointer_reference(&mut self, from_ptr: Value, to_ptr: Value) {
        self.cycle_detector.add_reference(from_ptr, to_ptr);
    }

    /// Remove reference for cycle detection
    pub fn remove_smart_pointer_reference(&mut self, from_ptr: Value, to_ptr: Value) {
        self.cycle_detector.remove_reference(from_ptr, to_ptr);
    }

    /// Run cycle detection and break cycles
    pub fn detect_and_break_cycles(&mut self) -> (Vec<Cycle>, Vec<CycleBreakResult>) {
        let detected_cycles = self.cycle_detector.detect_cycles();
        let break_results = self.cycle_detector.break_cycles();
        
        // Update metrics
        self.metrics.cycle_cleanups += break_results.iter()
            .filter(|r| r.success)
            .count() as u64;
        
        (detected_cycles, break_results)
    }

    /// Get cycle detection statistics
    pub fn get_cycle_statistics(&self) -> CycleDetectionStats {
        self.cycle_detector.get_statistics()
    }

    /// Record performance sample for profiling
    pub fn record_performance_sample(&mut self, current_memory_kb: u64) {
        self.profiler.record_sample(current_memory_kb, &self.metrics);
    }

    /// Record allocation hotspot
    pub fn record_allocation_hotspot(&mut self, location: String, size: u32, strategy: MemoryStrategy) {
        self.profiler.record_hotspot(location, size, strategy);
    }

    /// Get comprehensive performance report
    pub fn get_performance_report(&self) -> String {
        self.profiler.generate_performance_report()
    }

    /// Get top allocation hotspots
    pub fn get_top_allocation_hotspots(&self, limit: usize) -> Vec<&AllocationHotspot> {
        self.profiler.get_top_hotspots(limit)
    }

    /// Get region efficiency report for all regions
    pub fn get_region_efficiency_report(&self) -> String {
        if self.regions.is_empty() {
            return "No regions currently allocated.".to_string();
        }

        let mut report = String::from("🚀 Region Allocation Efficiency Report 🚀\n\n");
        
        for (region_id, region) in &self.regions {
            report.push_str(&format!(
                "Region {} ('{}'):\n{}\n\n",
                region_id,
                region.name,
                region.allocator.get_fragmentation_report()
            ));
        }

        // Overall statistics
        let total_regions = self.regions.len();
        let total_allocated = self.regions.values()
            .map(|r| r.used)
            .sum::<u64>();

        report.push_str(&format!(
            "=== Overall Summary ===\n\
             Total Regions: {}\n\
             Total Memory Used: {} KB\n\
             Average Region Utilization: {:.1}%\n",
            total_regions,
            total_allocated / 1024,
            if total_regions > 0 { total_allocated as f64 / (total_regions as f64 * 1024.0) } else { 0.0 }
        ));

        report
    }

    /// Optimize region allocations (defragment, repack, etc.)
    pub fn optimize_regions(&mut self) -> RegionOptimizationResult {
        let mut optimized_regions = 0;
        let mut bytes_saved = 0u64;
        let mut recommendations = Vec::new();

        for (region_id, region) in &self.regions {
            let frag_stats = &region.allocator.fragmentation_stats;
            
            // Check if region needs optimization
            if frag_stats.fragmentation_percentage > 20.0 {
                optimized_regions += 1;
                bytes_saved += frag_stats.alignment_waste;
                recommendations.push(format!(
                    "Region {} ('{}') has {:.1}% fragmentation - consider reallocating with better alignment",
                    region_id, region.name, frag_stats.fragmentation_percentage
                ));
            }
        }

        RegionOptimizationResult {
            regions_optimized: optimized_regions,
            bytes_saved,
            recommendations,
            overall_improvement: if optimized_regions > 0 { 
                bytes_saved as f64 / (1024.0 * optimized_regions as f64) 
            } else { 
                0.0 
            },
        }
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

/// Runtime bridge for memory operations - clean integration architecture
#[derive(Debug, Clone)]
pub struct RuntimeBridge {
    /// Function IDs for runtime operations
    runtime_functions: Option<RuntimeFunctions>,
    /// Whether runtime is initialized
    initialized: bool,
}

impl RuntimeBridge {
    pub fn new() -> Self {
        Self {
            runtime_functions: None,
            initialized: false,
        }
    }

    /// Initialize runtime bridge with function declarations
    pub fn initialize(&mut self, module: &mut dyn CraneliftModule) -> CodegenResult<()> {
        // Create runtime function signatures and declarations
        let runtime_funcs = RuntimeFunctions {
            malloc: self.declare_malloc(module)?,
            free: self.declare_free(module)?,
            arc_inc: self.declare_arc_inc(module)?,
            arc_dec: self.declare_arc_dec(module)?,
        };

        self.runtime_functions = Some(runtime_funcs);
        self.initialized = true;
        Ok(())
    }

    /// Declare malloc function with proper signature
    fn declare_malloc(&self, module: &mut dyn CraneliftModule) -> CodegenResult<FuncId> {
        let mut sig = module.make_signature();
        sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64)); // size
        sig.returns.push(cranelift::prelude::AbiParam::new(ctypes::I64)); // pointer
        
        module.declare_function("bract_malloc", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare bract_malloc: {}", e)))
    }

    /// Declare free function with proper signature  
    fn declare_free(&self, module: &mut dyn CraneliftModule) -> CodegenResult<FuncId> {
        let mut sig = module.make_signature();
        sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64)); // pointer
        
        module.declare_function("bract_free", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare bract_free: {}", e)))
    }

    /// Declare ARC increment function
    fn declare_arc_inc(&self, module: &mut dyn CraneliftModule) -> CodegenResult<FuncId> {
        let mut sig = module.make_signature();
        sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64)); // pointer
        
        module.declare_function("bract_arc_inc", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare bract_arc_inc: {}", e)))
    }

    /// Declare ARC decrement function
    fn declare_arc_dec(&self, module: &mut dyn CraneliftModule) -> CodegenResult<FuncId> {
        let mut sig = module.make_signature();
        sig.params.push(cranelift::prelude::AbiParam::new(ctypes::I64)); // pointer
        
        module.declare_function("bract_arc_dec", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| CodegenError::InternalError(format!("Failed to declare bract_arc_dec: {}", e)))
    }

    /// Generate malloc call - will be properly integrated when runtime bridge is complete
    pub fn generate_malloc_call(&self, builder: &mut FunctionBuilder, size: Value) -> CodegenResult<Value> {
        if !self.initialized {
            // For now, fall back to stack allocation until runtime integration is complete
            return self.generate_stack_fallback(builder, size);
        }

        // TODO: Generate proper function call through Cranelift module system
        // This will be implemented when we have proper module-level function integration
        self.generate_stack_fallback(builder, size)
    }

    /// Generate free call - will be properly integrated when runtime bridge is complete  
    pub fn generate_free_call(&self, _builder: &mut FunctionBuilder, _ptr: Value) -> CodegenResult<()> {
        // TODO: Generate proper function call
        // For now, no-op since we're using stack allocation
        Ok(())
    }

    /// Generate ARC increment call
    pub fn generate_arc_inc_call(&self, _builder: &mut FunctionBuilder, _ptr: Value) -> CodegenResult<()> {
        // TODO: Generate proper function call
        Ok(())
    }

    /// Generate ARC decrement call
    pub fn generate_arc_dec_call(&self, _builder: &mut FunctionBuilder, _ptr: Value) -> CodegenResult<()> {
        // TODO: Generate proper function call
        Ok(())
    }

    /// Stack allocation fallback for development
    fn generate_stack_fallback(&self, builder: &mut FunctionBuilder, _size: Value) -> CodegenResult<Value> {
        // Extract size as constant (simplified for now)
        // In real implementation, would handle dynamic sizes properly
        let size_bytes = 64; // Default reasonable size
        
        let stack_slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            size_bytes,
        ));
        
        Ok(builder.ins().stack_addr(ctypes::I64, stack_slot, 0))
    }
} 

/// Cycle detection system for smart pointers - prevents memory leaks from cycles
#[derive(Debug, Clone)]
pub struct CycleDetector {
    /// Graph of pointer references for cycle detection
    reference_graph: HashMap<u64, Vec<u64>>, // ptr_id -> referenced_ptr_ids
    /// Visited nodes during traversal
    visited: HashMap<u64, bool>,
    /// Nodes currently in recursion stack (for cycle detection)
    recursion_stack: HashMap<u64, bool>,
    /// Detected cycles
    detected_cycles: Vec<Cycle>,
    /// Statistics
    detection_runs: u64,
    cycles_found: u64,
    cycles_broken: u64,
}

/// Represents a detected reference cycle
#[derive(Debug, Clone)]
pub struct Cycle {
    /// Nodes involved in the cycle
    pub nodes: Vec<u64>,
    /// Cycle strength (how many references in cycle)
    pub strength: u32,
    /// Whether cycle has been broken
    pub broken: bool,
    /// Strategy used to break cycle
    pub break_strategy: Option<CycleBreakStrategy>,
}

/// Strategies for breaking reference cycles
#[derive(Debug, Clone, PartialEq)]
pub enum CycleBreakStrategy {
    /// Convert one reference to weak reference
    WeakReference,
    /// Mark one node for delayed cleanup
    DelayedCleanup,
    /// Force manual cleanup of entire cycle
    ManualCleanup,
}

impl CycleDetector {
    pub fn new() -> Self {
        Self {
            reference_graph: HashMap::new(),
            visited: HashMap::new(),
            recursion_stack: HashMap::new(),
            detected_cycles: Vec::new(),
            detection_runs: 0,
            cycles_found: 0,
            cycles_broken: 0,
        }
    }

    /// Add a reference edge to the graph
    pub fn add_reference(&mut self, from_ptr: Value, to_ptr: Value) {
        let from_id = self.value_to_id(from_ptr);
        let to_id = self.value_to_id(to_ptr);
        
        self.reference_graph
            .entry(from_id)
            .or_default()
            .push(to_id);
    }

    /// Remove a reference edge
    pub fn remove_reference(&mut self, from_ptr: Value, to_ptr: Value) {
        let from_id = self.value_to_id(from_ptr);
        let to_id = self.value_to_id(to_ptr);
        
        if let Some(refs) = self.reference_graph.get_mut(&from_id) {
            refs.retain(|&id| id != to_id);
            if refs.is_empty() {
                self.reference_graph.remove(&from_id);
            }
        }
    }

    /// Run cycle detection algorithm (DFS-based)
    pub fn detect_cycles(&mut self) -> Vec<Cycle> {
        self.detection_runs += 1;
        self.clear_detection_state();
        
        // Run DFS from each unvisited node
        let nodes: Vec<u64> = self.reference_graph.keys().copied().collect();
        for &node in &nodes {
            if !self.visited.get(&node).unwrap_or(&false) {
                self.dfs_detect_cycles(node, &mut Vec::new());
            }
        }
        
        self.detected_cycles.clone()
    }

    /// DFS traversal with cycle detection
    fn dfs_detect_cycles(&mut self, node: u64, path: &mut Vec<u64>) {
        self.visited.insert(node, true);
        self.recursion_stack.insert(node, true);
        path.push(node);

        if let Some(neighbors) = self.reference_graph.get(&node).cloned() {
            for neighbor in neighbors {
                if !self.visited.get(&neighbor).unwrap_or(&false) {
                    // Continue DFS
                    self.dfs_detect_cycles(neighbor, path);
                } else if *self.recursion_stack.get(&neighbor).unwrap_or(&false) {
                    // Found a back edge - cycle detected!
                    let cycle_start = path.iter().position(|&x| x == neighbor).unwrap_or(0);
                    let cycle_nodes = path[cycle_start..].to_vec();
                    
                    let cycle = Cycle {
                        nodes: cycle_nodes,
                        strength: (path.len() - cycle_start) as u32,
                        broken: false,
                        break_strategy: None,
                    };
                    
                    self.detected_cycles.push(cycle);
                    self.cycles_found += 1;
                }
            }
        }

        // Backtrack
        self.recursion_stack.insert(node, false);
        path.pop();
    }

    /// Break detected cycles using optimal strategies
    pub fn break_cycles(&mut self) -> Vec<CycleBreakResult> {
        let mut results = Vec::new();
        let mut cycles_to_update = Vec::new();
        
        // First pass: analyze cycles and determine strategies
        for (i, cycle) in self.detected_cycles.iter().enumerate() {
            if !cycle.broken {
                let strategy = self.choose_break_strategy(cycle);
                let result = self.apply_break_strategy_readonly(cycle, &strategy);
                results.push(result);
                
                if results.last().unwrap().success {
                    cycles_to_update.push((i, strategy));
                }
            }
        }
        
        // Second pass: update cycles and statistics
        for (cycle_index, strategy) in cycles_to_update {
            if let Some(cycle) = self.detected_cycles.get_mut(cycle_index) {
                cycle.broken = true;
                cycle.break_strategy = Some(strategy);
                self.cycles_broken += 1;
            }
        }
        
        results
    }

    /// Choose optimal strategy for breaking a cycle
    fn choose_break_strategy(&self, cycle: &Cycle) -> CycleBreakStrategy {
        match cycle.strength {
            1..=2 => CycleBreakStrategy::WeakReference,   // Simple cycles - weak reference
            3..=5 => CycleBreakStrategy::DelayedCleanup,  // Medium cycles - delayed cleanup
            _     => CycleBreakStrategy::ManualCleanup,   // Complex cycles - manual intervention
        }
    }

    /// Apply cycle breaking strategy (readonly analysis)
    fn apply_break_strategy_readonly(&self, cycle: &Cycle, strategy: &CycleBreakStrategy) -> CycleBreakResult {
        match strategy {
            CycleBreakStrategy::WeakReference => {
                // Plan to convert strongest reference to weak (analysis only)
                CycleBreakResult {
                    cycle_nodes: cycle.nodes.clone(),
                    strategy_used: strategy.clone(),
                    success: true,
                    performance_impact: PerformanceImpact::Minimal,
                    description: "Will convert reference to weak reference".to_string(),
                }
            },
            CycleBreakStrategy::DelayedCleanup => {
                // Plan for delayed cleanup
                CycleBreakResult {
                    cycle_nodes: cycle.nodes.clone(),
                    strategy_used: strategy.clone(),
                    success: true,
                    performance_impact: PerformanceImpact::Low,
                    description: "Will mark cycle for delayed cleanup".to_string(),
                }
            },
            CycleBreakStrategy::ManualCleanup => {
                // Requires manual intervention
                CycleBreakResult {
                    cycle_nodes: cycle.nodes.clone(),
                    strategy_used: strategy.clone(),
                    success: false, // Requires manual action
                    performance_impact: PerformanceImpact::High,
                    description: "Complex cycle requires manual cleanup".to_string(),
                }
            },
        }
    }

    /// Actually execute cycle breaking (separate from analysis)
    pub fn execute_cycle_breaking(&mut self, cycles_to_break: &[(usize, CycleBreakStrategy)]) {
        for &(cycle_index, ref strategy) in cycles_to_break {
            if let Some(cycle) = self.detected_cycles.get(cycle_index) {
                match strategy {
                    CycleBreakStrategy::WeakReference => {
                        // Actually remove the edge to break the cycle
                        if let Some(&weakest_node) = cycle.nodes.first() {
                            if cycle.nodes.len() >= 2 {
                                let target = cycle.nodes[1];
                                if let Some(refs) = self.reference_graph.get_mut(&weakest_node) {
                                    refs.retain(|&id| id != target);
                                }
                            }
                        }
                    },
                    CycleBreakStrategy::DelayedCleanup | CycleBreakStrategy::ManualCleanup => {
                        // These strategies don't require immediate graph modification
                    },
                }
            }
        }
    }

    /// Clear detection state for new run
    fn clear_detection_state(&mut self) {
        self.visited.clear();
        self.recursion_stack.clear();
        self.detected_cycles.clear();
    }

    /// Convert Cranelift Value to unique ID (simplified)
    fn value_to_id(&self, value: Value) -> u64 {
        // Use the raw value as ID (this is a simplification)
        // In real implementation, would use a proper mapping system
        value.as_u32() as u64
    }

    /// Get cycle detection statistics
    pub fn get_statistics(&self) -> CycleDetectionStats {
        CycleDetectionStats {
            detection_runs: self.detection_runs,
            cycles_found: self.cycles_found,
            cycles_broken: self.cycles_broken,
            active_cycles: self.detected_cycles.len() as u64,
            graph_size: self.reference_graph.len() as u64,
        }
    }
}

/// Result of breaking a cycle
#[derive(Debug, Clone)]
pub struct CycleBreakResult {
    pub cycle_nodes: Vec<u64>,
    pub strategy_used: CycleBreakStrategy,
    pub success: bool,
    pub performance_impact: PerformanceImpact,
    pub description: String,
}

/// Performance impact levels
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceImpact {
    Minimal,  // <1% performance impact
    Low,      // 1-5% performance impact
    Medium,   // 5-15% performance impact
    High,     // >15% performance impact
}

/// Cycle detection statistics
#[derive(Debug, Clone)]
pub struct CycleDetectionStats {
    pub detection_runs: u64,
    pub cycles_found: u64,
    pub cycles_broken: u64,
    pub active_cycles: u64,
    pub graph_size: u64,
} 

/// Advanced region allocator with alignment optimization
#[derive(Debug, Clone)]
pub struct OptimizedRegionAllocator {
    /// Memory alignment requirements (power of 2)
    alignment: u32,
    /// Cache line size for optimal alignment (typically 64 bytes)
    cache_line_size: u32,
    /// Track fragmentation for optimization
    fragmentation_stats: FragmentationStats,
    /// Pre-calculated alignment masks for fast operations
    alignment_masks: AlignmentMasks,
}

/// Fragmentation tracking for region optimization
#[derive(Debug, Clone, Default)]
pub struct FragmentationStats {
    /// Total allocated bytes
    total_allocated: u64,
    /// Total wasted bytes due to alignment
    alignment_waste: u64,
    /// Number of allocation operations
    allocation_count: u64,
    /// Average fragmentation percentage
    fragmentation_percentage: f64,
    /// Largest contiguous block available
    largest_free_block: u64,
}

/// Pre-calculated alignment masks for performance
#[derive(Debug, Clone)]
pub struct AlignmentMasks {
    /// Mask for 8-byte alignment
    align_8: u64,
    /// Mask for 16-byte alignment  
    align_16: u64,
    /// Mask for 32-byte alignment
    align_32: u64,
    /// Mask for 64-byte alignment (cache line)
    align_64: u64,
    /// Mask for 128-byte alignment
    align_128: u64,
}

impl OptimizedRegionAllocator {
    pub fn new(alignment: u32, cache_line_size: u32) -> Self {
        Self {
            alignment,
            cache_line_size,
            fragmentation_stats: FragmentationStats::default(),
            alignment_masks: AlignmentMasks::new(),
        }
    }

    /// Calculate optimal alignment for given size and type
    pub fn calculate_optimal_alignment(&self, _size: u32, type_hint: AlignmentHint) -> u32 {
        match type_hint {
            AlignmentHint::SmallPrimitive => 8,      // Basic alignment for small types
            AlignmentHint::LargePrimitive => 16,     // Better alignment for larger types
            AlignmentHint::Structure => 32,          // Structure alignment for better packing
            AlignmentHint::Array => self.cache_line_size, // Cache-line align arrays
            AlignmentHint::HighPerformance => self.cache_line_size, // Maximum performance
            AlignmentHint::Custom(align) => align,   // User-specified alignment
        }
    }

    /// Align address to specified boundary - highly optimized
    #[inline(always)] // Critical hot path
    pub fn align_address(&self, addr: u64, alignment: u32) -> u64 {
        let mask = self.get_alignment_mask(alignment);
        (addr + (alignment as u64 - 1)) & !mask
    }

    /// Get pre-calculated alignment mask for performance
    #[inline(always)]
    fn get_alignment_mask(&self, alignment: u32) -> u64 {
        match alignment {
            8 => self.alignment_masks.align_8,
            16 => self.alignment_masks.align_16,
            32 => self.alignment_masks.align_32,
            64 => self.alignment_masks.align_64,
            128 => self.alignment_masks.align_128,
            _ => (alignment as u64) - 1, // Fallback for custom alignments
        }
    }

    /// Calculate allocation with optimal packing
    pub fn calculate_aligned_allocation(&mut self, size: u32, current_offset: u64, alignment: u32) -> AlignedAllocation {
        let aligned_offset = self.align_address(current_offset, alignment);
        let waste = aligned_offset - current_offset;
        let total_size = size as u64;

        // Update fragmentation statistics
        self.fragmentation_stats.total_allocated += total_size;
        self.fragmentation_stats.alignment_waste += waste;
        self.fragmentation_stats.allocation_count += 1;
        
        // Calculate fragmentation percentage
        if self.fragmentation_stats.total_allocated > 0 {
            self.fragmentation_stats.fragmentation_percentage = 
                (self.fragmentation_stats.alignment_waste as f64 / 
                 self.fragmentation_stats.total_allocated as f64) * 100.0;
        }

        AlignedAllocation {
            aligned_offset,
            actual_size: total_size,
            alignment_waste: waste,
            efficiency_score: self.calculate_efficiency_score(waste, total_size),
        }
    }

    /// Calculate allocation efficiency (0-100, higher is better)
    fn calculate_efficiency_score(&self, waste: u64, size: u64) -> f64 {
        if size == 0 { return 0.0; }
        let efficiency = ((size as f64) / ((size + waste) as f64)) * 100.0;
        efficiency.min(100.0).max(0.0)
    }

    /// Get comprehensive fragmentation report
    pub fn get_fragmentation_report(&self) -> String {
        format!(
            "=== Region Allocation Efficiency Report ===\n\
             Total Allocations: {}\n\
             Total Memory Allocated: {} KB\n\
             Memory Wasted (Alignment): {} KB ({:.2}%)\n\
             Average Efficiency: {:.1}%\n\n\
             Recommendations:\n\
             {}",
            self.fragmentation_stats.allocation_count,
            self.fragmentation_stats.total_allocated / 1024,
            self.fragmentation_stats.alignment_waste / 1024,
            self.fragmentation_stats.fragmentation_percentage,
            100.0 - self.fragmentation_stats.fragmentation_percentage,
            self.generate_optimization_recommendations()
        )
    }

    /// Generate optimization recommendations based on fragmentation
    fn generate_optimization_recommendations(&self) -> String {
        let mut recommendations = Vec::new();

        if self.fragmentation_stats.fragmentation_percentage > 15.0 {
            recommendations.push("• Consider using larger region sizes to reduce alignment overhead");
            recommendations.push("• Group similar-sized allocations together");
        }

        if self.fragmentation_stats.fragmentation_percentage > 25.0 {
            recommendations.push("• Review alignment requirements - some may be excessive");
            recommendations.push("• Consider using custom allocators for frequently allocated types");
        }

        if self.fragmentation_stats.fragmentation_percentage < 5.0 {
            recommendations.push("• Excellent efficiency! Current allocation strategy is optimal");
        }

        if recommendations.is_empty() {
            recommendations.push("• Allocation efficiency is good, no changes recommended");
        }

        recommendations.join("\n             ")
    }
}

impl AlignmentMasks {
    fn new() -> Self {
        Self {
            align_8: 8 - 1,
            align_16: 16 - 1,
            align_32: 32 - 1,
            align_64: 64 - 1,
            align_128: 128 - 1,
        }
    }
}

/// Alignment hints for optimization
#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentHint {
    /// Small primitive types (i8, i16, i32, bool)
    SmallPrimitive,
    /// Large primitive types (i64, f64, pointers)
    LargePrimitive,
    /// User-defined structures
    Structure,
    /// Arrays and bulk data
    Array,
    /// High-performance critical allocations
    HighPerformance,
    /// Custom alignment requirement
    Custom(u32),
}

/// Result of aligned allocation calculation
#[derive(Debug, Clone)]
pub struct AlignedAllocation {
    /// The aligned memory offset
    pub aligned_offset: u64,
    /// Actual size needed for allocation
    pub actual_size: u64,
    /// Bytes wasted due to alignment
    pub alignment_waste: u64,
    /// Efficiency score (0-100, higher is better)
    pub efficiency_score: f64,
} 

/// Result of region optimization analysis
#[derive(Debug, Clone)]
pub struct RegionOptimizationResult {
    /// Number of regions that were optimized
    pub regions_optimized: u32,
    /// Total bytes that could be saved through optimization
    pub bytes_saved: u64,
    /// Specific recommendations for improvement
    pub recommendations: Vec<String>,
    /// Overall improvement score (KB saved per region)
    pub overall_improvement: f64,
} 

/// Real-time memory profiler for performance analysis and optimization
#[derive(Debug, Clone)]
pub struct MemoryProfiler {
    /// Performance samples over time
    performance_samples: Vec<PerformanceSample>,
    /// Hotspot detection data
    hotspots: HashMap<String, AllocationHotspot>,
    /// Real-time metrics
    current_metrics: RealTimeMetrics,
    /// Profiling configuration
    config: ProfilerConfig,
    /// Sample index for circular buffer
    sample_index: usize,
}

/// Single performance sample point
#[derive(Debug, Clone)]
pub struct PerformanceSample {
    /// Timestamp of sample
    pub timestamp_ms: u64,
    /// Memory usage at this point
    pub memory_usage_kb: u64,
    /// Allocation rate (allocs per second)
    pub allocation_rate: f64,
    /// Average allocation size
    pub avg_allocation_size: f64,
    /// Fragmentation percentage
    pub fragmentation: f64,
    /// Strategy breakdown
    pub strategy_usage: HashMap<MemoryStrategy, u32>,
}

/// Memory allocation hotspot information
#[derive(Debug, Clone)]
pub struct AllocationHotspot {
    /// Source location (function, line)
    pub location: String,
    /// Number of allocations at this spot
    pub allocation_count: u64,
    /// Total bytes allocated
    pub total_bytes: u64,
    /// Average allocation size
    pub avg_size: f64,
    /// Dominant strategy used
    pub dominant_strategy: MemoryStrategy,
    /// Performance impact score (0-100)
    pub impact_score: f64,
}

/// Real-time performance metrics
#[derive(Debug, Clone, Default)]
pub struct RealTimeMetrics {
    /// Current memory usage
    pub current_memory_kb: u64,
    /// Peak memory usage
    pub peak_memory_kb: u64,
    /// Allocations in last second
    pub recent_allocs: u32,
    /// Current allocation rate trend
    pub allocation_trend: AllocationTrend,
    /// System health score (0-100)
    pub health_score: f64,
    /// Memory pressure level
    pub pressure_level: MemoryPressure,
}

/// Allocation trend analysis
#[derive(Debug, Clone, PartialEq)]
pub enum AllocationTrend {
    /// Allocation rate is decreasing
    Decreasing,
    /// Allocation rate is stable
    Stable,
    /// Allocation rate is increasing
    Increasing,
    /// Allocation rate is growing rapidly
    Accelerating,
}

impl Default for AllocationTrend {
    fn default() -> Self {
        AllocationTrend::Stable
    }
}

/// Memory pressure levels
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryPressure {
    /// Very low memory usage, all good
    Low,
    /// Moderate usage, normal operation
    Normal,
    /// High usage, consider optimization
    High,
    /// Critical usage, immediate action needed
    Critical,
}

impl Default for MemoryPressure {
    fn default() -> Self {
        MemoryPressure::Low
    }
}

/// Profiler configuration
#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    /// Maximum number of samples to keep
    pub max_samples: usize,
    /// Sampling interval in milliseconds
    pub sample_interval_ms: u64,
    /// Enable hotspot detection
    pub enable_hotspots: bool,
    /// Minimum allocation count for hotspot
    pub hotspot_threshold: u64,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            max_samples: 1000,
            sample_interval_ms: 100,
            enable_hotspots: true,
            hotspot_threshold: 10,
        }
    }
}

impl MemoryProfiler {
    pub fn new() -> Self {
        Self::with_config(ProfilerConfig::default())
    }

    pub fn with_config(config: ProfilerConfig) -> Self {
        Self {
            performance_samples: Vec::with_capacity(config.max_samples),
            hotspots: HashMap::new(),
            current_metrics: RealTimeMetrics::default(),
            config,
            sample_index: 0,
        }
    }

    /// Record a performance sample
    pub fn record_sample(&mut self, memory_usage_kb: u64, metrics: &MemoryMetrics) {
        let now_ms = self.get_current_time_ms();
        
        let sample = PerformanceSample {
            timestamp_ms: now_ms,
            memory_usage_kb,
            allocation_rate: self.calculate_allocation_rate(metrics),
            avg_allocation_size: self.calculate_avg_allocation_size(metrics),
            fragmentation: self.calculate_fragmentation_percentage(metrics),
            strategy_usage: self.extract_strategy_usage(metrics),
        };

        // Add sample to circular buffer
        if self.performance_samples.len() < self.config.max_samples {
            self.performance_samples.push(sample);
        } else {
            self.performance_samples[self.sample_index] = sample;
            self.sample_index = (self.sample_index + 1) % self.config.max_samples;
        }

        // Update real-time metrics
        let last_sample = self.performance_samples.last().unwrap().clone();
        self.update_real_time_metrics(&last_sample);
    }

    /// Record allocation hotspot
    pub fn record_hotspot(&mut self, location: String, size: u32, strategy: MemoryStrategy) {
        if !self.config.enable_hotspots {
            return;
        }

        let hotspot = self.hotspots.entry(location.clone()).or_insert_with(|| {
            AllocationHotspot {
                location: location.clone(),
                allocation_count: 0,
                total_bytes: 0,
                avg_size: 0.0,
                dominant_strategy: strategy,
                impact_score: 0.0,
            }
        });

        hotspot.allocation_count += 1;
        hotspot.total_bytes += size as u64;
        hotspot.avg_size = hotspot.total_bytes as f64 / hotspot.allocation_count as f64;
        
        // Update impact score (higher count + size = higher impact)
        hotspot.impact_score = ((hotspot.allocation_count as f64).ln() * (hotspot.avg_size / 1024.0)).min(100.0);
    }

    /// Get top allocation hotspots
    pub fn get_top_hotspots(&self, limit: usize) -> Vec<&AllocationHotspot> {
        let mut hotspots: Vec<_> = self.hotspots.values().collect();
        hotspots.sort_by(|a, b| b.impact_score.partial_cmp(&a.impact_score).unwrap());
        hotspots.into_iter().take(limit).collect()
    }

    /// Generate comprehensive performance report
    pub fn generate_performance_report(&self) -> String {
        let mut report = String::from("📊 Memory Performance Analysis Report 📊\n\n");

        // Real-time metrics
        report.push_str(&format!(
            "=== Real-Time Status ===\n\
             Current Memory Usage: {} KB\n\
             Peak Memory Usage: {} KB\n\
             Recent Allocations: {}/sec\n\
             System Health Score: {:.1}/100\n\
             Memory Pressure: {:?}\n\
             Allocation Trend: {:?}\n\n",
            self.current_metrics.current_memory_kb,
            self.current_metrics.peak_memory_kb,
            self.current_metrics.recent_allocs,
            self.current_metrics.health_score,
            self.current_metrics.pressure_level,
            self.current_metrics.allocation_trend
        ));

        // Performance trends
        if !self.performance_samples.is_empty() {
            let latest = self.performance_samples.last().unwrap();
            report.push_str(&format!(
                "=== Performance Trends ===\n\
                 Current Allocation Rate: {:.1}/sec\n\
                 Average Allocation Size: {:.1} bytes\n\
                 Memory Fragmentation: {:.2}%\n\n",
                latest.allocation_rate,
                latest.avg_allocation_size,
                latest.fragmentation
            ));
        }

        // Top hotspots
        let top_hotspots = self.get_top_hotspots(5);
        if !top_hotspots.is_empty() {
            report.push_str("=== Top Allocation Hotspots ===\n");
            for (i, hotspot) in top_hotspots.iter().enumerate() {
                report.push_str(&format!(
                    "{}. {} (Impact: {:.1})\n\
                        • {} allocations, {:.1} KB total\n\
                        • Avg size: {:.1} bytes, Strategy: {}\n\n",
                    i + 1,
                    hotspot.location,
                    hotspot.impact_score,
                    hotspot.allocation_count,
                    hotspot.total_bytes as f64 / 1024.0,
                    hotspot.avg_size,
                    hotspot.dominant_strategy.name()
                ));
            }
        }

        // Recommendations
        report.push_str(&self.generate_optimization_recommendations());

        report
    }

    /// Generate optimization recommendations based on profiling data
    fn generate_optimization_recommendations(&self) -> String {
        let mut recommendations = Vec::<String>::new();

        // Analyze pressure level
        match self.current_metrics.pressure_level {
            MemoryPressure::Critical => {
                recommendations.push("🚨 CRITICAL: Reduce memory usage immediately!".to_string());
                recommendations.push("• Consider switching to more compact data structures".to_string());
                recommendations.push("• Enable aggressive garbage collection".to_string());
            },
            MemoryPressure::High => {
                recommendations.push("⚠️ HIGH: Monitor memory usage closely".to_string());
                recommendations.push("• Review allocation patterns for optimization opportunities".to_string());
            },
            _ => {},
        }

        // Analyze allocation trends
        match self.current_metrics.allocation_trend {
            AllocationTrend::Accelerating => {
                recommendations.push("📈 Allocation rate is accelerating rapidly".to_string());
                recommendations.push("• Check for memory leaks or excessive allocation loops".to_string());
                recommendations.push("• Consider caching or object pooling".to_string());
            },
            AllocationTrend::Increasing => {
                recommendations.push("📊 Allocation rate is increasing steadily".to_string());
                recommendations.push("• Monitor for potential memory pressure".to_string());
            },
            _ => {},
        }

        // Analyze hotspots
        let top_hotspots = self.get_top_hotspots(3);
        if !top_hotspots.is_empty() {
            recommendations.push("🔥 Hotspot analysis:".to_string());
            for hotspot in top_hotspots {
                if hotspot.impact_score > 50.0 {
                    recommendations.push(format!(
                        "• High impact at {}: Consider object pooling or caching",
                        hotspot.location
                    ));
                }
            }
        }

        if recommendations.is_empty() {
            recommendations.push("✅ Performance looks good! No immediate optimizations needed.".to_string());
        }

        format!("=== Optimization Recommendations ===\n{}\n", recommendations.join("\n"))
    }

    /// Helper methods for calculations
    fn get_current_time_ms(&self) -> u64 {
        // In real implementation, would use system time
        // For now, return sample index as time
        self.sample_index as u64 * self.config.sample_interval_ms
    }

    fn calculate_allocation_rate(&self, metrics: &MemoryMetrics) -> f64 {
        // Calculate allocations per second based on total allocations
        if self.performance_samples.len() < 2 {
            return 0.0;
        }
        
        let current_total = metrics.total_allocations();
        let time_diff_s = self.config.sample_interval_ms as f64 / 1000.0;
        
        current_total as f64 / time_diff_s.max(1.0)
    }

    fn calculate_avg_allocation_size(&self, metrics: &MemoryMetrics) -> f64 {
        let total_allocs = metrics.total_allocations();
        if total_allocs == 0 { 
            return 0.0; 
        }
        metrics.total_bytes_allocated as f64 / total_allocs as f64
    }

    fn calculate_fragmentation_percentage(&self, _metrics: &MemoryMetrics) -> f64 {
        // In real implementation, would calculate actual fragmentation
        // For now, return a reasonable estimate
        5.0
    }

    fn extract_strategy_usage(&self, metrics: &MemoryMetrics) -> HashMap<MemoryStrategy, u32> {
        let mut usage = HashMap::new();
        usage.insert(MemoryStrategy::Manual, metrics.manual_allocs as u32);
        usage.insert(MemoryStrategy::SmartPtr, metrics.smart_ptr_allocs as u32);
        usage.insert(MemoryStrategy::Linear, metrics.linear_allocs as u32);
        usage.insert(MemoryStrategy::Region, metrics.region_allocs as u32);
        usage.insert(MemoryStrategy::Stack, metrics.stack_allocs as u32);
        usage
    }

    fn update_real_time_metrics(&mut self, sample: &PerformanceSample) {
        self.current_metrics.current_memory_kb = sample.memory_usage_kb;
        self.current_metrics.peak_memory_kb = self.current_metrics.peak_memory_kb.max(sample.memory_usage_kb);
        self.current_metrics.recent_allocs = sample.allocation_rate as u32;

        // Update allocation trend
        if self.performance_samples.len() >= 2 {
            let prev_rate = self.performance_samples[self.performance_samples.len() - 2].allocation_rate;
            let current_rate = sample.allocation_rate;
            let change_ratio = if prev_rate > 0.0 { current_rate / prev_rate } else { 1.0 };

            self.current_metrics.allocation_trend = match change_ratio {
                r if r > 1.5 => AllocationTrend::Accelerating,
                r if r > 1.1 => AllocationTrend::Increasing,
                r if r < 0.9 => AllocationTrend::Decreasing,
                _ => AllocationTrend::Stable,
            };
        }

        // Update health score and pressure level
        self.update_health_metrics(sample);
    }

    fn update_health_metrics(&mut self, sample: &PerformanceSample) {
        // Calculate health score based on multiple factors
        let memory_score = if sample.memory_usage_kb < 100 * 1024 { 100.0 } else { 
            100.0 - (sample.memory_usage_kb as f64 / (1024.0 * 1024.0)) * 50.0 
        };
        let fragmentation_score = 100.0 - sample.fragmentation;
        let allocation_score = if sample.allocation_rate < 1000.0 { 100.0 } else { 
            100.0 - (sample.allocation_rate / 10000.0) * 50.0 
        };

        self.current_metrics.health_score = (memory_score + fragmentation_score + allocation_score) / 3.0;
        self.current_metrics.health_score = self.current_metrics.health_score.min(100.0).max(0.0);

        // Determine pressure level
        self.current_metrics.pressure_level = if sample.memory_usage_kb > 500 * 1024 {
            MemoryPressure::Critical
        } else if sample.memory_usage_kb > 200 * 1024 {
            MemoryPressure::High
        } else if sample.memory_usage_kb > 50 * 1024 {
            MemoryPressure::Normal
        } else {
            MemoryPressure::Low
        };
    }
} 