//! Semantic Analysis Module for Bract
//!
//! This module provides comprehensive semantic analysis capabilities including:
//! - Symbol table management and scope resolution
//! - Type checking and type inference
//! - Name resolution and dependency tracking
//! - Semantic error reporting

pub mod analyzer;
pub mod symbols;
pub mod types;
pub mod ownership;
pub mod escape_analysis;

// Re-export key types for convenience
pub use analyzer::{SemanticAnalyzer, SemanticError, SemanticWarning};
pub use symbols::{SymbolTable, SymbolTableBuilder, Symbol, SymbolKind, Scope};
pub use types::{TypeSystem, TypeChecker, TypeError, InferenceContext, OwnershipTracker};
pub use ownership::{OwnershipAnalyzer, OwnershipError, BorrowInfo, VariableState};
pub use escape_analysis::{EscapeAnalyzer, EscapeError, ValueFlow, EscapeContext}; 
