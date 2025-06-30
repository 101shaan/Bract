//! Expression parsing with operator precedence for Prism

use crate::lexer::TokenType;
use crate::ast::*;
use super::parser::Parser;
use super::error::{ParseError, ParseResult};

impl<'a> Parser<'a> {
    /// Parse an expression (entry point for expression parsing)
    pub fn parse_expression(&mut self) -> ParseResult<Expr> {
        self.parse_assignment_expression()
    }
    
    /// Parse assignment expressions (lowest precedence)
    pub fn parse_assignment_expression(&mut self) -> ParseResult<Expr> {
        let expr = self.parse_ternary_expression()?;
        
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::Equal => {
                    self.advance()?;
                    let value = self.parse_assignment_expression()?;
                    let span = Span::new(expr.span().start, value.span().end);
                    Ok(Expr::Binary {
                        left: Box::new(expr),
                        op: BinaryOp::Assign,
                        right: Box::new(value),
                        span,
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
                    let value = self.parse_assignment_expression()?;
                    let span = Span::new(expr.span().start, value.span().end);
                    Ok(Expr::Binary {
                        left: Box::new(expr),
                        op,
                        right: Box::new(value),
                        span,
                    })
                }
                _ => Ok(expr),
            }
        } else {
            Ok(expr)
        }
    }
    
    /// Parse ternary expressions (? :)
    pub fn parse_ternary_expression(&mut self) -> ParseResult<Expr> {
        let expr = self.parse_logical_or_expression()?;
        
        if self.match_token(&TokenType::Question) {
            let then_expr = self.parse_expression()?;
            self.expect(TokenType::Colon, "ternary expression")?;
            let else_expr = self.parse_ternary_expression()?;
            let span = Span::new(expr.span().start, else_expr.span().end);
            Ok(Expr::If {
                condition: Box::new(expr),
                then_block: Box::new(then_expr),
                else_block: Some(Box::new(else_expr)),
                span,
            })
        } else {
            Ok(expr)
        }
    }
    
    /// Parse logical OR expressions
    pub fn parse_logical_or_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_logical_and_expression()?;
        
        while self.match_token(&TokenType::LogicalOr) {
            let right = self.parse_logical_and_expression()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::LogicalOr,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(expr)
    }
    
    /// Parse logical AND expressions
    pub fn parse_logical_and_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_bitwise_or_expression()?;
        
        while self.match_token(&TokenType::LogicalAnd) {
            let right = self.parse_bitwise_or_expression()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::LogicalAnd,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(expr)
    }
    
    /// Parse bitwise OR expressions
    pub fn parse_bitwise_or_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_bitwise_xor_expression()?;
        
        while self.match_token(&TokenType::Or) {
            let right = self.parse_bitwise_xor_expression()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseOr,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(expr)
    }
    
    /// Parse bitwise XOR expressions
    pub fn parse_bitwise_xor_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_bitwise_and_expression()?;
        
        while self.match_token(&TokenType::Caret) {
            let right = self.parse_bitwise_and_expression()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseXor,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(expr)
    }
    
    /// Parse bitwise AND expressions
    pub fn parse_bitwise_and_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_equality_expression()?;
        
        while self.match_token(&TokenType::And) {
            let right = self.parse_equality_expression()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseAnd,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(expr)
    }
    
    /// Parse equality expressions
    pub fn parse_equality_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_additive_expression()?;
        
        while let Some(token) = &self.current_token {
            let op = match &token.token_type {
                TokenType::Eq => BinaryOp::Equal,
                TokenType::NotEq => BinaryOp::NotEqual,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_additive_expression()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(expr)
    }
    
    /// Parse additive expressions
    pub fn parse_additive_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_multiplicative_expression()?;
        
        while let Some(token) = &self.current_token {
            let op = match &token.token_type {
                TokenType::Plus => BinaryOp::Add,
                TokenType::Minus => BinaryOp::Subtract,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_multiplicative_expression()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(expr)
    }
    
    /// Parse multiplicative expressions
    pub fn parse_multiplicative_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_unary_expression()?;
        
        while let Some(token) = &self.current_token {
            let op = match &token.token_type {
                TokenType::Star => BinaryOp::Multiply,
                TokenType::Slash => BinaryOp::Divide,
                TokenType::Percent => BinaryOp::Modulo,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_unary_expression()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(expr)
    }
    
    /// Parse unary expressions
    pub fn parse_unary_expression(&mut self) -> ParseResult<Expr> {
        if let Some(token) = &self.current_token {
            let op = match &token.token_type {
                TokenType::Not => UnaryOp::Not,
                TokenType::Tilde => UnaryOp::BitwiseNot,
                TokenType::Minus => UnaryOp::Negate,
                TokenType::Plus => UnaryOp::Plus,
                _ => return self.parse_primary_expression(),
            };
            let start_pos = token.position;
            self.advance()?;
            
            let expr = self.parse_unary_expression()?;
            let span = Span::new(start_pos, expr.span().end);
            Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
                span,
            })
        } else {
            self.parse_primary_expression()
        }
    }
    
    /// Parse primary expressions (literals, identifiers, etc.)
    pub fn parse_primary_expression(&mut self) -> ParseResult<Expr> {
        if let Some(token) = &self.current_token {
            let start_pos = token.position;
            
            match &token.token_type {
                TokenType::Integer { value, base, suffix } => {
                    let literal = Literal::Integer {
                        value: value.clone(),
                        base: *base,
                        suffix: suffix.as_ref().map(|s| self.interner.intern(s)),
                    };
                    self.advance()?;
                    Ok(Expr::Literal {
                        literal,
                        span: Span::single(start_pos),
                    })
                }
                TokenType::Float { value, suffix } => {
                    let literal = Literal::Float {
                        value: value.clone(),
                        suffix: suffix.as_ref().map(|s| self.interner.intern(s)),
                    };
                    self.advance()?;
                    Ok(Expr::Literal {
                        literal,
                        span: Span::single(start_pos),
                    })
                }
                TokenType::String { value, raw, raw_delimiter } => {
                    let literal = Literal::String {
                        value: self.interner.intern(value),
                        raw: *raw,
                        raw_delimiter: *raw_delimiter,
                    };
                    self.advance()?;
                    Ok(Expr::Literal {
                        literal,
                        span: Span::single(start_pos),
                    })
                }
                TokenType::Char(ch) => {
                    let literal = Literal::Char(*ch);
                    self.advance()?;
                    Ok(Expr::Literal {
                        literal,
                        span: Span::single(start_pos),
                    })
                }
                TokenType::True => {
                    let literal = Literal::Bool(true);
                    self.advance()?;
                    Ok(Expr::Literal {
                        literal,
                        span: Span::single(start_pos),
                    })
                }
                TokenType::False => {
                    let literal = Literal::Bool(false);
                    self.advance()?;
                    Ok(Expr::Literal {
                        literal,
                        span: Span::single(start_pos),
                    })
                }
                TokenType::Null => {
                    let literal = Literal::Null;
                    self.advance()?;
                    Ok(Expr::Literal {
                        literal,
                        span: Span::single(start_pos),
                    })
                }
                TokenType::Identifier(name) => {
                    let name_interned = self.interner.intern(name);
                    self.advance()?;
                    Ok(Expr::Identifier {
                        name: name_interned,
                        span: Span::single(start_pos),
                    })
                }
                TokenType::LeftParen => {
                    self.advance()?;
                    let expr = self.parse_expression()?;
                    let end_token = self.expect(TokenType::RightParen, "parenthesized expression")?;
                    let span = Span::new(start_pos, end_token.position);
                    Ok(Expr::Parenthesized {
                        expr: Box::new(expr),
                        span,
                    })
                }
                _ => Err(ParseError::ExpectedExpression {
                    position: start_pos,
                }),
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec!["expression".to_string()],
                position: self.current_position(),
            })
        }
    }

    /// Check if the current token can start an expression
    fn is_expression_token(&self) -> bool {
        if let Some(token) = &self.current_token {
            matches!(token.token_type,
                TokenType::Integer { .. } | TokenType::Float { .. } |
                TokenType::String { .. } | TokenType::Char(_) |
                TokenType::True | TokenType::False | TokenType::Null |
                TokenType::Identifier(_) | TokenType::LeftParen |
                TokenType::LeftBracket | TokenType::LeftBrace |
                TokenType::Not | TokenType::Minus | TokenType::Plus |
                TokenType::Star | TokenType::And | TokenType::Tilde |
                TokenType::Box | TokenType::Move
            )
        } else {
            false
        }
    }
} 