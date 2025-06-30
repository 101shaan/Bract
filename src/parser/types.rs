//! Type parsing for Prism

use crate::ast::*;
use super::parser::Parser;
use super::error::{ParseError, ParseResult};

impl<'a> Parser<'a> {
    /// Parse a type annotation
    pub fn parse_type(&mut self) -> ParseResult<Type> {
        todo!("Type parsing will be implemented")
    }
} 