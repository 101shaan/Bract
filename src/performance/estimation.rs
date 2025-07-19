//! Performance Cost Estimation
//!
//! This module implements cost estimation for Bract expressions and statements,
//! providing the foundation for performance contract verification.

use crate::ast::{Expr, Stmt, BinaryOp, UnaryOp, Literal, InternedString, Type};
use super::{PerformanceCost, models::{CostModel, TargetArchitecture}};

/// Cost estimator - estimates performance costs from AST nodes
pub struct CostEstimator {
    /// Target architecture for cost modeling
    target_arch: TargetArchitecture,
    /// Cost model for the target architecture
    cost_model: CostModel,
}

impl CostEstimator {
    /// Create a new cost estimator for the target architecture
    pub fn new(target_arch: TargetArchitecture) -> Self {
        let cost_model = CostModel::for_architecture(target_arch);
        Self {
            target_arch,
            cost_model,
        }
    }

    /// Estimate cost of an expression
    pub fn estimate_expression_cost(&self, expr: &Expr) -> PerformanceCost {
        match expr {
            Expr::Literal(literal) => self.estimate_literal_cost(literal),
            Expr::Identifier(_) => PerformanceCost::zero(), // Variable access is essentially free
            Expr::Binary { left, op, right, .. } => self.estimate_binary_cost(left, op, right),
            Expr::Unary { op, operand, .. } => self.estimate_unary_cost(op, operand),
            Expr::Call { func, args, .. } => self.estimate_call_cost(func, args),
            Expr::Index { object, index, .. } => self.estimate_index_cost(object, index),
            Expr::FieldAccess { object, .. } => self.estimate_field_access_cost(object),
            Expr::Block { stmts, .. } => self.estimate_block_cost(stmts),
            Expr::If { condition, then_branch, else_branch, .. } => {
                self.estimate_if_cost(condition, then_branch, else_branch.as_deref())
            }
            Expr::While { condition, body, .. } => self.estimate_while_cost(condition, body),
            Expr::For { .. } => self.estimate_for_cost(), // TODO: Implement properly
            Expr::Loop { body, .. } => self.estimate_loop_cost(body),
            Expr::Match { .. } => self.estimate_match_cost(), // TODO: Implement properly
            Expr::Return { value, .. } => {
                let mut cost = PerformanceCost {
                    cycles: Some(self.cost_model.return_cost),
                    ..PerformanceCost::zero()
                };
                if let Some(val) = value {
                    cost += self.estimate_expression_cost(val);
                }
                cost
            }
            Expr::Break { .. } | Expr::Continue { .. } => PerformanceCost {
                cycles: Some(self.cost_model.control_flow_cost),
                ..PerformanceCost::zero()
            },
            Expr::Array { elements, .. } => self.estimate_array_cost(elements),
            Expr::ArrayRepeat { element, size, .. } => self.estimate_array_repeat_cost(element, size),
            Expr::Tuple { elements, .. } => self.estimate_tuple_cost(elements),
            Expr::Struct { .. } => self.estimate_struct_cost(), // TODO: Implement properly
            Expr::Closure { .. } => self.estimate_closure_cost(), // TODO: Implement properly
        }
    }

    /// Estimate cost of a statement
    pub fn estimate_statement_cost(&self, stmt: &Stmt) -> PerformanceCost {
        match stmt {
            Stmt::Expression { expr, .. } => self.estimate_expression_cost(expr),
            Stmt::Let { value, .. } => {
                let mut cost = PerformanceCost {
                    cycles: Some(self.cost_model.assignment_cost),
                    stack_bytes: Some(8), // Rough estimate for local variable
                    ..PerformanceCost::zero()
                };
                if let Some(init_expr) = value {
                    cost += self.estimate_expression_cost(init_expr);
                }
                cost
            }
            Stmt::Assignment { target, value, .. } => {
                let mut cost = PerformanceCost {
                    cycles: Some(self.cost_model.assignment_cost),
                    ..PerformanceCost::zero()
                };
                cost += self.estimate_expression_cost(target);
                cost += self.estimate_expression_cost(value);
                cost
            }
        }
    }

    /// Estimate cost of a literal value
    fn estimate_literal_cost(&self, _literal: &Literal) -> PerformanceCost {
        // Loading constants is very cheap
        PerformanceCost {
            cycles: Some(1),
            ..PerformanceCost::zero()
        }
    }

    /// Estimate cost of a binary operation
    fn estimate_binary_cost(&self, left: &Expr, op: &BinaryOp, right: &Expr) -> PerformanceCost {
        let mut cost = self.estimate_expression_cost(left);
        cost += self.estimate_expression_cost(right);
        
        // Add operation-specific cost
        let op_cost = match op {
            BinaryOp::Add | BinaryOp::Subtract => self.cost_model.arithmetic_cost,
            BinaryOp::Multiply => self.cost_model.multiply_cost,
            BinaryOp::Divide | BinaryOp::Modulo => self.cost_model.divide_cost,
            BinaryOp::BitwiseAnd | BinaryOp::BitwiseOr | BinaryOp::BitwiseXor => self.cost_model.bitwise_cost,
            BinaryOp::LeftShift | BinaryOp::RightShift => self.cost_model.shift_cost,
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => self.cost_model.logical_cost,
            BinaryOp::Equal | BinaryOp::NotEqual | 
            BinaryOp::Less | BinaryOp::LessEqual |
            BinaryOp::Greater | BinaryOp::GreaterEqual => self.cost_model.comparison_cost,
            BinaryOp::Assign => self.cost_model.assignment_cost,
        };
        
        cost.cycles = cost.cycles.map(|c| c + op_cost);
        cost
    }

    /// Estimate cost of a unary operation
    fn estimate_unary_cost(&self, op: &UnaryOp, operand: &Expr) -> PerformanceCost {
        let mut cost = self.estimate_expression_cost(operand);
        
        let op_cost = match op {
            UnaryOp::Not | UnaryOp::BitwiseNot => self.cost_model.logical_cost,
            UnaryOp::Negate | UnaryOp::Plus => self.cost_model.arithmetic_cost,
            UnaryOp::Dereference => self.cost_model.memory_access_cost,
            UnaryOp::AddressOf | UnaryOp::MutableRef => 1, // Very cheap
        };
        
        cost.cycles = cost.cycles.map(|c| c + op_cost);
        cost
    }

    /// Estimate cost of a function call
    fn estimate_call_cost(&self, _func: &Expr, args: &[Expr]) -> PerformanceCost {
        // Base function call overhead
        let mut cost = PerformanceCost {
            cycles: Some(self.cost_model.function_call_cost),
            stack_bytes: Some(32), // Rough estimate for call frame
            ..PerformanceCost::zero()
        };

        // Add cost of evaluating arguments
        for arg in args {
            cost += self.estimate_expression_cost(arg);
        }

        // TODO: Look up actual function costs from symbol table
        // For now, assume unknown cost for the function body
        cost.confidence = 0.5; // Lower confidence for function calls

        cost
    }

    /// Estimate cost of array indexing
    fn estimate_index_cost(&self, object: &Expr, index: &Expr) -> PerformanceCost {
        let mut cost = self.estimate_expression_cost(object);
        cost += self.estimate_expression_cost(index);
        
        // Array indexing cost (address calculation + memory access)
        cost.cycles = cost.cycles.map(|c| c + self.cost_model.memory_access_cost + self.cost_model.arithmetic_cost);
        cost
    }

    /// Estimate cost of field access
    fn estimate_field_access_cost(&self, object: &Expr) -> PerformanceCost {
        let mut cost = self.estimate_expression_cost(object);
        
        // Field access is essentially free (just offset calculation)
        cost.cycles = cost.cycles.map(|c| c + 1);
        cost
    }

    /// Estimate cost of a block expression
    fn estimate_block_cost(&self, stmts: &[Stmt]) -> PerformanceCost {
        let mut total_cost = PerformanceCost::zero();
        
        for stmt in stmts {
            total_cost += self.estimate_statement_cost(stmt);
        }
        
        total_cost
    }

    /// Estimate cost of an if expression
    fn estimate_if_cost(&self, condition: &Expr, then_branch: &Expr, else_branch: Option<&Expr>) -> PerformanceCost {
        let mut cost = self.estimate_expression_cost(condition);
        
        // Add branch cost
        cost.cycles = cost.cycles.map(|c| c + self.cost_model.branch_cost);
        
        // For if expressions, we take the maximum of both branches (worst case)
        let then_cost = self.estimate_expression_cost(then_branch);
        let else_cost = else_branch.map(|e| self.estimate_expression_cost(e))
            .unwrap_or_else(PerformanceCost::zero);
        
        // Take the maximum of both branches for worst-case estimation
        let branch_cost = PerformanceCost {
            cycles: match (then_cost.cycles, else_cost.cycles) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            memory_bytes: match (then_cost.memory_bytes, else_cost.memory_bytes) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            allocations: match (then_cost.allocations, else_cost.allocations) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            stack_bytes: match (then_cost.stack_bytes, else_cost.stack_bytes) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            has_contract: then_cost.has_contract || else_cost.has_contract,
            confidence: then_cost.confidence.min(else_cost.confidence),
        };
        
        cost.add(&branch_cost)
    }

    /// Estimate cost of a while loop
    fn estimate_while_cost(&self, condition: &Expr, body: &Expr) -> PerformanceCost {
        let condition_cost = self.estimate_expression_cost(condition);
        let body_cost = self.estimate_expression_cost(body);
        
        // For loops, we can't know iteration count, so we return unknown costs
        // In a more sophisticated system, we might analyze loop bounds
        PerformanceCost {
            cycles: None, // Unknown - depends on iteration count
            memory_bytes: body_cost.memory_bytes, // Memory usage per iteration
            allocations: body_cost.allocations, // Allocations per iteration
            stack_bytes: body_cost.stack_bytes, // Stack usage
            has_contract: condition_cost.has_contract || body_cost.has_contract,
            confidence: 0.0, // Very low confidence for loops
        }
    }

    /// Estimate cost of a for loop
    fn estimate_for_cost(&self) -> PerformanceCost {
        // TODO: Implement proper for loop analysis
        PerformanceCost::unknown()
    }

    /// Estimate cost of an infinite loop
    fn estimate_loop_cost(&self, _body: &Expr) -> PerformanceCost {
        // Infinite loops have infinite cost
        PerformanceCost::unknown()
    }

    /// Estimate cost of a match expression
    fn estimate_match_cost(&self) -> PerformanceCost {
        // TODO: Implement proper match analysis
        PerformanceCost::unknown()
    }

    /// Estimate cost of array creation
    fn estimate_array_cost(&self, elements: &[Expr]) -> PerformanceCost {
        let mut cost = PerformanceCost {
            cycles: Some(self.cost_model.allocation_cost), // Array allocation
            memory_bytes: Some(elements.len() as u64 * 8), // Rough estimate
            allocations: Some(1),
            ..PerformanceCost::zero()
        };

        // Add cost of evaluating each element
        for element in elements {
            cost += self.estimate_expression_cost(element);
        }

        cost
    }

    /// Estimate cost of array repeat
    fn estimate_array_repeat_cost(&self, element: &Expr, _size: &Expr) -> PerformanceCost {
        let mut cost = self.estimate_expression_cost(element);
        
        // Array repeat involves allocation and potentially copying
        cost.cycles = cost.cycles.map(|c| c + self.cost_model.allocation_cost);
        cost.allocations = Some(cost.allocations.unwrap_or(0) + 1);
        
        // TODO: Analyze size expression for better estimates
        cost.confidence = 0.5; // Lower confidence without size analysis
        cost
    }

    /// Estimate cost of tuple creation
    fn estimate_tuple_cost(&self, elements: &[Expr]) -> PerformanceCost {
        let mut cost = PerformanceCost {
            stack_bytes: Some(elements.len() as u32 * 8), // Rough estimate
            ..PerformanceCost::zero()
        };

        // Add cost of evaluating each element
        for element in elements {
            cost += self.estimate_expression_cost(element);
        }

        cost
    }

    /// Estimate cost of struct creation
    fn estimate_struct_cost(&self) -> PerformanceCost {
        // TODO: Implement proper struct analysis
        PerformanceCost {
            cycles: Some(10), // Rough estimate
            stack_bytes: Some(32), // Rough estimate
            ..PerformanceCost::zero()
        }
    }

    /// Estimate cost of closure creation
    fn estimate_closure_cost(&self) -> PerformanceCost {
        // TODO: Implement proper closure analysis
        PerformanceCost {
            cycles: Some(self.cost_model.allocation_cost),
            allocations: Some(1), // Closure typically involves heap allocation
            ..PerformanceCost::zero()
        }
    }
}

impl Default for CostEstimator {
    fn default() -> Self {
        Self::new(TargetArchitecture::X86_64)
    }
} 