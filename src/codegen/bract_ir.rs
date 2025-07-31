//! Bract Intermediate Representation (IR) 
//!
//! This module defines Bract's custom IR that preserves high-level semantics:
//! - Memory strategy information for optimal lowering
//! - Ownership and lifetime annotations for safety verification
//! - Performance contracts and cost estimates
//! - Region and allocation annotations
//!
//! The IR serves as an intermediate step: AST → Bract IR → Cranelift IR
//! This allows for high-level optimizations before final code generation.

use crate::ast::{InternedString, Span, MemoryStrategy, Ownership, LifetimeId};
use cranelift_codegen::ir::Type as ClifType;
use std::collections::HashMap;
use std::fmt;

/// Unique identifier for Bract IR values
pub type BIRValueId = u32;

/// Unique identifier for Bract IR blocks
pub type BIRBlockId = u32;

/// Unique identifier for Bract IR functions  
pub type BIRFunctionId = u32;

/// Bract IR Type with memory management information
#[derive(Debug, Clone, PartialEq)]
pub enum BIRType {
    /// Integer types with memory strategy
    Integer {
        width: u8, // 8, 16, 32, 64, 128
        signed: bool,
        memory_strategy: MemoryStrategy,
    },
    /// Floating point types
    Float {
        width: u8, // 32, 64
        memory_strategy: MemoryStrategy,
    },
    /// Boolean type
    Bool {
        memory_strategy: MemoryStrategy,
    },
    /// Pointer type with ownership information
    Pointer {
        target_type: Box<BIRType>,
        is_mutable: bool,
        memory_strategy: MemoryStrategy,
        ownership: Ownership,
    },
    /// Reference type with lifetime
    Reference {
        target_type: Box<BIRType>,
        is_mutable: bool,
        lifetime: LifetimeId,
        ownership: Ownership,
    },
    /// Array type with size information
    Array {
        element_type: Box<BIRType>,
        size: u64,
        memory_strategy: MemoryStrategy,
    },
    /// Struct type with field layout
    Struct {
        name: InternedString,
        fields: Vec<BIRStructField>,
        memory_strategy: MemoryStrategy,
        size_hint: Option<u64>, // For performance optimization
    },
    /// Function type with calling convention
    Function {
        params: Vec<BIRType>,
        return_type: Box<BIRType>,
        calling_convention: CallingConvention,
    },
    /// Linear type that must be consumed exactly once
    Linear {
        inner_type: Box<BIRType>,
        consumed: bool, // Tracking for linear type analysis
    },
}

/// Struct field in Bract IR
#[derive(Debug, Clone, PartialEq)]
pub struct BIRStructField {
    pub name: InternedString,
    pub field_type: BIRType,
    pub offset: u32, // Byte offset for efficient access
    pub alignment: u8, // Alignment requirement
}

/// Calling convention for functions
#[derive(Debug, Clone, PartialEq)]
pub enum CallingConvention {
    /// Fast calling convention for internal functions
    Fast,
    /// C calling convention for FFI
    C,
    /// System V AMD64 ABI
    SystemV,
    /// Windows x64 calling convention
    Win64,
}

/// Bract IR Value with ownership and lifetime annotations
#[derive(Debug, Clone, PartialEq)]
pub struct BIRValue {
    pub id: BIRValueId,
    pub value_type: BIRType,
    pub ownership: Ownership,
    pub lifetime: Option<LifetimeId>,
    pub span: Span, // For debugging and error reporting
    pub performance_cost: u64, // Estimated cost for this value
}

impl BIRValue {
    pub fn new(
        id: BIRValueId, 
        value_type: BIRType, 
        ownership: Ownership, 
        span: Span
    ) -> Self {
        let performance_cost = value_type.estimated_cost();
        Self {
            id,
            value_type,
            ownership,
            lifetime: None,
            span,
            performance_cost,
        }
    }
    
    pub fn with_lifetime(mut self, lifetime: LifetimeId) -> Self {
        self.lifetime = Some(lifetime);
        self
    }
    
    /// Check if this value requires move semantics
    pub fn requires_move(&self) -> bool {
        self.value_type.requires_move() || matches!(self.ownership, Ownership::Linear)
    }
    
    /// Check if this value can be copied
    pub fn can_copy(&self) -> bool {
        self.value_type.allows_copy() && self.ownership.can_copy()
    }
}

/// Bract IR Operations with memory management semantics
#[derive(Debug, Clone, PartialEq)]
pub enum BIROp {
    /// Load value from memory
    Load {
        address: BIRValueId,
        memory_order: MemoryOrder,
        bounds_check: bool, // Enable runtime bounds checking
    },
    /// Store value to memory
    Store {
        address: BIRValueId,
        value: BIRValueId,
        memory_order: MemoryOrder,
        bounds_check: bool,
    },
    /// Allocate memory with specific strategy
    Allocate {
        size: BIRValueId,
        alignment: u8,
        strategy: MemoryStrategy,
        region_id: Option<u32>, // For region-based allocation
    },
    /// Deallocate memory
    Deallocate {
        pointer: BIRValueId,
        strategy: MemoryStrategy,
        region_id: Option<u32>,
    },
    /// Move operation for linear types
    Move {
        source: BIRValueId,
        check_consumed: bool, // Verify linear type not already consumed
    },
    /// Copy operation (only for copyable types)
    Copy {
        source: BIRValueId,
        deep_copy: bool, // Whether to perform deep copy
    },
    /// Borrow operation with lifetime
    Borrow {
        source: BIRValueId,
        is_mutable: bool,
        lifetime: LifetimeId,
    },
    /// Drop operation with strategy-specific cleanup
    Drop {
        value: BIRValueId,
        strategy: MemoryStrategy,
    },
    /// Arithmetic operations
    Add { lhs: BIRValueId, rhs: BIRValueId },
    Sub { lhs: BIRValueId, rhs: BIRValueId },
    Mul { lhs: BIRValueId, rhs: BIRValueId },
    Div { lhs: BIRValueId, rhs: BIRValueId },
    /// Comparison operations
    Eq { lhs: BIRValueId, rhs: BIRValueId },
    Lt { lhs: BIRValueId, rhs: BIRValueId },
    /// Control flow
    Call {
        function: BIRFunctionId,
        args: Vec<BIRValueId>,
        tail_call: bool, // For optimization
    },
    Return { value: Option<BIRValueId> },
    Branch {
        condition: BIRValueId,
        true_block: BIRBlockId,
        false_block: BIRBlockId,
    },
    Jump { target: BIRBlockId },
    /// Memory management specific operations
    ArcNew { value: BIRValueId }, // Create new ARC
    ArcIncref { arc: BIRValueId }, // Increment reference count
    ArcDecref { arc: BIRValueId }, // Decrement reference count
    RegionAlloc { 
        region: u32,
        size: BIRValueId,
        alignment: u8,
    },
    /// Performance profiling operations
    ProfileStart { marker: InternedString },
    ProfileEnd { marker: InternedString },
    /// Bounds checking operations  
    BoundsCheck {
        pointer: BIRValueId,
        offset: BIRValueId,
        size: BIRValueId,
    },
}

/// Memory ordering for load/store operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryOrder {
    Relaxed,
    Acquire,
    Release,
    AcquireRelease,
    SequentiallyConsistent,
}

/// Bract IR Instruction
#[derive(Debug, Clone, PartialEq)]
pub struct BIRInstruction {
    pub id: u32,
    pub op: BIROp,
    pub result: Option<BIRValue>,
    pub span: Span,
    pub cost_estimate: u64, // Performance cost estimate
}

impl BIRInstruction {
    pub fn new(id: u32, op: BIROp, span: Span) -> Self {
        let cost_estimate = Self::estimate_cost(&op);
        Self {
            id,
            op,
            result: None,
            span,
            cost_estimate,
        }
    }
    
    pub fn with_result(mut self, result: BIRValue) -> Self {
        self.result = Some(result);
        self
    }
    
    /// Estimate the performance cost of an operation
    fn estimate_cost(op: &BIROp) -> u64 {
        match op {
            // Memory operations are expensive
            BIROp::Allocate { strategy, .. } => strategy.allocation_cost() as u64 * 10,
            BIROp::Deallocate { strategy, .. } => strategy.allocation_cost() as u64 * 5,
            BIROp::Load { bounds_check, .. } => if *bounds_check { 15 } else { 10 },
            BIROp::Store { bounds_check, .. } => if *bounds_check { 20 } else { 15 },
            // ARC operations have overhead
            BIROp::ArcNew { .. } => 25,
            BIROp::ArcIncref { .. } => 5,
            BIROp::ArcDecref { .. } => 10,
            // Arithmetic is cheap
            BIROp::Add { .. } | BIROp::Sub { .. } | BIROp::Mul { .. } => 1,
            BIROp::Div { .. } => 5, // Division is more expensive
            // Control flow
            BIROp::Call { .. } => 20,
            BIROp::Branch { .. } => 3,
            BIROp::Jump { .. } => 2,
            // Other operations
            _ => 5,
        }
    }
}

/// Basic block in Bract IR
#[derive(Debug, Clone, PartialEq)]
pub struct BIRBasicBlock {
    pub id: BIRBlockId,
    pub instructions: Vec<BIRInstruction>,
    pub predecessors: Vec<BIRBlockId>,
    pub successors: Vec<BIRBlockId>,
    pub live_values: Vec<BIRValueId>, // Values live at block entry
}

impl BIRBasicBlock {
    pub fn new(id: BIRBlockId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
            live_values: Vec::new(),
        }
    }
    
    pub fn add_instruction(&mut self, instruction: BIRInstruction) {
        self.instructions.push(instruction);
    }
    
    /// Get the total cost estimate for this block
    pub fn total_cost(&self) -> u64 {
        self.instructions.iter().map(|inst| inst.cost_estimate).sum()
    }
}

/// Function in Bract IR with ownership and performance information
#[derive(Debug, Clone, PartialEq)]
pub struct BIRFunction {
    pub id: BIRFunctionId,
    pub name: InternedString,
    pub params: Vec<BIRValue>,
    pub return_type: BIRType,
    pub blocks: Vec<BIRBasicBlock>,
    pub calling_convention: CallingConvention,
    pub memory_regions: Vec<MemoryRegion>, // Active memory regions
    pub performance_contract: Option<PerformanceContract>,
    pub span: Span,
}

/// Memory region for region-based allocation
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryRegion {
    pub id: u32,
    pub size_hint: Option<u64>, // Expected size for pre-allocation
    pub alignment: u8,
    pub lifetime: LifetimeId,
}

/// Performance contract for functions
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceContract {
    pub max_allocation_cost: u64,
    pub max_stack_depth: u32,
    pub max_execution_time: Option<u64>, // In CPU cycles
    pub memory_bound: Option<u64>, // Maximum memory usage
}

/// Complete Bract IR module
#[derive(Debug, Clone, PartialEq)]
pub struct BIRModule {
    pub functions: HashMap<BIRFunctionId, BIRFunction>,
    pub global_values: HashMap<InternedString, BIRValue>,
    pub type_definitions: HashMap<InternedString, BIRType>,
    pub memory_regions: HashMap<u32, MemoryRegion>,
    pub performance_profile: PerformanceProfile,
}

/// Performance profiling information
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceProfile {
    pub total_allocation_cost: u64,
    pub memory_strategies: HashMap<MemoryStrategy, u64>, // Count of each strategy used
    pub hotspots: Vec<Hotspot>, // Performance-critical locations
}

/// Performance hotspot information
#[derive(Debug, Clone, PartialEq)]
pub struct Hotspot {
    pub location: Span,
    pub cost_estimate: u64,
    pub optimization_suggestions: Vec<String>,
}

impl BIRModule {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            global_values: HashMap::new(),
            type_definitions: HashMap::new(),
            memory_regions: HashMap::new(),
            performance_profile: PerformanceProfile {
                total_allocation_cost: 0,
                memory_strategies: HashMap::new(),
                hotspots: Vec::new(),
            },
        }
    }
    
    pub fn add_function(&mut self, function: BIRFunction) {
        self.functions.insert(function.id, function);
    }
    
    pub fn add_global(&mut self, name: InternedString, value: BIRValue) {
        self.global_values.insert(name, value);
    }
    
    /// Analyze performance characteristics of the module
    pub fn analyze_performance(&mut self) {
        let mut total_cost = 0u64;
        let mut strategy_counts: HashMap<MemoryStrategy, u64> = HashMap::new();
        let mut hotspots = Vec::new();
        
        for function in self.functions.values() {
            for block in &function.blocks {
                let block_cost = block.total_cost();
                total_cost += block_cost;
                
                // Identify performance hotspots
                if block_cost > 1000 { // Configurable threshold
                    hotspots.push(Hotspot {
                        location: function.span, // Should be more specific
                        cost_estimate: block_cost,
                        optimization_suggestions: self.suggest_optimizations(block),
                    });
                }
                
                // Count memory strategy usage
                for instruction in &block.instructions {
                    if let Some(strategy) = self.extract_memory_strategy(&instruction.op) {
                        *strategy_counts.entry(strategy).or_insert(0) += 1;
                    }
                }
            }
        }
        
        self.performance_profile = PerformanceProfile {
            total_allocation_cost: total_cost,
            memory_strategies: strategy_counts,
            hotspots,
        };
    }
    
    /// Suggest optimizations for a basic block
    fn suggest_optimizations(&self, block: &BIRBasicBlock) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Count expensive operations
        let mut alloc_count = 0;
        let mut arc_operations = 0;
        let mut bounds_checks = 0;
        
        for instruction in &block.instructions {
            match &instruction.op {
                BIROp::Allocate { .. } | BIROp::Deallocate { .. } => alloc_count += 1,
                BIROp::ArcNew { .. } | BIROp::ArcIncref { .. } | BIROp::ArcDecref { .. } => {
                    arc_operations += 1;
                }
                BIROp::BoundsCheck { .. } => bounds_checks += 1,
                _ => {}
            }
        }
        
        if alloc_count > 5 {
            suggestions.push("Consider using region-based allocation to batch memory operations".to_string());
        }
        
        if arc_operations > 10 {
            suggestions.push("High ARC overhead detected. Consider using linear types for single ownership".to_string());
        }
        
        if bounds_checks > 20 {
            suggestions.push("Many bounds checks detected. Consider using safe iterator patterns".to_string());
        }
        
        suggestions
    }
    
    /// Extract memory strategy from an operation
    fn extract_memory_strategy(&self, op: &BIROp) -> Option<MemoryStrategy> {
        match op {
            BIROp::Allocate { strategy, .. } | BIROp::Deallocate { strategy, .. } => Some(*strategy),
            _ => None,
        }
    }
}

impl BIRType {
    /// Estimate the performance cost of operations on this type
    pub fn estimated_cost(&self) -> u64 {
        match self {
            BIRType::Integer { width, memory_strategy, .. } => {
                let base_cost = (*width as u64) / 8; // Wider types cost more
                base_cost + memory_strategy.allocation_cost() as u64
            }
            BIRType::Float { memory_strategy, .. } => {
                5 + memory_strategy.allocation_cost() as u64 // Floating point operations
            }
            BIRType::Bool { memory_strategy } => {
                1 + memory_strategy.allocation_cost() as u64
            }
            BIRType::Pointer { memory_strategy, .. } => {
                8 + memory_strategy.allocation_cost() as u64 // Pointer size
            }
            BIRType::Reference { .. } => 8, // References are always cheap
            BIRType::Array { size, memory_strategy, .. } => {
                size * 2 + memory_strategy.allocation_cost() as u64
            }
            BIRType::Struct { size_hint, memory_strategy, .. } => {
                size_hint.unwrap_or(32) + memory_strategy.allocation_cost() as u64
            }
            BIRType::Function { .. } => 16, // Function pointers
            BIRType::Linear { inner_type, .. } => {
                inner_type.estimated_cost() + 5 // Linear tracking overhead
            }
        }
    }
    
    /// Check if this type requires move semantics
    pub fn requires_move(&self) -> bool {
        match self {
            BIRType::Linear { .. } => true,
            BIRType::Pointer { memory_strategy: MemoryStrategy::Linear, .. } => true,
            BIRType::Array { memory_strategy: MemoryStrategy::Linear, .. } => true,
            BIRType::Struct { memory_strategy: MemoryStrategy::Linear, .. } => true,
            _ => false,
        }
    }
    
    /// Check if this type allows copying
    pub fn allows_copy(&self) -> bool {
        match self {
            BIRType::Linear { .. } => false,
            BIRType::Pointer { memory_strategy, .. } => memory_strategy.allows_copy(),
            BIRType::Array { memory_strategy, .. } => memory_strategy.allows_copy(),
            BIRType::Struct { memory_strategy, .. } => memory_strategy.allows_copy(),
            _ => true,
        }
    }
    
    /// Get the Cranelift type for this Bract IR type
    pub fn to_cranelift_type(&self) -> Result<ClifType, String> {
        match self {
            BIRType::Integer { width: 8, .. } => Ok(ClifType::int(8).unwrap()),
            BIRType::Integer { width: 16, .. } => Ok(ClifType::int(16).unwrap()),
            BIRType::Integer { width: 32, .. } => Ok(ClifType::int(32).unwrap()),
            BIRType::Integer { width: 64, .. } => Ok(ClifType::int(64).unwrap()),
            BIRType::Float { width: 32, .. } => Ok(cranelift_codegen::ir::types::F32),
            BIRType::Float { width: 64, .. } => Ok(cranelift_codegen::ir::types::F64),
            BIRType::Bool { .. } => Ok(ClifType::int(8).unwrap()), // Represent bool as i8
            BIRType::Pointer { .. } | BIRType::Reference { .. } => {
                Ok(ClifType::int(64).unwrap()) // 64-bit pointers
            }
            _ => Err(format!("Cannot convert BIR type to Cranelift: {:?}", self)),
        }
    }
}

impl fmt::Display for BIRType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BIRType::Integer { width, signed, memory_strategy, .. } => {
                write!(f, "{}{} [{:?}]", if *signed { "i" } else { "u" }, width, memory_strategy)
            }
            BIRType::Float { width, memory_strategy, .. } => {
                write!(f, "f{} [{:?}]", width, memory_strategy)
            }
            BIRType::Bool { memory_strategy, .. } => {
                write!(f, "bool [{:?}]", memory_strategy)
            }
            BIRType::Pointer { target_type, memory_strategy, ownership, .. } => {
                write!(f, "*{:?} {:?} [{:?}]", ownership, target_type, memory_strategy)
            }
            BIRType::Reference { target_type, ownership, .. } => {
                write!(f, "&{:?} {:?}", ownership, target_type)
            }
            BIRType::Linear { inner_type, .. } => {
                write!(f, "linear {:?}", inner_type)
            }
            _ => write!(f, "{:?}", self),
        }
    }
} 