//! Advanced Type System for Bract Programming Language
//!
//! This module implements a comprehensive type system with:
//! - Memory strategy integration (Manual, SmartPtr, Linear, Region, Stack)
//! - Ownership and lifetime analysis for memory safety
//! - Hindley-Milner style type inference with constraint solving
//! - Performance-optimized type checking
//! - Integration with hybrid memory management

use crate::ast::{
    Type, Expr, Stmt, Item, Module, Literal, PrimitiveType, Span, InternedString,
    MemoryStrategy, Ownership, LifetimeId, TypeBound, TypeConstraint, BinaryOp, UnaryOp
};
use crate::semantic::symbols::{SymbolTable, SymbolKind};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// Result type for type operations
pub type TypeResult<T> = Result<T, TypeError>;

/// Comprehensive type errors with actionable diagnostics
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    /// Type mismatch with memory strategy conflict
    Mismatch {
        expected: Type,
        actual: Type,
        span: Span,
        suggestion: Option<String>,
    },
    /// Memory strategy conflict
    StrategyConflict {
        expected_strategy: MemoryStrategy,
        actual_strategy: MemoryStrategy,
        span: Span,
        performance_note: String,
    },
    /// Ownership violation (use after move, multiple mutable borrows, etc.)
    OwnershipViolation {
        violation: OwnershipViolation,
        span: Span,
        help: String,
    },
    /// Lifetime error
    LifetimeError {
        error: LifetimeError,
        span: Span,
    },
    /// Linear type usage error
    LinearTypeError {
        name: InternedString,
        error: LinearError,
        span: Span,
    },
    /// Type inference failure
    InferenceFailure {
        reason: String,
        span: Span,
        constraints: Vec<TypeConstraint>,
    },
    /// Undefined type
    UndefinedType {
        name: InternedString,
        span: Span,
        suggestions: Vec<String>,
    },
    /// Performance contract violation
    PerformanceViolation {
        message: String,
        cost_estimate: u64,
        threshold: u64,
        span: Span,
    },
}

/// Ownership violation types
#[derive(Debug, Clone, PartialEq)]
pub enum OwnershipViolation {
    UseAfterMove(InternedString),
    MultipleMutableBorrows(InternedString),
    BorrowAfterMove(InternedString),
    MutateImmutableBorrow(InternedString),
    DropWhileBorrowed(InternedString),
}

/// Lifetime error types
#[derive(Debug, Clone, PartialEq)]
pub enum LifetimeError {
    OutlivesRegion(LifetimeId, LifetimeId),
    UseAfterFree(LifetimeId),
    DanglingReference(LifetimeId),
}

/// Linear type error types
#[derive(Debug, Clone, PartialEq)]
pub enum LinearError {
    NotConsumed,
    ConsumedMultipleTimes,
    PartialConsumption,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::Mismatch { expected, actual, suggestion, .. } => {
                write!(f, "Type mismatch: expected {:?}, found {:?}", expected, actual)?;
                if let Some(suggestion) = suggestion {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            TypeError::StrategyConflict { expected_strategy, actual_strategy, performance_note, .. } => {
                write!(f, "Memory strategy conflict: expected {:?}, found {:?}\nPerformance impact: {}", 
                       expected_strategy, actual_strategy, performance_note)
            }
            TypeError::OwnershipViolation { violation, help, .. } => {
                write!(f, "Ownership violation: {:?}\nHelp: {}", violation, help)
            }
            TypeError::LifetimeError { error, .. } => {
                write!(f, "Lifetime error: {:?}", error)
            }
            TypeError::LinearTypeError { name, error, .. } => {
                write!(f, "Linear type error for '{}': {:?}", name.id, error)
            }
            TypeError::InferenceFailure { reason, .. } => {
                write!(f, "Type inference failed: {}", reason)
            }
            TypeError::UndefinedType { name, suggestions, .. } => {
                write!(f, "Undefined type '{}'", name.id)?;
                if !suggestions.is_empty() {
                    write!(f, "\nDid you mean: {}", suggestions.join(", "))?;
                }
                Ok(())
            }
            TypeError::PerformanceViolation { message, cost_estimate, threshold, .. } => {
                write!(f, "Performance violation: {}\nEstimated cost: {}, threshold: {}", 
                       message, cost_estimate, threshold)
            }
        }
    }
}

/// Type inference context for constraint solving
#[derive(Debug, Clone)]
pub struct InferenceContext {
    /// Type variables and their constraints
    type_vars: HashMap<u32, Vec<TypeConstraint>>,
    /// Substitutions found during unification
    substitutions: HashMap<u32, Type>,
    /// Next type variable ID
    next_type_var: u32,
    /// Lifetime variables and their bounds
    lifetime_vars: HashMap<u32, Vec<LifetimeId>>,
    /// Next lifetime variable ID
    next_lifetime_var: u32,
}

impl InferenceContext {
    pub fn new() -> Self {
        Self {
            type_vars: HashMap::new(),
            substitutions: HashMap::new(),
            next_type_var: 0,
            lifetime_vars: HashMap::new(),
            next_lifetime_var: 0,
        }
    }
    
    /// Create a new type variable
    pub fn new_type_var(&mut self, constraints: Vec<TypeConstraint>) -> u32 {
        let id = self.next_type_var;
        self.next_type_var += 1;
        self.type_vars.insert(id, constraints);
        id
    }
    
    /// Create a new lifetime variable
    pub fn new_lifetime_var(&mut self, bounds: Vec<LifetimeId>) -> u32 {
        let id = self.next_lifetime_var;
        self.next_lifetime_var += 1;
        self.lifetime_vars.insert(id, bounds);
        id
    }
    
    /// Add constraint to type variable
    pub fn add_constraint(&mut self, type_var: u32, constraint: TypeConstraint) {
        self.type_vars.entry(type_var).or_insert_with(Vec::new).push(constraint);
    }
    
    /// Solve constraints using unification
    pub fn solve(&mut self) -> TypeResult<()> {
        // Implement constraint solving algorithm
        // This is a simplified version - full implementation would be much more complex
        let mut changed = true;
        while changed {
            changed = false;
            
            // Process each type variable
            for (&type_var, constraints) in self.type_vars.iter() {
                if self.substitutions.contains_key(&type_var) {
                    continue; // Already solved
                }
                
                // Try to find a concrete type that satisfies all constraints
                if let Some(solution) = self.find_solution(constraints) {
                    self.substitutions.insert(type_var, solution);
                    changed = true;
                }
            }
        }
        
        Ok(())
    }
    
    /// Find a solution for a set of constraints
    fn find_solution(&self, constraints: &[TypeConstraint]) -> Option<Type> {
        // Simplified constraint solving - real implementation would be much more sophisticated
        for constraint in constraints {
            match constraint {
                TypeConstraint::CompatibleWith(ty) => return Some(ty.clone()),
                TypeConstraint::SupportsStrategy(strategy) => {
                    // Return a basic type with the required strategy
                                         return Some(Type::Primitive {
                         kind: PrimitiveType::I32,
                         memory_strategy: *strategy,
                         span: Span::single(crate::lexer::Position::start(0)),
                     });
                }
                _ => continue,
            }
        }
        None
    }
}

/// Ownership tracking for memory safety analysis
#[derive(Debug, Clone)]
pub struct OwnershipTracker {
    /// Variables and their current ownership state
    variables: HashMap<InternedString, OwnershipState>,
    /// Borrowed references and their lifetimes
    borrows: HashMap<InternedString, Vec<BorrowInfo>>,
    /// Linear resources that must be consumed
    linear_resources: HashMap<InternedString, LinearState>,
}

#[derive(Debug, Clone)]
pub struct OwnershipState {
    ownership: Ownership,
    moved: bool,
    borrowed: bool,
    lifetime: Option<LifetimeId>,
}

#[derive(Debug, Clone)]
pub struct BorrowInfo {
    is_mutable: bool,
    lifetime: LifetimeId,
    span: Span,
}

#[derive(Debug, Clone)]
pub struct LinearState {
    consumed: bool,
    span: Span,
}

impl OwnershipTracker {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            borrows: HashMap::new(),
            linear_resources: HashMap::new(),
        }
    }
    
    /// Track a new variable
    pub fn track_variable(&mut self, name: InternedString, ownership: Ownership, ty: &Type) {
        self.variables.insert(name, OwnershipState {
            ownership,
            moved: false,
            borrowed: false,
            lifetime: ty.lifetime(),
        });
        
        // Track linear resources separately
        if ty.is_linear() {
            self.linear_resources.insert(name, LinearState {
                consumed: false,
                span: ty.span(),
            });
        }
    }
    
    /// Check if a move operation is valid
    pub fn check_move(&mut self, name: InternedString, span: Span) -> TypeResult<()> {
        if let Some(state) = self.variables.get_mut(&name) {
            if state.moved {
                return Err(TypeError::OwnershipViolation {
                    violation: OwnershipViolation::UseAfterMove(name),
                    span,
                    help: "This value was moved earlier and cannot be used again".to_string(),
                });
            }
            if state.borrowed {
                return Err(TypeError::OwnershipViolation {
                    violation: OwnershipViolation::BorrowAfterMove(name),
                    span,
                    help: "Cannot move value while it is borrowed".to_string(),
                });
            }
            state.moved = true;
            
            // Mark linear resource as consumed
            if let Some(linear_state) = self.linear_resources.get_mut(&name) {
                linear_state.consumed = true;
            }
        }
        Ok(())
    }
    
    /// Check if a borrow operation is valid
    pub fn check_borrow(&mut self, name: InternedString, is_mutable: bool, lifetime: LifetimeId, span: Span) -> TypeResult<()> {
        if let Some(state) = self.variables.get_mut(&name) {
            if state.moved {
                return Err(TypeError::OwnershipViolation {
                    violation: OwnershipViolation::UseAfterMove(name),
                    span,
                    help: "Cannot borrow moved value".to_string(),
                });
            }
            
            // Check for multiple mutable borrows
            if is_mutable {
                let empty_borrows = Vec::new();
                let existing_borrows = self.borrows.get(&name).unwrap_or(&empty_borrows);
                for borrow in existing_borrows {
                    if borrow.is_mutable {
                        return Err(TypeError::OwnershipViolation {
                            violation: OwnershipViolation::MultipleMutableBorrows(name),
                            span,
                            help: "Cannot have multiple mutable borrows of the same value".to_string(),
                        });
                    }
                }
            }
            
            state.borrowed = true;
            self.borrows.entry(name).or_insert_with(Vec::new).push(BorrowInfo {
                is_mutable,
                lifetime,
                span,
            });
        }
        Ok(())
    }
    
    /// Check linear resource consumption at scope end
    pub fn check_linear_consumption(&self) -> Vec<TypeError> {
        let mut errors = Vec::new();
        
        for (name, state) in &self.linear_resources {
            if !state.consumed {
                errors.push(TypeError::LinearTypeError {
                    name: *name,
                    error: LinearError::NotConsumed,
                    span: state.span,
                });
            }
        }
        
        errors
    }
}

/// Main type system implementation with memory management integration
pub struct TypeSystem {
    /// Symbol table for type resolution
    symbol_table: SymbolTable,
    /// Type inference context
    inference_context: InferenceContext,
    /// Ownership tracking for memory safety
    ownership_tracker: OwnershipTracker,
    /// Type errors collected during checking
    errors: Vec<TypeError>,
    /// Performance thresholds for contract enforcement
    performance_thresholds: HashMap<String, u64>,
}

impl TypeSystem {
    pub fn new(symbol_table: SymbolTable) -> Self {
        let mut performance_thresholds = HashMap::new();
        performance_thresholds.insert("allocation_cost".to_string(), 1000); // Max allocation cost
        performance_thresholds.insert("stack_depth".to_string(), 1024);     // Max stack depth
        
        Self {
            symbol_table,
            inference_context: InferenceContext::new(),
            ownership_tracker: OwnershipTracker::new(),
            errors: Vec::new(),
            performance_thresholds,
        }
    }
    
    /// Add a type error
    pub fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }
    
    /// Get all type errors
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }
    
    /// Check if two types are compatible considering memory strategies
    pub fn types_compatible(&self, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (Type::Primitive { kind: k1, memory_strategy: s1, .. }, 
             Type::Primitive { kind: k2, memory_strategy: s2, .. }) => {
                k1 == k2 && (s1 == s2 || *s1 == MemoryStrategy::Inferred || *s2 == MemoryStrategy::Inferred)
            }
            (Type::Path { segments: seg1, memory_strategy: s1, .. },
             Type::Path { segments: seg2, memory_strategy: s2, .. }) => {
                seg1 == seg2 && (s1 == s2 || *s1 == MemoryStrategy::Inferred || *s2 == MemoryStrategy::Inferred)
            }
            (Type::Reference { target_type: t1, ownership: o1, .. },
             Type::Reference { target_type: t2, ownership: o2, .. }) => {
                self.types_compatible(t1, t2) && o1 == o2
            }
            _ => false,
        }
    }
    
    /// Infer optimal memory strategy for a type
    pub fn infer_memory_strategy(&mut self, ty: &Type, usage_context: &UsageContext) -> MemoryStrategy {
        // Analyze usage patterns to determine optimal strategy
        match usage_context {
            UsageContext::ShortLived => MemoryStrategy::Stack,
            UsageContext::SingleOwnership => MemoryStrategy::Linear,
            UsageContext::SharedReadOnly => MemoryStrategy::SmartPtr,
            UsageContext::ManualManagement => MemoryStrategy::Manual,
            UsageContext::RegionBound => MemoryStrategy::Region,
        }
    }
    
    /// Check performance contracts
    pub fn check_performance_contract(&mut self, ty: &Type, operation: &str, span: Span) {
        let cost = ty.allocation_cost() as u64;
        if let Some(&threshold) = self.performance_thresholds.get(operation) {
            if cost > threshold {
                self.add_error(TypeError::PerformanceViolation {
                    message: format!("Operation '{}' exceeds performance threshold", operation),
                    cost_estimate: cost,
                    threshold,
                    span,
                });
            }
        }
    }
}

/// Usage context for memory strategy inference
#[derive(Debug, Clone, PartialEq)]
pub enum UsageContext {
    ShortLived,        // Function local, destroyed at scope end
    SingleOwnership,   // Owned by one entity, moved around
    SharedReadOnly,    // Shared between multiple readers
    ManualManagement,  // Explicit control over lifecycle
    RegionBound,       // Tied to a specific region/arena
}

/// Type checker that performs comprehensive analysis
pub struct TypeChecker {
    type_system: TypeSystem,
    expression_types: HashMap<*const Expr, Type>,
    scope_depth: usize,
}

impl TypeChecker {
    pub fn new(symbol_table: SymbolTable) -> Self {
        Self {
            type_system: TypeSystem::new(symbol_table),
            expression_types: HashMap::new(),
            scope_depth: 0,
        }
    }
    
    /// Type check a complete module
    pub fn check_module(&mut self, module: &Module) -> TypeResult<()> {
        for item in &module.items {
            self.check_item(item)?;
        }
        
        // Solve type constraints
        self.type_system.inference_context.solve()?;
        
        // Check linear resource consumption
        let linear_errors = self.type_system.ownership_tracker.check_linear_consumption();
        for error in linear_errors {
            self.type_system.add_error(error);
        }
        
        Ok(())
    }
    
    /// Type check an item
    pub fn check_item(&mut self, item: &Item) -> TypeResult<()> {
        match item {
            Item::Function { body: Some(body), .. } => {
                self.scope_depth += 1;
                let result = self.check_expr(body);
                self.scope_depth -= 1;
                result.map(|_| ())
            }
            _ => Ok(()), // TODO: Implement other items
        }
    }
    
    /// Type check an expression with comprehensive analysis
    pub fn check_expr(&mut self, expr: &Expr) -> TypeResult<Type> {
        let result_type = match expr {
            Expr::Literal { literal, span } => {
                self.check_literal(literal, *span)
            }
            
            Expr::Identifier { name, span } => {
                self.check_identifier(*name, *span)
            }
            
            Expr::Binary { left, right, op, span } => {
                self.check_binary_expr(left, right, *op, *span)
            }
            
            Expr::Unary { expr: inner_expr, op, span } => {
                self.check_unary_expr(inner_expr, *op, *span)
            }
            
            Expr::Call { callee, args, span } => {
                self.check_call_expr(callee, args, *span)
            }
            
            Expr::Reference { expr: inner_expr, is_mutable, span } => {
                self.check_reference_expr(inner_expr, *is_mutable, *span)
            }
            
            Expr::Dereference { expr: inner_expr, span } => {
                self.check_dereference_expr(inner_expr, *span)
            }
            
            _ => {
                // TODO: Implement remaining expressions
                Ok(Type::stack_primitive(PrimitiveType::Unit, expr.span()))
            }
        };
        
        // Cache the type for this expression
        if let Ok(ref ty) = result_type {
            self.expression_types.insert(expr as *const Expr, ty.clone());
        }
        
        result_type
    }
    
    /// Check literal expressions
    fn check_literal(&mut self, literal: &Literal, span: Span) -> TypeResult<Type> {
        let ty = match literal {
            Literal::Integer { .. } => Type::stack_primitive(PrimitiveType::I32, span),
            Literal::Float { .. } => Type::stack_primitive(PrimitiveType::F64, span),
            Literal::Bool(_) => Type::stack_primitive(PrimitiveType::Bool, span),
            Literal::Char(_) => Type::stack_primitive(PrimitiveType::Char, span),
            Literal::String { .. } => Type::stack_primitive(PrimitiveType::Str, span),
            Literal::Null => Type::Pointer {
                is_mutable: false,
                target_type: Box::new(Type::stack_primitive(PrimitiveType::Unit, span)),
                memory_strategy: MemoryStrategy::Manual,
                span,
            },
        };
        Ok(ty)
    }
    
    /// Check identifier expressions
    fn check_identifier(&mut self, name: InternedString, span: Span) -> TypeResult<Type> {
        if let Some(symbol) = self.type_system.symbol_table.lookup_symbol(&name) {
            match &symbol.kind {
                SymbolKind::Variable { type_info: Some(ty), .. } => {
                    // Check ownership rules
                    if ty.requires_move() {
                        self.type_system.ownership_tracker.check_move(name, span)?;
                    }
                    Ok(ty.clone())
                }
                SymbolKind::Variable { type_info: None, .. } => {
                    Err(TypeError::InferenceFailure {
                        reason: "Variable type not yet inferred".to_string(),
                        span,
                        constraints: Vec::new(),
                    })
                }
                _ => Err(TypeError::Mismatch {
                    expected: Type::stack_primitive(PrimitiveType::Unit, span),
                    actual: Type::stack_primitive(PrimitiveType::Unit, span),
                    span,
                    suggestion: Some("Expected a variable, found something else".to_string()),
                }),
            }
        } else {
            Err(TypeError::UndefinedType { 
                name, 
                span,
                suggestions: Vec::new(), // TODO: Add suggestion generation
            })
        }
    }
    
    /// Check binary expressions with memory strategy compatibility
    fn check_binary_expr(&mut self, left: &Expr, right: &Expr, op: BinaryOp, span: Span) -> TypeResult<Type> {
        let left_type = self.check_expr(left)?;
        let right_type = self.check_expr(right)?;
        
        // Check memory strategy compatibility
        if let (Some(left_strategy), Some(right_strategy)) = 
           (left_type.memory_strategy(), right_type.memory_strategy()) {
            if left_strategy != right_strategy && 
               left_strategy != MemoryStrategy::Inferred && 
               right_strategy != MemoryStrategy::Inferred {
                self.type_system.add_error(TypeError::StrategyConflict {
                    expected_strategy: left_strategy,
                    actual_strategy: right_strategy,
                    span,
                    performance_note: format!(
                        "Strategy conflict adds conversion overhead. Left: {} (cost: {}), Right: {} (cost: {})",
                        format!("{:?}", left_strategy).to_lowercase(),
                        left_strategy.allocation_cost(),
                        format!("{:?}", right_strategy).to_lowercase(),
                        right_strategy.allocation_cost()
                    ),
                });
            }
        }
        
        // Type check the operation
        match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                if self.type_system.types_compatible(&left_type, &right_type) {
                    Ok(left_type)
                } else {
                    Err(TypeError::Mismatch {
                        expected: left_type,
                        actual: right_type,
                        span,
                        suggestion: Some("Both operands must have compatible types".to_string()),
                    })
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual |
            BinaryOp::Greater | BinaryOp::GreaterEqual => {
                Ok(Type::stack_primitive(PrimitiveType::Bool, span))
            }
            _ => {
                // TODO: Implement other binary operations
                Ok(left_type)
            }
        }
    }
    
    /// Check unary expressions
    fn check_unary_expr(&mut self, expr: &Expr, op: UnaryOp, span: Span) -> TypeResult<Type> {
        let expr_type = self.check_expr(expr)?;
        
        match op {
            UnaryOp::Not => {
                if matches!(expr_type, Type::Primitive { kind: PrimitiveType::Bool, .. }) {
                    Ok(expr_type)
                } else {
                    Err(TypeError::Mismatch {
                        expected: Type::stack_primitive(PrimitiveType::Bool, span),
                        actual: expr_type,
                        span,
                        suggestion: Some("Logical NOT requires boolean operand".to_string()),
                    })
                }
            }
            UnaryOp::Negate => {
                // Check if type supports negation
                match &expr_type {
                    Type::Primitive { kind, .. } if matches!(kind, 
                        PrimitiveType::I8 | PrimitiveType::I16 | PrimitiveType::I32 | PrimitiveType::I64 |
                        PrimitiveType::F32 | PrimitiveType::F64) => Ok(expr_type),
                    _ => Err(TypeError::Mismatch {
                        expected: Type::stack_primitive(PrimitiveType::I32, span),
                        actual: expr_type,
                        span,
                        suggestion: Some("Negation requires numeric type".to_string()),
                    }),
                }
            }
            _ => {
                // TODO: Implement other unary operations
                Ok(expr_type)
            }
        }
    }
    
    /// Check call expressions with performance analysis
    fn check_call_expr(&mut self, callee: &Expr, args: &[Expr], span: Span) -> TypeResult<Type> {
        let _callee_type = self.check_expr(callee)?;
        
        // Type check arguments and analyze performance
        let mut total_cost = 0u64;
        for arg in args {
            let arg_type = self.check_expr(arg)?;
            total_cost += arg_type.allocation_cost() as u64;
        }
        
        // Check performance contract for function calls
        self.type_system.check_performance_contract(
            &Type::stack_primitive(PrimitiveType::Unit, span),
            "function_call",
            span
        );
        
        // TODO: Implement proper function type checking
        Ok(Type::stack_primitive(PrimitiveType::Unit, span))
    }
    
    /// Check reference expressions with lifetime analysis
    fn check_reference_expr(&mut self, expr: &Expr, is_mutable: bool, span: Span) -> TypeResult<Type> {
        let expr_type = self.check_expr(expr)?;
        
        // Create a reference type with appropriate lifetime
        let lifetime = LifetimeId::new(self.scope_depth as u32);
        
        // Check ownership rules for references
        if let Expr::Identifier { name, .. } = expr {
            self.type_system.ownership_tracker.check_borrow(*name, is_mutable, lifetime, span)?;
        }
        
        Ok(Type::borrowed_ref(expr_type, is_mutable, Some(lifetime), span))
    }
    
    /// Check dereference expressions
    fn check_dereference_expr(&mut self, expr: &Expr, span: Span) -> TypeResult<Type> {
        let expr_type = self.check_expr(expr)?;
        
        match expr_type {
            Type::Reference { target_type, .. } => Ok(*target_type),
            Type::Pointer { target_type, .. } => Ok(*target_type),
            _ => Err(TypeError::Mismatch {
                expected: Type::Pointer {
                    is_mutable: false,
                    target_type: Box::new(Type::stack_primitive(PrimitiveType::Unit, span)),
                    memory_strategy: MemoryStrategy::Manual,
                    span,
                },
                actual: expr_type,
                span,
                suggestion: Some("Dereference requires pointer or reference type".to_string()),
            }),
        }
    }
    
    /// Get type information for expressions (for IDE/LSP integration)
    pub fn get_expression_type(&self, expr: &Expr) -> Option<&Type> {
        self.expression_types.get(&(expr as *const Expr))
    }
    
    /// Get all errors from type checking
    pub fn get_all_errors(&self) -> &[TypeError] {
        self.type_system.errors()
    }
} 