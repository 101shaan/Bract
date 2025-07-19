pub mod ast;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod codegen;
pub mod visitor;
pub mod lsp;

/// Performance analysis module - implements contract verification and cost estimation
// TODO: Re-enable after fixing core compilation
// pub mod performance;

pub use lexer::{Lexer, Token, TokenType, Position, LexerError};
pub use ast::{Module, Expr, Stmt, Item, Pattern, Type, Span};
pub use parser::{Parser, ParseError, ParseResult};
pub use codegen::{CCodeGenerator, CodegenContext, CodegenResult, CodegenError}; 