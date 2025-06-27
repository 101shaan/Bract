/// A position in a source file, used for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Absolute offset from the start of the file (0-based)
    pub offset: usize,
    /// The file ID this position refers to
    pub file_id: usize,
}

impl Position {
    /// Create a new Position with the given coordinates
    pub fn new(line: usize, column: usize, offset: usize, file_id: usize) -> Self {
        Self {
            line,
            column, 
            offset,
            file_id,
        }
    }

    /// Create a default position at the start of a file
    pub fn start(file_id: usize) -> Self {
        Self {
            line: 1,
            column: 1,
            offset: 0,
            file_id,
        }
    }
    
    /// Advance the position by a single character
    pub fn advance(&mut self, ch: char) {
        self.offset += ch.len_utf8();
        
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }
    
    /// Advance the position by multiple characters
    pub fn advance_multiple(&mut self, text: &str) {
        for ch in text.chars() {
            self.advance(ch);
        }
    }
} 