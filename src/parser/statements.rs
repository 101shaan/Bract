//! Statement parsing for Prism programming language
//!
//! This module handles parsing of all statement types including:
//! - Let bindings with patterns and type annotations
//! - Assignment statements (=, +=, -=, etc.)
//! - Expression statements
//! - Control flow (if, while, for, loop, match)
//! - Break/continue/return statements
//! - Block statements

use crate::lexer::{TokenType, Token};
use crate::ast::*;
use super::parser::Parser;
use super::error::{ParseError, ParseResult};

impl<'a> Parser<'a> {
    /// Parse a statement
    pub fn parse_statement(&mut self) -> ParseResult<Stmt> {
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::Let => self.parse_let_statement(),
                TokenType::If => self.parse_if_statement(),
                TokenType::While => self.parse_while_statement(),
                TokenType::For => self.parse_for_statement(),
                TokenType::Loop => self.parse_loop_statement(),
                TokenType::Match => self.parse_match_statement(),
                TokenType::Break => self.parse_break_statement(),
                TokenType::Continue => self.parse_continue_statement(),
                TokenType::Return => self.parse_return_statement(),
                TokenType::LeftBrace => self.parse_block_statement(),
                _ => {
                    // Try to parse as expression statement or assignment
                    let start_pos = self.current_position();
                    let expr = self.parse_expression()?;
                    
                    // Check if this is an assignment
                    if let Some(token) = &self.current_token {
                        match &token.token_type {
                            TokenType::Equal => {
                                self.advance()?;
                                let value = self.parse_expression()?;
                                self.expect(TokenType::Semicolon, "assignment statement")?;
                                let end_pos = self.current_position();
                                Ok(Stmt::Assignment {
                                    target: expr,
                                    value,
                                    span: Span::new(start_pos, end_pos),
                                })
                            }
                            TokenType::PlusEq | TokenType::MinusEq | TokenType::StarEq |
                            TokenType::SlashEq | TokenType::PercentEq | TokenType::AndEq |
                            TokenType::OrEq | TokenType::CaretEq | TokenType::LeftShiftEq |
                            TokenType::RightShiftEq => {
                                let op = match &token.token_type {
                                    TokenType::PlusEq => BinaryOp::Add,
                                    TokenType::MinusEq => BinaryOp::Subtract,
                                    TokenType::StarEq => BinaryOp::Multiply,
                                    TokenType::SlashEq => BinaryOp::Divide,
                                    TokenType::PercentEq => BinaryOp::Modulo,
                                    TokenType::AndEq => BinaryOp::BitwiseAnd,
                                    TokenType::OrEq => BinaryOp::BitwiseOr,
                                    TokenType::CaretEq => BinaryOp::BitwiseXor,
                                    TokenType::LeftShiftEq => BinaryOp::LeftShift,
                                    TokenType::RightShiftEq => BinaryOp::RightShift,
                                    _ => unreachable!(),
                                };
                                self.advance()?;
                                let value = self.parse_expression()?;
                                self.expect(TokenType::Semicolon, "compound assignment")?;
                                let end_pos = self.current_position();
                                Ok(Stmt::CompoundAssignment {
                                    target: expr,
                                    op,
                                    value,
                                    span: Span::new(start_pos, end_pos),
                                })
                            }
                            _ => {
                                // Regular expression statement
                                self.expect(TokenType::Semicolon, "expression statement")?;
                                let end_pos = self.current_position();
                                Ok(Stmt::Expression {
                                    expr,
                                    span: Span::new(start_pos, end_pos),
                                })
                            }
                        }
                    } else {
                        Err(ParseError::UnexpectedEof {
                            expected: vec!["semicolon or assignment operator".to_string()],
                            position: self.current_position(),
                        })
                    }
                }
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["statement".to_string()],
                position: self.current_position(),
            })
        }
    }
    
    /// Parse a let statement: let [mut] pattern [: type] [= expr];
    fn parse_let_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        self.expect(TokenType::Let, "let statement")?;
        
        // Check for mut keyword
        let is_mutable = self.match_token(&TokenType::Mut);
        
        // Parse pattern
        let pattern = self.parse_pattern()?;
        
        // Parse optional type annotation
        let type_annotation = if self.match_token(&TokenType::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        // Parse optional initializer
        let initializer = if self.match_token(&TokenType::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "let statement")?;
        let end_pos = self.current_position();
        
        Ok(Stmt::Let {
            pattern,
            type_annotation,
            initializer,
            is_mutable,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse an if statement: if expr block [else (if_stmt | block)]
    fn parse_if_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        self.expect(TokenType::If, "if statement")?;
        
        let condition = self.parse_expression()?;
        let then_block = self.parse_block_statement_inner()?;
        
        let else_block = if self.match_token(&TokenType::Else) {
            if self.check(&TokenType::If) {
                // else if
                Some(Box::new(self.parse_if_statement()?))
            } else {
                // else block
                Some(Box::new(self.parse_block_statement()?))
            }
        } else {
            None
        };
        
        let end_pos = self.current_position();
        Ok(Stmt::If {
            condition,
            then_block,
            else_block,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a while statement: while expr block
    fn parse_while_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        self.expect(TokenType::While, "while statement")?;
        
        let condition = self.parse_expression()?;
        let body = self.parse_block_statement_inner()?;
        
        let end_pos = self.current_position();
        Ok(Stmt::While {
            condition,
            body,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a for statement: for pattern in expr block
    fn parse_for_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        self.expect(TokenType::For, "for statement")?;
        
        let pattern = self.parse_pattern()?;
        self.expect(TokenType::In, "for statement")?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block_statement_inner()?;
        
        let end_pos = self.current_position();
        Ok(Stmt::For {
            pattern,
            iterable,
            body,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a loop statement: [label:] loop block
    fn parse_loop_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        
        // Check for optional label
        let label = if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                if self.peek().map(|t| matches!(t.token_type, TokenType::Colon)).unwrap_or(false) {
                    let label_name = self.interner.intern(name);
                    self.advance()?; // consume identifier
                    self.advance()?; // consume colon
                    Some(label_name)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        self.expect(TokenType::Loop, "loop statement")?;
        let body = self.parse_block_statement_inner()?;
        
        let end_pos = self.current_position();
        Ok(Stmt::Loop {
            label,
            body,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a match statement: match expr { arms }
    fn parse_match_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        self.expect(TokenType::Match, "match statement")?;
        
        let expr = self.parse_expression()?;
        self.expect(TokenType::LeftBrace, "match arms")?;
        
        let mut arms = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            let arm_start = self.current_position();
            let pattern = self.parse_pattern()?;
            
            // Parse optional guard
            let guard = if self.match_token(&TokenType::If) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            self.expect(TokenType::FatArrow, "match arm")?;
            
            // Parse arm body (expression or block)
            let body = if self.check(&TokenType::LeftBrace) {
                self.parse_block_expression()?
            } else {
                self.parse_expression()?
            };
            
            // Optional comma
            if !self.check(&TokenType::RightBrace) {
                self.match_token(&TokenType::Comma);
            }
            
            let arm_end = self.current_position();
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: Span::new(arm_start, arm_end),
            });
        }
        
        self.expect(TokenType::RightBrace, "match statement")?;
        let end_pos = self.current_position();
        
        Ok(Stmt::Match {
            expr,
            arms,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a break statement: break [label] [expr];
    fn parse_break_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        self.expect(TokenType::Break, "break statement")?;
        
        // Check for optional label
        let label = if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                let label_name = self.interner.intern(name);
                self.advance()?;
                Some(label_name)
            } else {
                None
            }
        } else {
            None
        };
        
        // Check for optional expression (break value)
        let expr = if !self.check(&TokenType::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "break statement")?;
        let end_pos = self.current_position();
        
        Ok(Stmt::Break {
            label,
            expr,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a continue statement: continue [label];
    fn parse_continue_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        self.expect(TokenType::Continue, "continue statement")?;
        
        // Check for optional label
        let label = if let Some(token) = &self.current_token {
            if let TokenType::Identifier(name) = &token.token_type {
                let label_name = self.interner.intern(name);
                self.advance()?;
                Some(label_name)
            } else {
                None
            }
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "continue statement")?;
        let end_pos = self.current_position();
        
        Ok(Stmt::Continue {
            label,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a return statement: return [expr];
    fn parse_return_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        self.expect(TokenType::Return, "return statement")?;
        
        // Check for optional expression
        let expr = if !self.check(&TokenType::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.expect(TokenType::Semicolon, "return statement")?;
        let end_pos = self.current_position();
        
        Ok(Stmt::Return {
            expr,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a block statement: { statements... }
    fn parse_block_statement(&mut self) -> ParseResult<Stmt> {
        let start_pos = self.current_position();
        let statements = self.parse_block_statement_inner()?;
        let end_pos = self.current_position();
        
        Ok(Stmt::Block {
            statements,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse the inner part of a block statement (helper)
    fn parse_block_statement_inner(&mut self) -> ParseResult<Vec<Stmt>> {
        self.expect(TokenType::LeftBrace, "block statement")?;
        
        let mut statements = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    self.add_error(err);
                    self.synchronize();
                }
            }
        }
        
        self.expect(TokenType::RightBrace, "block statement")?;
        Ok(statements)
    }
    
    /// Check if the current token can start a statement
    pub fn is_statement_start(&self) -> bool {
        if let Some(token) = &self.current_token {
            matches!(token.token_type, 
                TokenType::Let | TokenType::If | TokenType::While | 
                TokenType::For | TokenType::Loop | TokenType::Match |
                TokenType::Break | TokenType::Continue | TokenType::Return |
                TokenType::LeftBrace
            )
        } else {
            false
        }
    }
    
    /// Get the current token for lookahead (helper method)
    fn peek(&self) -> Option<&Token> {
        // This is a simplified peek - in a real implementation, 
        // we'd need to look ahead in the token stream
        // For now, we'll return None as a placeholder
        None
    }
} 