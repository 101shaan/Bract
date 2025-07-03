//! Symbol Table and Scope Management for Prism Semantic Analysis
//!
//! This module implements the core symbol table infrastructure required for
//! name resolution, scope management, and symbol tracking throughout the
//! compilation pipeline.

use crate::ast::{Item, Visibility, Span, InternedString, StructFields, EnumVariant, GenericParam, Type, Expr, Parameter, Pattern, Module};
use crate::lexer::Position;

use std::collections::{HashMap, HashSet};
use std::fmt;

/// Unique identifier for scopes
pub type ScopeId = u32;

/// Unique identifier for symbols
pub type SymbolId = u32;

/// Result type for symbol operations
pub type SymbolResult<T> = Result<T, SymbolError>;

/// Errors that can occur during symbol resolution
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolError {
    /// Symbol already defined in current scope
    DuplicateSymbol {
        name: InternedString,
        existing_span: Span,
        new_span: Span,
    },
    /// Symbol not found in any accessible scope
    UndefinedSymbol {
        name: InternedString,
        span: Span,
    },
    /// Symbol exists but is not accessible from current scope
    InaccessibleSymbol {
        name: InternedString,
        span: Span,
        reason: String,
    },
    /// Circular dependency detected
    CircularDependency {
        symbols: Vec<InternedString>,
        spans: Vec<Span>,
    },
    /// Invalid symbol usage (e.g., using type as value)
    InvalidUsage {
        name: InternedString,
        expected: SymbolKind,
        actual: SymbolKind,
        span: Span,
    },
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolError::DuplicateSymbol { name, .. } => {
                write!(f, "Symbol '{}' is already defined in this scope", name.id)
            }
            SymbolError::UndefinedSymbol { name, .. } => {
                write!(f, "Undefined symbol '{}'", name.id)
            }
            SymbolError::InaccessibleSymbol { name, reason, .. } => {
                write!(f, "Symbol '{}' is not accessible: {}", name.id, reason)
            }
            SymbolError::CircularDependency { symbols, .. } => {
                write!(f, "Circular dependency detected involving symbols: {:?}", 
                       symbols.iter().map(|s| s.id).collect::<Vec<_>>())
            }
            SymbolError::InvalidUsage { name, expected, actual, .. } => {
                write!(f, "Invalid usage of symbol '{}': expected {:?}, found {:?}", 
                       name.id, expected, actual)
            }
        }
    }
}

/// Types of symbols in the symbol table
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    /// Variable symbol
    Variable {
        is_mutable: bool,
        type_info: Option<Type>,
    },
    /// Function symbol
    Function {
        params: Vec<Parameter>,
        return_type: Option<Type>,
        is_extern: bool,
        is_method: bool,
    },
    /// Type symbol (struct, enum, type alias)
    Type {
        definition: TypeDefinition,
    },
    /// Module symbol
    Module {
        is_external: bool,
    },
    /// Constant symbol
    Constant {
        type_info: Type,
        value: Option<Expr>,
    },
    /// Generic parameter symbol
    GenericParam {
        bounds: Vec<Type>,
    },
}

/// Type definitions for type symbols
#[derive(Debug, Clone, PartialEq)]
pub enum TypeDefinition {
    Struct {
        fields: StructFields,
        generics: Vec<GenericParam>,
    },
    Enum {
        variants: Vec<EnumVariant>,
        generics: Vec<GenericParam>,
    },
    Alias {
        target: Type,
        generics: Vec<GenericParam>,
    },
}

/// Symbol entry in the symbol table
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: InternedString,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub span: Span,
    pub scope_id: ScopeId,
    pub is_used: bool,
    pub dependencies: HashSet<SymbolId>,
}

impl Symbol {
    pub fn new(
        id: SymbolId,
        name: InternedString,
        kind: SymbolKind,
        visibility: Visibility,
        span: Span,
        scope_id: ScopeId,
    ) -> Self {
        Self {
            id,
            name,
            kind,
            visibility,
            span,
            scope_id,
            is_used: false,
            dependencies: HashSet::new(),
        }
    }
    
    /// Mark this symbol as used
    pub fn mark_used(&mut self) {
        self.is_used = true;
    }
    
    /// Add a dependency to another symbol
    pub fn add_dependency(&mut self, symbol_id: SymbolId) {
        self.dependencies.insert(symbol_id);
    }
    
    /// Check if this symbol is accessible from the given scope
    pub fn is_accessible_from(&self, scope_id: ScopeId, symbol_table: &SymbolTable) -> bool {
        match self.visibility {
            Visibility::Public => true,
            Visibility::Private => {
                // Private symbols are accessible within the same module
                symbol_table.is_same_module(self.scope_id, scope_id)
            }
        }
    }
}

/// Scope information
#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub id: ScopeId,
    pub parent_id: Option<ScopeId>,
    pub kind: ScopeKind,
    pub symbols: HashMap<InternedString, SymbolId>,
    pub children: Vec<ScopeId>,
    pub span: Span,
}

/// Types of scopes
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeKind {
    /// Global/module scope
    Module,
    /// Function scope
    Function,
    /// Block scope
    Block,
    /// Struct/enum scope
    Type,
    /// Impl block scope
    Impl,
}

impl Scope {
    pub fn new(id: ScopeId, parent_id: Option<ScopeId>, kind: ScopeKind, span: Span) -> Self {
        Self {
            id,
            parent_id,
            kind,
            symbols: HashMap::new(),
            children: Vec::new(),
            span,
        }
    }
    
    /// Add a symbol to this scope
    pub fn add_symbol(&mut self, name: InternedString, symbol_id: SymbolId) -> Result<(), SymbolError> {
        if self.symbols.contains_key(&name) {
            // Symbol already exists in this scope
            Err(SymbolError::DuplicateSymbol {
                name,
                existing_span: Span::single(Position::new(0, 0, 0, 0)), // TODO: Get actual span
                new_span: Span::single(Position::new(0, 0, 0, 0)),     // TODO: Get actual span
            })
        } else {
            self.symbols.insert(name, symbol_id);
            Ok(())
        }
    }
    
    /// Look up a symbol in this scope
    pub fn lookup_symbol(&self, name: &InternedString) -> Option<SymbolId> {
        self.symbols.get(name).copied()
    }
}

/// Main symbol table managing all symbols and scopes
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// All symbols indexed by ID
    symbols: HashMap<SymbolId, Symbol>,
    /// All scopes indexed by ID
    scopes: HashMap<ScopeId, Scope>,
    /// Current scope during traversal
    current_scope_id: ScopeId,
    /// Next available symbol ID
    next_symbol_id: SymbolId,
    /// Next available scope ID
    next_scope_id: ScopeId,
    /// Root scope (global/module scope)
    root_scope_id: ScopeId,
    /// Module hierarchy for visibility checking
    _module_hierarchy: HashMap<ScopeId, ScopeId>,
}

impl SymbolTable {
    /// Create a new symbol table with a root scope
    pub fn new() -> Self {
        let root_scope_id = 0;
        let mut scopes = HashMap::new();
        
        let root_scope = Scope::new(
            root_scope_id,
            None,
            ScopeKind::Module,
            Span::single(Position::new(0, 0, 0, 0)),
        );
        scopes.insert(root_scope_id, root_scope);
        
        Self {
            symbols: HashMap::new(),
            scopes,
            current_scope_id: root_scope_id,
            next_symbol_id: 0,
            next_scope_id: 1,
            root_scope_id,
            _module_hierarchy: HashMap::new(),
        }
    }
    
    /// Enter a new scope
    pub fn enter_scope(&mut self, kind: ScopeKind, span: Span) -> ScopeId {
        let scope_id = self.next_scope_id;
        self.next_scope_id += 1;
        
        let scope = Scope::new(scope_id, Some(self.current_scope_id), kind, span);
        
        // Add this scope as a child of the current scope
        if let Some(current_scope) = self.scopes.get_mut(&self.current_scope_id) {
            current_scope.children.push(scope_id);
        }
        
        self.scopes.insert(scope_id, scope);
        self.current_scope_id = scope_id;
        
        scope_id
    }
    
    /// Exit the current scope
    pub fn exit_scope(&mut self) -> SymbolResult<()> {
        if let Some(scope) = self.scopes.get(&self.current_scope_id) {
            if let Some(parent_id) = scope.parent_id {
                self.current_scope_id = parent_id;
                Ok(())
            } else {
                // Cannot exit root scope
                Ok(())
            }
        } else {
            Ok(())
        }
    }
    
    /// Get the current scope ID
    pub fn current_scope(&self) -> ScopeId {
        self.current_scope_id
    }
    
    /// Add a symbol to the current scope
    pub fn add_symbol(
        &mut self,
        name: InternedString,
        kind: SymbolKind,
        visibility: Visibility,
        span: Span,
    ) -> SymbolResult<SymbolId> {
        let symbol_id = self.next_symbol_id;
        self.next_symbol_id += 1;
        
        let symbol = Symbol::new(
            symbol_id,
            name,
            kind,
            visibility,
            span,
            self.current_scope_id,
        );
        
        // Add symbol to current scope
        if let Some(scope) = self.scopes.get_mut(&self.current_scope_id) {
            scope.add_symbol(name, symbol_id)?;
        }
        
        self.symbols.insert(symbol_id, symbol);
        Ok(symbol_id)
    }
    
    /// Look up a symbol by name, searching up the scope chain
    pub fn lookup_symbol(&self, name: &InternedString) -> Option<&Symbol> {
        let mut current_scope_id = self.current_scope_id;
        
        loop {
            if let Some(scope) = self.scopes.get(&current_scope_id) {
                if let Some(symbol_id) = scope.lookup_symbol(name) {
                    return self.symbols.get(&symbol_id);
                }
                
                // Move to parent scope
                if let Some(parent_id) = scope.parent_id {
                    current_scope_id = parent_id;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        
        None
    }
    
    /// Look up a symbol by ID
    pub fn get_symbol(&self, symbol_id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(&symbol_id)
    }
    
    /// Get a mutable reference to a symbol
    pub fn get_symbol_mut(&mut self, symbol_id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(&symbol_id)
    }
    
    /// Check if two scopes are in the same module
    pub fn is_same_module(&self, scope1: ScopeId, scope2: ScopeId) -> bool {
        let module1 = self.get_module_scope(scope1);
        let module2 = self.get_module_scope(scope2);
        module1 == module2
    }
    
    /// Get the module scope containing the given scope
    fn get_module_scope(&self, mut scope_id: ScopeId) -> ScopeId {
        loop {
            if let Some(scope) = self.scopes.get(&scope_id) {
                match scope.kind {
                    ScopeKind::Module => return scope_id,
                    _ => {
                        if let Some(parent_id) = scope.parent_id {
                            scope_id = parent_id;
                        } else {
                            return self.root_scope_id;
                        }
                    }
                }
            } else {
                return self.root_scope_id;
            }
        }
    }
    
    /// Get all symbols in the current scope
    pub fn current_scope_symbols(&self) -> Vec<&Symbol> {
        if let Some(scope) = self.scopes.get(&self.current_scope_id) {
            scope.symbols.values()
                .filter_map(|id| self.symbols.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all unused symbols (for warnings)
    pub fn unused_symbols(&self) -> Vec<&Symbol> {
        self.symbols.values()
            .filter(|symbol| !symbol.is_used && symbol.visibility == Visibility::Private)
            .collect()
    }
}

/// Symbol table builder that walks the AST and builds the symbol table
pub struct SymbolTableBuilder {
    symbol_table: SymbolTable,
    errors: Vec<SymbolError>,
}

impl SymbolTableBuilder {
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
        }
    }
    
    /// Build symbol table from AST module
    pub fn build(mut self, module: &Module) -> (SymbolTable, Vec<SymbolError>) {
        self.visit_module(module);
        (self.symbol_table, self.errors)
    }
    
    /// Visit a module and collect all symbols
    fn visit_module(&mut self, module: &Module) {
        for item in &module.items {
            self.visit_item(item);
        }
    }
    
    /// Visit an item and add it to the symbol table
    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Function { visibility, name, params, return_type, body, is_extern, span, .. } => {
                let kind = SymbolKind::Function {
                    params: params.clone(),
                    return_type: return_type.clone(),
                    is_extern: *is_extern,
                    is_method: false,
                };
                
                match self.symbol_table.add_symbol(*name, kind, *visibility, *span) {
                    Ok(_) => {
                        // Enter function scope and process body
                        if let Some(body_expr) = body {
                            let _scope_id = self.symbol_table.enter_scope(ScopeKind::Function, *span);
                            
                            // Add parameters to function scope
                            for param in params {
                                self.visit_pattern(&param.pattern);
                            }
                            
                            self.visit_expr(body_expr);
                            let _ = self.symbol_table.exit_scope();
                        }
                    }
                    Err(err) => self.errors.push(err),
                }
            }
            
            Item::Struct { visibility, name, fields, span, .. } => {
                let definition = TypeDefinition::Struct {
                    fields: fields.clone(),
                    generics: Vec::new(), // TODO: Handle generics
                };
                let kind = SymbolKind::Type { definition };
                
                if let Err(err) = self.symbol_table.add_symbol(*name, kind, *visibility, *span) {
                    self.errors.push(err);
                }
            }
            
            _ => {
                // Handle other item types
            }
        }
    }
    
    /// Visit an expression and collect symbols
    fn visit_expr(&mut self, _expr: &Expr) {
        // TODO: Implement expression symbol collection
    }
    
    /// Visit a pattern and collect symbol references
    fn visit_pattern(&mut self, _pattern: &Pattern) {
        // TODO: Implement pattern symbol collection
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
    fn test_symbol_table_creation() {
        let mut table = SymbolTable::new();
        assert_eq!(table.current_scope(), 0);
        
        let name = InternedString::new(1);
        let kind = SymbolKind::Variable {
            is_mutable: false,
            type_info: None,
        };
        
        let symbol_id = table.add_symbol(name, kind, Visibility::Private, dummy_span()).unwrap();
        assert_eq!(symbol_id, 0);
        
        let symbol = table.get_symbol(symbol_id).unwrap();
        assert_eq!(symbol.name, name);
    }
    
    #[test]
    fn test_scope_management() {
        let mut table = SymbolTable::new();
        let root_scope = table.current_scope();
        
        let child_scope = table.enter_scope(ScopeKind::Function, dummy_span());
        assert_ne!(child_scope, root_scope);
        assert_eq!(table.current_scope(), child_scope);
        
        table.exit_scope().unwrap();
        assert_eq!(table.current_scope(), root_scope);
    }
} 