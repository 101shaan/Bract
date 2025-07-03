//! Main Semantic Analyzer for Prism
//!
//! This module coordinates the complete semantic analysis pipeline including:
//! - Symbol table construction
//! - Type checking and inference
//! - Error collection and reporting
//! - Analysis result aggregation

use crate::ast::{Module, Expr, Type, Span, InternedString};
use crate::semantic::symbols::{SymbolTable, SymbolTableBuilder, SymbolError};
use crate::semantic::types::{TypeChecker, TypeError};
use std::collections::HashMap;

/// Result of semantic analysis
#[derive(Debug)]
pub struct AnalysisResult {
    /// Symbol table with all resolved symbols
    pub symbol_table: SymbolTable,
    /// Type information for expressions
    pub expression_types: HashMap<*const Expr, Type>,
    /// All semantic errors found
    pub errors: Vec<SemanticError>,
    /// Warnings generated during analysis
    pub warnings: Vec<SemanticWarning>,
    /// Analysis statistics
    pub stats: AnalysisStats,
}

/// Semantic errors that can occur during analysis
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    /// Symbol-related errors
    Symbol(SymbolError),
    /// Type-related errors
    Type(TypeError),
    /// Semantic rule violations
    SemanticViolation {
        message: String,
        span: Span,
        suggestion: Option<String>,
    },
}

impl From<SymbolError> for SemanticError {
    fn from(error: SymbolError) -> Self {
        SemanticError::Symbol(error)
    }
}

impl From<TypeError> for SemanticError {
    fn from(error: TypeError) -> Self {
        SemanticError::Type(error)
    }
}

/// Semantic warnings
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticWarning {
    /// Unused symbol
    UnusedSymbol {
        name: InternedString,
        span: Span,
        kind: String,
    },
    /// Unreachable code
    UnreachableCode {
        span: Span,
        reason: String,
    },
    /// Deprecated usage
    Deprecated {
        item: String,
        span: Span,
        replacement: Option<String>,
    },
    /// Performance warning
    Performance {
        message: String,
        span: Span,
        suggestion: String,
    },
}

/// Analysis statistics
#[derive(Debug, Clone, Default)]
pub struct AnalysisStats {
    /// Number of symbols analyzed
    pub symbols_analyzed: usize,
    /// Number of expressions type-checked
    pub expressions_checked: usize,
    /// Number of scopes created
    pub scopes_created: usize,
    /// Analysis time in milliseconds
    pub analysis_time_ms: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: usize,
}

/// Main semantic analyzer
pub struct SemanticAnalyzer {
    /// Configuration options
    config: AnalyzerConfig,
    /// Collected errors
    errors: Vec<SemanticError>,
    /// Collected warnings
    warnings: Vec<SemanticWarning>,
    /// Analysis statistics
    stats: AnalysisStats,
}

/// Configuration for semantic analysis
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// Enable strict type checking
    pub strict_types: bool,
    /// Enable unused symbol warnings
    pub warn_unused: bool,
    /// Enable performance warnings
    pub warn_performance: bool,
    /// Maximum number of errors before stopping
    pub max_errors: Option<usize>,
    /// Enable experimental features
    pub experimental: bool,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            strict_types: true,
            warn_unused: true,
            warn_performance: false,
            max_errors: Some(100),
            experimental: false,
        }
    }
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer with default configuration
    pub fn new() -> Self {
        Self::with_config(AnalyzerConfig::default())
    }
    
    /// Create a new semantic analyzer with custom configuration
    pub fn with_config(config: AnalyzerConfig) -> Self {
        Self {
            config,
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: AnalysisStats::default(),
        }
    }
    
    /// Perform complete semantic analysis on a module
    pub fn analyze(&mut self, module: &Module) -> AnalysisResult {
        let start_time = std::time::Instant::now();
        
        // Phase 1: Build symbol table
        let (symbol_table, symbol_errors) = self.build_symbol_table(module);
        
        // Collect symbol errors
        for error in symbol_errors {
            self.add_error(SemanticError::Symbol(error));
        }
        
        // Phase 2: Type checking (only if no critical symbol errors)
        let mut expression_types = HashMap::new();
        if !self.has_critical_errors() {
            let type_result = self.perform_type_checking(module, &symbol_table);
            match type_result {
                Ok(types) => expression_types = types,
                Err(type_errors) => {
                    for error in type_errors {
                        self.add_error(SemanticError::Type(error));
                    }
                }
            }
        }
        
        // Phase 3: Additional semantic checks
        self.perform_semantic_checks(module, &symbol_table);
        
        // Phase 4: Generate warnings
        if self.config.warn_unused {
            self.generate_unused_warnings(&symbol_table);
        }
        
        // Update statistics
        self.stats.analysis_time_ms = start_time.elapsed().as_millis() as u64;
        self.stats.symbols_analyzed = symbol_table.current_scope_symbols().len();
        
        AnalysisResult {
            symbol_table,
            expression_types,
            errors: std::mem::take(&mut self.errors),
            warnings: std::mem::take(&mut self.warnings),
            stats: self.stats.clone(),
        }
    }
    
    /// Build symbol table from AST
    fn build_symbol_table(&mut self, module: &Module) -> (SymbolTable, Vec<SymbolError>) {
        let builder = SymbolTableBuilder::new();
        let (symbol_table, errors) = builder.build(module);
        
        self.stats.scopes_created = 1; // At least the root scope
        
        (symbol_table, errors)
    }
    
    /// Perform type checking
    fn perform_type_checking(
        &mut self,
        module: &Module,
        symbol_table: &SymbolTable,
    ) -> Result<HashMap<*const Expr, Type>, Vec<TypeError>> {
        let mut type_checker = TypeChecker::new(symbol_table.clone());
        
        match type_checker.check_module(module) {
            Ok(()) => {
                let expression_types = HashMap::new();
                
                // Extract expression types from type checker
                // Note: This would require additional API in TypeChecker to extract all types
                // For now, return empty map
                
                self.stats.expressions_checked = expression_types.len();
                Ok(expression_types)
            }
            Err(_) => {
                let errors = type_checker.errors().to_vec();
                Err(errors)
            }
        }
    }
    
    /// Perform additional semantic checks
    fn perform_semantic_checks(&mut self, _module: &Module, _symbol_table: &SymbolTable) {
        // TODO: Implement semantic checks
    }
    
    /// Generate warnings for unused symbols
    fn generate_unused_warnings(&mut self, symbol_table: &SymbolTable) {
        for symbol in symbol_table.unused_symbols() {
            self.add_warning(SemanticWarning::UnusedSymbol {
                name: symbol.name,
                span: symbol.span,
                kind: format!("{:?}", symbol.kind),
            });
        }
    }
    
    /// Add a semantic error
    fn add_error(&mut self, error: SemanticError) {
        if let Some(max_errors) = self.config.max_errors {
            if self.errors.len() >= max_errors {
                return; // Stop collecting errors
            }
        }
        
        self.errors.push(error);
    }
    
    /// Add a semantic warning
    fn add_warning(&mut self, warning: SemanticWarning) {
        self.warnings.push(warning);
    }
    
    /// Check if there are critical errors that prevent further analysis
    fn has_critical_errors(&self) -> bool {
        self.errors.iter().any(|error| match error {
            SemanticError::Symbol(SymbolError::CircularDependency { .. }) => true,
            SemanticError::Symbol(SymbolError::UndefinedSymbol { .. }) => true,
            _ => false,
        })
    }
    
    /// Get analysis statistics
    pub fn stats(&self) -> &AnalysisStats {
        &self.stats
    }
    
    /// Get configuration
    pub fn config(&self) -> &AnalyzerConfig {
        &self.config
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
    fn test_analyzer_creation() {
        let analyzer = SemanticAnalyzer::new();
        assert!(analyzer.config.strict_types);
        assert!(analyzer.config.warn_unused);
        assert_eq!(analyzer.errors.len(), 0);
        assert_eq!(analyzer.warnings.len(), 0);
    }
    
    #[test]
    fn test_analyzer_config() {
        let config = AnalyzerConfig {
            strict_types: false,
            warn_unused: false,
            warn_performance: true,
            max_errors: Some(50),
            experimental: true,
        };
        
        let analyzer = SemanticAnalyzer::with_config(config.clone());
        assert_eq!(analyzer.config.strict_types, config.strict_types);
        assert_eq!(analyzer.config.max_errors, config.max_errors);
    }
} 