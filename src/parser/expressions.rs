//! Expression parsing with operator precedence for Bract

use crate::lexer::TokenType;
use crate::ast::{Expr, Span, BinaryOp, UnaryOp, Literal};
use super::parser::Parser;
use super::error::{ParseError, ParseResult};

impl<'a> Parser<'a> {
    /// Parse an expression (entry point for expression parsing)
    pub fn parse_expression(&mut self) -> ParseResult<Expr> {
        self.parse_assignment_expression()
    }
    
    /// Parse assignment expressions (lowest precedence)
    pub fn parse_assignment_expression(&mut self) -> ParseResult<Expr> {
        // Assignment is handled in statement parsing, not expression parsing
        // This just delegates to ternary expressions
        self.parse_ternary_expression()
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
        let mut expr = self.parse_relational_expression()?;
        
        while let Some(token) = &self.current_token {
            let op = match &token.token_type {
                TokenType::Eq => BinaryOp::Equal,
                TokenType::NotEq => BinaryOp::NotEqual,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_relational_expression()?;
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
    
    /// Parse relational expressions (< > <= >=)
    pub fn parse_relational_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_range_expression()?;
        
        while let Some(token) = &self.current_token {
            let op = match &token.token_type {
                TokenType::Less => BinaryOp::Less,
                TokenType::Greater => BinaryOp::Greater,
                TokenType::LessEq => BinaryOp::LessEqual,
                TokenType::GreaterEq => BinaryOp::GreaterEqual,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_range_expression()?;
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
    
    /// Parse range expressions (.. and ..=)
    pub fn parse_range_expression(&mut self) -> ParseResult<Expr> {
        let expr = self.parse_additive_expression()?;
        
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::DotDot => {
                    self.advance()?;
                    // Check for inclusive range (..=)
                    let inclusive = if self.match_token(&TokenType::Equal) {
                        true
                    } else {
                        false
                    };
                    
                    // Check if there's an end expression
                    let end = if self.is_expression_token() {
                        Some(Box::new(self.parse_additive_expression()?))
                    } else {
                        None
                    };
                    
                    let end_pos = end.as_ref()
                        .map(|e| e.span().end)
                        .unwrap_or(self.current_position());
                    let span = Span::new(expr.span().start, end_pos);
                    
                    Ok(Expr::Range {
                        start: Some(Box::new(expr)),
                        end,
                        inclusive,
                        span,
                    })
                }
                _ => Ok(expr),
            }
        } else {
            Ok(expr)
        }
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
                TokenType::And => UnaryOp::AddressOf,
                TokenType::Star => UnaryOp::Dereference,
                _ => return self.parse_postfix_expression(),
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
            self.parse_postfix_expression()
        }
    }
    
    /// Parse postfix expressions (function calls, method calls, field access, indexing)
    pub fn parse_postfix_expression(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary_expression()?;
        
        loop {
            if let Some(token) = &self.current_token {
                match &token.token_type {
                    // Function calls: expr(args)
                    TokenType::LeftParen => {
                        self.advance()?; // consume '('
                        let mut args = Vec::new();
                        
                        if !self.check(&TokenType::RightParen) {
                            args.push(self.parse_expression()?);
                            
                            while self.match_token(&TokenType::Comma) {
                                if self.check(&TokenType::RightParen) {
                                    break; // trailing comma
                                }
                                args.push(self.parse_expression()?);
                            }
                        }
                        
                        let end_token = self.expect(TokenType::RightParen, "function call")?;
                        let span = Span::new(expr.span().start, end_token.position);
                        
                        expr = Expr::Call {
                            callee: Box::new(expr),
                            args,
                            span,
                        };
                    }
                    
                    // Field access: expr.field or Method call: expr.method(args)
                    TokenType::Dot => {
                        self.advance()?; // consume '.'
                        
                        if let Some(field_token) = &self.current_token {
                            if let TokenType::Identifier(field_name) = &field_token.token_type {
                                let field = self.interner.intern(field_name);
                                let field_pos = field_token.position;
                                self.advance()?;
                                
                                // Check if this is a method call (followed by '(')
                                if self.check(&TokenType::LeftParen) {
                                    // This is a method call: expr.method(args)
                                    self.advance()?; // consume '('
                                    let mut args = Vec::new();
                                    
                                    if !self.check(&TokenType::RightParen) {
                                        args.push(self.parse_expression()?);
                                        
                                        while self.match_token(&TokenType::Comma) {
                                            if self.check(&TokenType::RightParen) {
                                                break; // trailing comma
                                            }
                                            args.push(self.parse_expression()?);
                                        }
                                    }
                                    
                                    let end_token = self.expect(TokenType::RightParen, "method call")?;
                                    let span = Span::new(expr.span().start, end_token.position);
                                    
                                    expr = Expr::MethodCall {
                                        receiver: Box::new(expr),
                                        method: field,
                                        args,
                                        span,
                                    };
                                } else {
                                    // This is field access: expr.field
                                    let span = Span::new(expr.span().start, field_pos);
                                    expr = Expr::FieldAccess {
                                        object: Box::new(expr),
                                        field,
                                        span,
                                    };
                                }
                            } else {
                                return Err(ParseError::UnexpectedToken {
                                    expected: vec!["field name".to_string()],
                                    found: field_token.token_type.clone(),
                                    position: field_token.position,
                                });
                            }
                        } else {
                            return Err(ParseError::UnexpectedEof {
                                expected: vec!["field name".to_string()],
                                position: self.current_position(),
                            });
                        }
                    }
                    
                    // Array indexing: expr[index]
                    TokenType::LeftBracket => {
                        self.advance()?; // consume '['
                        let index = self.parse_expression()?;
                        let end_token = self.expect(TokenType::RightBracket, "array indexing")?;
                        let span = Span::new(expr.span().start, end_token.position);
                        
                        expr = Expr::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                            span,
                        };
                    }
                    
                    // Struct initialization: expr { field: value, ... }
                    TokenType::LeftBrace => {
                        // Only handle struct initialization if the current expression is a path
                        match &expr {
                            Expr::Identifier { name, .. } => {
                                // This is struct initialization: StructName { fields... }
                                let struct_path = vec![*name];
                                self.advance()?; // consume '{'
                                
                                let mut fields = Vec::new();
                                
                                if !self.check(&TokenType::RightBrace) {
                                    loop {
                                        // Parse field name
                                        let field_name = if let Some(token) = &self.current_token {
                                            if let TokenType::Identifier(name) = &token.token_type {
                                                let field = self.interner.intern(name);
                                                self.advance()?;
                                                field
                                            } else {
                                                return Err(ParseError::UnexpectedToken {
                                                    expected: vec!["field name".to_string()],
                                                    found: token.token_type.clone(),
                                                    position: token.position,
                                                });
                                            }
                                        } else {
                                            return Err(ParseError::UnexpectedEof {
                                                expected: vec!["field name".to_string()],
                                                position: self.current_position(),
                                            });
                                        };
                                        
                                        // Expect colon
                                        self.expect(TokenType::Colon, "struct field initialization")?;
                                        
                                        // Parse field value
                                        let field_value = self.parse_expression()?;
                                        
                                        fields.push(crate::ast::FieldInit {
                                            name: field_name,
                                            value: Some(field_value),
                                            span: Span::single(self.current_position()),
                                        });
                                        
                                        if !self.match_token(&TokenType::Comma) {
                                            break;
                                        }
                                        
                                        if self.check(&TokenType::RightBrace) {
                                            break; // trailing comma
                                        }
                                    }
                                }
                                
                                let end_token = self.expect(TokenType::RightBrace, "struct initialization")?;
                                let span = Span::new(expr.span().start, end_token.position);
                                
                                expr = Expr::StructInit {
                                    path: struct_path,
                                    fields,
                                    span,
                                };
                            }
                            _ => break, // Not a struct initialization, stop postfix parsing
                        }
                    }
                    
                    _ => break, // No more postfix operators
                }
            } else {
                break; // End of input
            }
        }
        
        Ok(expr)
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
                TokenType::LeftBrace => {
                    // Parse block expression
                    self.advance()?; // consume '{'
                    let mut statements = Vec::new();
                    let mut trailing_expr = None;
                    
                    while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                        // Check if this looks like a statement keyword
                        if self.is_statement_start() {
                            statements.push(self.parse_statement()?);
                        } else {
                            // Try to parse as expression
                            let expr = self.parse_expression()?;
                            
                            // Check if there's a semicolon (making it a statement)
                            if self.match_token(&TokenType::Semicolon) {
                                let expr_span = expr.span();
                                statements.push(crate::ast::Stmt::Expression {
                                    expr,
                                    span: expr_span,
                                });
                            } else {
                                // No semicolon - this is the trailing expression
                                trailing_expr = Some(Box::new(expr));
                                break;
                            }
                        }
                    }
                    
                    let end_token = self.expect(TokenType::RightBrace, "block expression")?;
                    let span = Span::new(start_pos, end_token.position);
                    
                    Ok(Expr::Block {
                        statements,
                        trailing_expr,
                        span,
                    })
                }
                TokenType::LeftBracket => {
                    // Parse array literal: [expr1, expr2, ...]
                    self.advance()?; // consume '['
                    let mut elements = Vec::new();
                    
                    while !self.check(&TokenType::RightBracket) && !self.is_at_end() {
                        elements.push(self.parse_expression()?);
                        
                        if !self.match_token(&TokenType::Comma) {
                            break;
                        }
                        
                        // Allow trailing comma
                        if self.check(&TokenType::RightBracket) {
                            break;
                        }
                    }
                    
                    let end_token = self.expect(TokenType::RightBracket, "array literal")?;
                    let span = Span::new(start_pos, end_token.position);
                    
                    Ok(Expr::Array {
                        elements,
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
