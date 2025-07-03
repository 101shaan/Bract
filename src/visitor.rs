//! AST Visitor Infrastructure for Semantic Analysis
//!
//! This module provides comprehensive AST traversal patterns essential for semantic analysis.
//! Following the roadmap Step 8.5 requirements for world-class visitor infrastructure.

use crate::ast::*;
use std::collections::HashMap;

/// Result type for visitor operations that can fail
pub type VisitorResult<T> = Result<T, VisitorError>;

/// Errors that can occur during AST traversal
#[derive(Debug, Clone, PartialEq)]
pub enum VisitorError {
    /// Early termination requested
    EarlyTermination,
    /// Custom error with message
    Custom(String),
    /// Type error during traversal
    TypeError(String),
    /// Symbol resolution error
    SymbolError(String),
}

/// Context for AST traversal - tracks scope and analysis state
#[derive(Debug, Clone)]
pub struct VisitorContext {
    /// Current scope depth
    pub scope_depth: usize,
    /// Symbol table for current traversal
    pub symbols: HashMap<InternedString, SymbolInfo>,
    /// Custom data for specific analyses
    pub data: HashMap<String, String>,
}

impl VisitorContext {
    pub fn new() -> Self {
        Self {
            scope_depth: 0,
            symbols: HashMap::new(),
            data: HashMap::new(),
        }
    }
    
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }
    
    pub fn exit_scope(&mut self) {
        self.scope_depth = self.scope_depth.saturating_sub(1);
    }
}

/// Symbol information for visitor context
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    pub name: InternedString,
    pub kind: SymbolKind,
    pub scope_depth: usize,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable { is_mutable: bool },
    Function,
    Type,
    Module,
}

/// Enhanced visitor trait with context and error handling
pub trait ContextVisitor<T> {
    fn visit_module(&mut self, module: &Module, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_item(&mut self, item: &Item, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_stmt(&mut self, stmt: &Stmt, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_expr(&mut self, expr: &Expr, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_pattern(&mut self, pattern: &Pattern, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_type(&mut self, ty: &Type, ctx: &mut VisitorContext) -> VisitorResult<T>;
}

/// Mutable visitor with context for AST transformations
pub trait ContextVisitorMut<T> {
    fn visit_module_mut(&mut self, module: &mut Module, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_item_mut(&mut self, item: &mut Item, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_stmt_mut(&mut self, stmt: &mut Stmt, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_expr_mut(&mut self, expr: &mut Expr, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_pattern_mut(&mut self, pattern: &mut Pattern, ctx: &mut VisitorContext) -> VisitorResult<T>;
    fn visit_type_mut(&mut self, ty: &mut Type, ctx: &mut VisitorContext) -> VisitorResult<T>;
}

/// Traversal order for AST walking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraversalOrder {
    PreOrder,   // Visit node before children
    PostOrder,  // Visit node after children
}

/// AST Walker with configurable traversal patterns
pub struct AstWalker<V, T> {
    visitor: V,
    order: TraversalOrder,
    early_termination: bool,
    _phantom: std::marker::PhantomData<T>,
}

impl<V, T> AstWalker<V, T> {
    pub fn new(visitor: V, order: TraversalOrder) -> Self {
        Self {
            visitor,
            order,
            early_termination: false,
            _phantom: std::marker::PhantomData,
        }
    }
    
    pub fn with_early_termination(mut self, enabled: bool) -> Self {
        self.early_termination = enabled;
        self
    }
}

impl<V, T> AstWalker<V, T>
where
    V: ContextVisitor<T>,
{
    /// Walk a complete module
    pub fn walk_module(&mut self, module: &Module, ctx: &mut VisitorContext) -> VisitorResult<Vec<T>> {
        let mut results = Vec::new();
        
        if self.order == TraversalOrder::PreOrder {
            match self.visitor.visit_module(module, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        // Walk all items
        for item in &module.items {
            match self.walk_item(item, ctx) {
                Ok(mut item_results) => results.append(&mut item_results),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        if self.order == TraversalOrder::PostOrder {
            match self.visitor.visit_module(module, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        Ok(results)
    }
    
    /// Walk an item
    pub fn walk_item(&mut self, item: &Item, ctx: &mut VisitorContext) -> VisitorResult<Vec<T>> {
        let mut results = Vec::new();
        
        if self.order == TraversalOrder::PreOrder {
            match self.visitor.visit_item(item, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        // Walk item contents based on type
        match item {
            Item::Function { body: Some(body), params, .. } => {
                ctx.enter_scope();
                
                // Walk parameters
                for param in params {
                    match self.walk_pattern(&param.pattern, ctx) {
                        Ok(mut param_results) => results.append(&mut param_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => {
                            ctx.exit_scope();
                            return Ok(results);
                        },
                        Err(e) => {
                            ctx.exit_scope();
                            return Err(e);
                        }
                    }
                }
                
                // Walk function body
                match self.walk_expr(body, ctx) {
                    Ok(mut body_results) => results.append(&mut body_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => {
                        ctx.exit_scope();
                        return Ok(results);
                    },
                    Err(e) => {
                        ctx.exit_scope();
                        return Err(e);
                    }
                }
                
                ctx.exit_scope();
            },
            Item::Struct { fields, .. } => {
                match fields {
                    StructFields::Named(field_list) => {
                        for field in field_list {
                            match self.walk_type(&field.field_type, ctx) {
                                Ok(mut field_results) => results.append(&mut field_results),
                                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                                Err(e) => return Err(e),
                            }
                        }
                    },
                    StructFields::Tuple(types) => {
                        for ty in types {
                            match self.walk_type(ty, ctx) {
                                Ok(mut type_results) => results.append(&mut type_results),
                                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                                Err(e) => return Err(e),
                            }
                        }
                    },
                    StructFields::Unit => {
                        // No fields to walk
                    }
                }
            },
            Item::Enum { variants, .. } => {
                for variant in variants {
                    match &variant.fields {
                        StructFields::Named(field_list) => {
                            for field in field_list {
                                match self.walk_type(&field.field_type, ctx) {
                                    Ok(mut field_results) => results.append(&mut field_results),
                                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                                    Err(e) => return Err(e),
                                }
                            }
                        },
                        StructFields::Tuple(types) => {
                            for ty in types {
                                match self.walk_type(ty, ctx) {
                                    Ok(mut type_results) => results.append(&mut type_results),
                                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                                    Err(e) => return Err(e),
                                }
                            }
                        },
                        StructFields::Unit => {
                            // No fields to walk
                        }
                    }
                }
            },
            Item::Module { items: Some(items), .. } => {
                ctx.enter_scope();
                for item in items {
                    match self.walk_item(item, ctx) {
                        Ok(mut item_results) => results.append(&mut item_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => {
                            ctx.exit_scope();
                            return Ok(results);
                        },
                        Err(e) => {
                            ctx.exit_scope();
                            return Err(e);
                        }
                    }
                }
                ctx.exit_scope();
            },
            _ => {
                // Other items don't have children to walk
            }
        }
        
        if self.order == TraversalOrder::PostOrder {
            match self.visitor.visit_item(item, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        Ok(results)
    }
    
    /// Walk an expression
    pub fn walk_expr(&mut self, expr: &Expr, ctx: &mut VisitorContext) -> VisitorResult<Vec<T>> {
        let mut results = Vec::new();
        
        if self.order == TraversalOrder::PreOrder {
            match self.visitor.visit_expr(expr, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        // Walk expression children based on type
        match expr {
            Expr::Binary { left, right, .. } => {
                match self.walk_expr(left, ctx) {
                    Ok(mut left_results) => results.append(&mut left_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
                match self.walk_expr(right, ctx) {
                    Ok(mut right_results) => results.append(&mut right_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
            },
            Expr::Unary { expr: inner, .. } => {
                match self.walk_expr(inner, ctx) {
                    Ok(mut inner_results) => results.append(&mut inner_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
            },
            Expr::Call { callee, args, .. } => {
                match self.walk_expr(callee, ctx) {
                    Ok(mut callee_results) => results.append(&mut callee_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
                for arg in args {
                    match self.walk_expr(arg, ctx) {
                        Ok(mut arg_results) => results.append(&mut arg_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                        Err(e) => return Err(e),
                    }
                }
            },
            Expr::Block { statements, trailing_expr, .. } => {
                ctx.enter_scope();
                
                for stmt in statements {
                    match self.walk_stmt(stmt, ctx) {
                        Ok(mut stmt_results) => results.append(&mut stmt_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => {
                            ctx.exit_scope();
                            return Ok(results);
                        },
                        Err(e) => {
                            ctx.exit_scope();
                            return Err(e);
                        }
                    }
                }
                
                if let Some(trailing) = trailing_expr {
                    match self.walk_expr(trailing, ctx) {
                        Ok(mut trailing_results) => results.append(&mut trailing_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => {
                            ctx.exit_scope();
                            return Ok(results);
                        },
                        Err(e) => {
                            ctx.exit_scope();
                            return Err(e);
                        }
                    }
                }
                
                ctx.exit_scope();
            },
            // Add more expression types as needed
            _ => {
                // Handle other expression types
            }
        }
        
        if self.order == TraversalOrder::PostOrder {
            match self.visitor.visit_expr(expr, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        Ok(results)
    }
    
    /// Walk a statement
    pub fn walk_stmt(&mut self, stmt: &Stmt, ctx: &mut VisitorContext) -> VisitorResult<Vec<T>> {
        let mut results = Vec::new();
        
        if self.order == TraversalOrder::PreOrder {
            match self.visitor.visit_stmt(stmt, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        // Walk statement children
        match stmt {
            Stmt::Let { pattern, initializer, .. } => {
                match self.walk_pattern(pattern, ctx) {
                    Ok(mut pattern_results) => results.append(&mut pattern_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
                if let Some(init) = initializer {
                    match self.walk_expr(init, ctx) {
                        Ok(mut init_results) => results.append(&mut init_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                        Err(e) => return Err(e),
                    }
                }
            },
            Stmt::Expression { expr, .. } => {
                match self.walk_expr(expr, ctx) {
                    Ok(mut expr_results) => results.append(&mut expr_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
            },
            // Add more statement types as needed
            _ => {
                // Handle other statement types
            }
        }
        
        if self.order == TraversalOrder::PostOrder {
            match self.visitor.visit_stmt(stmt, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        Ok(results)
    }
    
    /// Walk a pattern
    pub fn walk_pattern(&mut self, pattern: &Pattern, ctx: &mut VisitorContext) -> VisitorResult<Vec<T>> {
        let mut results = Vec::new();
        
        if self.order == TraversalOrder::PreOrder {
            match self.visitor.visit_pattern(pattern, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        // Walk pattern children
        match pattern {
            Pattern::Tuple { patterns, .. } => {
                for pat in patterns {
                    match self.walk_pattern(pat, ctx) {
                        Ok(mut pat_results) => results.append(&mut pat_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                        Err(e) => return Err(e),
                    }
                }
            },
            Pattern::Array { patterns, .. } => {
                for pat in patterns {
                    match self.walk_pattern(pat, ctx) {
                        Ok(mut pat_results) => results.append(&mut pat_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                        Err(e) => return Err(e),
                    }
                }
            },
            Pattern::Reference { pattern: inner, .. } => {
                match self.walk_pattern(inner, ctx) {
                    Ok(mut inner_results) => results.append(&mut inner_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
            },
            _ => {
                // Handle other pattern types
            }
        }
        
        if self.order == TraversalOrder::PostOrder {
            match self.visitor.visit_pattern(pattern, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        Ok(results)
    }
    
    /// Walk a type
    pub fn walk_type(&mut self, ty: &Type, ctx: &mut VisitorContext) -> VisitorResult<Vec<T>> {
        let mut results = Vec::new();
        
        if self.order == TraversalOrder::PreOrder {
            match self.visitor.visit_type(ty, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        // Walk type children
        match ty {
            Type::Array { element_type, .. } => {
                match self.walk_type(element_type, ctx) {
                    Ok(mut elem_results) => results.append(&mut elem_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
            },
            Type::Reference { target_type, .. } => {
                match self.walk_type(target_type, ctx) {
                    Ok(mut target_results) => results.append(&mut target_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
            },
            Type::Function { params, return_type, .. } => {
                for param in params {
                    match self.walk_type(param, ctx) {
                        Ok(mut param_results) => results.append(&mut param_results),
                        Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                        Err(e) => return Err(e),
                    }
                }
                match self.walk_type(return_type, ctx) {
                    Ok(mut return_results) => results.append(&mut return_results),
                    Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                    Err(e) => return Err(e),
                }
            },
            _ => {
                // Handle other type variants
            }
        }
        
        if self.order == TraversalOrder::PostOrder {
            match self.visitor.visit_type(ty, ctx) {
                Ok(result) => results.push(result),
                Err(VisitorError::EarlyTermination) if self.early_termination => return Ok(results),
                Err(e) => return Err(e),
            }
        }
        
        Ok(results)
    }
}

/// Utility functions for AST analysis
pub mod utils {
    use super::*;
    
    /// Pretty print an AST node for debugging
    pub fn ast_to_string(module: &Module) -> String {
        let mut printer = AstPrinter::new();
        printer.print_module(module)
    }
    
    /// Count nodes in an AST
    pub fn count_nodes(module: &Module) -> usize {
        let counter = NodeCounter::new();
        let mut ctx = VisitorContext::new();
        let mut walker = AstWalker::new(counter, TraversalOrder::PreOrder);
        
        match walker.walk_module(module, &mut ctx) {
            Ok(counts) => counts.iter().sum(),
            Err(_) => 0,
        }
    }
    
    /// Extract all identifiers from an AST
    pub fn extract_identifiers(module: &Module) -> Vec<InternedString> {
        let extractor = IdentifierExtractor::new();
        let mut ctx = VisitorContext::new();
        let mut walker = AstWalker::new(extractor, TraversalOrder::PreOrder);
        
        match walker.walk_module(module, &mut ctx) {
            Ok(identifiers) => identifiers.into_iter().flatten().collect(),
            Err(_) => Vec::new(),
        }
    }
    
    /// Simple AST printer for debugging
    struct AstPrinter {
        indent: usize,
    }
    
    impl AstPrinter {
        fn new() -> Self {
            Self { indent: 0 }
        }
        
        fn print_module(&mut self, module: &Module) -> String {
            let mut result = String::new();
            result.push_str("Module {\n");
            self.indent += 2;
            
            for item in &module.items {
                result.push_str(&self.print_item(item));
            }
            
            self.indent -= 2;
            result.push_str("}\n");
            result
        }
        
        fn print_item(&self, item: &Item) -> String {
            let indent_str = " ".repeat(self.indent);
            match item {
                Item::Function { name, .. } => {
                    format!("{}Function({})\n", indent_str, name.id)
                },
                Item::Struct { name, .. } => {
                    format!("{}Struct({})\n", indent_str, name.id)
                },
                _ => format!("{}Item\n", indent_str),
            }
        }
    }
    
    /// Node counter visitor
    struct NodeCounter;
    
    impl NodeCounter {
        fn new() -> Self {
            Self
        }
    }
    
    impl ContextVisitor<usize> for NodeCounter {
        fn visit_module(&mut self, _: &Module, _: &mut VisitorContext) -> VisitorResult<usize> {
            Ok(1)
        }
        
        fn visit_item(&mut self, _: &Item, _: &mut VisitorContext) -> VisitorResult<usize> {
            Ok(1)
        }
        
        fn visit_stmt(&mut self, _: &Stmt, _: &mut VisitorContext) -> VisitorResult<usize> {
            Ok(1)
        }
        
        fn visit_expr(&mut self, _: &Expr, _: &mut VisitorContext) -> VisitorResult<usize> {
            Ok(1)
        }
        
        fn visit_pattern(&mut self, _: &Pattern, _: &mut VisitorContext) -> VisitorResult<usize> {
            Ok(1)
        }
        
        fn visit_type(&mut self, _: &Type, _: &mut VisitorContext) -> VisitorResult<usize> {
            Ok(1)
        }
    }
    
    /// Identifier extractor visitor
    struct IdentifierExtractor;
    
    impl IdentifierExtractor {
        fn new() -> Self {
            Self
        }
    }
    
    impl ContextVisitor<Vec<InternedString>> for IdentifierExtractor {
        fn visit_module(&mut self, _: &Module, _: &mut VisitorContext) -> VisitorResult<Vec<InternedString>> {
            Ok(Vec::new())
        }
        
        fn visit_item(&mut self, item: &Item, _: &mut VisitorContext) -> VisitorResult<Vec<InternedString>> {
            match item {
                Item::Function { name, .. } => Ok(vec![*name]),
                Item::Struct { name, .. } => Ok(vec![*name]),
                Item::Enum { name, .. } => Ok(vec![*name]),
                _ => Ok(Vec::new()),
            }
        }
        
        fn visit_stmt(&mut self, _: &Stmt, _: &mut VisitorContext) -> VisitorResult<Vec<InternedString>> {
            Ok(Vec::new())
        }
        
        fn visit_expr(&mut self, expr: &Expr, _: &mut VisitorContext) -> VisitorResult<Vec<InternedString>> {
            match expr {
                Expr::Identifier { name, .. } => Ok(vec![*name]),
                _ => Ok(Vec::new()),
            }
        }
        
        fn visit_pattern(&mut self, pattern: &Pattern, _: &mut VisitorContext) -> VisitorResult<Vec<InternedString>> {
            match pattern {
                Pattern::Identifier { name, .. } => Ok(vec![*name]),
                _ => Ok(Vec::new()),
            }
        }
        
        fn visit_type(&mut self, _: &Type, _: &mut VisitorContext) -> VisitorResult<Vec<InternedString>> {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;
    
    fn dummy_position() -> Position {
        Position::new(1, 1, 0, 0)
    }
    
    fn dummy_span() -> Span {
        Span::single(dummy_position())
    }
    
    #[test]
    fn test_visitor_context() {
        let mut ctx = VisitorContext::new();
        assert_eq!(ctx.scope_depth, 0);
        
        ctx.enter_scope();
        assert_eq!(ctx.scope_depth, 1);
        
        ctx.exit_scope();
        assert_eq!(ctx.scope_depth, 0);
    }
    
    #[test]
    fn test_node_counter() {
        let module = Module {
            items: vec![
                Item::Function {
                    visibility: Visibility::Private,
                    name: InternedString::new(1),
                    generics: Vec::new(),
                    params: Vec::new(),
                    return_type: None,
                    body: None,
                    is_extern: false,
                    span: dummy_span(),
                }
            ],
            span: dummy_span(),
        };
        
        let count = utils::count_nodes(&module);
        assert!(count > 0);
    }
    
    #[test]
    fn test_identifier_extraction() {
        let module = Module {
            items: vec![
                Item::Function {
                    visibility: Visibility::Private,
                    name: InternedString::new(42),
                    generics: Vec::new(),
                    params: Vec::new(),
                    return_type: None,
                    body: None,
                    is_extern: false,
                    span: dummy_span(),
                }
            ],
            span: dummy_span(),
        };
        
        let identifiers = utils::extract_identifiers(&module);
        assert_eq!(identifiers.len(), 1);
        assert_eq!(identifiers[0].id, 42);
    }
} 