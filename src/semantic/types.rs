//! Type System and Type Checking for Prism
//!
//! This module implements Prism's type system including:
//! - Type representation and manipulation
//! - Hindley-Milner style type inference
//! - Type constraint generation and solving
//! - Unification algorithm
//! - Type checking for all AST nodes

use crate::ast::*;
use crate::semantic::symbols::{SymbolTable, Symbol, SymbolKind};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Unique identifier for type variables
pub type TypeVarId = u32;

/// Result type for type operations
pub type TypeResult<T> = Result<T, TypeError>;

/// Type errors that can occur during type checking
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    /// Type mismatch between expected and actual types
    Mismatch {
        expected: TypeInfo,
        actual: TypeInfo,
        span: Span,
    },
    /// Undefined type name
    UndefinedType {
        name: InternedString,
        span: Span,
    },
    /// Recursive type definition
    RecursiveType {
        name: InternedString,
        span: Span,
    },
    /// Cannot infer type (insufficient information)
    CannotInfer {
        span: Span,
        reason: String,
    },
    /// Unification failure
    UnificationFailure {
        type1: TypeInfo,
        type2: TypeInfo,
        span: Span,
    },
    /// Invalid type operation
    InvalidOperation {
        operation: String,
        type_info: TypeInfo,
        span: Span,
    },
    /// Arity mismatch (wrong number of type arguments)
    ArityMismatch {
        expected: usize,
        actual: usize,
        span: Span,
    },
    /// Constraint violation
    ConstraintViolation {
        constraint: String,
        type_info: TypeInfo,
        span: Span,
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::Mismatch { expected, actual, .. } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, actual)
            }
            TypeError::UndefinedType { name, .. } => {
                write!(f, "Undefined type '{}'", name.id)
            }
            TypeError::RecursiveType { name, .. } => {
                write!(f, "Recursive type definition for '{}'", name.id)
            }
            TypeError::CannotInfer { reason, .. } => {
                write!(f, "Cannot infer type: {}", reason)
            }
            TypeError::UnificationFailure { type1, type2, .. } => {
                write!(f, "Cannot unify types {} and {}", type1, type2)
            }
            TypeError::InvalidOperation { operation, type_info, .. } => {
                write!(f, "Invalid operation '{}' on type {}", operation, type_info)
            }
            TypeError::ArityMismatch { expected, actual, .. } => {
                write!(f, "Arity mismatch: expected {} type arguments, found {}", expected, actual)
            }
            TypeError::ConstraintViolation { constraint, type_info, .. } => {
                write!(f, "Type {} violates constraint {}", type_info, constraint)
            }
        }
    }
}

/// Enhanced type information for type checking
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub kind: TypeKind,
    pub span: Span,
    pub constraints: Vec<TypeConstraint>,
    pub is_inferred: bool,
}

impl TypeInfo {
    pub fn new(kind: TypeKind, span: Span) -> Self {
        Self {
            kind,
            span,
            constraints: Vec::new(),
            is_inferred: false,
        }
    }
    
    pub fn with_constraints(mut self, constraints: Vec<TypeConstraint>) -> Self {
        self.constraints = constraints;
        self
    }
    
    pub fn inferred(mut self) -> Self {
        self.is_inferred = true;
        self
    }
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

/// Internal type representation for type checking
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    /// Primitive types
    Primitive(PrimitiveType),
    
    /// Type variables for inference
    Variable(TypeVarId),
    
    /// Function types
    Function {
        params: Vec<TypeInfo>,
        return_type: Box<TypeInfo>,
        is_variadic: bool,
    },
    
    /// Array types
    Array {
        element_type: Box<TypeInfo>,
        size: Option<u64>, // None for dynamic arrays
    },
    
    /// Tuple types
    Tuple(Vec<TypeInfo>),
    
    /// User-defined types (structs, enums)
    UserDefined {
        name: InternedString,
        type_args: Vec<TypeInfo>,
    },
    
    /// Reference types
    Reference {
        is_mutable: bool,
        target_type: Box<TypeInfo>,
    },
    
    /// Pointer types
    Pointer {
        is_mutable: bool,
        target_type: Box<TypeInfo>,
    },
    
    /// Generic type parameters
    Generic {
        name: InternedString,
        bounds: Vec<TypeInfo>,
    },
    
    /// Never type (!)
    Never,
    
    /// Unit type (())
    Unit,
    
    /// Error type (for error recovery)
    Error,
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Primitive(prim) => write!(f, "{:?}", prim),
            TypeKind::Variable(id) => write!(f, "?{}", id),
            TypeKind::Function { params, return_type, is_variadic } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                if *is_variadic { write!(f, ", ...")?; }
                write!(f, ") -> {}", return_type)
            }
            TypeKind::Array { element_type, size } => {
                if let Some(s) = size {
                    write!(f, "[{}; {}]", element_type, s)
                } else {
                    write!(f, "[{}]", element_type)
                }
            }
            TypeKind::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            TypeKind::UserDefined { name, type_args } => {
                write!(f, "{}", name.id)?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            TypeKind::Reference { is_mutable, target_type } => {
                if *is_mutable {
                    write!(f, "&mut {}", target_type)
                } else {
                    write!(f, "&{}", target_type)
                }
            }
            TypeKind::Pointer { is_mutable, target_type } => {
                if *is_mutable {
                    write!(f, "*mut {}", target_type)
                } else {
                    write!(f, "*const {}", target_type)
                }
            }
            TypeKind::Generic { name, bounds } => {
                write!(f, "{}", name.id)?;
                if !bounds.is_empty() {
                    write!(f, ": ")?;
                    for (i, bound) in bounds.iter().enumerate() {
                        if i > 0 { write!(f, " + ")?; }
                        write!(f, "{}", bound)?;
                    }
                }
                Ok(())
            }
            TypeKind::Never => write!(f, "!"),
            TypeKind::Unit => write!(f, "()"),
            TypeKind::Error => write!(f, "<error>"),
        }
    }
}

/// Type constraints for generic types
#[derive(Debug, Clone, PartialEq)]
pub struct TypeConstraint {
    pub kind: ConstraintKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    /// Type must implement a trait
    Trait(TypeInfo),
    /// Type must be copyable
    Copy,
    /// Type must be sized at compile time
    Sized,
    /// Type must support equality comparison
    Eq,
    /// Type must support ordering comparison
    Ord,
    /// Custom constraint
    Custom(String),
}

/// Type substitution for unification
pub type Substitution = HashMap<TypeVarId, TypeInfo>;

/// Type environment for type checking
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    /// Type variable bindings
    bindings: HashMap<TypeVarId, TypeInfo>,
    /// Next available type variable ID
    next_var_id: TypeVarId,
    /// Type constraints
    constraints: Vec<TypeConstraint>,
    /// Generic type parameters in scope
    generics: HashMap<InternedString, TypeInfo>,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            next_var_id: 0,
            constraints: Vec::new(),
            generics: HashMap::new(),
        }
    }
    
    /// Create a fresh type variable
    pub fn fresh_var(&mut self, span: Span) -> TypeInfo {
        let var_id = self.next_var_id;
        self.next_var_id += 1;
        
        TypeInfo::new(TypeKind::Variable(var_id), span)
    }
    
    /// Bind a type variable to a type
    pub fn bind_var(&mut self, var_id: TypeVarId, type_info: TypeInfo) {
        self.bindings.insert(var_id, type_info);
    }
    
    /// Look up a type variable binding
    pub fn lookup_var(&self, var_id: TypeVarId) -> Option<&TypeInfo> {
        self.bindings.get(&var_id)
    }
    
    /// Add a type constraint
    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }
    
    /// Add a generic type parameter
    pub fn add_generic(&mut self, name: InternedString, type_info: TypeInfo) {
        self.generics.insert(name, type_info);
    }
    
    /// Look up a generic type parameter
    pub fn lookup_generic(&self, name: &InternedString) -> Option<&TypeInfo> {
        self.generics.get(name)
    }
    
    /// Apply substitutions to a type
    pub fn apply_substitution(&self, type_info: &TypeInfo) -> TypeInfo {
        match &type_info.kind {
            TypeKind::Variable(var_id) => {
                if let Some(bound_type) = self.lookup_var(*var_id) {
                    self.apply_substitution(bound_type)
                } else {
                    type_info.clone()
                }
            }
            TypeKind::Function { params, return_type, is_variadic } => {
                let new_params = params.iter()
                    .map(|p| self.apply_substitution(p))
                    .collect();
                let new_return = Box::new(self.apply_substitution(return_type));
                
                TypeInfo::new(
                    TypeKind::Function {
                        params: new_params,
                        return_type: new_return,
                        is_variadic: *is_variadic,
                    },
                    type_info.span,
                )
            }
            TypeKind::Array { element_type, size } => {
                let new_element = Box::new(self.apply_substitution(element_type));
                TypeInfo::new(
                    TypeKind::Array {
                        element_type: new_element,
                        size: *size,
                    },
                    type_info.span,
                )
            }
            TypeKind::Tuple(types) => {
                let new_types = types.iter()
                    .map(|t| self.apply_substitution(t))
                    .collect();
                TypeInfo::new(TypeKind::Tuple(new_types), type_info.span)
            }
            TypeKind::Reference { is_mutable, target_type } => {
                let new_target = Box::new(self.apply_substitution(target_type));
                TypeInfo::new(
                    TypeKind::Reference {
                        is_mutable: *is_mutable,
                        target_type: new_target,
                    },
                    type_info.span,
                )
            }
            _ => type_info.clone(),
        }
    }
}

/// Main type system implementation
pub struct TypeSystem {
    /// Type environment for current checking context
    env: TypeEnvironment,
    /// Symbol table for type resolution
    symbol_table: SymbolTable,
    /// Type errors collected during checking
    errors: Vec<TypeError>,
}

impl TypeSystem {
    pub fn new(symbol_table: SymbolTable) -> Self {
        Self {
            env: TypeEnvironment::new(),
            symbol_table,
            errors: Vec::new(),
        }
    }
    
    /// Convert AST type to internal type representation
    pub fn ast_type_to_type_info(&mut self, ast_type: &Type) -> TypeResult<TypeInfo> {
        match ast_type {
            Type::Primitive { kind, span } => {
                Ok(TypeInfo::new(TypeKind::Primitive(*kind), *span))
            }
            
            Type::Path { segments, generics, span } => {
                if segments.len() == 1 {
                    let name = segments[0];
                    
                    // Check if it's a generic parameter
                    if let Some(generic_type) = self.env.lookup_generic(&name) {
                        return Ok(generic_type.clone());
                    }
                    
                    // Look up in symbol table
                    if let Some(symbol) = self.symbol_table.lookup_symbol(&name) {
                        match &symbol.kind {
                            SymbolKind::Type { .. } => {
                                let type_args = generics.iter()
                                    .map(|g| self.ast_type_to_type_info(g))
                                    .collect::<Result<Vec<_>, _>>()?;
                                
                                Ok(TypeInfo::new(
                                    TypeKind::UserDefined {
                                        name,
                                        type_args,
                                    },
                                    *span,
                                ))
                            }
                            _ => Err(TypeError::UndefinedType { name, span: *span }),
                        }
                    } else {
                        Err(TypeError::UndefinedType { name, span: *span })
                    }
                } else {
                    // Qualified type name - TODO: implement module resolution
                    Err(TypeError::UndefinedType {
                        name: segments[0],
                        span: *span,
                    })
                }
            }
            
            Type::Array { element_type, size, span } => {
                let elem_type = Box::new(self.ast_type_to_type_info(element_type)?);
                // TODO: Evaluate size expression
                let size_val = None; // Placeholder
                
                Ok(TypeInfo::new(
                    TypeKind::Array {
                        element_type: elem_type,
                        size: size_val,
                    },
                    *span,
                ))
            }
            
            Type::Tuple { types, span } => {
                let type_infos = types.iter()
                    .map(|t| self.ast_type_to_type_info(t))
                    .collect::<Result<Vec<_>, _>>()?;
                
                Ok(TypeInfo::new(TypeKind::Tuple(type_infos), *span))
            }
            
            Type::Function { params, return_type, is_variadic, span } => {
                let param_types = params.iter()
                    .map(|p| self.ast_type_to_type_info(p))
                    .collect::<Result<Vec<_>, _>>()?;
                let ret_type = Box::new(self.ast_type_to_type_info(return_type)?);
                
                Ok(TypeInfo::new(
                    TypeKind::Function {
                        params: param_types,
                        return_type: ret_type,
                        is_variadic: *is_variadic,
                    },
                    *span,
                ))
            }
            
            Type::Reference { is_mutable, target_type, span } => {
                let target = Box::new(self.ast_type_to_type_info(target_type)?);
                
                Ok(TypeInfo::new(
                    TypeKind::Reference {
                        is_mutable: *is_mutable,
                        target_type: target,
                    },
                    *span,
                ))
            }
            
            Type::Inferred { span } => {
                Ok(self.env.fresh_var(*span))
            }
            
            Type::Never { span } => {
                Ok(TypeInfo::new(TypeKind::Never, *span))
            }
            
            _ => {
                // Handle other type variants
                Ok(TypeInfo::new(TypeKind::Error, ast_type.span()))
            }
        }
    }
    
    /// Unify two types
    pub fn unify(&mut self, type1: &TypeInfo, type2: &TypeInfo) -> TypeResult<()> {
        let t1 = self.env.apply_substitution(type1);
        let t2 = self.env.apply_substitution(type2);
        
        match (&t1.kind, &t2.kind) {
            // Same types unify
            (TypeKind::Primitive(p1), TypeKind::Primitive(p2)) if p1 == p2 => Ok(()),
            (TypeKind::Unit, TypeKind::Unit) => Ok(()),
            (TypeKind::Never, _) | (_, TypeKind::Never) => Ok(()), // Never unifies with anything
            
            // Variable unification
            (TypeKind::Variable(v1), TypeKind::Variable(v2)) if v1 == v2 => Ok(()),
            (TypeKind::Variable(var_id), other) | (other, TypeKind::Variable(var_id)) => {
                if self.occurs_check(*var_id, &TypeInfo::new(other.clone(), t2.span)) {
                    Err(TypeError::RecursiveType {
                        name: InternedString::new(*var_id),
                        span: t1.span.merge(t2.span),
                    })
                } else {
                    self.env.bind_var(*var_id, TypeInfo::new(other.clone(), t2.span));
                    Ok(())
                }
            }
            
            // Function type unification
            (
                TypeKind::Function { params: p1, return_type: r1, is_variadic: v1 },
                TypeKind::Function { params: p2, return_type: r2, is_variadic: v2 }
            ) => {
                if v1 != v2 || p1.len() != p2.len() {
                    return Err(TypeError::UnificationFailure {
                        type1: t1.clone(),
                        type2: t2.clone(),
                        span: t1.span.merge(t2.span),
                    });
                }
                
                // Unify parameters
                for (param1, param2) in p1.iter().zip(p2.iter()) {
                    self.unify(param1, param2)?;
                }
                
                // Unify return types
                self.unify(r1, r2)
            }
            
            // Array type unification
            (
                TypeKind::Array { element_type: e1, size: s1 },
                TypeKind::Array { element_type: e2, size: s2 }
            ) => {
                if s1 != s2 {
                    return Err(TypeError::UnificationFailure {
                        type1: t1.clone(),
                        type2: t2.clone(),
                        span: t1.span.merge(t2.span),
                    });
                }
                
                self.unify(e1, e2)
            }
            
            // Tuple type unification
            (TypeKind::Tuple(types1), TypeKind::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    return Err(TypeError::UnificationFailure {
                        type1: t1.clone(),
                        type2: t2.clone(),
                        span: t1.span.merge(t2.span),
                    });
                }
                
                for (ty1, ty2) in types1.iter().zip(types2.iter()) {
                    self.unify(ty1, ty2)?;
                }
                
                Ok(())
            }
            
            // User-defined type unification
            (
                TypeKind::UserDefined { name: n1, type_args: args1 },
                TypeKind::UserDefined { name: n2, type_args: args2 }
            ) => {
                if n1 != n2 || args1.len() != args2.len() {
                    return Err(TypeError::UnificationFailure {
                        type1: t1.clone(),
                        type2: t2.clone(),
                        span: t1.span.merge(t2.span),
                    });
                }
                
                for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                    self.unify(arg1, arg2)?;
                }
                
                Ok(())
            }
            
            // Reference type unification
            (
                TypeKind::Reference { is_mutable: m1, target_type: t1 },
                TypeKind::Reference { is_mutable: m2, target_type: t2 }
            ) => {
                if m1 != m2 {
                    return Err(TypeError::UnificationFailure {
                        type1: t1.clone(),
                        type2: t2.clone(),
                        span: t1.span.merge(t2.span),
                    });
                }
                
                self.unify(t1, t2)
            }
            
            // Error type unifies with anything (for error recovery)
            (TypeKind::Error, _) | (_, TypeKind::Error) => Ok(()),
            
            // Default case - types don't unify
            _ => Err(TypeError::UnificationFailure {
                type1: t1.clone(),
                type2: t2.clone(),
                span: t1.span.merge(t2.span),
            }),
        }
    }
    
    /// Occurs check to prevent infinite types
    fn occurs_check(&self, var_id: TypeVarId, type_info: &TypeInfo) -> bool {
        match &type_info.kind {
            TypeKind::Variable(id) => *id == var_id,
            TypeKind::Function { params, return_type, .. } => {
                params.iter().any(|p| self.occurs_check(var_id, p)) ||
                self.occurs_check(var_id, return_type)
            }
            TypeKind::Array { element_type, .. } => {
                self.occurs_check(var_id, element_type)
            }
            TypeKind::Tuple(types) => {
                types.iter().any(|t| self.occurs_check(var_id, t))
            }
            TypeKind::Reference { target_type, .. } => {
                self.occurs_check(var_id, target_type)
            }
            TypeKind::UserDefined { type_args, .. } => {
                type_args.iter().any(|arg| self.occurs_check(var_id, arg))
            }
            _ => false,
        }
    }
    
    /// Get the final type after applying all substitutions
    pub fn resolve_type(&self, type_info: &TypeInfo) -> TypeInfo {
        self.env.apply_substitution(type_info)
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
    expression_types: HashMap<*const Expr, TypeInfo>,
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
            Item::Function { params, return_type, body, .. } => {
                // Check parameter types
                for param in params {
                    if let Some(type_annotation) = &param.type_annotation {
                        self.type_system.ast_type_to_type_info(type_annotation)?;
                    }
                }
                
                // Check return type
                let expected_return = if let Some(ret_type) = return_type {
                    self.type_system.ast_type_to_type_info(ret_type)?
                } else {
                    TypeInfo::new(TypeKind::Unit, item.span())
                };
                
                // Check function body
                if let Some(body_expr) = body {
                    let body_type = self.check_expr(body_expr)?;
                    self.type_system.unify(&body_type, &expected_return)?;
                }
                
                Ok(())
            }
            
            Item::Struct { fields, .. } => {
                match fields {
                    StructFields::Named(field_list) => {
                        for field in field_list {
                            self.type_system.ast_type_to_type_info(&field.field_type)?;
                        }
                    }
                    StructFields::Tuple(types) => {
                        for ty in types {
                            self.type_system.ast_type_to_type_info(ty)?;
                        }
                    }
                    StructFields::Unit => {}
                }
                
                Ok(())
            }
            
            _ => Ok(()), // Handle other items as needed
        }
    }
    
    /// Type check an expression
    fn check_expr(&mut self, expr: &Expr) -> TypeResult<TypeInfo> {
        let type_info = match expr {
            Expr::Literal { literal, span } => {
                self.check_literal(literal, *span)
            }
            
            Expr::Identifier { name, span } => {
                if let Some(symbol) = self.type_system.symbol_table.lookup_symbol(name) {
                    match &symbol.kind {
                        SymbolKind::Variable { type_info: Some(ti), .. } => Ok(ti.clone()),
                        SymbolKind::Variable { type_info: None, .. } => {
                            // Type will be inferred
                            Ok(self.type_system.env.fresh_var(*span))
                        }
                        SymbolKind::Function { params, return_type, .. } => {
                            // Create function type
                            let param_types = params.iter()
                                .map(|p| {
                                    if let Some(type_annotation) = &p.type_annotation {
                                        self.type_system.ast_type_to_type_info(type_annotation)
                                    } else {
                                        Ok(self.type_system.env.fresh_var(*span))
                                    }
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            
                            let ret_type = if let Some(ret) = return_type {
                                Box::new(self.type_system.ast_type_to_type_info(ret)?)
                            } else {
                                Box::new(TypeInfo::new(TypeKind::Unit, *span))
                            };
                            
                            Ok(TypeInfo::new(
                                TypeKind::Function {
                                    params: param_types,
                                    return_type: ret_type,
                                    is_variadic: false,
                                },
                                *span,
                            ))
                        }
                        _ => Err(TypeError::InvalidOperation {
                            operation: "reference".to_string(),
                            type_info: TypeInfo::new(TypeKind::Error, *span),
                            span: *span,
                        }),
                    }
                } else {
                    Err(TypeError::UndefinedType { name: *name, span: *span })
                }
            }
            
            Expr::Binary { left, op, right, span } => {
                let left_type = self.check_expr(left)?;
                let right_type = self.check_expr(right)?;
                
                self.check_binary_op(&left_type, *op, &right_type, *span)
            }
            
            Expr::Call { callee, args, span } => {
                let callee_type = self.check_expr(callee)?;
                let arg_types: Vec<TypeInfo> = args.iter()
                    .map(|arg| self.check_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                
                self.check_function_call(&callee_type, &arg_types, *span)
            }
            
            Expr::Block { statements, trailing_expr, span } => {
                // Check all statements
                for stmt in statements {
                    self.check_stmt(stmt)?;
                }
                
                // Check trailing expression or return unit
                if let Some(trailing) = trailing_expr {
                    self.check_expr(trailing)
                } else {
                    Ok(TypeInfo::new(TypeKind::Unit, *span))
                }
            }
            
            _ => {
                // Handle other expression types
                Ok(TypeInfo::new(TypeKind::Error, expr.span()))
            }
        }?;
        
        // Store the type for this expression
        self.expression_types.insert(expr as *const Expr, type_info.clone());
        
        Ok(type_info)
    }
    
    /// Type check a literal
    fn check_literal(&mut self, literal: &Literal, span: Span) -> TypeResult<TypeInfo> {
        let type_kind = match literal {
            Literal::Integer { .. } => TypeKind::Primitive(PrimitiveType::I32), // Default to i32
            Literal::Float { .. } => TypeKind::Primitive(PrimitiveType::F64),   // Default to f64
            Literal::String { .. } => TypeKind::Primitive(PrimitiveType::Str),
            Literal::Char(_) => TypeKind::Primitive(PrimitiveType::Char),
            Literal::Bool(_) => TypeKind::Primitive(PrimitiveType::Bool),
            Literal::Null => TypeKind::Pointer {
                is_mutable: false,
                target_type: Box::new(TypeInfo::new(TypeKind::Unit, span)),
            },
        };
        
        Ok(TypeInfo::new(type_kind, span))
    }
    
    /// Type check a binary operation
    fn check_binary_op(
        &mut self,
        left_type: &TypeInfo,
        op: BinaryOp,
        right_type: &TypeInfo,
        span: Span,
    ) -> TypeResult<TypeInfo> {
        // Unify operand types for most operations
        match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                self.type_system.unify(left_type, right_type)?;
                Ok(left_type.clone())
            }
            
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual |
            BinaryOp::Greater | BinaryOp::GreaterEqual => {
                self.type_system.unify(left_type, right_type)?;
                Ok(TypeInfo::new(TypeKind::Primitive(PrimitiveType::Bool), span))
            }
            
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                let bool_type = TypeInfo::new(TypeKind::Primitive(PrimitiveType::Bool), span);
                self.type_system.unify(left_type, &bool_type)?;
                self.type_system.unify(right_type, &bool_type)?;
                Ok(bool_type)
            }
            
            _ => {
                // Handle other binary operations
                Ok(TypeInfo::new(TypeKind::Error, span))
            }
        }
    }
    
    /// Type check a function call
    fn check_function_call(
        &mut self,
        callee_type: &TypeInfo,
        arg_types: &[TypeInfo],
        span: Span,
    ) -> TypeResult<TypeInfo> {
        match &callee_type.kind {
            TypeKind::Function { params, return_type, .. } => {
                if params.len() != arg_types.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: params.len(),
                        actual: arg_types.len(),
                        span,
                    });
                }
                
                // Unify argument types with parameter types
                for (param_type, arg_type) in params.iter().zip(arg_types.iter()) {
                    self.type_system.unify(param_type, arg_type)?;
                }
                
                Ok(return_type.as_ref().clone())
            }
            
            TypeKind::Variable(_) => {
                // Create a fresh function type and unify
                let fresh_params: Vec<TypeInfo> = arg_types.iter()
                    .map(|_| self.type_system.env.fresh_var(span))
                    .collect();
                let fresh_return = self.type_system.env.fresh_var(span);
                
                let function_type = TypeInfo::new(
                    TypeKind::Function {
                        params: fresh_params.clone(),
                        return_type: Box::new(fresh_return.clone()),
                        is_variadic: false,
                    },
                    span,
                );
                
                self.type_system.unify(callee_type, &function_type)?;
                
                // Unify arguments with parameters
                for (param_type, arg_type) in fresh_params.iter().zip(arg_types.iter()) {
                    self.type_system.unify(param_type, arg_type)?;
                }
                
                Ok(fresh_return)
            }
            
            _ => Err(TypeError::InvalidOperation {
                operation: "function call".to_string(),
                type_info: callee_type.clone(),
                span,
            }),
        }
    }
    
    /// Type check a statement
    fn check_stmt(&mut self, stmt: &Stmt) -> TypeResult<()> {
        match stmt {
            Stmt::Let { pattern, type_annotation, initializer, .. } => {
                let declared_type = if let Some(type_annotation) = type_annotation {
                    Some(self.type_system.ast_type_to_type_info(type_annotation)?)
                } else {
                    None
                };
                
                if let Some(init_expr) = initializer {
                    let init_type = self.check_expr(init_expr)?;
                    
                    if let Some(declared) = &declared_type {
                        self.type_system.unify(&init_type, declared)?;
                    }
                    
                    // TODO: Update symbol table with inferred type
                }
                
                Ok(())
            }
            
            Stmt::Expression { expr, .. } => {
                self.check_expr(expr)?;
                Ok(())
            }
            
            _ => Ok(()), // Handle other statement types
        }
    }
    
    /// Get the type of an expression
    pub fn get_expr_type(&self, expr: &Expr) -> Option<&TypeInfo> {
        self.expression_types.get(&(expr as *const Expr))
    }
    
    /// Get all type errors
    pub fn errors(&self) -> &[TypeError] {
        self.type_system.errors()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;
    use crate::semantic::symbols::SymbolTable;
    
    fn dummy_position() -> Position {
        Position::new(1, 1, 0, 0)
    }
    
    fn dummy_span() -> Span {
        Span::single(dummy_position())
    }
    
    #[test]
    fn test_type_info_creation() {
        let type_info = TypeInfo::new(
            TypeKind::Primitive(PrimitiveType::I32),
            dummy_span(),
        );
        
        assert_eq!(type_info.kind, TypeKind::Primitive(PrimitiveType::I32));
        assert!(!type_info.is_inferred);
    }
    
    #[test]
    fn test_type_environment() {
        let mut env = TypeEnvironment::new();
        
        let var1 = env.fresh_var(dummy_span());
        let var2 = env.fresh_var(dummy_span());
        
        // Variables should be different
        assert_ne!(var1.kind, var2.kind);
        
        // Bind a variable
        if let TypeKind::Variable(var_id) = var1.kind {
            let int_type = TypeInfo::new(TypeKind::Primitive(PrimitiveType::I32), dummy_span());
            env.bind_var(var_id, int_type.clone());
            
            assert_eq!(env.lookup_var(var_id), Some(&int_type));
        }
    }
    
    #[test]
    fn test_unification() {
        let symbol_table = SymbolTable::new();
        let mut type_system = TypeSystem::new(symbol_table);
        
        let int_type1 = TypeInfo::new(TypeKind::Primitive(PrimitiveType::I32), dummy_span());
        let int_type2 = TypeInfo::new(TypeKind::Primitive(PrimitiveType::I32), dummy_span());
        
        // Same types should unify
        assert!(type_system.unify(&int_type1, &int_type2).is_ok());
        
        let float_type = TypeInfo::new(TypeKind::Primitive(PrimitiveType::F32), dummy_span());
        
        // Different types should not unify
        assert!(type_system.unify(&int_type1, &float_type).is_err());
    }
    
    #[test]
    fn test_variable_unification() {
        let symbol_table = SymbolTable::new();
        let mut type_system = TypeSystem::new(symbol_table);
        
        let var_type = type_system.env.fresh_var(dummy_span());
        let int_type = TypeInfo::new(TypeKind::Primitive(PrimitiveType::I32), dummy_span());
        
        // Variable should unify with concrete type
        assert!(type_system.unify(&var_type, &int_type).is_ok());
        
        // Variable should now be bound to int type
        let resolved = type_system.resolve_type(&var_type);
        assert_eq!(resolved.kind, TypeKind::Primitive(PrimitiveType::I32));
    }
} 