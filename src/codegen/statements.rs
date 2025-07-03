//! Statement Code Generation for Prism
//!
//! This module handles the translation of Prism statements to C code.
//! It manages control flow, variable declarations, and statement sequences.

use super::{CodegenContext, CCodeBuilder, CodegenResult, CodegenError, expressions::ExpressionGenerator};
use crate::ast::*;

/// Statement code generator
pub struct StatementGenerator<'a> {
    /// Generation context
    context: &'a mut CodegenContext,
}

impl<'a> StatementGenerator<'a> {
    /// Create a new statement generator
    pub fn new(context: &'a mut CodegenContext) -> Self {
        Self {
            context,
        }
    }

    /// Create an expression generator on demand
    fn create_expr_generator(&mut self) -> ExpressionGenerator {
        ExpressionGenerator::new(self.context)
    }
    
    /// Generate C code for a statement
    pub fn generate_statement(&mut self, stmt: &Stmt, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        match stmt {
            Stmt::Expression { expr, .. } => {
                self.generate_expression_statement(expr, builder)
            },
            Stmt::Let { pattern, type_annotation, initializer, is_mutable, .. } => {
                self.generate_let_statement(pattern, type_annotation, initializer, *is_mutable, builder)
            },
            Stmt::Assignment { target, value, .. } => {
                self.generate_assignment(target, value, builder)
            },
            Stmt::CompoundAssignment { target, op, value, .. } => {
                self.generate_compound_assignment(target, *op, value, builder)
            },
            Stmt::If { condition, then_block, else_block, .. } => {
                self.generate_if_statement(condition, then_block, else_block.as_deref(), builder)
            },
            Stmt::While { condition, body, .. } => {
                self.generate_while_loop(condition, body, builder)
            },
            Stmt::For { pattern, iterable, body, .. } => {
                self.generate_for_loop(pattern, iterable, body, builder)
            },
            Stmt::Loop { label, body, .. } => {
                self.generate_infinite_loop(label, body, builder)
            },
            Stmt::Match { expr, arms, .. } => {
                self.generate_match_statement(expr, arms, builder)
            },
            Stmt::Break { label, expr, .. } => {
                self.generate_break_statement(label, expr.as_ref(), builder)
            },
            Stmt::Continue { label, .. } => {
                self.generate_continue_statement(label, builder)
            },
            Stmt::Return { expr, .. } => {
                self.generate_return_statement(expr.as_ref(), builder)
            },
            Stmt::Block { statements, .. } => {
                self.generate_block_statement(statements, builder)
            },
            Stmt::Item { item, .. } => {
                // Items within statements are handled separately
                builder.comment("Item within statement - handled separately");
                Ok(())
            },
            Stmt::Empty { .. } => {
                // Empty statement
                builder.line(";");
                Ok(())
            },
        }
    }
    
    /// Generate expression statement
    fn generate_expression_statement(&mut self, expr: &Expr, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        let mut expr_gen = self.create_expr_generator();
        let expr_code = expr_gen.generate_expression(expr)?;
        builder.line(&format!("{};", expr_code));
        Ok(())
    }
    
    /// Generate let statement
    fn generate_let_statement(
        &mut self,
        pattern: &Pattern,
        type_annotation: &Option<Type>,
        initializer: &Option<Expr>,
        is_mutable: bool,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        match pattern {
            Pattern::Identifier { name, .. } => {
                let var_name = self.format_identifier(name);
                
                // Determine type
                let type_name = if let Some(type_ann) = type_annotation {
                    self.generate_type_name(type_ann)?
                } else if let Some(init) = initializer {
                    // Type inference - for now, use a generic type
                    "auto".to_string()
                } else {
                    return Err(CodegenError::TypeConversion(
                        "Variable declaration requires either type annotation or initializer".to_string()
                    ));
                };
                
                // Generate declaration
                if let Some(init) = initializer {
                    let mut expr_gen = self.create_expr_generator();
                    let init_code = expr_gen.generate_expression(init)?;
                    builder.line(&format!("{} {} = {};", type_name, var_name, init_code));
                } else {
                    builder.line(&format!("{} {};", type_name, var_name));
                }
                
                Ok(())
            },
            Pattern::Tuple { patterns, .. } => {
                // Destructuring assignment
                self.generate_tuple_destructuring(patterns, type_annotation, initializer, is_mutable, builder)
            },
            Pattern::Struct { path, fields, .. } => {
                // Struct destructuring
                self.generate_struct_destructuring(path, fields, type_annotation, initializer, is_mutable, builder)
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    "Complex pattern in let statement not yet supported".to_string()
                ))
            }
        }
    }
    
    /// Generate assignment statement
    fn generate_assignment(&mut self, target: &Expr, value: &Expr, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        let target_code = self.expr_gen.generate_expression(target)?;
        let value_code = self.expr_gen.generate_expression(value)?;
        
        builder.line(&format!("{} = {};", target_code, value_code));
        Ok(())
    }
    
    /// Generate compound assignment
    fn generate_compound_assignment(&mut self, target: &Expr, op: BinaryOp, value: &Expr, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        let target_code = self.expr_gen.generate_expression(target)?;
        let value_code = self.expr_gen.generate_expression(value)?;
        
        let op_str = match op {
            BinaryOp::Add => "+=",
            BinaryOp::Subtract => "-=",
            BinaryOp::Multiply => "*=",
            BinaryOp::Divide => "/=",
            BinaryOp::Modulo => "%=",
            BinaryOp::BitwiseAnd => "&=",
            BinaryOp::BitwiseOr => "|=",
            BinaryOp::BitwiseXor => "^=",
            BinaryOp::LeftShift => "<<=",
            BinaryOp::RightShift => ">>=",
            _ => {
                return Err(CodegenError::UnsupportedFeature(
                    format!("Compound assignment operator not supported: {:?}", op)
                ));
            }
        };
        
        builder.line(&format!("{} {} {};", target_code, op_str, value_code));
        Ok(())
    }
    
    /// Generate if statement
    fn generate_if_statement(
        &mut self,
        condition: &Expr,
        then_block: &[Stmt],
        else_block: Option<&Stmt>,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        let condition_code = self.expr_gen.generate_expression(condition)?;
        
        builder.line(&format!("if ({}) {{", condition_code));
        builder.indent_inc();
        
        // Generate then block
        for stmt in then_block {
            self.generate_statement(stmt, builder)?;
        }
        
        builder.indent_dec();
        
        if let Some(else_stmt) = else_block {
            builder.line("} else {");
            builder.indent_inc();
            
            match else_stmt {
                Stmt::Block { statements, .. } => {
                    for stmt in statements {
                        self.generate_statement(stmt, builder)?;
                    }
                },
                _ => {
                    self.generate_statement(else_stmt, builder)?;
                }
            }
            
            builder.indent_dec();
            builder.line("}");
        } else {
            builder.line("}");
        }
        
        Ok(())
    }
    
    /// Generate while loop
    fn generate_while_loop(&mut self, condition: &Expr, body: &[Stmt], builder: &mut CCodeBuilder) -> CodegenResult<()> {
        let condition_code = self.expr_gen.generate_expression(condition)?;
        
        self.context.enter_loop(None);
        let loop_ctx = self.context.current_loop().unwrap().clone();
        
        builder.line(&format!("while ({}) {{", condition_code));
        builder.indent_inc();
        
        // Generate loop body
        for stmt in body {
            self.generate_statement(stmt, builder)?;
        }
        
        // Generate continue label
        builder.line(&format!("{}:;", loop_ctx.continue_label));
        
        builder.indent_dec();
        builder.line("}");
        
        // Generate break label
        builder.line(&format!("{}:;", loop_ctx.break_label));
        
        self.context.exit_loop();
        Ok(())
    }
    
    /// Generate for loop
    fn generate_for_loop(&mut self, pattern: &Pattern, iterable: &Expr, body: &[Stmt], builder: &mut CCodeBuilder) -> CodegenResult<()> {
        // For now, generate a simple iteration pattern
        let iterable_code = self.expr_gen.generate_expression(iterable)?;
        
        match pattern {
            Pattern::Identifier { name, .. } => {
                let var_name = self.format_identifier(name);
                
                self.context.enter_loop(None);
                let loop_ctx = self.context.current_loop().unwrap().clone();
                
                builder.line(&format!("for (auto {} : {}) {{", var_name, iterable_code));
                builder.indent_inc();
                
                // Generate loop body
                for stmt in body {
                    self.generate_statement(stmt, builder)?;
                }
                
                // Generate continue label
                builder.line(&format!("{}:;", loop_ctx.continue_label));
                
                builder.indent_dec();
                builder.line("}");
                
                // Generate break label
                builder.line(&format!("{}:;", loop_ctx.break_label));
                
                self.context.exit_loop();
                Ok(())
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    "Complex patterns in for loops not yet supported".to_string()
                ))
            }
        }
    }
    
    /// Generate infinite loop
    fn generate_infinite_loop(&mut self, label: &Option<InternedString>, body: &[Stmt], builder: &mut CCodeBuilder) -> CodegenResult<()> {
        let label_str = label.as_ref().map(|l| self.format_identifier(l));
        
        self.context.enter_loop(label_str);
        let loop_ctx = self.context.current_loop().unwrap().clone();
        
        builder.line("while (true) {");
        builder.indent_inc();
        
        // Generate loop body
        for stmt in body {
            self.generate_statement(stmt, builder)?;
        }
        
        // Generate continue label
        builder.line(&format!("{}:;", loop_ctx.continue_label));
        
        builder.indent_dec();
        builder.line("}");
        
        // Generate break label
        builder.line(&format!("{}:;", loop_ctx.break_label));
        
        self.context.exit_loop();
        Ok(())
    }
    
    /// Generate match statement
    fn generate_match_statement(&mut self, expr: &Expr, arms: &[MatchArm], builder: &mut CCodeBuilder) -> CodegenResult<()> {
        let expr_code = self.expr_gen.generate_expression(expr)?;
        let temp_var = self.context.temp_var();
        
        builder.line(&format!("auto {} = {};", temp_var, expr_code));
        
        for (i, arm) in arms.iter().enumerate() {
            if i == 0 {
                builder.line("if (");
            } else {
                builder.line("} else if (");
            }
            
            // Generate pattern matching condition
            let pattern_code = self.generate_pattern_match(&arm.pattern, &temp_var)?;
            builder.push_str(&pattern_code);
            builder.line(") {");
            builder.indent_inc();
            
            // Generate guard condition if present
            if let Some(guard) = &arm.guard {
                let guard_code = self.expr_gen.generate_expression(guard)?;
                builder.line(&format!("if ({}) {{", guard_code));
                builder.indent_inc();
            }
            
            // Generate body
            self.generate_expression_statement(&arm.body, builder)?;
            
            if arm.guard.is_some() {
                builder.indent_dec();
                builder.line("}");
            }
            
            builder.indent_dec();
        }
        
        builder.line("}");
        Ok(())
    }
    
    /// Generate break statement
    fn generate_break_statement(&mut self, label: &Option<InternedString>, expr: Option<&Expr>, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        if let Some(value) = expr {
            let value_code = self.expr_gen.generate_expression(value)?;
            builder.line(&format!("break_value = {};", value_code));
        }
        
        if let Some(loop_ctx) = self.context.current_loop() {
            builder.line(&format!("goto {};", loop_ctx.break_label));
        } else {
            builder.line("break;");
        }
        
        Ok(())
    }
    
    /// Generate continue statement
    fn generate_continue_statement(&mut self, label: &Option<InternedString>, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        if let Some(loop_ctx) = self.context.current_loop() {
            builder.line(&format!("goto {};", loop_ctx.continue_label));
        } else {
            builder.line("continue;");
        }
        
        Ok(())
    }
    
    /// Generate return statement
    fn generate_return_statement(&mut self, expr: Option<&Expr>, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        if let Some(value) = expr {
            let value_code = self.expr_gen.generate_expression(value)?;
            builder.line(&format!("return {};", value_code));
        } else {
            builder.line("return;");
        }
        
        Ok(())
    }
    
    /// Generate block statement
    fn generate_block_statement(&mut self, statements: &[Stmt], builder: &mut CCodeBuilder) -> CodegenResult<()> {
        self.context.enter_scope();
        
        builder.line("{");
        builder.indent_inc();
        
        for stmt in statements {
            self.generate_statement(stmt, builder)?;
        }
        
        builder.indent_dec();
        builder.line("}");
        
        self.context.exit_scope();
        Ok(())
    }
    
    /// Generate tuple destructuring
    fn generate_tuple_destructuring(
        &mut self,
        patterns: &[Pattern],
        type_annotation: &Option<Type>,
        initializer: &Option<Expr>,
        is_mutable: bool,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        if let Some(init) = initializer {
            let init_code = self.expr_gen.generate_expression(init)?;
            let temp_var = self.context.temp_var();
            
            builder.line(&format!("auto {} = {};", temp_var, init_code));
            
            for (i, pattern) in patterns.iter().enumerate() {
                if let Pattern::Identifier { name, .. } = pattern {
                    let var_name = self.format_identifier(name);
                    builder.line(&format!("auto {} = {}.field_{};", var_name, temp_var, i));
                }
            }
            
            Ok(())
        } else {
            Err(CodegenError::UnsupportedFeature(
                "Tuple destructuring without initializer not supported".to_string()
            ))
        }
    }
    
    /// Generate struct destructuring
    fn generate_struct_destructuring(
        &mut self,
        path: &[InternedString],
        fields: &[FieldPattern],
        type_annotation: &Option<Type>,
        initializer: &Option<Expr>,
        is_mutable: bool,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        if let Some(init) = initializer {
            let init_code = self.expr_gen.generate_expression(init)?;
            let temp_var = self.context.temp_var();
            
            builder.line(&format!("auto {} = {};", temp_var, init_code));
            
            for field in fields {
                let field_name = self.format_identifier(&field.name);
                
                if let Some(pattern) = &field.pattern {
                    if let Pattern::Identifier { name, .. } = pattern {
                        let var_name = self.format_identifier(name);
                        builder.line(&format!("auto {} = {}.{};", var_name, temp_var, field_name));
                    }
                } else {
                    // Shorthand destructuring
                    builder.line(&format!("auto {} = {}.{};", field_name, temp_var, field_name));
                }
            }
            
            Ok(())
        } else {
            Err(CodegenError::UnsupportedFeature(
                "Struct destructuring without initializer not supported".to_string()
            ))
        }
    }
    
    /// Generate pattern match condition
    fn generate_pattern_match(&mut self, pattern: &Pattern, expr_var: &str) -> CodegenResult<String> {
        match pattern {
            Pattern::Literal { literal, .. } => {
                let literal_code = self.expr_gen.generate_literal(literal)?;
                Ok(format!("{} == {}", expr_var, literal_code))
            },
            Pattern::Identifier { name, .. } => {
                let var_name = self.format_identifier(name);
                Ok(format!("({} = {}, true)", var_name, expr_var))
            },
            Pattern::Wildcard { .. } => {
                Ok("true".to_string())
            },
            Pattern::Tuple { patterns, .. } => {
                let mut conditions = Vec::new();
                for (i, pattern) in patterns.iter().enumerate() {
                    let field_expr = format!("{}.field_{}", expr_var, i);
                    conditions.push(self.generate_pattern_match(pattern, &field_expr)?);
                }
                Ok(format!("({})", conditions.join(" && ")))
            },
            Pattern::Struct { path, fields, .. } => {
                let mut conditions = Vec::new();
                for field in fields {
                    let field_name = self.format_identifier(&field.name);
                    let field_expr = format!("{}.{}", expr_var, field_name);
                    
                    if let Some(pattern) = &field.pattern {
                        conditions.push(self.generate_pattern_match(pattern, &field_expr)?);
                    }
                }
                Ok(format!("({})", conditions.join(" && ")))
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    "Complex pattern matching not yet implemented".to_string()
                ))
            }
        }
    }
    
    /// Generate type name
    fn generate_type_name(&self, ty: &Type) -> CodegenResult<String> {
        match ty {
            Type::Primitive { kind, .. } => {
                let type_name = match kind {
                    PrimitiveType::I8 => "int8_t",
                    PrimitiveType::I16 => "int16_t",
                    PrimitiveType::I32 => "int32_t",
                    PrimitiveType::I64 => "int64_t",
                    PrimitiveType::I128 => "int128_t",
                    PrimitiveType::ISize => "intptr_t",
                    PrimitiveType::U8 => "uint8_t",
                    PrimitiveType::U16 => "uint16_t",
                    PrimitiveType::U32 => "uint32_t",
                    PrimitiveType::U64 => "uint64_t",
                    PrimitiveType::U128 => "uint128_t",
                    PrimitiveType::USize => "uintptr_t",
                    PrimitiveType::F32 => "float",
                    PrimitiveType::F64 => "double",
                    PrimitiveType::Bool => "bool",
                    PrimitiveType::Char => "char32_t",
                    PrimitiveType::Str => "prism_str_t",
                    PrimitiveType::Unit => "void",
                };
                Ok(type_name.to_string())
            },
            Type::Path { segments, .. } => {
                let type_name = segments.iter()
                    .map(|s| self.format_identifier(s))
                    .collect::<Vec<_>>()
                    .join("_");
                Ok(type_name)
            },
            Type::Reference { target_type, .. } => {
                let target = self.generate_type_name(target_type)?;
                Ok(format!("{}*", target))
            },
            Type::Array { element_type, .. } => {
                let element = self.generate_type_name(element_type)?;
                Ok(format!("{}*", element))
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    format!("Type not yet supported: {:?}", ty)
                ))
            }
        }
    }
    
    /// Format identifier
    fn format_identifier(&self, name: &InternedString) -> String {
        // For now, just use the ID as the identifier
        // In a real implementation, this would look up the actual string
        format!("id_{}", name.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::SymbolTable;
    use crate::lexer::Position;
    
    fn dummy_span() -> Span {
        Span::single(Position::new(1, 1, 0, 0))
    }
    
    fn create_test_context() -> CodegenContext {
        let symbol_table = SymbolTable::new();
        CodegenContext::new(symbol_table)
    }
    
    #[test]
    fn test_expression_statement() {
        let mut ctx = create_test_context();
        let mut gen = StatementGenerator::new(&mut ctx);
        let mut builder = CCodeBuilder::new();
        
        let expr = Expr::Literal {
            literal: Literal::Integer {
                value: "42".to_string(),
                base: crate::lexer::token::NumberBase::Decimal,
                suffix: None,
            },
            span: dummy_span(),
        };
        
        let stmt = Stmt::Expression {
            expr,
            span: dummy_span(),
        };
        
        assert!(gen.generate_statement(&stmt, &mut builder).is_ok());
        assert!(builder.code().contains("42;"));
    }
    
    #[test]
    fn test_let_statement() {
        let mut ctx = create_test_context();
        let mut gen = StatementGenerator::new(&mut ctx);
        let mut builder = CCodeBuilder::new();
        
        let pattern = Pattern::Identifier {
            name: InternedString::new(1),
            is_mutable: false,
            span: dummy_span(),
        };
        
        let initializer = Some(Expr::Literal {
            literal: Literal::Integer {
                value: "42".to_string(),
                base: crate::lexer::token::NumberBase::Decimal,
                suffix: None,
            },
            span: dummy_span(),
        });
        
        let stmt = Stmt::Let {
            pattern,
            type_annotation: None,
            initializer,
            is_mutable: false,
            span: dummy_span(),
        };
        
        assert!(gen.generate_statement(&stmt, &mut builder).is_ok());
        assert!(builder.code().contains("auto id_1 = 42;"));
    }
    
    #[test]
    fn test_if_statement() {
        let mut ctx = create_test_context();
        let mut gen = StatementGenerator::new(&mut ctx);
        let mut builder = CCodeBuilder::new();
        
        let condition = Expr::Literal {
            literal: Literal::Bool(true),
            span: dummy_span(),
        };
        
        let then_block = vec![
            Stmt::Expression {
                expr: Expr::Literal {
                    literal: Literal::Integer {
                        value: "1".to_string(),
                        base: crate::lexer::token::NumberBase::Decimal,
                        suffix: None,
                    },
                    span: dummy_span(),
                },
                span: dummy_span(),
            }
        ];
        
        let stmt = Stmt::If {
            condition,
            then_block,
            else_block: None,
            span: dummy_span(),
        };
        
        assert!(gen.generate_statement(&stmt, &mut builder).is_ok());
        assert!(builder.code().contains("if (true)"));
    }
} 