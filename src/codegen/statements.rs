//! Statement Code Generation for Bract
//!
//! This module handles the translation of Bract statements to C code.
//! It manages control flow, variable declarations, and statement sequences.

use super::{CodegenContext, CCodeBuilder, CodegenResult, CodegenError, expressions::ExpressionGenerator};
use crate::ast::*;

/// Statement code generator
pub struct StatementGenerator;

impl StatementGenerator {
    /// Create a new statement generator
    pub fn new() -> Self {
        Self
    }

    /// Generate C code for a statement
    pub fn generate_statement(
        &mut self,
        stmt: &Stmt,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        match stmt {
            Stmt::Expression { expr, .. } => {
                self.generate_expression_statement(expr, context, builder)
            },
            Stmt::Let { pattern, type_annotation, initializer, is_mutable, .. } => {
                self.generate_let_statement(pattern, type_annotation, initializer, *is_mutable, context, builder)
            },
            Stmt::Assignment { target, value, .. } => {
                self.generate_assignment(target, value, context, builder)
            },
            Stmt::CompoundAssignment { target, op, value, .. } => {
                self.generate_compound_assignment(target, *op, value, context, builder)
            },
            Stmt::If { condition, then_block, else_block, .. } => {
                self.generate_if_statement(condition, then_block, else_block.as_ref(), context, builder)
            },
            Stmt::While { condition, body, .. } => {
                self.generate_while_loop(condition, body, context, builder)
            },
            Stmt::For { pattern, iterable, body, .. } => {
                self.generate_for_loop(pattern, iterable, body, context, builder)
            },
            Stmt::Loop { label, body, .. } => {
                self.generate_infinite_loop(label, body, context, builder)
            },
            Stmt::Match { expr, arms, .. } => {
                self.generate_match_statement(expr, arms, context, builder)
            },
            Stmt::Break { label, expr, .. } => {
                self.generate_break_statement(label, expr.as_ref(), context, builder)
            },
            Stmt::Continue { label, .. } => {
                self.generate_continue_statement(label, context, builder)
            },
            Stmt::Return { expr, .. } => {
                self.generate_return_statement(expr.as_ref(), context, builder)
            },
            Stmt::Block { statements, .. } => {
                self.generate_block_statement(statements, context, builder)
            },
            Stmt::Item { .. } => {
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
    fn generate_expression_statement(
        &mut self,
        expr: &Expr,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        let mut expr_gen = ExpressionGenerator::new(context);
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
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        match pattern {
            Pattern::Identifier { name, .. } => {
                let var_name = self.format_identifier(name);
                
                // Determine type
                let type_name = if let Some(type_ann) = type_annotation {
                    self.generate_type_name(type_ann)?
                } else if let Some(init) = initializer {
                    // Basic type inference based on initializer
                    self.infer_type_from_expression(init)?
                } else {
                    return Err(CodegenError::TypeConversion(
                        "Variable declaration requires either type annotation or initializer".to_string()
                    ));
                };
                
                // Generate declaration
                if let Some(init) = initializer {
                    let mut expr_gen = ExpressionGenerator::new(context);
                    let init_code = expr_gen.generate_expression(init)?;
                    builder.line(&format!("{} {} = {};", type_name, var_name, init_code));
                } else {
                    builder.line(&format!("{} {};", type_name, var_name));
                }
                
                Ok(())
            },
            Pattern::Tuple { patterns, .. } => {
                // Destructuring assignment
                self.generate_tuple_destructuring(patterns, type_annotation, initializer, is_mutable, context, builder)
            },
            Pattern::Struct { path, fields, .. } => {
                // Struct destructuring
                self.generate_struct_destructuring(path, fields, type_annotation, initializer, is_mutable, context, builder)
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    "Complex pattern in let statement not yet supported".to_string()
                ))
            }
        }
    }
    
    /// Generate assignment statement
    fn generate_assignment(
        &mut self,
        target: &Expr,
        value: &Expr,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        let mut expr_gen = ExpressionGenerator::new(context);
        let target_code = expr_gen.generate_expression(target)?;
        let value_code = expr_gen.generate_expression(value)?;
        
        builder.line(&format!("{} = {};", target_code, value_code));
        Ok(())
    }
    
    /// Generate compound assignment
    fn generate_compound_assignment(
        &mut self,
        target: &Expr,
        op: BinaryOp,
        value: &Expr,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        let mut expr_gen = ExpressionGenerator::new(context);
        let target_code = expr_gen.generate_expression(target)?;
        let value_code = expr_gen.generate_expression(value)?;
        
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
        else_block: Option<&Box<Stmt>>,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        let mut expr_gen = ExpressionGenerator::new(context);
        let condition_code = expr_gen.generate_expression(condition)?;
        
        builder.line(&format!("if ({}) {{", condition_code));
        builder.indent_inc();
        
        // Generate then block
        for stmt in then_block {
            self.generate_statement(stmt, context, builder)?;
        }
        
        builder.indent_dec();
        
        if let Some(else_stmt) = else_block {
            builder.line("} else {");
            builder.indent_inc();
            
            match else_stmt.as_ref() {
                Stmt::Block { statements, .. } => {
                    for stmt in statements {
                        self.generate_statement(stmt, context, builder)?;
                    }
                },
                _ => {
                    self.generate_statement(else_stmt, context, builder)?;
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
    fn generate_while_loop(
        &mut self,
        condition: &Expr,
        body: &[Stmt],
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        let mut expr_gen = ExpressionGenerator::new(context);
        let condition_code = expr_gen.generate_expression(condition)?;
        
        builder.line(&format!("while ({}) {{", condition_code));
        builder.indent_inc();
        
        // Generate loop body
        for stmt in body {
            self.generate_statement(stmt, context, builder)?;
        }
        
        builder.indent_dec();
        builder.line("}");
        
        Ok(())
    }
    
    /// Generate for loop
    fn generate_for_loop(
        &mut self,
        pattern: &Pattern,
        iterable: &Expr,
        body: &[Stmt],
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        let mut expr_gen = ExpressionGenerator::new(context);
        let iterable_code = expr_gen.generate_expression(iterable)?;
        
        // For now, assume iterating over arrays
        match pattern {
            Pattern::Identifier { name, .. } => {
                let var_name = self.format_identifier(name);
                let iter_var = format!("{}_iter", var_name);
                let len_var = format!("{}_len", var_name);
                
                builder.line(&format!("Bract_array_t {} = {};", iter_var, iterable_code));
                builder.line(&format!("size_t {} = {}.length;", len_var, iter_var));
                builder.line(&format!("for (size_t i = 0; i < {}; i++) {{", len_var));
                builder.indent_inc();
                
                // Extract current element
                builder.line(&format!("auto {} = {}.data[i];", var_name, iter_var));
                
                // Generate loop body
                for stmt in body {
                    self.generate_statement(stmt, context, builder)?;
                }
                
                builder.indent_dec();
                builder.line("}");
                
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
    fn generate_infinite_loop(
        &mut self,
        label: &Option<InternedString>,
        body: &[Stmt],
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        // Enter loop context
        let loop_label = label.as_ref().map(|l| self.format_identifier(l));
        context.enter_loop(loop_label);
        
        builder.line("while (1) {");
        builder.indent_inc();
        
        // Generate loop body
        for stmt in body {
            self.generate_statement(stmt, context, builder)?;
        }
        
        builder.indent_dec();
        builder.line("}");
        
        // Exit loop context
        context.exit_loop();
        
        Ok(())
    }
    
    /// Generate match statement
    fn generate_match_statement(
        &mut self,
        expr: &Expr,
        arms: &[MatchArm],
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        let mut expr_gen = ExpressionGenerator::new(context);
        let expr_code = expr_gen.generate_expression(expr)?;
        
        // For now, generate a simple switch statement
        builder.line(&format!("switch ({}) {{", expr_code));
        builder.indent_inc();
        
        for arm in arms {
            // Generate pattern matching
            let pattern_code = self.generate_pattern_match(&arm.pattern, &expr_code)?;
            builder.line(&format!("case {}: {{", pattern_code));
            builder.indent_inc();
            
            // Generate guard if present
            if let Some(guard) = &arm.guard {
                let mut guard_expr_gen = ExpressionGenerator::new(context);
                let guard_code = guard_expr_gen.generate_expression(guard)?;
                builder.line(&format!("if (!({}) ) {{ break; }}", guard_code));
            }
            
            // Generate arm body
            let mut arm_expr_gen = ExpressionGenerator::new(context);
            let body_code = arm_expr_gen.generate_expression(&arm.body)?;
            builder.line(&format!("{};", body_code));
            builder.line("break;");
            
            builder.indent_dec();
            builder.line("}");
        }
        
        builder.indent_dec();
        builder.line("}");
        
        Ok(())
    }
    
    /// Generate break statement
    fn generate_break_statement(
        &mut self,
        _label: &Option<InternedString>,
        expr: Option<&Expr>,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        if let Some(value) = expr {
            let mut expr_gen = ExpressionGenerator::new(context);
            let value_code = expr_gen.generate_expression(value)?;
            builder.line(&format!("{{ break_value = {}; break; }}", value_code));
        } else {
            builder.line("break;");
        }
        Ok(())
    }
    
    /// Generate continue statement
    fn generate_continue_statement(
        &mut self,
        _label: &Option<InternedString>,
        _context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        builder.line("continue;");
        Ok(())
    }
    
    /// Generate return statement
    fn generate_return_statement(
        &mut self,
        expr: Option<&Expr>,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        if let Some(value) = expr {
            let mut expr_gen = ExpressionGenerator::new(context);
            let value_code = expr_gen.generate_expression(value)?;
            builder.line(&format!("return {};", value_code));
        } else {
            builder.line("return;");
        }
        Ok(())
    }
    
    /// Generate block statement
    fn generate_block_statement(
        &mut self,
        statements: &[Stmt],
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        builder.line("{");
        builder.indent_inc();
        
        context.enter_scope();
        
        for stmt in statements {
            self.generate_statement(stmt, context, builder)?;
        }
        
        context.exit_scope();
        
        builder.indent_dec();
        builder.line("}");
        
        Ok(())
    }
    
    /// Generate tuple destructuring
    fn generate_tuple_destructuring(
        &mut self,
        patterns: &[Pattern],
        _type_annotation: &Option<Type>,
        initializer: &Option<Expr>,
        _is_mutable: bool,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        if let Some(init) = initializer {
            let mut expr_gen = ExpressionGenerator::new(context);
            let init_code = expr_gen.generate_expression(init)?;
            let temp_var = context.temp_var();
            
            // Generate temporary variable for the tuple
            builder.line(&format!("auto {} = {};", temp_var, init_code));
            
            // Destructure into individual variables
            for (i, pattern) in patterns.iter().enumerate() {
                if let Pattern::Identifier { name, .. } = pattern {
                    let var_name = self.format_identifier(name);
                    builder.line(&format!("auto {} = {}.field_{};", var_name, temp_var, i));
                }
            }
            
            Ok(())
        } else {
            Err(CodegenError::UnsupportedFeature(
                "Tuple destructuring requires initializer".to_string()
            ))
        }
    }
    
    /// Generate struct destructuring  
    fn generate_struct_destructuring(
        &mut self,
        _path: &[InternedString],
        fields: &[FieldPattern],
        _type_annotation: &Option<Type>,
        initializer: &Option<Expr>,
        _is_mutable: bool,
        context: &mut CodegenContext,
        builder: &mut CCodeBuilder
    ) -> CodegenResult<()> {
        if let Some(init) = initializer {
            let mut expr_gen = ExpressionGenerator::new(context);
            let init_code = expr_gen.generate_expression(init)?;
            let temp_var = context.temp_var();
            
            // Generate temporary variable for the struct
            builder.line(&format!("auto {} = {};", temp_var, init_code));
            
            // Destructure into individual variables
            for field in fields {
                let field_name = self.format_identifier(&field.name);
                if let Some(pattern) = &field.pattern {
                    if let Pattern::Identifier { name, .. } = pattern {
                        let var_name = self.format_identifier(name);
                        builder.line(&format!("auto {} = {}.{};", var_name, temp_var, field_name));
                    }
                } else {
                    // Shorthand syntax: field name becomes variable name
                    builder.line(&format!("auto {} = {}.{};", field_name, temp_var, field_name));
                }
            }
            
            Ok(())
        } else {
            Err(CodegenError::UnsupportedFeature(
                "Struct destructuring requires initializer".to_string()
            ))
        }
    }
    
    /// Generate pattern matching code
    fn generate_pattern_match(&mut self, pattern: &Pattern, _expr_var: &str) -> CodegenResult<String> {
        match pattern {
            Pattern::Literal { literal, .. } => {
                match literal {
                    Literal::Integer { value, .. } => Ok(value.clone()),
                    Literal::Bool(b) => Ok(if *b { "true".to_string() } else { "false".to_string() }),
                    Literal::Char(c) => Ok(format!("'{}'", c)),
                    _ => Err(CodegenError::UnsupportedFeature(
                        "Literal type not supported in pattern matching".to_string()
                    ))
                }
            },
            Pattern::Identifier { name, .. } => {
                // For now, assume this is a constant or enum variant
                Ok(self.format_identifier(name))
            },
            Pattern::Wildcard { .. } => {
                Ok("default".to_string())
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    "Pattern type not yet supported in match statements".to_string()
                ))
            }
        }
    }
    
    /// Generate a type name
    fn generate_type_name(&self, ty: &Type) -> CodegenResult<String> {
        match ty {
            Type::Primitive { kind, .. } => {
                let type_name = match kind {
                    PrimitiveType::I8 => "int8_t",
                    PrimitiveType::I16 => "int16_t",
                    PrimitiveType::I32 => "int32_t",
                    PrimitiveType::I64 => "int64_t",
                    PrimitiveType::I128 => "__int128", // GCC extension
                    PrimitiveType::ISize => "intptr_t",
                    PrimitiveType::U8 => "uint8_t",
                    PrimitiveType::U16 => "uint16_t",
                    PrimitiveType::U32 => "uint32_t",
                    PrimitiveType::U64 => "uint64_t",
                    PrimitiveType::U128 => "__uint128_t", // GCC extension
                    PrimitiveType::USize => "uintptr_t",
                    PrimitiveType::F32 => "float",
                    PrimitiveType::F64 => "double",
                    PrimitiveType::Bool => "bool",
                    PrimitiveType::Char => "char32_t",
                    PrimitiveType::Str => "Bract_str_t",
                    PrimitiveType::Unit => "void",
                };
                Ok(type_name.to_string())
            },
            Type::Array { element_type, .. } => {
                let elem_type = self.generate_type_name(element_type)?;
                Ok(format!("Bract_array_t /* {} */", elem_type))
            },
            Type::Reference { target_type, .. } => {
                let target = self.generate_type_name(target_type)?;
                Ok(format!("{}*", target))
            },
            Type::Path { segments, .. } => {
                if segments.len() == 1 {
                    Ok(self.format_identifier(&segments[0]))
                } else {
                    Ok(segments.iter()
                        .map(|s| self.format_identifier(s))
                        .collect::<Vec<_>>()
                        .join("_"))
                }
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    format!("Type not yet supported: {:?}", ty)
                ))
            }
        }
    }
    
    /// Format an identifier for C code
    fn format_identifier(&self, name: &InternedString) -> String {
        format!("Bract_symbol_{}", name.id)
    }
    
    /// Infer C type from expression
    fn infer_type_from_expression(&self, expr: &Expr) -> CodegenResult<String> {
        match expr {
            Expr::Literal { literal, .. } => {
                match literal {
                    Literal::Integer { .. } => Ok("int32_t".to_string()),
                    Literal::Float { .. } => Ok("double".to_string()),
                    Literal::String { .. } => Ok("const char*".to_string()),
                    Literal::Char(_) => Ok("char".to_string()),
                    Literal::Bool(_) => Ok("bool".to_string()),
                    Literal::Null => Ok("void*".to_string()),
                }
            },
            Expr::StructInit { path, .. } => {
                // For struct initialization, use the struct type
                let struct_name = path.iter()
                    .map(|s| format!("id_{}", s.id))
                    .collect::<Vec<_>>()
                    .join("_");
                Ok(struct_name)
            },
            Expr::Identifier { .. } => {
                // For now, assume generic type - would need symbol table lookup
                Ok("int32_t".to_string())
            },
            Expr::Binary { .. } => {
                // For binary operations, assume int for now
                Ok("int32_t".to_string())
            },
            _ => {
                // Default to int for other expressions
                Ok("int32_t".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;
    use crate::semantic::SymbolTable;
    
    fn dummy_span() -> Span {
        Span::single(Position::new(1, 1, 0, 0))
    }
    
    fn create_test_context() -> CodegenContext {
        let symbol_table = SymbolTable::new();
        CodegenContext::new(symbol_table)
    }
    
    #[test]
    fn test_expression_statement() {
        let mut stmt_gen = StatementGenerator::new();
        let mut context = create_test_context();
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
        
        stmt_gen.generate_statement(&stmt, &mut context, &mut builder).unwrap();
        assert!(builder.code().contains("42"));
    }
    
    #[test]
    fn test_let_statement() {
        let mut stmt_gen = StatementGenerator::new();
        let mut context = create_test_context();
        let mut builder = CCodeBuilder::new();
        
        let pattern = Pattern::Identifier {
            name: InternedString::new(1),
            is_mutable: false,
            span: dummy_span(),
        };
        
        let init = Some(Expr::Literal {
            literal: Literal::Integer {
                value: "10".to_string(),
                base: crate::lexer::token::NumberBase::Decimal,
                suffix: None,
            },
            span: dummy_span(),
        });
        
        let stmt = Stmt::Let {
            pattern,
            type_annotation: None,
            initializer: init,
            is_mutable: false,
            span: dummy_span(),
        };
        
        stmt_gen.generate_statement(&stmt, &mut context, &mut builder).unwrap();
        let code = builder.code();
        assert!(code.contains("Bract_symbol_1"));
        assert!(code.contains("10"));
    }
    
    #[test]
    fn test_if_statement() {
        let mut stmt_gen = StatementGenerator::new();
        let mut context = create_test_context();
        let mut builder = CCodeBuilder::new();
        
        let condition = Expr::Literal {
            literal: Literal::Bool(true),
            span: dummy_span(),
        };
        
        let then_stmt = Stmt::Expression {
            expr: Expr::Literal {
                literal: Literal::Integer {
                    value: "1".to_string(),
                    base: crate::lexer::token::NumberBase::Decimal,
                    suffix: None,
                },
                span: dummy_span(),
            },
            span: dummy_span(),
        };
        
        let stmt = Stmt::If {
            condition,
            then_block: vec![then_stmt],
            else_block: None,
            span: dummy_span(),
        };
        
        stmt_gen.generate_statement(&stmt, &mut context, &mut builder).unwrap();
        let code = builder.code();
        assert!(code.contains("if (true)"));
        assert!(code.contains("1;"));
    }
} 
