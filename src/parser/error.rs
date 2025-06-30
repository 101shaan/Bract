//! Parser error types and error handling utilities

use crate::lexer::{LexerError, Position, TokenType};
use std::fmt;

/// Parser errors with detailed information for user-friendly error messages
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Unexpected token encountered
    UnexpectedToken {
        expected: Vec<String>,
        found: TokenType,
        position: Position,
    },
    /// Unexpected end of file
    UnexpectedEof {
        expected: Vec<String>,
        position: Position,
    },
    /// Lexer error during parsing
    LexerError(LexerError),
    /// Invalid syntax construct
    InvalidSyntax {
        message: String,
        position: Position,
    },
    /// Expression expected but not found
    ExpectedExpression {
        position: Position,
    },
    /// Pattern expected but not found
    ExpectedPattern {
        position: Position,
    },
    /// Type expected but not found
    ExpectedType {
        position: Position,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found, position } => {
                write!(f, "Unexpected token at {}:{}: expected {}, found {:?}", 
                       position.line, position.column, 
                       expected.join(" or "), found)
            }
            ParseError::UnexpectedEof { expected, position } => {
                write!(f, "Unexpected end of file at {}:{}: expected {}", 
                       position.line, position.column, expected.join(" or "))
            }
            ParseError::LexerError(err) => write!(f, "Lexer error: {}", err),
            ParseError::InvalidSyntax { message, position } => {
                write!(f, "Invalid syntax at {}:{}: {}", 
                       position.line, position.column, message)
            }
            ParseError::ExpectedExpression { position } => {
                write!(f, "Expected expression at {}:{}", position.line, position.column)
            }
            ParseError::ExpectedPattern { position } => {
                write!(f, "Expected pattern at {}:{}", position.line, position.column)
            }
            ParseError::ExpectedType { position } => {
                write!(f, "Expected type at {}:{}", position.line, position.column)
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        ParseError::LexerError(err)
    }
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>; 