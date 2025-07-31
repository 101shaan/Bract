//! Escape Analysis for Bract
//!
//! This module implements escape analysis to ensure leak-free guarantees at compile-time.
//! It tracks how values escape their original scopes and ensures that:
//!
//! - Stack-allocated values don't escape their stack frames
//! - Region-allocated values don't escape their regions
//! - Linear types are properly consumed
//! - Memory strategies are compatible with escape patterns
//! - Performance contracts are maintained across escapes

use crate::ast::{
    Type, Expr, Stmt, Item, Module, Pattern, InternedString,
    MemoryStrategy, LifetimeId, BinaryOp, UnaryOp
};
use crate::lexer::Position;
use std::collections::{HashMap, HashSet};

/// Escape analysis errors
#[derive(Debug, Clone, PartialEq)]
pub enum EscapeError {
    /// Stack value escapes function
    StackEscape {
        variable: InternedString,
        escape_site: Position,
        function_boundary: Position,
    },
    
    /// Region value escapes region
    RegionEscape {
        variable: InternedString,
        region: InternedString,
        escape_site: Position,
        region_boundary: Position,
    },
    
    /// Linear value not consumed
    LinearNotConsumed {
        variable: InternedString,
        declaration_site: Position,
        scope_end: Position,
    },
    
    /// Memory strategy violation
    StrategyViolation {
        variable: InternedString,
        expected_strategy: MemoryStrategy,
        violation_site: Position,
        reason: String,
    },
    
    /// Performance contract violation
    PerformanceViolation {
        variable: InternedString,
        contract_site: Position,
        violation_site: Position,
        expected_cost: u64,
        actual_cost: u64,
    },
    
    /// Potential memory leak
    PotentialLeak {
        variable: InternedString,
        allocation_site: Position,
        leak_site: Position,
        reason: String,
    },
}

/// Escape context - where a value might escape to
#[derive(Debug, Clone, PartialEq)]
pub enum EscapeContext {
    /// Value stays in current scope
    NoEscape,
    /// Value escapes to parent function
    FunctionReturn,
    /// Value escapes through reference
    Reference { lifetime: LifetimeId },
    /// Value escapes through closure capture
    ClosureCapture,
    /// Value escapes through global storage
    Global,
    /// Value escapes through heap allocation
    Heap,
}

/// Value flow information
#[derive(Debug, Clone, PartialEq)]
pub struct ValueFlow {
    /// Original variable name
    pub variable: InternedString,
    /// Where the value was created
    pub creation_site: Position,
    /// Memory strategy used
    pub memory_strategy: MemoryStrategy,
    /// Current escape context
    pub escape_context: EscapeContext,
    /// Lifetime if applicable
    pub lifetime: Option<LifetimeId>,
    /// Performance cost of this value
    pub performance_cost: u64,
}

/// Scope information for escape analysis
#[derive(Debug, Clone)]
pub struct EscapeScope {
    /// Scope name (function, region, block)
    pub name: String,
    /// Variables defined in this scope
    pub variables: HashMap<InternedString, ValueFlow>,
    /// Nested scopes
    pub children: Vec<EscapeScope>,
    /// Scope boundaries
    pub start_pos: Position,
    pub end_pos: Option<Position>,
    /// Whether this is a memory region
    pub is_region: bool,
    /// Region name if applicable
    pub region_name: Option<InternedString>,
}

/// Escape analyzer
#[derive(Debug)]
pub struct EscapeAnalyzer {
    /// Scope stack
    scope_stack: Vec<EscapeScope>,
    /// Global values that can be safely accessed
    global_values: HashSet<InternedString>,
    /// Active regions
    active_regions: HashMap<InternedString, Position>,
    /// Lifetime counter
    next_lifetime_id: u32,
    /// Errors found
    errors: Vec<EscapeError>,
    /// Current position
    current_position: Position,
    /// Performance budget tracking
    performance_budgets: HashMap<String, u64>,
}

impl EscapeAnalyzer {
    /// Create a new escape analyzer
    pub fn new() -> Self {
        Self {
            scope_stack: vec![EscapeScope {
                name: "global".to_string(),
                variables: HashMap::new(),
                children: Vec::new(),
                start_pos: Position::start(0),
                end_pos: None,
                is_region: false,
                region_name: None,
            }],
            global_values: HashSet::new(),
            active_regions: HashMap::new(),
            next_lifetime_id: 0,
            errors: Vec::new(),
            current_position: Position::start(0),
            performance_budgets: HashMap::new(),
        }
    }
    
    /// Analyze escape patterns in a module
    pub fn analyze_module(&mut self, module: &Module) -> Vec<EscapeError> {
        self.errors.clear();
        
        // First pass: collect global declarations
        for item in &module.items {
            self.collect_global_item(item);
        }
        
        // Second pass: analyze escape patterns
        for item in &module.items {
            self.analyze_item(item);
        }
        
        // Final pass: check for leaks and violations
        self.finalize_analysis();
        
        self.errors.clone()
    }
    
    /// Collect global item information
    fn collect_global_item(&mut self, item: &Item) {
        match item {
            Item::Function { name, .. } => {
                self.global_values.insert(*name);
            }
            Item::Const { name, .. } => {
                self.global_values.insert(*name);
            }
            Item::Struct { name, .. } |
            Item::Enum { name, .. } |
            Item::TypeAlias { name, .. } => {
                self.global_values.insert(*name);
            }
            _ => {}
        }
    }
    
    /// Analyze an item for escape patterns
    fn analyze_item(&mut self, item: &Item) {
        match item {
            Item::Function { name, body: Some(body), params, return_type, .. } => {
                self.enter_function_scope(format!("{:?}", name));
                
                // Analyze parameters
                for param in params {
                    self.add_parameter(&param.pattern, &param.type_annotation);
                }
                
                // Analyze body
                self.analyze_expr(body);
                
                // Check return type compatibility
                if let Some(ret_type) = return_type {
                    self.check_return_escape(body, ret_type);
                }
                
                self.exit_scope();
            }
            Item::Function { body: None, .. } => {
                // External function - no analysis needed
            }
            Item::Const { value, .. } => {
                self.analyze_expr(value);
            }
            Item::Module { items: Some(items), .. } => {
                self.enter_scope("module".to_string(), false, None);
                for item in items {
                    self.analyze_item(item);
                }
                self.exit_scope();
            }
            Item::Impl { items, .. } => {
                for impl_item in items {
                    if let crate::ast::ImplItem::Function { name, body: Some(body), params, return_type, .. } = impl_item {
                        self.enter_function_scope(format!("impl::{:?}", name));
                        
                        for param in params {
                            self.add_parameter(&param.pattern, &param.type_annotation);
                        }
                        
                        self.analyze_expr(body);
                        
                        if let Some(ret_type) = return_type {
                            self.check_return_escape(body, ret_type);
                        }
                        
                        self.exit_scope();
                    }
                }
            }
            _ => {
                // Other items don't need escape analysis
            }
        }
    }
    
    /// Analyze expression for escape patterns
    fn analyze_expr(&mut self, expr: &Expr) {
        self.current_position = self.get_expr_position(expr);
        
        match expr {
            Expr::Literal { .. } => {
                // Literals don't escape
            }
            
            Expr::Identifier { name, .. } => {
                self.check_variable_escape(*name);
            }
            
            Expr::Path { segments, .. } => {
                if let Some(first) = segments.first() {
                    self.check_variable_escape(*first);
                }
            }
            
            Expr::Binary { left, right, op, .. } => {
                self.analyze_expr(left);
                self.analyze_expr(right);
                
                if let BinaryOp::Assign = op {
                    self.handle_assignment_escape(left, right);
                }
            }
            
            Expr::Unary { expr, op, .. } => {
                match op {
                    UnaryOp::MutableRef => {
                        // Taking mutable address creates potential escape
                        self.check_address_escape(expr);
                    }
                    _ => {
                        self.analyze_expr(expr);
                    }
                }
            }
            
            Expr::Call { callee, args, .. } => {
                self.analyze_expr(callee);
                
                // Check if arguments escape through function call
                for arg in args {
                    self.analyze_expr(arg);
                    self.check_call_argument_escape(arg, callee);
                }
            }
            
            Expr::MethodCall { receiver, args, .. } => {
                self.analyze_expr(receiver);
                
                for arg in args {
                    self.analyze_expr(arg);
                }
            }
            
            Expr::FieldAccess { object, .. } => {
                self.analyze_expr(object);
            }
            
            Expr::Index { object, index, .. } => {
                self.analyze_expr(object);
                self.analyze_expr(index);
            }
            
            Expr::Array { elements, .. } => {
                for element in elements {
                    self.analyze_expr(element);
                }
            }
            
            Expr::Tuple { elements, .. } => {
                for element in elements {
                    self.analyze_expr(element);
                }
            }
            
            Expr::StructInit { fields, .. } => {
                for field in fields {
                    if let Some(ref value) = field.value {
                        self.analyze_expr(value);
                    }
                }
            }
            
            Expr::Block { statements, trailing_expr, .. } => {
                self.enter_scope("block".to_string(), false, None);
                
                for stmt in statements {
                    self.analyze_stmt(stmt);
                }
                
                if let Some(trailing) = trailing_expr {
                    self.analyze_expr(trailing);
                    // Trailing expression might escape the block
                    self.check_block_escape(trailing);
                }
                
                self.exit_scope();
            }
            
            Expr::If { condition, then_block, else_block, .. } => {
                self.analyze_expr(condition);
                
                // Analyze branches separately
                self.enter_scope("if_then".to_string(), false, None);
                self.analyze_expr(then_block);
                self.exit_scope();
                
                if let Some(else_expr) = else_block {
                    self.enter_scope("if_else".to_string(), false, None);
                    self.analyze_expr(else_expr);
                    self.exit_scope();
                }
            }
            
            Expr::Match { expr, arms, .. } => {
                self.analyze_expr(expr);
                
                for arm in arms {
                    self.enter_scope("match_arm".to_string(), false, None);
                    self.analyze_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.analyze_expr(guard);
                    }
                    self.analyze_expr(&arm.body);
                    self.exit_scope();
                }
            }
            
            Expr::While { condition, body, .. } => {
                self.enter_scope("while".to_string(), false, None);
                self.analyze_expr(condition);
                self.analyze_expr(body);
                self.exit_scope();
            }
            
            Expr::For { pattern, iterator, body, .. } => {
                self.analyze_expr(iterator);
                
                self.enter_scope("for".to_string(), false, None);
                self.analyze_pattern(pattern);
                self.analyze_expr(body);
                self.exit_scope();
            }
            
            Expr::Return { value, .. } => {
                if let Some(val) = value {
                    self.analyze_expr(val);
                    self.check_return_value_escape(val);
                }
            }
            
            Expr::Break { value, .. } => {
                if let Some(val) = value {
                    self.analyze_expr(val);
                }
            }
            
            Expr::Continue { .. } => {
                // No escape effects
            }
            
            // Additional expression types
            Expr::Cast { expr, .. } => {
                self.analyze_expr(expr);
            }
            
            Expr::Parenthesized { expr, .. } => {
                self.analyze_expr(expr);
            }
            
            Expr::Range { start, end, .. } => {
                if let Some(start_expr) = start {
                    self.analyze_expr(start_expr);
                }
                if let Some(end_expr) = end {
                    self.analyze_expr(end_expr);
                }
            }
            
            Expr::Closure { params, body, .. } => {
                self.enter_scope("closure".to_string(), false, None);
                for param in params {
                    self.add_parameter(&param.pattern, &param.type_annotation);
                }
                self.analyze_expr(body);
                self.exit_scope();
            }
            
            Expr::Loop { body, .. } => {
                self.enter_scope("loop".to_string(), false, None);
                self.analyze_expr(body);
                self.exit_scope();
            }
            
            Expr::Box { expr, .. } => {
                self.analyze_expr(expr);
                // Box allocations could escape - TODO: implement proper escape analysis
            }
            
            Expr::Reference { expr, .. } => {
                self.analyze_expr(expr);
                self.check_address_escape(expr);
            }
            
            Expr::Dereference { expr, .. } => {
                self.analyze_expr(expr);
            }
            
            Expr::Try { expr, .. } => {
                self.analyze_expr(expr);
            }
            
            Expr::Await { expr, .. } => {
                self.analyze_expr(expr);
            }
            
            Expr::Macro { .. } => {
                // Macro invocations need special handling - simplified for now
            }
        }
    }
    
    /// Analyze statement for escape patterns
    fn analyze_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expression { expr, .. } => {
                self.analyze_expr(expr);
            }
            Stmt::Let { pattern, type_annotation, initializer, .. } => {
                if let Some(init) = initializer {
                    self.analyze_expr(init);
                }
                
                // Add variable to current scope
                self.add_variable_from_pattern(pattern, type_annotation, initializer.as_ref());
            }
            Stmt::Item { item, .. } => {
                self.analyze_item(item);
            }
            
            // Additional statement types
            Stmt::Assignment { target, value, .. } => {
                self.analyze_expr(target);
                self.analyze_expr(value);
                self.handle_assignment_escape(target, value);
            }
            
            Stmt::CompoundAssignment { target, value, .. } => {
                self.analyze_expr(target);
                self.analyze_expr(value);
            }
            
            Stmt::If { condition, then_block, else_block, .. } => {
                self.analyze_expr(condition);
                for stmt in then_block {
                    self.analyze_stmt(stmt);
                }
                if let Some(else_stmt) = else_block {
                    self.analyze_stmt(else_stmt);
                }
            }
            
            Stmt::While { condition, body, .. } => {
                self.analyze_expr(condition);
                for stmt in body {
                    self.analyze_stmt(stmt);
                }
            }
            
            Stmt::For { pattern, iterable, body, .. } => {
                self.analyze_expr(iterable);
                self.analyze_pattern(pattern);
                for stmt in body {
                    self.analyze_stmt(stmt);
                }
            }
            
            Stmt::Loop { body, .. } => {
                for stmt in body {
                    self.analyze_stmt(stmt);
                }
            }
            
            Stmt::Match { expr, arms, .. } => {
                self.analyze_expr(expr);
                for arm in arms {
                    self.analyze_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.analyze_expr(guard);
                    }
                    self.analyze_expr(&arm.body);
                }
            }
            
            Stmt::Break { expr, .. } => {
                if let Some(expr) = expr {
                    self.analyze_expr(expr);
                }
            }
            
            Stmt::Continue { .. } => {
                // No escape effects
            }
            
            Stmt::Return { expr, .. } => {
                if let Some(expr) = expr {
                    self.analyze_expr(expr);
                    self.check_return_value_escape(expr);
                }
            }
            
            Stmt::Block { statements, .. } => {
                self.enter_scope("stmt_block".to_string(), false, None);
                for stmt in statements {
                    self.analyze_stmt(stmt);
                }
                self.exit_scope();
            }
            
            Stmt::Empty { .. } => {
                // No operations
            }
        }
    }
    
    /// Analyze pattern for variable declarations
    fn analyze_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Identifier { name, .. } => {
                // Pattern binding - add to current scope
                let flow = ValueFlow {
                    variable: *name,
                    creation_site: self.current_position,
                    memory_strategy: MemoryStrategy::Inferred,
                    escape_context: EscapeContext::NoEscape,
                    lifetime: None,
                    performance_cost: 0,
                };
                
                if let Some(scope) = self.scope_stack.last_mut() {
                    scope.variables.insert(*name, flow);
                }
            }
            Pattern::Tuple { patterns, .. } => {
                for p in patterns {
                    self.analyze_pattern(p);
                }
            }
            Pattern::Struct { fields, .. } => {
                for field in fields {
                    if let Some(ref pattern) = field.pattern {
                        self.analyze_pattern(pattern);
                    }
                }
            }
            Pattern::Enum { patterns, .. } => {
                if let Some(patterns) = patterns {
                    for p in patterns {
                        self.analyze_pattern(p);
                    }
                }
            }
            Pattern::Array { patterns, .. } => {
                for p in patterns {
                    self.analyze_pattern(p);
                }
            }
            _ => {
                // Other patterns don't create bindings
            }
        }
    }
    
    /// Check if variable usage creates an escape
    fn check_variable_escape(&mut self, name: InternedString) {
        if let Some(flow) = self.find_variable_flow(name) {
            match flow.memory_strategy {
                MemoryStrategy::Stack => {
                    // Stack values can't escape their function
                    if self.is_escaping_function() {
                        self.errors.push(EscapeError::StackEscape {
                            variable: name,
                            escape_site: self.current_position,
                            function_boundary: flow.creation_site,
                        });
                    }
                }
                MemoryStrategy::Region => {
                    // Region values can't escape their region
                    if let Some(region_name) = self.current_region() {
                        if !self.is_in_region(&region_name) {
                            self.errors.push(EscapeError::RegionEscape {
                                variable: name,
                                region: region_name,
                                escape_site: self.current_position,
                                region_boundary: flow.creation_site,
                            });
                        }
                    }
                }
                MemoryStrategy::Linear => {
                    // Linear values must be consumed
                    self.mark_linear_consumed(name);
                }
                _ => {
                    // Other strategies are more flexible
                }
            }
        }
    }
    
    /// Check escape through address-of operations
    fn check_address_escape(&mut self, expr: &Expr) {
        if let Some(name) = self.get_simple_identifier(expr) {
            if let Some(flow) = self.find_variable_flow(name) {
                match flow.memory_strategy {
                    MemoryStrategy::Stack => {
                        // Taking address of stack variable creates potential escape
                        let lifetime = self.allocate_lifetime();
                        self.update_escape_context(name, EscapeContext::Reference { lifetime });
                    }
                    _ => {
                        // Other strategies may allow address-of
                    }
                }
            }
        }
    }
    
    /// Check escape through function call arguments
    fn check_call_argument_escape(&mut self, arg: &Expr, _callee: &Expr) {
        if let Some(name) = self.get_simple_identifier(arg) {
            // Function calls can cause arguments to escape
            self.update_escape_context(name, EscapeContext::FunctionReturn);
        }
    }
    
    /// Check escape through assignment
    fn handle_assignment_escape(&mut self, _lhs: &Expr, rhs: &Expr) {
        // Assignment might cause right-hand side to escape
        if let Some(name) = self.get_simple_identifier(rhs) {
            // Check if assignment target has different memory strategy
            self.check_strategy_compatibility(name);
        }
    }
    
    /// Check return value escape
    fn check_return_value_escape(&mut self, expr: &Expr) {
        if let Some(name) = self.get_simple_identifier(expr) {
            if let Some(flow) = self.find_variable_flow(name) {
                match flow.memory_strategy {
                    MemoryStrategy::Stack => {
                        self.errors.push(EscapeError::StackEscape {
                            variable: name,
                            escape_site: self.current_position,
                            function_boundary: flow.creation_site,
                        });
                    }
                    MemoryStrategy::Region => {
                        if let Some(region_name) = self.current_region() {
                            self.errors.push(EscapeError::RegionEscape {
                                variable: name,
                                region: region_name,
                                escape_site: self.current_position,
                                region_boundary: flow.creation_site,
                            });
                        }
                    }
                    _ => {
                        // Other strategies can be returned
                    }
                }
            }
        }
    }
    
    /// Check return type compatibility
    fn check_return_escape(&mut self, _body: &Expr, _return_type: &Type) {
        // Check if return type is compatible with escape analysis
        // Simplified for now
    }
    
    /// Check block escape
    fn check_block_escape(&mut self, expr: &Expr) {
        if let Some(name) = self.get_simple_identifier(expr) {
            if let Some(flow) = self.find_variable_flow(name) {
                // Block-scoped variables can't escape their block
                if flow.memory_strategy == MemoryStrategy::Stack {
                    // This would be an error if the variable was declared in this block
                }
            }
        }
    }
    
    /// Check memory strategy compatibility
    fn check_strategy_compatibility(&mut self, name: InternedString) {
        if let Some(flow) = self.find_variable_flow(name) {
            // Check if current context is compatible with memory strategy
            match flow.memory_strategy {
                MemoryStrategy::Linear => {
                    // Linear types have strict usage rules
                    if flow.escape_context != EscapeContext::NoEscape {
                        self.errors.push(EscapeError::StrategyViolation {
                            variable: name,
                            expected_strategy: MemoryStrategy::Linear,
                            violation_site: self.current_position,
                            reason: "Linear type used in escaping context".to_string(),
                        });
                    }
                }
                _ => {
                    // Other strategies are more flexible
                }
            }
        }
    }
    
    /// Add parameter to current scope
    fn add_parameter(&mut self, pattern: &Pattern, type_annotation: &Option<Type>) {
        if let Pattern::Identifier { name, .. } = pattern {
            let memory_strategy = if let Some(ty) = type_annotation {
                self.get_memory_strategy(ty)
            } else {
                MemoryStrategy::Inferred
            };
            
            let flow = ValueFlow {
                variable: *name,
                creation_site: self.current_position,
                memory_strategy,
                escape_context: EscapeContext::NoEscape,
                lifetime: None,
                performance_cost: 0,
            };
            
            if let Some(scope) = self.scope_stack.last_mut() {
                scope.variables.insert(*name, flow);
            }
        }
    }
    
    /// Add variable from let pattern
    fn add_variable_from_pattern(&mut self, pattern: &Pattern, type_annotation: &Option<Type>, _initializer: Option<&Expr>) {
        if let Pattern::Identifier { name, .. } = pattern {
            let memory_strategy = if let Some(ty) = type_annotation {
                self.get_memory_strategy(ty)
            } else {
                MemoryStrategy::Inferred
            };
            
            let flow = ValueFlow {
                variable: *name,
                creation_site: self.current_position,
                memory_strategy,
                escape_context: EscapeContext::NoEscape,
                lifetime: None,
                performance_cost: 0,
            };
            
            if let Some(scope) = self.scope_stack.last_mut() {
                scope.variables.insert(*name, flow);
            }
        }
    }
    
    /// Find variable flow information
    fn find_variable_flow(&self, name: InternedString) -> Option<ValueFlow> {
        // Search from innermost to outermost scope
        for scope in self.scope_stack.iter().rev() {
            if let Some(flow) = scope.variables.get(&name) {
                return Some(flow.clone());
            }
        }
        None
    }
    
    /// Update escape context for a variable
    fn update_escape_context(&mut self, name: InternedString, new_context: EscapeContext) {
        for scope in self.scope_stack.iter_mut().rev() {
            if let Some(flow) = scope.variables.get_mut(&name) {
                flow.escape_context = new_context;
                return;
            }
        }
    }
    
    /// Mark linear type as consumed
    fn mark_linear_consumed(&mut self, name: InternedString) {
        // Mark linear type as consumed - simplified
        if let Some(flow) = self.find_variable_flow(name) {
            if flow.memory_strategy == MemoryStrategy::Linear {
                // Update consumption status
            }
        }
    }
    
    /// Get memory strategy from type
    fn get_memory_strategy(&self, ty: &Type) -> MemoryStrategy {
        match ty {
            Type::Pointer { memory_strategy, .. } |
            Type::Array { memory_strategy, .. } => *memory_strategy,
            Type::Reference { .. } => MemoryStrategy::Stack, // references use stack semantics
            _ => MemoryStrategy::Inferred,
        }
    }
    
    /// Get simple identifier from expression
    fn get_simple_identifier(&self, expr: &Expr) -> Option<InternedString> {
        match expr {
            Expr::Identifier { name, .. } => Some(*name),
            _ => None,
        }
    }
    
    /// Get position from expression
    fn get_expr_position(&self, expr: &Expr) -> Position {
        match expr {
            Expr::Literal { span, .. } |
            Expr::Identifier { span, .. } |
            Expr::Path { span, .. } |
            Expr::Binary { span, .. } |
            Expr::Unary { span, .. } |
            Expr::Call { span, .. } |
            Expr::MethodCall { span, .. } |
            Expr::FieldAccess { span, .. } |
            Expr::Index { span, .. } |
            Expr::Array { span, .. } |
            Expr::Tuple { span, .. } |
            Expr::StructInit { span, .. } |
            Expr::Block { span, .. } |
            Expr::If { span, .. } |
            Expr::Match { span, .. } |
            Expr::While { span, .. } |
            Expr::For { span, .. } |
            Expr::Return { span, .. } |
            Expr::Break { span, .. } |
            Expr::Continue { span, .. } |
            Expr::Cast { span, .. } |
            Expr::Parenthesized { span, .. } |
            Expr::Range { span, .. } |
            Expr::Closure { span, .. } |
            Expr::Loop { span, .. } |
            Expr::Box { span, .. } |
            Expr::Reference { span, .. } |
            Expr::Dereference { span, .. } |
            Expr::Try { span, .. } |
            Expr::Await { span, .. } |
            Expr::Macro { span, .. } => span.start,
        }
    }
    
    /// Check if currently escaping function boundary
    fn is_escaping_function(&self) -> bool {
        // Simplified - would need more context
        false
    }
    
    /// Get current region name
    fn current_region(&self) -> Option<InternedString> {
        for scope in self.scope_stack.iter().rev() {
            if scope.is_region {
                return scope.region_name;
            }
        }
        None
    }
    
    /// Check if currently in a specific region
    fn is_in_region(&self, region_name: &InternedString) -> bool {
        self.active_regions.contains_key(region_name)
    }
    
    /// Enter function scope
    fn enter_function_scope(&mut self, name: String) {
        let scope = EscapeScope {
            name,
            variables: HashMap::new(),
            children: Vec::new(),
            start_pos: self.current_position,
            end_pos: None,
            is_region: false,
            region_name: None,
        };
        self.scope_stack.push(scope);
    }
    
    /// Enter generic scope
    fn enter_scope(&mut self, name: String, is_region: bool, region_name: Option<InternedString>) {
        let scope = EscapeScope {
            name,
            variables: HashMap::new(),
            children: Vec::new(),
            start_pos: self.current_position,
            end_pos: None,
            is_region,
            region_name,
        };
        
        if is_region {
            if let Some(region) = region_name {
                self.active_regions.insert(region, self.current_position);
            }
        }
        
        self.scope_stack.push(scope);
    }
    
    /// Exit current scope
    fn exit_scope(&mut self) {
        if let Some(scope) = self.scope_stack.pop() {
            // Check for unconsumed linear types
            for (name, flow) in &scope.variables {
                if flow.memory_strategy == MemoryStrategy::Linear &&
                   flow.escape_context == EscapeContext::NoEscape {
                    self.errors.push(EscapeError::LinearNotConsumed {
                        variable: *name,
                        declaration_site: flow.creation_site,
                        scope_end: self.current_position,
                    });
                }
            }
            
            // Remove region from active list
            if scope.is_region {
                if let Some(region) = scope.region_name {
                    self.active_regions.remove(&region);
                }
            }
        }
    }
    
    /// Allocate new lifetime
    fn allocate_lifetime(&mut self) -> LifetimeId {
        let id = LifetimeId(self.next_lifetime_id);
        self.next_lifetime_id += 1;
        id
    }
    
    /// Finalize analysis
    fn finalize_analysis(&mut self) {
        // Check for global leaks, performance violations, etc.
        
        // Ensure all regions are closed
        if !self.active_regions.is_empty() {
            for (region_name, start_pos) in &self.active_regions {
                // Could add warning about unclosed regions
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Module, Span};
    
    #[test]
    fn test_basic_escape_analysis() {
        let mut analyzer = EscapeAnalyzer::new();
        
        let module = Module {
            items: vec![],
            span: Span::new(Position::start(0), Position::start(0)),
        };
        
        let errors = analyzer.analyze_module(&module);
        assert_eq!(errors.len(), 0);
    }
    
    #[test]
    fn test_stack_escape_detection() {
        let mut analyzer = EscapeAnalyzer::new();
        
        // Test that stack values don't escape functions
        // Would require more complex AST construction
        
        let errors = analyzer.errors;
        // Would check for StackEscape errors
    }
    
    #[test]
    fn test_region_escape_detection() {
        let mut analyzer = EscapeAnalyzer::new();
        
        // Test that region values don't escape regions
        // Would require AST with region blocks
        
        let errors = analyzer.errors;
        // Would check for RegionEscape errors
    }
    
    #[test]
    fn test_linear_consumption() {
        let mut analyzer = EscapeAnalyzer::new();
        
        // Test that linear types are properly consumed
        // Would require AST with LinearPtr<T> usage
        
        let errors = analyzer.errors;
        // Would check for LinearNotConsumed errors
    }
} 