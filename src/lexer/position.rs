//! Source position tracking for error reporting
//!
//! This module provides utilities for tracking positions in source code,
//! enabling precise error location reporting and source mapping.

use std::fmt;

/// Represents a position in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)  
    pub column: usize,
    /// Byte offset from start of file (0-based)
    pub offset: usize,
    /// File identifier
    pub file_id: usize,
}

impl Position {
    /// Create a new position
    pub fn new(line: usize, column: usize, offset: usize, file_id: usize) -> Self {
        Self {
            line,
            column,
            offset,
            file_id,
        }
    }
    
    /// Create a position at the start of a file
    pub fn start(file_id: usize) -> Self {
        Self::new(1, 1, 0, file_id)
    }
    
    /// Advance to the next column
    pub fn next_column(&mut self) {
        self.column += 1;
        self.offset += 1;
    }
    
    /// Advance to the next line
    pub fn next_line(&mut self) {
        self.line += 1;
        self.column = 1;
        self.offset += 1;
    }
    
    /// Advance by a specific number of bytes
    pub fn advance(&mut self, bytes: usize) {
        self.column += bytes;
        self.offset += bytes;
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
} 