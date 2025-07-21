//! Advanced Parse Error System for Bract Programming Language
//!
//! This module provides IDE-tier error reporting with:
//! - Contextual error messages with spans
//! - Intelligent suggestions based on similarity matching
//! - Recovery hints for common mistakes
//! - Multi-error reporting capabilities
//! - Help text and fix suggestions

use crate::lexer::{TokenType, Position};
use crate::lexer::error::LexerError;
use std::fmt;

/// Advanced parse error with IDE-tier diagnostics
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Unexpected token with contextual information
    UnexpectedToken {
        expected: Vec<ExpectedToken>,
        found: TokenType,
        position: Position,
        context: ParseContext,
        suggestions: Vec<Suggestion>,
        help: Option<String>,
    },
    
    /// Unexpected end of file with recovery information
    UnexpectedEof {
        expected: Vec<ExpectedToken>,
        position: Position,
        context: ParseContext,
        unclosed_delimiters: Vec<UnclosedDelimiter>,
        suggestions: Vec<Suggestion>,
    },
    
    /// Invalid syntax with detailed analysis
    InvalidSyntax {
        message: String,
        position: Position,
        context: ParseContext,
        suggestions: Vec<Suggestion>,
        help: Option<String>,
        related_errors: Vec<RelatedError>,
    },
    
    /// Missing delimiter (unclosed parentheses, braces, etc.)
    MissingDelimiter {
        delimiter: TokenType,
        open_position: Position,
        expected_close_position: Position,
        context: ParseContext,
        suggestion: String,
    },
    
    /// Mismatched delimiters
    MismatchedDelimiter {
        expected: TokenType,
        found: TokenType,
        expected_position: Position,
        found_position: Position,
        suggestion: String,
    },
    
    /// Invalid identifier
    InvalidIdentifier {
        found: String,
        position: Position,
        reason: InvalidIdentifierReason,
        suggestions: Vec<String>,
    },
    
    /// Type annotation error
    TypeAnnotationError {
        message: String,
        position: Position,
        expected_type_context: String,
        suggestions: Vec<String>,
    },
    
    /// Pattern matching error
    PatternError {
        message: String,
        position: Position,
        context: PatternContext,
        suggestions: Vec<Suggestion>,
    },
    
    /// Expression context error
    ExpressionError {
        message: String,
        position: Position,
        context: ExpressionContext,
        suggestions: Vec<Suggestion>,
        help: Option<String>,
    },
    
    /// Statement context error
    StatementError {
        message: String,
        position: Position,
        context: StatementContext,
        suggestions: Vec<Suggestion>,
    },
    
    /// Memory strategy annotation error
    MemoryAnnotationError {
        message: String,
        position: Position,
        found_annotation: String,
        valid_annotations: Vec<String>,
        suggestions: Vec<String>,
    },
    
    /// Internal parser error (should not occur in production)
    InternalError {
        message: String,
        position: Position,
        debug_info: Option<String>,
    },
    
    /// Lexer error with enhanced context
    LexerError {
        error: LexerError,
        suggestions: Vec<Suggestion>,
        help: Option<String>,
    },
    
    /// Multiple related errors
    MultipleErrors {
        primary: Box<ParseError>,
        related: Vec<ParseError>,
        summary: String,
    },
}

/// Expected token information with context
#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedToken {
    pub token: String,
    pub description: String,
    pub example: Option<String>,
}

impl ExpectedToken {
    pub fn new(token: &str, description: &str) -> Self {
        Self {
            token: token.to_string(),
            description: description.to_string(),
            example: None,
        }
    }
    
    pub fn with_example(mut self, example: &str) -> Self {
        self.example = Some(example.to_string());
        self
    }
}

/// Parse context for better error messages
#[derive(Debug, Clone, PartialEq)]
pub enum ParseContext {
    TopLevel,
    FunctionDeclaration,
    FunctionParameters,
    FunctionBody,
    StructDeclaration,
    StructFields,
    EnumDeclaration,
    EnumVariants,
    TypeAnnotation,
    Expression,
    Statement,
    Pattern,
    Block,
    IfCondition,
    IfBody,
    WhileCondition,
    WhileBody,
    ForLoop,
    MatchExpression,
    MatchArm,
    GenericParameters,
    ImplBlock,
    UseDeclaration,
    ModuleDeclaration,
    MemoryAnnotation,
    PerformanceAnnotation,
}

impl fmt::Display for ParseContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseContext::TopLevel => write!(f, "at top level"),
            ParseContext::FunctionDeclaration => write!(f, "in function declaration"),
            ParseContext::FunctionParameters => write!(f, "in function parameters"),
            ParseContext::FunctionBody => write!(f, "in function body"),
            ParseContext::StructDeclaration => write!(f, "in struct declaration"),
            ParseContext::StructFields => write!(f, "in struct fields"),
            ParseContext::EnumDeclaration => write!(f, "in enum declaration"),
            ParseContext::EnumVariants => write!(f, "in enum variants"),
            ParseContext::TypeAnnotation => write!(f, "in type annotation"),
            ParseContext::Expression => write!(f, "in expression"),
            ParseContext::Statement => write!(f, "in statement"),
            ParseContext::Pattern => write!(f, "in pattern"),
            ParseContext::Block => write!(f, "in block"),
            ParseContext::IfCondition => write!(f, "in if condition"),
            ParseContext::IfBody => write!(f, "in if body"),
            ParseContext::WhileCondition => write!(f, "in while condition"),
            ParseContext::WhileBody => write!(f, "in while body"),
            ParseContext::ForLoop => write!(f, "in for loop"),
            ParseContext::MatchExpression => write!(f, "in match expression"),
            ParseContext::MatchArm => write!(f, "in match arm"),
            ParseContext::GenericParameters => write!(f, "in generic parameters"),
            ParseContext::ImplBlock => write!(f, "in impl block"),
            ParseContext::UseDeclaration => write!(f, "in use declaration"),
            ParseContext::ModuleDeclaration => write!(f, "in module declaration"),
            ParseContext::MemoryAnnotation => write!(f, "in memory annotation"),
            ParseContext::PerformanceAnnotation => write!(f, "in performance annotation"),
        }
    }
}

/// Specific pattern contexts
#[derive(Debug, Clone, PartialEq)]
pub enum PatternContext {
    LetBinding,
    FunctionParameter,
    MatchArm,
    ForLoop,
    Destructuring,
}

/// Specific expression contexts
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionContext {
    FunctionCall,
    BinaryOperation,
    UnaryOperation,
    ArrayIndex,
    FieldAccess,
    Assignment,
    IfCondition,
    WhileCondition,
    ReturnValue,
    ArrayLiteral,
    StructInitialization,
}

/// Specific statement contexts
#[derive(Debug, Clone, PartialEq)]
pub enum StatementContext {
    VariableDeclaration,
    Assignment,
    FunctionCall,
    ControlFlow,
    Block,
}

/// Unclosed delimiter information
#[derive(Debug, Clone, PartialEq)]
pub struct UnclosedDelimiter {
    pub delimiter: TokenType,
    pub open_position: Position,
    pub context: String,
}

/// Related error information
#[derive(Debug, Clone, PartialEq)]
pub struct RelatedError {
    pub message: String,
    pub position: Position,
    pub severity: ErrorSeverity,
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Invalid identifier reasons
#[derive(Debug, Clone, PartialEq)]
pub enum InvalidIdentifierReason {
    StartsWithDigit,
    ContainsInvalidCharacters,
    ReservedKeyword,
    TooLong,
    Empty,
}

/// Intelligent suggestion system
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub message: String,
    pub replacement: Option<String>,
    pub position: Position,
    pub confidence: f32, // 0.0 to 1.0
    pub category: SuggestionCategory,
}

impl Suggestion {
    pub fn new(message: &str, position: Position) -> Self {
        Self {
            message: message.to_string(),
            replacement: None,
            position,
            confidence: 0.8,
            category: SuggestionCategory::General,
        }
    }
    
    pub fn with_replacement(mut self, replacement: &str) -> Self {
        self.replacement = Some(replacement.to_string());
        self
    }
    
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_category(mut self, category: SuggestionCategory) -> Self {
        self.category = category;
        self
    }
}

/// Suggestion categories
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionCategory {
    General,
    Syntax,
    Semantics,
    Style,
    Performance,
    Memory,
    Type,
}

/// Enhanced error display with colors and formatting
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found, position, context, suggestions, help } => {
                write!(f, "Unexpected token `{:?}` at {}", found, position)?;
                if !expected.is_empty() {
                    write!(f, "\nExpected one of:")?;
                    for exp in expected.iter().take(5) { // Limit to 5 suggestions
                        write!(f, "\n  - {} ({})", exp.token, exp.description)?;
                        if let Some(example) = &exp.example {
                            write!(f, " [example: {}]", example)?;
                        }
                    }
                }
                write!(f, "\nContext: {}", context)?;
                
                if !suggestions.is_empty() {
                    write!(f, "\nSuggestions:")?;
                    for suggestion in suggestions.iter().take(3) {
                        write!(f, "\n  - {} (confidence: {:.0}%)", 
                               suggestion.message, suggestion.confidence * 100.0)?;
                        if let Some(replacement) = &suggestion.replacement {
                            write!(f, " → `{}`", replacement)?;
                        }
                    }
                }
                
                if let Some(help_text) = help {
                    write!(f, "\nHelp: {}", help_text)?;
                }
                Ok(())
            }
            
            ParseError::UnexpectedEof { expected, position, context, unclosed_delimiters, suggestions } => {
                write!(f, "Unexpected end of file at {}", position)?;
                write!(f, "\nContext: {}", context)?;
                
                if !unclosed_delimiters.is_empty() {
                    write!(f, "\nUnclosed delimiters:")?;
                    for delim in unclosed_delimiters {
                        write!(f, "\n  - `{:?}` opened at {} in {}", 
                               delim.delimiter, delim.open_position, delim.context)?;
                    }
                }
                
                if !expected.is_empty() {
                    write!(f, "\nExpected:")?;
                    for exp in expected.iter().take(3) {
                        write!(f, "\n  - {}", exp.token)?;
                    }
                }
                
                if !suggestions.is_empty() {
                    write!(f, "\nSuggestions:")?;
                    for suggestion in suggestions.iter().take(2) {
                        write!(f, "\n  - {}", suggestion.message)?;
                    }
                }
                Ok(())
            }
            
            ParseError::InvalidSyntax { message, position, context, suggestions, help, related_errors } => {
                write!(f, "Invalid syntax at {}: {}", position, message)?;
                write!(f, "\nContext: {}", context)?;
                
                if !suggestions.is_empty() {
                    write!(f, "\nSuggestions:")?;
                    for suggestion in suggestions.iter().take(3) {
                        write!(f, "\n  - {}", suggestion.message)?;
                    }
                }
                
                if !related_errors.is_empty() {
                    write!(f, "\nRelated issues:")?;
                    for related in related_errors.iter().take(2) {
                        write!(f, "\n  - {} at {} [{:?}]", 
                               related.message, related.position, related.severity)?;
                    }
                }
                
                if let Some(help_text) = help {
                    write!(f, "\nHelp: {}", help_text)?;
                }
                Ok(())
            }
            
            ParseError::MissingDelimiter { delimiter, open_position, expected_close_position, context, suggestion } => {
                write!(f, "Missing closing `{:?}` at {}", delimiter, expected_close_position)?;
                write!(f, "\nOpened at {} {}", open_position, context)?;
                write!(f, "\nSuggestion: {}", suggestion)?;
                Ok(())
            }
            
            ParseError::MismatchedDelimiter { expected, found, expected_position, found_position, suggestion } => {
                write!(f, "Mismatched delimiter: expected `{:?}` at {}, found `{:?}` at {}", 
                       expected, expected_position, found, found_position)?;
                write!(f, "\nSuggestion: {}", suggestion)?;
                Ok(())
            }
            
            ParseError::InvalidIdentifier { found, position, reason, suggestions } => {
                write!(f, "Invalid identifier `{}` at {}: ", found, position)?;
                match reason {
                    InvalidIdentifierReason::StartsWithDigit => 
                        write!(f, "identifiers cannot start with a digit")?,
                    InvalidIdentifierReason::ContainsInvalidCharacters => 
                        write!(f, "contains invalid characters")?,
                    InvalidIdentifierReason::ReservedKeyword => 
                        write!(f, "is a reserved keyword")?,
                    InvalidIdentifierReason::TooLong => 
                        write!(f, "is too long (max 255 characters)")?,
                    InvalidIdentifierReason::Empty => 
                        write!(f, "is empty")?,
                }
                
                if !suggestions.is_empty() {
                    write!(f, "\nDid you mean: {}", suggestions.join(", "))?;
                }
                Ok(())
            }
            
            ParseError::TypeAnnotationError { message, position, expected_type_context, suggestions } => {
                write!(f, "Type annotation error at {}: {}", position, message)?;
                write!(f, "\nExpected: {}", expected_type_context)?;
                if !suggestions.is_empty() {
                    write!(f, "\nSuggestions: {}", suggestions.join(", "))?;
                }
                Ok(())
            }
            
            ParseError::MemoryAnnotationError { message, position, found_annotation, valid_annotations, suggestions } => {
                write!(f, "Memory annotation error at {}: {}", position, message)?;
                write!(f, "\nFound: `{}`", found_annotation)?;
                write!(f, "\nValid annotations: {}", valid_annotations.join(", "))?;
                if !suggestions.is_empty() {
                    write!(f, "\nSuggestions: {}", suggestions.join(", "))?;
                }
                Ok(())
            }
            
            ParseError::MultipleErrors { primary, related, summary } => {
                write!(f, "Multiple parse errors found: {}", summary)?;
                write!(f, "\nPrimary error: {}", primary)?;
                write!(f, "\nAdditional errors ({}):", related.len())?;
                for (i, error) in related.iter().enumerate().take(5) {
                    write!(f, "\n  {}: {}", i + 1, error)?;
                }
                if related.len() > 5 {
                    write!(f, "\n  ... and {} more errors", related.len() - 5)?;
                }
                Ok(())
            }
            
            _ => {
                // Fallback for other error types
                write!(f, "Parse error: {:?}", self)
            }
        }
    }
}

impl From<LexerError> for ParseError {
    fn from(error: LexerError) -> Self {
        ParseError::LexerError {
            error,
            suggestions: Vec::new(),
            help: None,
        }
    }
}

impl std::error::Error for ParseError {}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Utility functions for error creation and suggestion generation

/// Generate similarity-based suggestions for identifiers
pub fn suggest_similar_identifiers(input: &str, candidates: &[&str]) -> Vec<String> {
    let mut suggestions: Vec<(String, f32)> = candidates
        .iter()
        .map(|&candidate| (candidate.to_string(), similarity_score(input, candidate)))
        .filter(|(_, score)| *score > 0.3) // Minimum similarity threshold
        .collect();
    
    suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    suggestions.into_iter().take(3).map(|(name, _)| name).collect()
}

/// Calculate similarity score between two strings (Levenshtein-based)
fn similarity_score(a: &str, b: &str) -> f32 {
    let len_a = a.len();
    let len_b = b.len();
    
    if len_a == 0 || len_b == 0 {
        return 0.0;
    }
    
    let distance = levenshtein_distance(a, b);
    let max_len = len_a.max(len_b);
    
    1.0 - (distance as f32 / max_len as f32)
}

/// Calculate Levenshtein distance
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();
    
    let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];
    
    for i in 0..=len_a {
        matrix[i][0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }
    
    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    
    matrix[len_a][len_b]
}

/// Create context-aware suggestions for common parsing scenarios
pub fn suggest_for_context(context: &ParseContext, found_token: &TokenType) -> Vec<Suggestion> {
    match (context, found_token) {
        (ParseContext::FunctionParameters, TokenType::Identifier(_)) => {
            vec![Suggestion::new("Add type annotation to parameter", Position::start(0))
                .with_replacement(": Type")
                .with_category(SuggestionCategory::Syntax)]
        }
        (ParseContext::StructFields, TokenType::RightBrace) => {
            vec![Suggestion::new("Add field to struct", Position::start(0))
                .with_replacement("field_name: Type,")
                .with_category(SuggestionCategory::Syntax)]
        }
        (ParseContext::Expression, TokenType::LeftBrace) => {
            vec![Suggestion::new("Use block expression or struct initialization", Position::start(0))
                .with_category(SuggestionCategory::Syntax)]
        }
        _ => Vec::new(),
    }
}

/// Helper functions for creating common error patterns
impl ParseError {
    /// Create a simple InvalidSyntax error with context
    pub fn invalid_syntax(message: &str, position: Position, context: ParseContext) -> Self {
        ParseError::InvalidSyntax {
            message: message.to_string(),
            position,
            context,
            suggestions: Vec::new(),
            help: None,
            related_errors: Vec::new(),
        }
    }
    
    /// Create an UnexpectedToken error with context
    pub fn unexpected_token(
        expected: &str,
        description: &str,
        found: TokenType,
        position: Position,
        context: ParseContext,
    ) -> Self {
        ParseError::UnexpectedToken {
            expected: vec![ExpectedToken::new(expected, description)],
            found,
            position,
            context,
            suggestions: Vec::new(),
            help: None,
        }
    }
    
    /// Create an UnexpectedEof error with context
    pub fn unexpected_eof(
        expected: &str,
        description: &str,
        position: Position,
        context: ParseContext,
    ) -> Self {
        ParseError::UnexpectedEof {
            expected: vec![ExpectedToken::new(expected, description)],
            position,
            context,
            unclosed_delimiters: Vec::new(),
            suggestions: Vec::new(),
        }
    }
    
    /// Create a MemoryAnnotationError
    pub fn memory_annotation_error(
        message: &str,
        position: Position,
        found_annotation: &str,
        valid_annotations: Vec<String>,
    ) -> Self {
        ParseError::MemoryAnnotationError {
            message: message.to_string(),
            position,
            found_annotation: found_annotation.to_string(),
            valid_annotations,
            suggestions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_similarity_score() {
        assert!(similarity_score("hello", "helo") > 0.8);
        assert!(similarity_score("function", "functon") > 0.8);
        assert!(similarity_score("hello", "world") < 0.3);
    }
    
    #[test]
    fn test_suggest_similar_identifiers() {
        let candidates = ["function", "struct", "enum", "impl", "match"];
        let suggestions = suggest_similar_identifiers("functon", &candidates);
        assert!(suggestions.contains(&"function".to_string()));
    }
    
    #[test]
    fn test_error_display() {
        let error = ParseError::UnexpectedToken {
            expected: vec![ExpectedToken::new("identifier", "variable name")],
            found: TokenType::Integer { value: "42".to_string(), base: crate::lexer::token::NumberBase::Decimal, suffix: None },
            position: Position::start(0),
            context: ParseContext::FunctionDeclaration,
            suggestions: vec![Suggestion::new("Use a valid identifier", Position::start(0))],
            help: Some("Identifiers must start with a letter or underscore".to_string()),
        };
        
        let display = format!("{}", error);
        assert!(display.contains("Unexpected token"));
        assert!(display.contains("function declaration"));
        assert!(display.contains("Suggestions"));
    }
} 