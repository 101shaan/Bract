//! Performance Analysis Module
//!
//! This module implements Bract's revolutionary performance-guaranteed systems programming:
//! - Compile-time performance contract verification
//! - Hardware-aware cost estimation
//! - Memory allocation strategy analysis
//! - Runtime performance profiling (debug mode)

use crate::ast::{
    PerformanceContract, CpuBound, MemoryBound, AllocationBound, 
    LatencyBound, StackBound, AllocationStrategy, Item, Module,
    BigOComplexity, Span
};
use std::collections::HashMap;
use std::time::Duration;

pub mod contracts;
pub mod estimation;
pub mod profiler;
pub mod models;

pub use contracts::ContractVerifier;
pub use estimation::{CostEstimator, PerformanceCost};
pub use profiler::PerformanceProfiler;
pub use models::{CostModel, TargetArchitecture};

/// Performance analysis results for a module
#[derive(Debug, Clone)]
pub struct PerformanceAnalysis {
    /// Function performance costs
    pub function_costs: HashMap<String, PerformanceCost>,
    /// Contract violations found
    pub violations: Vec<ContractViolation>,
    /// Performance warnings
    pub warnings: Vec<PerformanceWarning>,
    /// Analysis statistics
    pub stats: AnalysisStats,
}

/// Performance contract violation
#[derive(Debug, Clone, PartialEq)]
pub struct ContractViolation {
    /// Function name where violation occurred
    pub function_name: String,
    /// Type of violation
    pub violation_type: ViolationType,
    /// Expected bound from contract
    pub expected: String,
    /// Actual estimated cost
    pub actual: String,
    /// Source location
    pub span: Span,
    /// Detailed explanation
    pub message: String,
}

/// Types of contract violations
#[derive(Debug, Clone, PartialEq)]
pub enum ViolationType {
    CpuExceeded,
    MemoryExceeded,
    AllocationExceeded,
    LatencyExceeded,
    StackExceeded,
    NonDeterministic,
    NotWaitFree,
}

/// Performance warnings (non-breaking issues)
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceWarning {
    /// Function name
    pub function_name: String,
    /// Warning type
    pub warning_type: WarningType,
    /// Warning message
    pub message: String,
    /// Source location
    pub span: Span,
}

/// Types of performance warnings
#[derive(Debug, Clone, PartialEq)]
pub enum WarningType {
    MissingContract,
    InaccurateEstimate,
    SuboptimalAllocation,
    PotentialBottleneck,
    UnverifiedExtern,
}

/// Analysis statistics
#[derive(Debug, Clone, Default)]
pub struct AnalysisStats {
    pub functions_analyzed: usize,
    pub contracts_verified: usize,
    pub violations_found: usize,
    pub warnings_generated: usize,
    pub analysis_time: Duration,
}

/// Main performance analyzer
pub struct PerformanceAnalyzer {
    /// Cost estimator for different architectures
    cost_estimator: CostEstimator,
    /// Contract verifier
    contract_verifier: ContractVerifier,
    /// Target architecture
    target_arch: TargetArchitecture,
}

impl PerformanceAnalyzer {
    /// Create a new performance analyzer for the target architecture
    pub fn new(target_arch: TargetArchitecture) -> Self {
        Self {
            cost_estimator: CostEstimator::new(target_arch),
            contract_verifier: ContractVerifier::new(),
            target_arch,
        }
    }

    /// Analyze performance for an entire module
    pub fn analyze_module(&mut self, module: &Module) -> PerformanceAnalysis {
        let start_time = std::time::Instant::now();
        
        let mut analysis = PerformanceAnalysis {
            function_costs: HashMap::new(),
            violations: Vec::new(),
            warnings: Vec::new(),
            stats: AnalysisStats::default(),
        };

        // Analyze each item in the module
        for item in &module.items {
            if let Item::Function { 
                name, 
                performance_contract,
                allocation_strategy,
                body,
                .. 
            } = item {
                self.analyze_function(
                    name, 
                    performance_contract.as_ref(),
                    allocation_strategy.as_ref(),
                    body.as_ref(),
                    &mut analysis
                );
            }
        }

        // Update statistics
        analysis.stats.analysis_time = start_time.elapsed();
        analysis.stats.functions_analyzed = analysis.function_costs.len();
        analysis.stats.contracts_verified = analysis.function_costs.values()
            .filter(|cost| cost.has_contract)
            .count();
        analysis.stats.violations_found = analysis.violations.len();
        analysis.stats.warnings_generated = analysis.warnings.len();

        analysis
    }

    /// Analyze a single function
    fn analyze_function(
        &mut self,
        name: &crate::ast::InternedString,
        contract: Option<&PerformanceContract>,
        _allocation_strategy: Option<&AllocationStrategy>,
        body: Option<&crate::ast::Expr>,
        analysis: &mut PerformanceAnalysis,
    ) {
        // For now, use a placeholder function name
        let func_name = format!("function_{}", name.id);

        // Estimate performance cost from the function body
        let estimated_cost = if let Some(body_expr) = body {
            self.cost_estimator.estimate_expression_cost(body_expr)
        } else {
            // External function - unknown cost
            PerformanceCost::unknown()
        };

        // Verify contract if present
        if let Some(contract) = contract {
            let violations = self.contract_verifier.verify_contract(
                &func_name,
                contract,
                &estimated_cost,
            );
            analysis.violations.extend(violations);
        } else if body.is_some() {
            // Missing contract warning
            analysis.warnings.push(PerformanceWarning {
                function_name: func_name.clone(),
                warning_type: WarningType::MissingContract,
                message: "Function lacks performance contract - consider adding @guarantee annotation".to_string(),
                span: Span::single(crate::lexer::Position::default()), // TODO: Get actual span
            });
        }

        analysis.function_costs.insert(func_name, estimated_cost);
    }
}

impl Default for PerformanceAnalyzer {
    fn default() -> Self {
        Self::new(TargetArchitecture::X86_64)
    }
}

/// Performance cost estimation for expressions, statements, and functions
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceCost {
    /// Estimated CPU cycles
    pub cycles: Option<u64>,
    /// Memory footprint in bytes
    pub memory_bytes: Option<u64>,
    /// Number of heap allocations
    pub allocations: Option<u32>,
    /// Stack frame size in bytes
    pub stack_bytes: Option<u32>,
    /// Whether the cost has an associated contract
    pub has_contract: bool,
    /// Confidence level in the estimate (0.0 - 1.0)
    pub confidence: f32,
}

impl PerformanceCost {
    /// Create zero cost
    pub fn zero() -> Self {
        Self {
            cycles: Some(0),
            memory_bytes: Some(0),
            allocations: Some(0),
            stack_bytes: Some(0),
            has_contract: false,
            confidence: 1.0,
        }
    }

    /// Create unknown cost
    pub fn unknown() -> Self {
        Self {
            cycles: None,
            memory_bytes: None,
            allocations: None,
            stack_bytes: None,
            has_contract: false,
            confidence: 0.0,
        }
    }

    /// Add two costs together
    pub fn add(&self, other: &PerformanceCost) -> PerformanceCost {
        PerformanceCost {
            cycles: match (self.cycles, other.cycles) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            },
            memory_bytes: match (self.memory_bytes, other.memory_bytes) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            },
            allocations: match (self.allocations, other.allocations) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            },
            stack_bytes: match (self.stack_bytes, other.stack_bytes) {
                (Some(a), Some(b)) => Some(a.max(b)), // Stack is not additive
                _ => None,
            },
            has_contract: self.has_contract || other.has_contract,
            confidence: self.confidence.min(other.confidence),
        }
    }
}

impl std::ops::Add for PerformanceCost {
    type Output = PerformanceCost;

    fn add(self, other: PerformanceCost) -> PerformanceCost {
        self.add(&other)
    }
}

impl std::ops::AddAssign for PerformanceCost {
    fn add_assign(&mut self, other: PerformanceCost) {
        *self = self.add(&other);
    }
} 