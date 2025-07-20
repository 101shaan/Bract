//! Parse error types and utilities
//!
//! This module defines comprehensive error types for the Bract parser,
//! providing detailed error messages and recovery information.

use crate::lexer::{TokenType, Position};
use crate::lexer::error::LexerError;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Unexpected token found
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
    
    /// Invalid syntax error with custom message
    InvalidSyntax {
        message: String,
        position: Position,
    },
    
    /// Internal parser error
    InternalError {
        message: String,
        position: Position,
    },
    
    /// Lexer error wrapped in parser error
    LexerError(LexerError),
}

impl From<LexerError> for ParseError {
    fn from(error: LexerError) -> Self {
        ParseError::LexerError(error)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found, position } => {
                write!(f, "Unexpected token {:?} at {}, expected one of: {}", 
                       found, position, expected.join(", "))
            }
            ParseError::UnexpectedEof { expected, position } => {
                write!(f, "Unexpected end of file at {}, expected one of: {}", 
                       position, expected.join(", "))
            }
            ParseError::InvalidSyntax { message, position } => {
                write!(f, "Invalid syntax at {}: {}", position, message)
            }
            ParseError::InternalError { message, position } => {
                write!(f, "Internal parser error at {}: {}", position, message)
            }
            ParseError::LexerError(err) => {
                write!(f, "Lexer error: {}", err)
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>; 