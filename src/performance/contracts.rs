//! Performance Contract Verification
//!
//! This module implements the contract verification engine that validates
//! estimated performance costs against declared @guarantee contracts.

use crate::ast::{PerformanceContract, CpuBound, MemoryBound, AllocationBound, LatencyBound, StackBound, BigOComplexity};
use super::{ContractViolation, ViolationType, PerformanceCost};
use std::time::Duration;

/// Contract verifier - validates estimated costs against declared contracts
pub struct ContractVerifier {
    /// Strict mode - fail on any uncertainty
    strict_mode: bool,
}

impl ContractVerifier {
    /// Create a new contract verifier
    pub fn new() -> Self {
        Self {
            strict_mode: true, // Default to strict mode
        }
    }

    /// Create a lenient contract verifier (allows some uncertainty)
    pub fn lenient() -> Self {
        Self {
            strict_mode: false,
        }
    }

    /// Verify a performance contract against estimated cost
    pub fn verify_contract(
        &self,
        function_name: &str,
        contract: &PerformanceContract,
        estimated_cost: &PerformanceCost,
    ) -> Vec<ContractViolation> {
        let mut violations = Vec::new();

        // Verify CPU bounds
        if let Some(ref cpu_bound) = contract.cpu_bound {
            if let Some(violation) = self.verify_cpu_bound(function_name, cpu_bound, estimated_cost) {
                violations.push(violation);
            }
        }

        // Verify memory bounds
        if let Some(ref memory_bound) = contract.memory_bound {
            if let Some(violation) = self.verify_memory_bound(function_name, memory_bound, estimated_cost) {
                violations.push(violation);
            }
        }

        // Verify allocation bounds
        if let Some(ref allocation_bound) = contract.allocation_bound {
            if let Some(violation) = self.verify_allocation_bound(function_name, allocation_bound, estimated_cost) {
                violations.push(violation);
            }
        }

        // Verify latency bounds
        if let Some(ref latency_bound) = contract.latency_bound {
            if let Some(violation) = self.verify_latency_bound(function_name, latency_bound, estimated_cost) {
                violations.push(violation);
            }
        }

        // Verify stack bounds
        if let Some(ref stack_bound) = contract.stack_bound {
            if let Some(violation) = self.verify_stack_bound(function_name, stack_bound, estimated_cost) {
                violations.push(violation);
            }
        }

        // Verify deterministic requirement
        if contract.deterministic {
            if let Some(violation) = self.verify_deterministic(function_name, estimated_cost) {
                violations.push(violation);
            }
        }

        // Verify wait-free requirement
        if contract.wait_free {
            if let Some(violation) = self.verify_wait_free(function_name, estimated_cost) {
                violations.push(violation);
            }
        }

        violations
    }

    /// Verify CPU bound
    fn verify_cpu_bound(
        &self,
        function_name: &str,
        cpu_bound: &CpuBound,
        estimated_cost: &PerformanceCost,
    ) -> Option<ContractViolation> {
        match cpu_bound {
            CpuBound::Cycles(max_cycles) => {
                if let Some(estimated_cycles) = estimated_cost.cycles {
                    if estimated_cycles > *max_cycles {
                        return Some(ContractViolation {
                            function_name: function_name.to_string(),
                            violation_type: ViolationType::CpuExceeded,
                            expected: format!("{} cycles", max_cycles),
                            actual: format!("{} cycles", estimated_cycles),
                            span: crate::ast::Span::single(crate::lexer::Position::default()),
                            message: format!(
                                "Function '{}' exceeds CPU cycle bound: estimated {} cycles > contract limit {} cycles",
                                function_name, estimated_cycles, max_cycles
                            ),
                        });
                    }
                } else if self.strict_mode {
                    // Cannot verify - treat as violation in strict mode
                    return Some(ContractViolation {
                        function_name: function_name.to_string(),
                        violation_type: ViolationType::CpuExceeded,
                        expected: format!("{} cycles", max_cycles),
                        actual: "unknown".to_string(),
                        span: crate::ast::Span::single(crate::lexer::Position::default()),
                        message: format!(
                            "Function '{}' has unknown CPU cost but declares cycle bound",
                            function_name
                        ),
                    });
                }
            }
            CpuBound::Time(max_time) => {
                // For time-based bounds, we need to estimate from cycles
                if let Some(estimated_cycles) = estimated_cost.cycles {
                    // Rough estimate: 3GHz = 3 billion cycles per second
                    let estimated_time_ns = (estimated_cycles as f64 / 3_000_000_000.0) * 1_000_000_000.0;
                    let max_time_ns = max_time.as_nanos() as f64;
                    
                    if estimated_time_ns > max_time_ns {
                        return Some(ContractViolation {
                            function_name: function_name.to_string(),
                            violation_type: ViolationType::CpuExceeded,
                            expected: format!("{:?}", max_time),
                            actual: format!("{:.2}μs", estimated_time_ns / 1000.0),
                            span: crate::ast::Span::single(crate::lexer::Position::default()),
                            message: format!(
                                "Function '{}' exceeds time bound: estimated {:.2}μs > contract limit {:?}",
                                function_name, estimated_time_ns / 1000.0, max_time
                            ),
                        });
                    }
                }
            }
            CpuBound::Complexity(complexity, _param) => {
                // For complexity bounds, we can only verify if we have more sophisticated analysis
                // For now, we'll just warn that complexity verification is not yet implemented
                // TODO: Implement complexity analysis
            }
        }
        None
    }

    /// Verify memory bound
    fn verify_memory_bound(
        &self,
        function_name: &str,
        memory_bound: &MemoryBound,
        estimated_cost: &PerformanceCost,
    ) -> Option<ContractViolation> {
        match memory_bound {
            MemoryBound::Bytes(max_bytes) => {
                if let Some(estimated_bytes) = estimated_cost.memory_bytes {
                    if estimated_bytes > *max_bytes {
                        return Some(ContractViolation {
                            function_name: function_name.to_string(),
                            violation_type: ViolationType::MemoryExceeded,
                            expected: format!("{} bytes", max_bytes),
                            actual: format!("{} bytes", estimated_bytes),
                            span: crate::ast::Span::single(crate::lexer::Position::default()),
                            message: format!(
                                "Function '{}' exceeds memory bound: {} bytes > {} bytes",
                                function_name, estimated_bytes, max_bytes
                            ),
                        });
                    }
                }
            }
            MemoryBound::Zero => {
                if let Some(estimated_bytes) = estimated_cost.memory_bytes {
                    if estimated_bytes > 0 {
                        return Some(ContractViolation {
                            function_name: function_name.to_string(),
                            violation_type: ViolationType::MemoryExceeded,
                            expected: "0 bytes".to_string(),
                            actual: format!("{} bytes", estimated_bytes),
                            span: crate::ast::Span::single(crate::lexer::Position::default()),
                            message: format!(
                                "Function '{}' uses memory but declares zero memory usage",
                                function_name
                            ),
                        });
                    }
                }
            }
            MemoryBound::Proportional(coeff, _param) => {
                // TODO: Implement proportional memory verification
                // This requires parameter size analysis
                let _ = coeff; // Suppress unused warning for now
            }
        }
        None
    }

    /// Verify allocation bound
    fn verify_allocation_bound(
        &self,
        function_name: &str,
        allocation_bound: &AllocationBound,
        estimated_cost: &PerformanceCost,
    ) -> Option<ContractViolation> {
        match allocation_bound {
            AllocationBound::Count(max_allocs) => {
                if let Some(estimated_allocs) = estimated_cost.allocations {
                    if estimated_allocs > *max_allocs {
                        return Some(ContractViolation {
                            function_name: function_name.to_string(),
                            violation_type: ViolationType::AllocationExceeded,
                            expected: format!("{} allocations", max_allocs),
                            actual: format!("{} allocations", estimated_allocs),
                            span: crate::ast::Span::single(crate::lexer::Position::default()),
                            message: format!(
                                "Function '{}' exceeds allocation bound: {} > {} allocations",
                                function_name, estimated_allocs, max_allocs
                            ),
                        });
                    }
                }
            }
            AllocationBound::None => {
                if let Some(estimated_allocs) = estimated_cost.allocations {
                    if estimated_allocs > 0 {
                        return Some(ContractViolation {
                            function_name: function_name.to_string(),
                            violation_type: ViolationType::AllocationExceeded,
                            expected: "0 allocations".to_string(),
                            actual: format!("{} allocations", estimated_allocs),
                            span: crate::ast::Span::single(crate::lexer::Position::default()),
                            message: format!(
                                "Function '{}' performs heap allocations but declares zero allocation usage",
                                function_name
                            ),
                        });
                    }
                }
            }
        }
        None
    }

    /// Verify latency bound
    fn verify_latency_bound(
        &self,
        function_name: &str,
        _latency_bound: &LatencyBound,
        _estimated_cost: &PerformanceCost,
    ) -> Option<ContractViolation> {
        // TODO: Implement latency verification
        // This requires runtime measurement or more sophisticated modeling
        let _ = (function_name, _latency_bound, _estimated_cost);
        None
    }

    /// Verify stack bound
    fn verify_stack_bound(
        &self,
        function_name: &str,
        stack_bound: &StackBound,
        estimated_cost: &PerformanceCost,
    ) -> Option<ContractViolation> {
        match stack_bound {
            StackBound::Bytes(max_stack) => {
                if let Some(estimated_stack) = estimated_cost.stack_bytes {
                    if estimated_stack > *max_stack {
                        return Some(ContractViolation {
                            function_name: function_name.to_string(),
                            violation_type: ViolationType::StackExceeded,
                            expected: format!("{} stack bytes", max_stack),
                            actual: format!("{} stack bytes", estimated_stack),
                            span: crate::ast::Span::single(crate::lexer::Position::default()),
                            message: format!(
                                "Function '{}' exceeds stack bound: {} > {} bytes",
                                function_name, estimated_stack, max_stack
                            ),
                        });
                    }
                }
            }
            StackBound::Zero => {
                if let Some(estimated_stack) = estimated_cost.stack_bytes {
                    if estimated_stack > 0 {
                        return Some(ContractViolation {
                            function_name: function_name.to_string(),
                            violation_type: ViolationType::StackExceeded,
                            expected: "0 stack bytes".to_string(),
                            actual: format!("{} stack bytes", estimated_stack),
                            span: crate::ast::Span::single(crate::lexer::Position::default()),
                            message: format!(
                                "Function '{}' uses stack but declares zero stack usage",
                                function_name
                            ),
                        });
                    }
                }
            }
        }
        None
    }

    /// Verify deterministic requirement
    fn verify_deterministic(
        &self,
        function_name: &str,
        _estimated_cost: &PerformanceCost,
    ) -> Option<ContractViolation> {
        // TODO: Implement deterministic analysis
        // This requires control flow analysis to detect non-deterministic operations
        let _ = (function_name, _estimated_cost);
        None
    }

    /// Verify wait-free requirement
    fn verify_wait_free(
        &self,
        function_name: &str,
        _estimated_cost: &PerformanceCost,
    ) -> Option<ContractViolation> {
        // TODO: Implement wait-free analysis
        // This requires concurrency analysis to detect blocking operations
        let _ = (function_name, _estimated_cost);
        None
    }
}

impl Default for ContractVerifier {
    fn default() -> Self {
        Self::new()
    }
} 