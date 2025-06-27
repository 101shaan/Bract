use std::fmt;
use crate::lexer::position::Position;

/// Represents errors that can occur during lexical analysis
#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    /// Invalid character encountered
    InvalidCharacter(char, Position),
    /// Invalid escape sequence in a string or character literal
    InvalidEscapeSequence(String, Position),
    /// Unterminated string literal
    UnterminatedString(Position),
    /// Unterminated character literal
    UnterminatedChar(Position),
    /// Unterminated block comment
    UnterminatedBlockComment(Position),
    /// Invalid number literal
    InvalidNumber(String, Position),
    /// Invalid unicode escape sequence
    InvalidUnicodeEscape(String, Position),
    /// Empty character literal
    EmptyCharLiteral(Position),
    /// Multi-character literal (more than one character in a char literal)
    MultiCharLiteral(Position),
    /// Invalid hexadecimal digit in a numeric literal
    InvalidHexDigit(char, Position),
    /// Invalid binary digit in a numeric literal
    InvalidBinaryDigit(char, Position),
    /// Invalid octal digit in a numeric literal
    InvalidOctalDigit(char, Position),
    /// Invalid numeric suffix
    InvalidNumericSuffix(String, Position),
    /// Invalid raw string delimiter
    InvalidRawStringDelimiter(Position),
    /// Unterminated raw string
    UnterminatedRawString(Position),
    /// UTF-8 encoding error
    Utf8Error(Position),
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexerError::InvalidCharacter(c, pos) => {
                write!(f, "Invalid character '{}' at {}:{}", c, pos.line, pos.column)
            }
            LexerError::InvalidEscapeSequence(s, pos) => {
                write!(f, "Invalid escape sequence '{}' at {}:{}", s, pos.line, pos.column)
            }
            LexerError::UnterminatedString(pos) => {
                write!(f, "Unterminated string literal starting at {}:{}", pos.line, pos.column)
            }
            LexerError::UnterminatedChar(pos) => {
                write!(f, "Unterminated character literal starting at {}:{}", pos.line, pos.column)
            }
            LexerError::UnterminatedBlockComment(pos) => {
                write!(f, "Unterminated block comment starting at {}:{}", pos.line, pos.column)
            }
            LexerError::InvalidNumber(s, pos) => {
                write!(f, "Invalid number literal '{}' at {}:{}", s, pos.line, pos.column)
            }
            LexerError::InvalidUnicodeEscape(s, pos) => {
                write!(f, "Invalid Unicode escape sequence '{}' at {}:{}", s, pos.line, pos.column)
            }
            LexerError::EmptyCharLiteral(pos) => {
                write!(f, "Empty character literal at {}:{}", pos.line, pos.column)
            }
            LexerError::MultiCharLiteral(pos) => {
                write!(f, "Multiple characters in character literal at {}:{}", pos.line, pos.column)
            }
            LexerError::InvalidHexDigit(c, pos) => {
                write!(f, "Invalid hexadecimal digit '{}' at {}:{}", c, pos.line, pos.column)
            }
            LexerError::InvalidBinaryDigit(c, pos) => {
                write!(f, "Invalid binary digit '{}' at {}:{}", c, pos.line, pos.column)
            }
            LexerError::InvalidOctalDigit(c, pos) => {
                write!(f, "Invalid octal digit '{}' at {}:{}", c, pos.line, pos.column)
            }
            LexerError::InvalidNumericSuffix(s, pos) => {
                write!(f, "Invalid numeric suffix '{}' at {}:{}", s, pos.line, pos.column)
            }
            LexerError::InvalidRawStringDelimiter(pos) => {
                write!(f, "Invalid raw string delimiter at {}:{}", pos.line, pos.column)
            }
            LexerError::UnterminatedRawString(pos) => {
                write!(f, "Unterminated raw string starting at {}:{}", pos.line, pos.column)
            }
            LexerError::Utf8Error(pos) => {
                write!(f, "Invalid UTF-8 sequence at {}:{}", pos.line, pos.column)
            }
        }
    }
}

impl std::error::Error for LexerError {} 