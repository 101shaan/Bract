//! Statement parsing for Prism

use crate::ast::*;
use super::parser::Parser;
use super::error::{ParseError, ParseResult};

impl<'a> Parser<'a> {
    /// Parse a statement
    pub fn parse_statement(&mut self) -> ParseResult<Stmt> {
        todo!("Statement parsing will be implemented")
    }
} 