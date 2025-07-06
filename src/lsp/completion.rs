//! Code Completion Support for Bract LSP
//!
//! This module provides intelligent code completion based on semantic analysis,
//! context-aware suggestions, and performance optimization.

use crate::semantic::{SymbolTable, SymbolKind};
use super::{Position, Range, LspServer, Document};
use serde::{Deserialize, Serialize};

/// Completion item kinds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompletionItemKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Unit = 11,
    Value = 12,
    Enum = 13,
    Keyword = 14,
    Snippet = 15,
    Color = 16,
    File = 17,
    Reference = 18,
    Folder = 19,
    EnumMember = 20,
    Constant = 21,
    Struct = 22,
    Event = 23,
    Operator = 24,
    TypeParameter = 25,
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    /// The label of the completion item
    pub label: String,
    /// The kind of the completion item
    pub kind: Option<CompletionItemKind>,
    /// Additional text edits
    #[serde(rename = "additionalTextEdits")]
    pub additional_text_edits: Option<Vec<TextEdit>>,
    /// Detail information
    pub detail: Option<String>,
    /// Documentation
    pub documentation: Option<String>,
    /// Deprecated flag
    pub deprecated: Option<bool>,
    /// Text to insert
    #[serde(rename = "insertText")]
    pub insert_text: Option<String>,
    /// Insert text format
    #[serde(rename = "insertTextFormat")]
    pub insert_text_format: Option<InsertTextFormat>,
    /// Filter text for completion
    #[serde(rename = "filterText")]
    pub filter_text: Option<String>,
    /// Sort text for completion
    #[serde(rename = "sortText")]
    pub sort_text: Option<String>,
    /// Preselect this item
    pub preselect: Option<bool>,
}

/// Text edit for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// Range to replace
    pub range: Range,
    /// New text
    #[serde(rename = "newText")]
    pub new_text: String,
}

/// Insert text format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsertTextFormat {
    PlainText = 1,
    Snippet = 2,
}

/// Completion context
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// Current document URI
    pub uri: String,
    /// Current position
    pub position: Position,
    /// Current line content
    pub line_content: String,
    /// Character before cursor
    pub char_before: Option<char>,
    /// Word at cursor
    pub word_at_cursor: String,
    /// Is in function call
    pub in_function_call: bool,
    /// Is in struct initialization
    pub in_struct_init: bool,
    /// Is in pattern matching
    pub in_pattern_match: bool,
    /// Current scope depth
    pub scope_depth: usize,
}

/// Completion provider
pub struct CompletionProvider {
    /// Keyword completions
    keywords: Vec<CompletionItem>,
    /// Built-in type completions
    builtin_types: Vec<CompletionItem>,
    /// Snippet completions
    snippets: Vec<CompletionItem>,
}

impl CompletionProvider {
    /// Create a new completion provider
    pub fn new() -> Self {
        Self {
            keywords: Self::create_keyword_completions(),
            builtin_types: Self::create_builtin_type_completions(),
            snippets: Self::create_snippet_completions(),
        }
    }

    /// Provide completions for a position
    pub fn provide_completions(
        &self,
        server: &LspServer,
        uri: &str,
        _position: &Position,
        context: &CompletionContext,
    ) -> Result<Vec<CompletionItem>, String> {
        let mut completions = Vec::new();

        // Get document and symbols
        let document = server.get_document(uri)?
            .ok_or("Document not found")?;

        // Add keyword completions
        if context.word_at_cursor.len() > 0 {
            completions.extend(self.filter_completions(&self.keywords, &context.word_at_cursor));
        }

        // Add built-in type completions
        if self.is_type_context(context) {
            completions.extend(self.filter_completions(&self.builtin_types, &context.word_at_cursor));
        }

        // Add symbol completions from current scope
        if let Some(symbols) = &document.symbols {
            completions.extend(self.get_symbol_completions(symbols, context)?);
        }

        // Add snippet completions
        if context.word_at_cursor.len() > 2 {
            completions.extend(self.filter_completions(&self.snippets, &context.word_at_cursor));
        }

        // Add method completions for dot notation
        if context.char_before == Some('.') {
            completions.extend(self.get_method_completions(context)?);
        }

        // Sort completions by relevance
        completions.sort_by(|a, b| {
            let a_score = self.calculate_relevance_score(a, context);
            let b_score = self.calculate_relevance_score(b, context);
            b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(completions)
    }

    /// Create keyword completions
    fn create_keyword_completions() -> Vec<CompletionItem> {
        let keywords = vec![
            ("fn", "Function declaration", "fn ${1:function_name}(${2:parameters}) {\n    ${3:body}\n}"),
            ("let", "Variable declaration", "let ${1:variable_name} = ${2:value};"),
            ("mut", "Mutable variable", "mut ${1:variable_name}"),
            ("struct", "Struct definition", "struct ${1:StructName} {\n    ${2:field}: ${3:Type},\n}"),
            ("enum", "Enum definition", "enum ${1:EnumName} {\n    ${2:Variant},\n}"),
            ("impl", "Implementation block", "impl ${1:Type} {\n    ${2:methods}\n}"),
            ("match", "Pattern matching", "match ${1:expression} {\n    ${2:pattern} => ${3:result},\n}"),
            ("if", "If statement", "if ${1:condition} {\n    ${2:body}\n}"),
            ("else", "Else clause", "else {\n    ${1:body}\n}"),
            ("while", "While loop", "while ${1:condition} {\n    ${2:body}\n}"),
            ("for", "For loop", "for ${1:item} in ${2:iterator} {\n    ${3:body}\n}"),
            ("loop", "Infinite loop", "loop {\n    ${1:body}\n}"),
            ("break", "Break statement", "break"),
            ("continue", "Continue statement", "continue"),
            ("return", "Return statement", "return ${1:value}"),
            ("pub", "Public visibility", "pub"),
            ("use", "Use statement", "use ${1:module}"),
            ("mod", "Module declaration", "mod ${1:module_name}"),
            ("const", "Constant declaration", "const ${1:CONSTANT_NAME}: ${2:Type} = ${3:value};"),
            ("type", "Type alias", "type ${1:TypeName} = ${2:Type};"),
            ("extern", "External declaration", "extern"),
            ("async", "Async function", "async fn ${1:function_name}(${2:parameters}) {\n    ${3:body}\n}"),
            ("await", "Await expression", "await"),
            ("true", "Boolean true", "true"),
            ("false", "Boolean false", "false"),
            ("null", "Null value", "null"),
        ];

        keywords.into_iter().map(|(label, detail, snippet)| {
            CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::Keyword),
                additional_text_edits: None,
                detail: Some(detail.to_string()),
                documentation: None,
                deprecated: None,
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::Snippet),
                filter_text: Some(label.to_string()),
                sort_text: Some(format!("1_{}", label)),
                preselect: None,
            }
        }).collect()
    }

    /// Create built-in type completions
    fn create_builtin_type_completions() -> Vec<CompletionItem> {
        let types = vec![
            ("i8", "8-bit signed integer"),
            ("i16", "16-bit signed integer"),
            ("i32", "32-bit signed integer"),
            ("i64", "64-bit signed integer"),
            ("u8", "8-bit unsigned integer"),
            ("u16", "16-bit unsigned integer"),
            ("u32", "32-bit unsigned integer"),
            ("u64", "64-bit unsigned integer"),
            ("f32", "32-bit floating point"),
            ("f64", "64-bit floating point"),
            ("bool", "Boolean type"),
            ("char", "Character type"),
            ("str", "String slice"),
            ("String", "Owned string"),
            ("Vec", "Vector type"),
            ("Option", "Optional type"),
            ("Result", "Result type"),
            ("Box", "Heap allocated type"),
        ];

        types.into_iter().map(|(label, detail)| {
            CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::Class),
                additional_text_edits: None,
                detail: Some(detail.to_string()),
                documentation: None,
                deprecated: None,
                insert_text: Some(label.to_string()),
                insert_text_format: Some(InsertTextFormat::PlainText),
                filter_text: Some(label.to_string()),
                sort_text: Some(format!("2_{}", label)),
                preselect: None,
            }
        }).collect()
    }

    /// Create snippet completions
    fn create_snippet_completions() -> Vec<CompletionItem> {
        let snippets = vec![
            ("main", "Main function", "fn main() {\n    ${1:println!(\"Hello, world!\");}\n}"),
            ("test", "Test function", "#[test]\nfn ${1:test_name}() {\n    ${2:assert_eq!(1, 1);}\n}"),
            ("derive", "Derive macro", "#[derive(${1:Debug, Clone})]"),
            ("cfg", "Conditional compilation", "#[cfg(${1:feature = \"feature_name\"})]"),
            ("allow", "Allow lint", "#[allow(${1:dead_code})]"),
            ("warn", "Warn lint", "#[warn(${1:unused_variables})]"),
            ("deny", "Deny lint", "#[deny(${1:unused_imports})]"),
            ("println", "Print macro", "println!(\"${1:message}\");"),
            ("eprintln", "Error print macro", "eprintln!(\"${1:error message}\");"),
            ("dbg", "Debug macro", "dbg!(${1:expression});"),
            ("todo", "TODO macro", "todo!(\"${1:implement this}\");"),
            ("unimplemented", "Unimplemented macro", "unimplemented!(\"${1:not yet implemented}\");"),
            ("panic", "Panic macro", "panic!(\"${1:panic message}\");"),
        ];

        snippets.into_iter().map(|(label, detail, snippet)| {
            CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::Snippet),
                additional_text_edits: None,
                detail: Some(detail.to_string()),
                documentation: None,
                deprecated: None,
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::Snippet),
                filter_text: Some(label.to_string()),
                sort_text: Some(format!("3_{}", label)),
                preselect: None,
            }
        }).collect()
    }

    /// Get symbol completions from symbol table
    fn get_symbol_completions(
        &self,
        symbols: &SymbolTable,
        _context: &CompletionContext,
    ) -> Result<Vec<CompletionItem>, String> {
        let mut completions = Vec::new();

        // Get current scope symbols
        let current_symbols = symbols.current_scope_symbols();

        for symbol in current_symbols {
            let (kind, detail) = match &symbol.kind {
                SymbolKind::Variable { is_mutable, type_info } => {
                    let mutability = if *is_mutable { "mut " } else { "" };
                    let type_str = type_info.as_ref()
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|| "unknown".to_string());
                    (CompletionItemKind::Variable, format!("{}variable: {}", mutability, type_str))
                },
                SymbolKind::Function { return_type, is_extern, .. } => {
                    let extern_str = if *is_extern { "extern " } else { "" };
                    let return_str = return_type.as_ref()
                        .map(|t| format!(" -> {:?}", t))
                        .unwrap_or_default();
                    (CompletionItemKind::Function, format!("{}function{}", extern_str, return_str))
                },
                SymbolKind::Type { definition } => {
                    (CompletionItemKind::Class, format!("type: {:?}", definition))
                },
                SymbolKind::Module { is_external } => {
                    let external_str = if *is_external { "external " } else { "" };
                    (CompletionItemKind::Module, format!("{}module", external_str))
                },
                SymbolKind::Constant { type_info, .. } => {
                    (CompletionItemKind::Constant, format!("const: {:?}", type_info))
                },
                SymbolKind::GenericParam { bounds } => {
                    (CompletionItemKind::TypeParameter, format!("generic parameter: {:?}", bounds))
                },
            };

            let symbol_name = format!("symbol_{}", symbol.name.id);
            completions.push(CompletionItem {
                label: symbol_name.clone(),
                kind: Some(kind),
                additional_text_edits: None,
                detail: Some(detail),
                documentation: None,
                deprecated: None,
                insert_text: Some(symbol_name.clone()),
                insert_text_format: Some(InsertTextFormat::PlainText),
                filter_text: Some(symbol_name.clone()),
                sort_text: Some(format!("4_{}", symbol_name)),
                preselect: None,
            });
        }

        Ok(completions)
    }

    /// Get method completions for dot notation
    fn get_method_completions(&self, _context: &CompletionContext) -> Result<Vec<CompletionItem>, String> {
        // This would analyze the type of the expression before the dot
        // and provide appropriate method completions
        let mut completions = Vec::new();

        // Common string methods
        let string_methods = vec![
            ("len", "Get string length", "len()"),
            ("chars", "Get character iterator", "chars()"),
            ("bytes", "Get byte iterator", "bytes()"),
            ("split", "Split string", "split(${1:pattern})"),
            ("replace", "Replace substring", "replace(${1:from}, ${2:to})"),
            ("trim", "Trim whitespace", "trim()"),
            ("to_uppercase", "Convert to uppercase", "to_uppercase()"),
            ("to_lowercase", "Convert to lowercase", "to_lowercase()"),
        ];

        for (label, detail, snippet) in string_methods {
            completions.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::Method),
                additional_text_edits: None,
                detail: Some(detail.to_string()),
                documentation: None,
                deprecated: None,
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::Snippet),
                filter_text: Some(label.to_string()),
                sort_text: Some(format!("5_{}", label)),
                preselect: None,
            });
        }

        Ok(completions)
    }

    /// Filter completions based on prefix
    fn filter_completions(&self, completions: &[CompletionItem], prefix: &str) -> Vec<CompletionItem> {
        if prefix.is_empty() {
            return completions.to_vec();
        }

        let prefix_lower = prefix.to_lowercase();
        completions.iter()
            .filter(|completion| {
                completion.label.to_lowercase().starts_with(&prefix_lower) ||
                completion.filter_text.as_ref()
                    .map_or(false, |text| text.to_lowercase().starts_with(&prefix_lower))
            })
            .cloned()
            .collect()
    }

    /// Check if we're in a type context
    fn is_type_context(&self, context: &CompletionContext) -> bool {
        context.line_content.contains(':') || 
        context.line_content.contains("struct") ||
        context.line_content.contains("enum") ||
        context.line_content.contains("type") ||
        context.line_content.contains("impl")
    }

    /// Calculate relevance score for completion
    fn calculate_relevance_score(&self, completion: &CompletionItem, context: &CompletionContext) -> f64 {
        let mut score = 0.0;

        // Exact match bonus
        if completion.label == context.word_at_cursor {
            score += 10.0;
        }

        // Prefix match bonus
        if completion.label.starts_with(&context.word_at_cursor) {
            score += 5.0;
        }

        // Kind-based scoring
        match completion.kind {
            Some(CompletionItemKind::Keyword) => score += 2.0,
            Some(CompletionItemKind::Function) => score += 3.0,
            Some(CompletionItemKind::Variable) => score += 3.0,
            Some(CompletionItemKind::Snippet) => score += 1.0,
            _ => {}
        }

        // Context-aware scoring
        if context.in_function_call && completion.kind == Some(CompletionItemKind::Function) {
            score += 2.0;
        }

        if context.in_struct_init && completion.kind == Some(CompletionItemKind::Field) {
            score += 2.0;
        }

        score
    }
}

/// Create completion context from position
pub fn create_completion_context(
    uri: String,
    position: Position,
    document: &Document,
) -> Result<CompletionContext, String> {
    let lines: Vec<&str> = document.content.lines().collect();
    let line_index = position.line as usize;
    
    if line_index >= lines.len() {
        return Err("Position out of bounds".to_string());
    }
    
    let line_content = lines[line_index].to_string();
    let char_pos = position.character as usize;
    
    let char_before = if char_pos > 0 && char_pos <= line_content.len() {
        line_content.chars().nth(char_pos - 1)
    } else {
        None
    };
    
    // Extract word at cursor
    let word_at_cursor = extract_word_at_position(&line_content, char_pos);
    
    Ok(CompletionContext {
        uri,
        position,
        line_content,
        char_before,
        word_at_cursor,
        in_function_call: false, // TODO: Implement context analysis
        in_struct_init: false,
        in_pattern_match: false,
        scope_depth: 0,
    })
}

/// Extract word at position
fn extract_word_at_position(line: &str, position: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut start = position;
    let mut end = position;
    
    // Find start of word
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    
    // Find end of word
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }
    
    chars[start..end].iter().collect()
}

/// Check if character is part of a word
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_provider_creation() {
        let provider = CompletionProvider::new();
        assert!(!provider.keywords.is_empty());
        assert!(!provider.builtin_types.is_empty());
        assert!(!provider.snippets.is_empty());
    }

    #[test]
    fn test_word_extraction() {
        assert_eq!(extract_word_at_position("fn main", 7), "main");
        assert_eq!(extract_word_at_position("let var_name", 8), "var_name");
        assert_eq!(extract_word_at_position("struct Test", 7), "Test");
        assert_eq!(extract_word_at_position("", 0), "");
    }

    #[test]
    fn test_completion_filtering() {
        let provider = CompletionProvider::new();
        let filtered = provider.filter_completions(&provider.keywords, "fn");
        assert!(!filtered.is_empty());
        assert!(filtered.iter().any(|c| c.label == "fn"));
    }

    #[test]
    fn test_relevance_scoring() {
        let provider = CompletionProvider::new();
        let context = CompletionContext {
            uri: "test.Bract".to_string(),
            position: Position { line: 0, character: 0 },
            line_content: "fn".to_string(),
            char_before: None,
            word_at_cursor: "fn".to_string(),
            in_function_call: false,
            in_struct_init: false,
            in_pattern_match: false,
            scope_depth: 0,
        };

        let completion = CompletionItem {
            label: "fn".to_string(),
            kind: Some(CompletionItemKind::Keyword),
            additional_text_edits: None,
            detail: None,
            documentation: None,
            deprecated: None,
            insert_text: None,
            insert_text_format: None,
            filter_text: None,
            sort_text: None,
            preselect: None,
        };

        let score = provider.calculate_relevance_score(&completion, &context);
        assert!(score > 0.0);
    }
} 
