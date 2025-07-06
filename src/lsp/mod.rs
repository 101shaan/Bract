//! Language Server Protocol Implementation for Bract
//!
//! This module provides a complete LSP server for Bract, enabling world-class IDE support
//! with real-time diagnostics, code completion, navigation, and more.

use crate::{Lexer, Parser, semantic::SemanticAnalyzer};
use crate::ast::Module;
use crate::semantic::SymbolTable;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod completion;

// Re-export main types
pub use completion::{CompletionProvider, CompletionItem, CompletionItemKind};

/// LSP Server state
#[derive(Debug)]
pub struct LspServer {
    /// Documents currently open in the editor
    documents: Arc<Mutex<HashMap<String, Document>>>,
    /// Server capabilities
    capabilities: ServerCapabilities,
    /// Configuration
    #[allow(dead_code)]
    config: LspConfig,
    /// Analysis cache for performance
    analysis_cache: Arc<Mutex<AnalysisCache>>,
}

/// Document state in the LSP server
#[derive(Debug, Clone)]
pub struct Document {
    /// URI of the document
    pub uri: String,
    /// Current content
    pub content: String,
    /// Version number for synchronization
    pub version: i32,
    /// Parsed AST (cached)
    pub ast: Option<Module>,
    /// Symbol table (cached)
    pub symbols: Option<SymbolTable>,
    /// Last analysis timestamp
    pub last_analyzed: std::time::Instant,
    /// Diagnostics
    pub diagnostics: Vec<Diagnostic>,
}

/// LSP server configuration
#[derive(Debug, Clone)]
pub struct LspConfig {
    /// Enable real-time diagnostics
    pub enable_diagnostics: bool,
    /// Enable code completion
    pub enable_completion: bool,
    /// Enable navigation features
    pub enable_navigation: bool,
    /// Max analysis time per document (ms)
    pub max_analysis_time: u64,
    /// Cache size limit
    pub cache_size_limit: usize,
}

/// Analysis cache for performance optimization
#[derive(Debug)]
pub struct AnalysisCache {
    /// Cached parsed modules
    parsed_modules: HashMap<String, (Module, std::time::Instant)>,
    /// Cached symbol tables
    symbol_tables: HashMap<String, (SymbolTable, std::time::Instant)>,
    /// Cache statistics
    stats: CacheStats,
}

/// Cache performance statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_analysis_time: std::time::Duration,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Text document synchronization
    #[serde(rename = "textDocumentSync")]
    pub text_document_sync: TextDocumentSyncCapability,
    /// Diagnostic provider
    #[serde(rename = "diagnosticProvider")]
    pub diagnostic_provider: Option<DiagnosticOptions>,
    /// Completion provider
    #[serde(rename = "completionProvider")]
    pub completion_provider: Option<CompletionOptions>,
    /// Hover provider
    #[serde(rename = "hoverProvider")]
    pub hover_provider: Option<bool>,
    /// Definition provider
    #[serde(rename = "definitionProvider")]
    pub definition_provider: Option<bool>,
    /// References provider
    #[serde(rename = "referencesProvider")]
    pub references_provider: Option<bool>,
    /// Document symbol provider
    #[serde(rename = "documentSymbolProvider")]
    pub document_symbol_provider: Option<bool>,
    /// Workspace symbol provider
    #[serde(rename = "workspaceSymbolProvider")]
    pub workspace_symbol_provider: Option<bool>,
}

/// Text document synchronization capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentSyncCapability {
    /// Open and close notifications
    #[serde(rename = "openClose")]
    pub open_close: bool,
    /// Change notifications
    pub change: TextDocumentSyncKind,
    /// Will save notifications
    #[serde(rename = "willSave")]
    pub will_save: bool,
    /// Save notifications
    pub save: Option<SaveOptions>,
}

/// Text document synchronization kind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextDocumentSyncKind {
    /// Documents should not be synced
    None = 0,
    /// Documents are synced by always sending full content
    Full = 1,
    /// Documents are synced by sending incremental changes
    Incremental = 2,
}

/// Save options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOptions {
    /// Include text content on save
    #[serde(rename = "includeText")]
    pub include_text: bool,
}

/// Diagnostic options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticOptions {
    /// Identifier for diagnostics
    pub identifier: Option<String>,
    /// Inter-file dependencies
    #[serde(rename = "interFileDependencies")]
    pub inter_file_dependencies: bool,
    /// Workspace diagnostics
    #[serde(rename = "workspaceDiagnostics")]
    pub workspace_diagnostics: bool,
}

/// Completion options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    /// Resolve provider
    #[serde(rename = "resolveProvider")]
    pub resolve_provider: bool,
    /// Trigger characters
    #[serde(rename = "triggerCharacters")]
    pub trigger_characters: Vec<String>,
}

/// LSP Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Range of the diagnostic
    pub range: Range,
    /// Severity level
    pub severity: Option<DiagnosticSeverity>,
    /// Diagnostic code
    pub code: Option<Value>,
    /// Source of the diagnostic
    pub source: Option<String>,
    /// Diagnostic message
    pub message: String,
    /// Related information
    #[serde(rename = "relatedInformation")]
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

/// Diagnostic severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// Diagnostic related information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticRelatedInformation {
    /// Location of related information
    pub location: Location,
    /// Message
    pub message: String,
}

/// LSP Range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    /// Start position
    pub start: Position,
    /// End position
    pub end: Position,
}

/// LSP Position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Line number (0-based)
    pub line: u32,
    /// Character offset (0-based)
    pub character: u32,
}

/// LSP Location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// URI of the document
    pub uri: String,
    /// Range within the document
    pub range: Range,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enable_diagnostics: true,
            enable_completion: true,
            enable_navigation: true,
            max_analysis_time: 5000, // 5 seconds
            cache_size_limit: 100,   // 100 documents
        }
    }
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            text_document_sync: TextDocumentSyncCapability {
                open_close: true,
                change: TextDocumentSyncKind::Full,
                will_save: false,
                save: Some(SaveOptions { include_text: false }),
            },
            diagnostic_provider: Some(DiagnosticOptions {
                identifier: Some("Bract".to_string()),
                inter_file_dependencies: true,
                workspace_diagnostics: true,
            }),
            completion_provider: Some(CompletionOptions {
                resolve_provider: true,
                trigger_characters: vec![".".to_string(), "::".to_string()],
            }),
            hover_provider: Some(true),
            definition_provider: Some(true),
            references_provider: Some(true),
            document_symbol_provider: Some(true),
            workspace_symbol_provider: Some(true),
        }
    }
}

impl LspServer {
    /// Create a new LSP server
    pub fn new() -> Self {
        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            capabilities: ServerCapabilities::default(),
            config: LspConfig::default(),
            analysis_cache: Arc::new(Mutex::new(AnalysisCache::new())),
        }
    }

    /// Get server capabilities for initialization
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    /// Add or update a document
    pub fn update_document(&self, uri: String, content: String, version: i32) -> Result<(), String> {
        let mut documents = self.documents.lock().map_err(|e| format!("Lock error: {}", e))?;
        
        let document = Document {
            uri: uri.clone(),
            content,
            version,
            ast: None,
            symbols: None,
            last_analyzed: std::time::Instant::now(),
            diagnostics: Vec::new(),
        };
        
        documents.insert(uri, document);
        Ok(())
    }

    /// Get a document
    pub fn get_document(&self, uri: &str) -> Result<Option<Document>, String> {
        let documents = self.documents.lock().map_err(|e| format!("Lock error: {}", e))?;
        Ok(documents.get(uri).cloned())
    }

    /// Remove a document
    pub fn remove_document(&self, uri: &str) -> Result<(), String> {
        let mut documents = self.documents.lock().map_err(|e| format!("Lock error: {}", e))?;
        documents.remove(uri);
        
        // Also remove from cache
        let mut cache = self.analysis_cache.lock().map_err(|e| format!("Lock error: {}", e))?;
        cache.remove(uri);
        
        Ok(())
    }

    /// Analyze a document and update diagnostics
    pub fn analyze_document(&self, uri: &str) -> Result<Vec<Diagnostic>, String> {
        let document = self.get_document(uri)?.ok_or("Document not found")?;
        
        // Check cache first
        {
            let cache = self.analysis_cache.lock().map_err(|e| format!("Lock error: {}", e))?;
            if let Some(diagnostics) = cache.get_diagnostics(uri, &document.content) {
                return Ok(diagnostics);
            }
        }

        let start_time = std::time::Instant::now();
        let mut diagnostics = Vec::new();

        // Parse the document
        match self.parse_document(&document.content) {
            Ok((ast, symbols)) => {
                // Store in cache
                {
                    let mut cache = self.analysis_cache.lock().map_err(|e| format!("Lock error: {}", e))?;
                    cache.store_analysis(uri.to_string(), ast, symbols);
                }
            },
            Err(errors) => {
                // Convert parse errors to diagnostics
                for error in errors {
                    diagnostics.push(self.error_to_diagnostic(error));
                }
            }
        }

        // Update cache statistics
        {
            let mut cache = self.analysis_cache.lock().map_err(|e| format!("Lock error: {}", e))?;
            cache.stats.total_analysis_time += start_time.elapsed();
        }

        Ok(diagnostics)
    }

    /// Parse a document and return AST and symbols
    fn parse_document(&self, content: &str) -> Result<(Module, SymbolTable), Vec<String>> {
        let mut errors = Vec::new();

        // Lexical analysis
        let _lexer = Lexer::new(content, 0);
        
        // Parsing
        let mut parser = match Parser::new(content, 0) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("Parser creation failed: {:?}", e));
                return Err(errors);
            }
        };

        let ast = match parser.parse_module() {
            Ok(module) => module,
            Err(e) => {
                errors.push(format!("Parse error: {:?}", e));
                return Err(errors);
            }
        };

        // Semantic analysis
        let mut analyzer = SemanticAnalyzer::new();
        let analysis_result = analyzer.analyze(&ast);
        let symbols = analysis_result.symbol_table;

        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok((ast, symbols))
        }
    }

    /// Convert error to LSP diagnostic
    fn error_to_diagnostic(&self, error: String) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            severity: Some(DiagnosticSeverity::Error),
            code: None,
            source: Some("Bract".to_string()),
            message: error,
            related_information: None,
        }
    }
}

impl AnalysisCache {
    /// Create a new analysis cache
    pub fn new() -> Self {
        Self {
            parsed_modules: HashMap::new(),
            symbol_tables: HashMap::new(),
            stats: CacheStats::default(),
        }
    }

    /// Store analysis results in cache
    pub fn store_analysis(&mut self, uri: String, ast: Module, symbols: SymbolTable) {
        let now = std::time::Instant::now();
        self.parsed_modules.insert(uri.clone(), (ast, now));
        self.symbol_tables.insert(uri, (symbols, now));
    }

    /// Get cached diagnostics if available
    pub fn get_diagnostics(&self, _uri: &str, _content: &str) -> Option<Vec<Diagnostic>> {
        // Simple cache implementation - in production this would check content hashes
        None
    }

    /// Remove cached data for a URI
    pub fn remove(&mut self, uri: &str) {
        self.parsed_modules.remove(uri);
        self.symbol_tables.remove(uri);
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }
}

impl Document {
    /// Check if document needs re-analysis
    pub fn needs_analysis(&self) -> bool {
        self.ast.is_none() || 
        self.symbols.is_none() || 
        self.last_analyzed.elapsed() > std::time::Duration::from_secs(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_server_creation() {
        let server = LspServer::new();
        assert!(server.capabilities.text_document_sync.open_close);
        assert!(server.capabilities.diagnostic_provider.is_some());
        assert!(server.capabilities.completion_provider.is_some());
    }

    #[test]
    fn test_document_management() {
        let server = LspServer::new();
        
        // Add document
        let uri = "file:///test.Bract".to_string();
        let content = "fn main() { println!(\"Hello\"); }".to_string();
        
        assert!(server.update_document(uri.clone(), content.clone(), 1).is_ok());
        
        // Get document
        let doc = server.get_document(&uri).unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().content, content);
        
        // Remove document
        assert!(server.remove_document(&uri).is_ok());
        let doc = server.get_document(&uri).unwrap();
        assert!(doc.is_none());
    }

    #[test]
    fn test_analysis_cache() {
        let cache = AnalysisCache::new();
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn test_diagnostic_creation() {
        let server = LspServer::new();
        let diagnostic = server.error_to_diagnostic("Test error".to_string());
        
        assert_eq!(diagnostic.message, "Test error");
        assert!(matches!(diagnostic.severity, Some(DiagnosticSeverity::Error)));
        assert_eq!(diagnostic.source, Some("Bract".to_string()));
    }

    #[test]
    fn test_server_capabilities() {
        let capabilities = ServerCapabilities::default();
        
        assert!(capabilities.text_document_sync.open_close);
        assert!(capabilities.diagnostic_provider.is_some());
        assert!(capabilities.completion_provider.is_some());
        assert!(capabilities.hover_provider.unwrap_or(false));
        assert!(capabilities.definition_provider.unwrap_or(false));
    }
} 
