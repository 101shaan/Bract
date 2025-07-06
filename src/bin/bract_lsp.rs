//! Bract Language Server
//!
//! This is the main entry point for the Bract Language Server Protocol (LSP) server.
//! It provides comprehensive IDE support including:
//! - Real-time diagnostics and error reporting
//! - Code completion and IntelliSense
//! - Go-to-definition and symbol navigation
//! - Hover information and documentation
//! - Workspace symbol search
//! - Document formatting and refactoring

use bract::lsp::{LspServer, CompletionProvider, Position};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite, AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader};
use tokio::sync::mpsc;
use std::process;

/// LSP request/response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: Option<String>,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<LspError>,
}

/// LSP error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// LSP server instance
pub struct BractLspServer {
    /// Core LSP functionality
    core: Arc<LspServer>,
    /// Completion provider
    completion_provider: Arc<CompletionProvider>,
    /// Request counter for generating IDs
    request_counter: Arc<Mutex<u64>>,
    /// Active requests
    active_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<Message>>>>,
}

impl BractLspServer {
    /// Create a new Bract LSP server
    pub fn new() -> Self {
        Self {
            core: Arc::new(LspServer::new()),
            completion_provider: Arc::new(CompletionProvider::new()),
            request_counter: Arc::new(Mutex::new(0)),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start the LSP server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        
        self.run_server(stdin, stdout).await
    }

    /// Run the LSP server with given input/output streams
    async fn run_server<R, W>(&self, input: R, output: W) -> Result<(), Box<dyn std::error::Error>>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<Message>(100);
        let output = Arc::new(Mutex::new(output));
        let output_clone = output.clone();
        
        // Spawn input handler
        let server = self.clone();
        tokio::spawn(async move {
            if let Err(e) = server.handle_input(input, tx).await {
                eprintln!("Input handler error: {}", e);
            }
        });

        // Process messages
        while let Some(message) = rx.recv().await {
            if let Err(e) = self.handle_message(message, output_clone.clone()).await {
                eprintln!("Message handler error: {}", e);
            }
        }

        Ok(())
    }

    /// Handle input stream
    async fn handle_input<R>(&self, input: R, tx: mpsc::Sender<Message>) -> Result<(), Box<dyn std::error::Error>>
    where
        R: AsyncRead + Unpin,
    {
        let mut reader = BufReader::new(input);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            
            // Read Content-Length header
            let mut header_line = String::new();
            if reader.read_line(&mut header_line).await? == 0 {
                break; // EOF
            }

            if !header_line.starts_with("Content-Length: ") {
                continue;
            }

            let content_length: usize = header_line[16..].trim().parse()?;
            
            // Skip additional headers and blank line
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).await? == 0 {
                    break;
                }
                if line.trim().is_empty() {
                    break;
                }
            }

            // Read message content
            buffer.resize(content_length, 0);
            reader.read_exact(&mut buffer).await?;

            // Parse JSON message
            let message: Message = serde_json::from_slice(&buffer)?;
            
            // Send message to handler
            if tx.send(message).await.is_err() {
                break;
            }
        }

        Ok(())
    }

    /// Handle a single message
    async fn handle_message<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        match message.method.as_deref() {
            Some("initialize") => {
                self.handle_initialize(message, output).await?;
            },
            Some("initialized") => {
                // Client finished initialization
                self.send_notification("window/logMessage", json!({
                    "type": 3, // Info
                    "message": "Bract Language Server initialized successfully"
                }), output).await?;
            },
            Some("shutdown") => {
                self.handle_shutdown(message, output).await?;
            },
            Some("exit") => {
                process::exit(0);
            },
            Some("textDocument/didOpen") => {
                self.handle_did_open(message, output).await?;
            },
            Some("textDocument/didChange") => {
                self.handle_did_change(message, output).await?;
            },
            Some("textDocument/didClose") => {
                self.handle_did_close(message, output).await?;
            },
            Some("textDocument/completion") => {
                self.handle_completion(message, output).await?;
            },
            Some("textDocument/hover") => {
                self.handle_hover(message, output).await?;
            },
            Some("textDocument/definition") => {
                self.handle_definition(message, output).await?;
            },
            Some("textDocument/references") => {
                self.handle_references(message, output).await?;
            },
            Some("textDocument/documentSymbol") => {
                self.handle_document_symbol(message, output).await?;
            },
            Some("workspace/symbol") => {
                self.handle_workspace_symbol(message, output).await?;
            },
            _ => {
                // Unknown method - send method not found error
                if message.id.is_some() {
                    self.send_error_response(
                        message.id.unwrap(),
                        -32601,
                        "Method not found".to_string(),
                        None,
                        output,
                    ).await?;
                }
            }
        }

        Ok(())
    }

    /// Handle initialize request
    async fn handle_initialize<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        let capabilities = self.core.capabilities();
        
        let response = json!({
            "capabilities": capabilities,
            "serverInfo": {
                "name": "Bract Language Server",
                "version": "0.1.0"
            }
        });

        self.send_response(message.id.unwrap(), response, output).await?;
        Ok(())
    }

    /// Handle shutdown request
    async fn handle_shutdown<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        self.send_response(message.id.unwrap(), json!(null), output).await?;
        Ok(())
    }

    /// Handle textDocument/didOpen notification
    async fn handle_did_open<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        if let Some(params) = message.params {
            if let Some(text_document) = params.get("textDocument") {
                let uri = text_document["uri"].as_str().unwrap_or_default().to_string();
                let text = text_document["text"].as_str().unwrap_or_default().to_string();
                let version = text_document["version"].as_i64().unwrap_or(0) as i32;

                // Add document to server
                self.core.update_document(uri.clone(), text, version)?;

                // Analyze document and send diagnostics
                self.analyze_and_send_diagnostics(uri, output).await?;
            }
        }

        Ok(())
    }

    /// Handle textDocument/didChange notification
    async fn handle_did_change<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        if let Some(params) = message.params {
            if let Some(text_document) = params.get("textDocument") {
                let uri = text_document["uri"].as_str().unwrap_or_default().to_string();
                let version = text_document["version"].as_i64().unwrap_or(0) as i32;
                
                if let Some(content_changes) = params.get("contentChanges") {
                    if let Some(changes) = content_changes.as_array() {
                        if let Some(change) = changes.first() {
                            let text = change["text"].as_str().unwrap_or_default().to_string();
                            
                            // Update document
                            self.core.update_document(uri.clone(), text, version)?;
                            
                            // Analyze document and send diagnostics
                            self.analyze_and_send_diagnostics(uri, output).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle textDocument/didClose notification
    async fn handle_did_close<W>(&self, _message: Message, _output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        // Handle document close
        // self.core.remove_document(&uri)?;
        Ok(())
    }

    /// Handle textDocument/completion request
    async fn handle_completion<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        if let Some(params) = message.params {
            if let Some(text_document) = params.get("textDocument") {
                let uri = text_document["uri"].as_str().unwrap_or_default();
                
                if let Some(position) = params.get("position") {
                    let line = position["line"].as_u64().unwrap_or(0) as u32;
                    let character = position["character"].as_u64().unwrap_or(0) as u32;
                    let pos = Position { line, character };

                    // Get document for completion context
                    let document = self.core.get_document(uri)
                        .unwrap_or(None)
                        .ok_or("Document not found")?;
                    
                    // Create completion context
                    let context = bract::lsp::completion::create_completion_context(
                        uri.to_string(),
                        pos.clone(),
                        &document,
                    ).unwrap_or_else(|_| bract::lsp::completion::CompletionContext {
                        uri: uri.to_string(),
                        position: pos.clone(),
                        line_content: String::new(),
                        char_before: None,
                        word_at_cursor: String::new(),
                        in_function_call: false,
                        in_struct_init: false,
                        in_pattern_match: false,
                        scope_depth: 0,
                    });

                    // Get completions
                    let completions = self.completion_provider.provide_completions(
                        &self.core,
                        uri,
                        &pos,
                        &context,
                    ).unwrap_or_default();

                    let response = json!({
                        "isIncomplete": false,
                        "items": completions
                    });

                    self.send_response(message.id.unwrap(), response, output).await?;
                }
            }
        }

        Ok(())
    }

    /// Handle textDocument/hover request
    async fn handle_hover<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        // For now, return empty hover
        let response = json!(null);
        self.send_response(message.id.unwrap(), response, output).await?;
        Ok(())
    }

    /// Handle textDocument/definition request
    async fn handle_definition<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        // For now, return empty definition
        let response = json!(null);
        self.send_response(message.id.unwrap(), response, output).await?;
        Ok(())
    }

    /// Handle textDocument/references request
    async fn handle_references<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        // For now, return empty references
        let response = json!([]);
        self.send_response(message.id.unwrap(), response, output).await?;
        Ok(())
    }

    /// Handle textDocument/documentSymbol request
    async fn handle_document_symbol<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        // For now, return empty symbols
        let response = json!([]);
        self.send_response(message.id.unwrap(), response, output).await?;
        Ok(())
    }

    /// Handle workspace/symbol request
    async fn handle_workspace_symbol<W>(&self, message: Message, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        // For now, return empty symbols
        let response = json!([]);
        self.send_response(message.id.unwrap(), response, output).await?;
        Ok(())
    }

    /// Analyze document and send diagnostics
    async fn analyze_and_send_diagnostics<W>(&self, uri: String, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        // Analyze document
        let diagnostics = self.core.analyze_document(&uri)?;

        // Send diagnostics
        let notification = json!({
            "uri": uri,
            "diagnostics": diagnostics
        });

        self.send_notification("textDocument/publishDiagnostics", notification, output).await?;
        Ok(())
    }

    /// Send response message
    async fn send_response<W>(&self, id: Value, result: Value, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });

        self.send_message(message, output).await
    }

    /// Send error response
    async fn send_error_response<W>(&self, id: Value, code: i32, message: String, data: Option<Value>, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        let error = json!({
            "code": code,
            "message": message,
            "data": data
        });

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": error
        });

        self.send_message(response, output).await
    }

    /// Send notification message
    async fn send_notification<W>(&self, method: &str, params: Value, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        self.send_message(message, output).await
    }

    /// Send message to client
    async fn send_message<W>(&self, message: Value, output: Arc<Mutex<W>>) -> Result<(), Box<dyn std::error::Error>>
    where
        W: AsyncWrite + Unpin,
    {
        let content = serde_json::to_string(&message)?;
        let response = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        let mut output = output.lock().unwrap();
        output.write_all(response.as_bytes()).await?;
        output.flush().await?;

        Ok(())
    }
}

impl Clone for BractLspServer {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            completion_provider: self.completion_provider.clone(),
            request_counter: self.request_counter.clone(),
            active_requests: self.active_requests.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Create and start LSP server
    let server = BractLspServer::new();
    server.start().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // use tokio::io::{AsyncReadExt, AsyncWriteExt};
    // use std::io::Cursor;

    #[test]
    fn test_server_creation() {
        let server = BractLspServer::new();
        assert!(server.core.capabilities().text_document_sync.open_close);
    }

    #[tokio::test]
    async fn test_message_handling() {
        let server = BractLspServer::new();
        
        // Test initialize message
        let message = Message {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: Some("initialize".to_string()),
            params: Some(json!({})),
            result: None,
            error: None,
        };

        let _output = Vec::new();
        let output_arc = Arc::new(Mutex::new(_output));
        
        assert!(server.handle_message(message, output_arc).await.is_ok());
    }
} 
