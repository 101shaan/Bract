//! Cranelift Compilation Context
//!
//! This module manages the compilation state for Cranelift code generation,
//! including variable tracking, function management, and type mapping.

use super::*;
use cranelift::prelude::{types as ctypes, Type, Value};
use cranelift_module::FuncId;
use std::collections::HashMap;

/// Cranelift compilation context
pub struct CraneliftContext {
    /// Variable tracking (variable name -> Cranelift value)
    variables: HashMap<String, Value>,
    /// Function ID mapping (function name -> Cranelift function ID)
    functions: HashMap<String, FuncId>,
    /// Function scope stack
    function_scopes: Vec<String>,
    /// Type mapping cache
    type_cache: HashMap<String, Type>,
    /// Current function has return statement
    has_return: bool,
}

impl CraneliftContext {
    /// Create a new Cranelift context
    pub fn new() -> Self {
        let mut context = Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            function_scopes: Vec::new(),
            type_cache: HashMap::new(),
            has_return: false,
        };
        
        // Initialize standard type mappings
        context.init_type_mappings();
        
        context
    }
    
    /// Initialize standard type mappings
    fn init_type_mappings(&mut self) {
        self.type_cache.insert("i8".to_string(), ctypes::I8);
        self.type_cache.insert("i16".to_string(), ctypes::I16);
        self.type_cache.insert("i32".to_string(), ctypes::I32);
        self.type_cache.insert("i64".to_string(), ctypes::I64);
        self.type_cache.insert("u8".to_string(), ctypes::I8);
        self.type_cache.insert("u16".to_string(), ctypes::I16);
        self.type_cache.insert("u32".to_string(), ctypes::I32);
        self.type_cache.insert("u64".to_string(), ctypes::I64);
        self.type_cache.insert("f32".to_string(), ctypes::F32);
        self.type_cache.insert("f64".to_string(), ctypes::F64);
        self.type_cache.insert("bool".to_string(), ctypes::I8);
    }
    
    /// Map a Bract type to a Cranelift type
    pub fn map_type(&self, bract_type: &str) -> CodegenResult<Type> {
        if let Some(&cranelift_type) = self.type_cache.get(bract_type) {
            Ok(cranelift_type)
        } else {
            Err(CodegenError::UnsupportedFeature(
                format!("Type not supported: {}", bract_type)
            ))
        }
    }
    
    /// Define a variable in the current scope
    pub fn define_variable(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }
    
    /// Get a variable value
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.variables.get(name).copied()
    }
    
    /// Register a function
    pub fn register_function(&mut self, name: &str, func_id: FuncId) {
        self.functions.insert(name.to_string(), func_id);
    }
    
    /// Get a function ID
    pub fn get_function_id(&self, name: &str) -> Option<FuncId> {
        self.functions.get(name).copied()
    }
    
    /// Push a function scope
    pub fn push_function_scope(&mut self, name: &str) {
        self.function_scopes.push(name.to_string());
        self.has_return = false;
    }
    
    /// Pop a function scope
    pub fn pop_function_scope(&mut self) {
        self.function_scopes.pop();
        self.has_return = false;
    }
    
    /// Get the current function name
    pub fn current_function(&self) -> Option<&str> {
        self.function_scopes.last().map(|s| s.as_str())
    }
    
    /// Set whether current function has return
    pub fn set_has_return(&mut self, has_return: bool) {
        self.has_return = has_return;
    }
    
    /// Check if current function has return
    pub fn has_return(&self) -> bool {
        self.has_return
    }
    
    /// Clear all variables (for new scope)
    pub fn clear_variables(&mut self) {
        self.variables.clear();
    }
    
    /// Get function count
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
    
    /// Get variable count
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Get all registered functions
    pub fn get_all_functions(&self) -> &HashMap<String, FuncId> {
        &self.functions
    }
} 