//! Type System and Type Checking for Prism

use crate::ast::{Type, Expr, Stmt, Item, Module, Literal, PrimitiveType, Span, InternedString};
use crate::semantic::symbols::{SymbolTable, SymbolKind};
use std::collections::HashMap;
use std::fmt;

/// Result type for type operations
pub type TypeResult<T> = Result<T, TypeError>;

/// Type errors that can occur during type checking
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    /// Type mismatch between expected and actual types
    Mismatch {
        expected: Type,
        actual: Type,
        span: Span,
    },
    /// Undefined type name
    UndefinedType {
        name: InternedString,
        span: Span,
    },
    /// Cannot infer type (insufficient information)
    CannotInfer {
        span: Span,
        reason: String,
    },
    /// Invalid type operation
    InvalidOperation {
        operation: String,
        span: Span,
    },
    /// Arity mismatch (wrong number of type arguments)
    ArityMismatch {
        expected: usize,
        actual: usize,
        span: Span,
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::Mismatch { expected, actual, .. } => {
                write!(f, "Type mismatch: expected {:?}, found {:?}", expected, actual)
            }
            TypeError::UndefinedType { name, .. } => {
                write!(f, "Undefined type '{}'", name.id)
            }
            TypeError::CannotInfer { reason, .. } => {
                write!(f, "Cannot infer type: {}", reason)
            }
            TypeError::InvalidOperation { operation, .. } => {
                write!(f, "Invalid operation '{}'", operation)
            }
            TypeError::ArityMismatch { expected, actual, .. } => {
                write!(f, "Arity mismatch: expected {} type arguments, found {}", expected, actual)
            }
        }
    }
}

/// Main type system implementation
pub struct TypeSystem {
    /// Symbol table for type resolution
    symbol_table: SymbolTable,
    /// Type errors collected during checking
    errors: Vec<TypeError>,
}

impl TypeSystem {
    pub fn new(symbol_table: SymbolTable) -> Self {
        Self {
            symbol_table,
            errors: Vec::new(),
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
}

/// Type checker that walks the AST and performs type checking
pub struct TypeChecker {
    type_system: TypeSystem,
    expression_types: HashMap<*const Expr, Type>,
}

impl TypeChecker {
    pub fn new(symbol_table: SymbolTable) -> Self {
        Self {
            type_system: TypeSystem::new(symbol_table),
            expression_types: HashMap::new(),
        }
    }
    
    /// Type check a complete module
    pub fn check_module(&mut self, module: &Module) -> TypeResult<()> {
        for item in &module.items {
            self.check_item(item)?;
        }
        Ok(())
    }
    
    /// Type check an item
    fn check_item(&mut self, item: &Item) -> TypeResult<()> {
        match item {
            Item::Function { body, .. } => {
                if let Some(body_expr) = body {
                    self.check_expr(body_expr)?;
                }
                Ok(())
            }
            Item::Struct { .. } => Ok(()),
            _ => Ok(()),
        }
    }
    
    /// Type check an expression
    fn check_expr(&mut self, expr: &Expr) -> TypeResult<Type> {
        let type_result = match expr {
            Expr::Literal { literal, span } => {
                self.check_literal(literal, *span)
            }
            
            Expr::Identifier { name, span } => {
                if let Some(symbol) = self.type_system.symbol_table.lookup_symbol(name) {
                    match &symbol.kind {
                        SymbolKind::Variable { type_info: Some(ty), .. } => Ok(ty.clone()),
                        SymbolKind::Variable { type_info: None, .. } => {
                            Err(TypeError::CannotInfer {
                                span: *span,
                                reason: "Variable type not yet inferred".to_string(),
                            })
                        }
                        _ => Err(TypeError::InvalidOperation {
                            operation: "reference".to_string(),
                            span: *span,
                        }),
                    }
                } else {
                    Err(TypeError::UndefinedType { name: *name, span: *span })
                }
            }
            
            Expr::Binary { left, right, span, .. } => {
                let _left_type = self.check_expr(left)?;
                let _right_type = self.check_expr(right)?;
                
                Ok(Type::Primitive {
                    kind: PrimitiveType::I32,
                    span: *span,
                })
            }
            
            Expr::Call { callee, args, span } => {
                let _callee_type = self.check_expr(callee)?;
                for arg in args {
                    self.check_expr(arg)?;
                }
                
                Ok(Type::Primitive {
                    kind: PrimitiveType::Unit,
                    span: *span,
                })
            }
            
            Expr::Block { statements, trailing_expr, span } => {
                for stmt in statements {
                    self.check_stmt(stmt)?;
                }
                
                if let Some(trailing) = trailing_expr {
                    self.check_expr(trailing)
                } else {
                    Ok(Type::Primitive {
                        kind: PrimitiveType::Unit,
                        span: *span,
                    })
                }
            }
            
            _ => {
                Ok(Type::Primitive {
                    kind: PrimitiveType::Unit,
                    span: expr.span(),
                })
            }
        }?;
        
        self.expression_types.insert(expr as *const Expr, type_result.clone());
        Ok(type_result)
    }
    
    /// Type check a literal
    fn check_literal(&mut self, literal: &Literal, span: Span) -> TypeResult<Type> {
        let primitive_kind = match literal {
            Literal::Integer { .. } => PrimitiveType::I32,
            Literal::Float { .. } => PrimitiveType::F64,
            Literal::String { .. } => PrimitiveType::Str,
            Literal::Char(_) => PrimitiveType::Char,
            Literal::Bool(_) => PrimitiveType::Bool,
            Literal::Null => {
                return Ok(Type::Pointer {
                    is_mutable: false,
                    target_type: Box::new(Type::Primitive {
                        kind: PrimitiveType::Unit,
                        span,
                    }),
                    span,
                });
            }
        };
        
        Ok(Type::Primitive {
            kind: primitive_kind,
            span,
        })
    }
    
    /// Type check a statement
    fn check_stmt(&mut self, stmt: &Stmt) -> TypeResult<()> {
        match stmt {
            Stmt::Let { initializer, .. } => {
                if let Some(init_expr) = initializer {
                    self.check_expr(init_expr)?;
                }
                Ok(())
            }
            
            Stmt::Expression { expr, .. } => {
                self.check_expr(expr)?;
                Ok(())
            }
            
            _ => Ok(()),
        }
    }
    
    /// Get the type of an expression
    pub fn get_expr_type(&self, expr: &Expr) -> Option<&Type> {
        self.expression_types.get(&(expr as *const Expr))
    }
    
    /// Get all type errors
    pub fn errors(&self) -> &[TypeError] {
        self.type_system.errors()
    }
} 