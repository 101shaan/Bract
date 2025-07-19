//! Hardware Cost Models
//!
//! This module defines cost models for different target architectures,
//! providing architecture-specific cycle counts and performance characteristics.

/// Supported target architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetArchitecture {
    X86_64,
    ARM64,
    RISCV64,
    WASM,
}

/// Architecture-specific cost model
#[derive(Debug, Clone)]
pub struct CostModel {
    /// Architecture identifier
    pub architecture: TargetArchitecture,
    
    // Arithmetic operation costs (in CPU cycles)
    pub arithmetic_cost: u64,       // +, -, basic operations
    pub multiply_cost: u64,         // * operation
    pub divide_cost: u64,           // /, % operations
    pub bitwise_cost: u64,          // &, |, ^
    pub shift_cost: u64,            // <<, >>
    pub logical_cost: u64,          // &&, ||, !
    pub comparison_cost: u64,       // ==, !=, <, etc.
    
    // Memory operation costs
    pub memory_access_cost: u64,    // Load/store operations
    pub allocation_cost: u64,       // Heap allocation overhead
    pub deallocation_cost: u64,     // Heap deallocation overhead
    
    // Control flow costs
    pub branch_cost: u64,           // Conditional branches
    pub function_call_cost: u64,    // Function call overhead
    pub return_cost: u64,           // Function return
    pub control_flow_cost: u64,     // break, continue, etc.
    
    // Assignment costs
    pub assignment_cost: u64,       // Variable assignment
    
    // Cache and memory hierarchy
    pub l1_cache_hit_cost: u64,     // L1 cache hit latency
    pub l2_cache_hit_cost: u64,     // L2 cache hit latency  
    pub l3_cache_hit_cost: u64,     // L3 cache hit latency
    pub memory_miss_cost: u64,      // Main memory access
    
    // Architecture characteristics
    pub register_count: u32,        // Number of available registers
    pub cache_line_size: u32,       // Cache line size in bytes
    pub page_size: u32,             // Virtual memory page size
    pub instruction_bytes: u32,     // Average instruction size
    
    // Frequency characteristics (for time estimation)
    pub base_frequency_hz: u64,     // Base CPU frequency
    pub boost_frequency_hz: u64,    // Maximum boost frequency
}

impl CostModel {
    /// Create a cost model for the specified architecture
    pub fn for_architecture(arch: TargetArchitecture) -> Self {
        match arch {
            TargetArchitecture::X86_64 => Self::x86_64(),
            TargetArchitecture::ARM64 => Self::arm64(),
            TargetArchitecture::RISCV64 => Self::riscv64(),
            TargetArchitecture::WASM => Self::wasm(),
        }
    }
    
    /// x86-64 cost model (Intel/AMD)
    pub fn x86_64() -> Self {
        Self {
            architecture: TargetArchitecture::X86_64,
            
            // x86-64 operation costs (optimistic modern CPU)
            arithmetic_cost: 1,      // Most arithmetic is 1 cycle
            multiply_cost: 3,        // Integer multiply ~3 cycles
            divide_cost: 25,         // Integer divide ~25 cycles
            bitwise_cost: 1,         // Bitwise ops are 1 cycle
            shift_cost: 1,           // Shift ops are 1 cycle
            logical_cost: 1,         // Logical ops are 1 cycle
            comparison_cost: 1,      // Comparisons are 1 cycle
            
            // Memory operations
            memory_access_cost: 4,   // L1 cache hit ~4 cycles
            allocation_cost: 100,    // Heap allocation ~100 cycles
            deallocation_cost: 50,   // Deallocation ~50 cycles
            
            // Control flow
            branch_cost: 1,          // Predicted branches ~1 cycle
            function_call_cost: 5,   // Call overhead ~5 cycles
            return_cost: 2,          // Return overhead ~2 cycles
            control_flow_cost: 1,    // Jump instructions ~1 cycle
            
            // Assignment
            assignment_cost: 1,      // Register/stack assignment ~1 cycle
            
            // Cache hierarchy (cycles)
            l1_cache_hit_cost: 4,
            l2_cache_hit_cost: 12,
            l3_cache_hit_cost: 40,
            memory_miss_cost: 200,   // Main memory ~200 cycles
            
            // Architecture characteristics
            register_count: 16,      // x86-64 has 16 general-purpose registers
            cache_line_size: 64,     // 64 bytes
            page_size: 4096,         // 4KB pages
            instruction_bytes: 4,    // Average instruction length
            
            // Frequency (3.0 GHz base, 4.5 GHz boost)
            base_frequency_hz: 3_000_000_000,
            boost_frequency_hz: 4_500_000_000,
        }
    }
    
    /// ARM64 cost model (Apple Silicon, AWS Graviton, etc.)
    pub fn arm64() -> Self {
        Self {
            architecture: TargetArchitecture::ARM64,
            
            // ARM64 operation costs
            arithmetic_cost: 1,
            multiply_cost: 2,        // ARM64 multiply is faster
            divide_cost: 15,         // ARM64 divide is faster than x86
            bitwise_cost: 1,
            shift_cost: 1,
            logical_cost: 1,
            comparison_cost: 1,
            
            // Memory operations
            memory_access_cost: 3,   // ARM64 typically has faster L1
            allocation_cost: 80,     // Slightly more efficient allocator
            deallocation_cost: 40,
            
            // Control flow
            branch_cost: 1,
            function_call_cost: 4,   // ARM64 calling convention is efficient
            return_cost: 1,
            control_flow_cost: 1,
            
            // Assignment
            assignment_cost: 1,
            
            // Cache hierarchy
            l1_cache_hit_cost: 3,
            l2_cache_hit_cost: 8,
            l3_cache_hit_cost: 25,
            memory_miss_cost: 150,
            
            // Architecture characteristics
            register_count: 31,      // ARM64 has 31 general-purpose registers
            cache_line_size: 64,
            page_size: 4096,
            instruction_bytes: 4,    // ARM64 instructions are 4 bytes
            
            // Frequency (varies widely, using conservative estimates)
            base_frequency_hz: 2_400_000_000,  // 2.4 GHz
            boost_frequency_hz: 3_200_000_000, // 3.2 GHz
        }
    }
    
    /// RISC-V cost model
    pub fn riscv64() -> Self {
        Self {
            architecture: TargetArchitecture::RISCV64,
            
            // RISC-V operation costs (simple, predictable)
            arithmetic_cost: 1,
            multiply_cost: 4,        // RISC-V multiply extension
            divide_cost: 35,         // RISC-V divide is slower
            bitwise_cost: 1,
            shift_cost: 1,
            logical_cost: 1,
            comparison_cost: 1,
            
            // Memory operations
            memory_access_cost: 5,   // Conservative estimate
            allocation_cost: 120,    // Less optimized ecosystem
            deallocation_cost: 60,
            
            // Control flow
            branch_cost: 2,          // RISC-V branch prediction varies
            function_call_cost: 6,
            return_cost: 2,
            control_flow_cost: 1,
            
            // Assignment
            assignment_cost: 1,
            
            // Cache hierarchy (conservative estimates)
            l1_cache_hit_cost: 5,
            l2_cache_hit_cost: 15,
            l3_cache_hit_cost: 50,
            memory_miss_cost: 250,
            
            // Architecture characteristics
            register_count: 32,      // RISC-V has 32 general-purpose registers
            cache_line_size: 64,
            page_size: 4096,
            instruction_bytes: 4,    // RISC-V instructions are 4 bytes
            
            // Frequency (conservative estimates for current RISC-V)
            base_frequency_hz: 1_500_000_000,  // 1.5 GHz
            boost_frequency_hz: 2_000_000_000, // 2.0 GHz
        }
    }
    
    /// WebAssembly cost model
    pub fn wasm() -> Self {
        Self {
            architecture: TargetArchitecture::WASM,
            
            // WASM operation costs (interpreter/JIT overhead)
            arithmetic_cost: 2,      // WASM has some overhead
            multiply_cost: 5,        // More expensive in WASM
            divide_cost: 40,         // Division is expensive
            bitwise_cost: 2,
            shift_cost: 2,
            logical_cost: 2,
            comparison_cost: 2,
            
            // Memory operations
            memory_access_cost: 10,  // WASM memory access has overhead
            allocation_cost: 200,    // WASM allocation is expensive
            deallocation_cost: 100,
            
            // Control flow
            branch_cost: 3,          // WASM branches have overhead
            function_call_cost: 15,  // WASM calls are expensive
            return_cost: 5,
            control_flow_cost: 3,
            
            // Assignment
            assignment_cost: 2,
            
            // "Cache" hierarchy (WASM doesn't have real caches)
            l1_cache_hit_cost: 10,
            l2_cache_hit_cost: 10,
            l3_cache_hit_cost: 10,
            memory_miss_cost: 10,    // WASM linear memory
            
            // Architecture characteristics
            register_count: 8,       // WASM stack machine (estimated)
            cache_line_size: 64,     // Host architecture dependent
            page_size: 65536,        // WASM page size is 64KB
            instruction_bytes: 2,    // WASM instructions vary
            
            // Frequency (depends on host, using conservative estimates)
            base_frequency_hz: 2_000_000_000,  // Depends on host
            boost_frequency_hz: 3_000_000_000,
        }
    }
    
    /// Estimate cycles to nanoseconds conversion
    pub fn cycles_to_nanoseconds(&self, cycles: u64) -> f64 {
        (cycles as f64 / self.base_frequency_hz as f64) * 1_000_000_000.0
    }
    
    /// Estimate nanoseconds to cycles conversion
    pub fn nanoseconds_to_cycles(&self, nanoseconds: f64) -> u64 {
        ((nanoseconds / 1_000_000_000.0) * self.base_frequency_hz as f64) as u64
    }
    
    /// Get a performance factor relative to x86-64 baseline
    pub fn relative_performance_factor(&self) -> f32 {
        match self.architecture {
            TargetArchitecture::X86_64 => 1.0,     // Baseline
            TargetArchitecture::ARM64 => 0.95,     // Slightly slower on average
            TargetArchitecture::RISCV64 => 0.7,    // Slower, less optimized
            TargetArchitecture::WASM => 0.3,       // Much slower due to interpretation
        }
    }
    
    /// Estimate memory bandwidth (bytes per second)
    pub fn memory_bandwidth_bytes_per_sec(&self) -> u64 {
        match self.architecture {
            TargetArchitecture::X86_64 => 50_000_000_000,  // ~50 GB/s DDR4
            TargetArchitecture::ARM64 => 40_000_000_000,   // ~40 GB/s 
            TargetArchitecture::RISCV64 => 20_000_000_000, // ~20 GB/s (conservative)
            TargetArchitecture::WASM => 10_000_000_000,    // Limited by host
        }
    }
}

impl Default for CostModel {
    fn default() -> Self {
        Self::x86_64()
    }
} 