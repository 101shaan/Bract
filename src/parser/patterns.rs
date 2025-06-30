//! Pattern parsing for Prism

use crate::ast::*;
use super::parser::Parser;
use super::error::{ParseError, ParseResult};

impl<'a> Parser<'a> {
    /// Parse a pattern
    pub fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        todo!("Pattern parsing will be implemented")
    }
} 