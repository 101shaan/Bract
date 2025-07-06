//! Semantic Analysis Module for Bract
//!
//! This module provides comprehensive semantic analysis capabilities including:
//! - Symbol table management and scope resolution
//! - Type checking and type inference
//! - Name resolution and dependency tracking
//! - Semantic error reporting

pub mod symbols;
pub mod types;
pub mod analyzer;

pub use symbols::{SymbolTable, SymbolTableBuilder, Symbol, SymbolKind, SymbolError};
pub use types::{TypeChecker, TypeSystem, TypeError};
pub use analyzer::{SemanticAnalyzer, AnalysisResult}; 
