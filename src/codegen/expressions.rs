//! Expression Code Generation for Prism
//!
//! This module handles the translation of Prism expressions to C code.
//! It manages operator precedence, type conversions, and complex expression patterns.

use super::{CodegenContext, CCodeBuilder, CodegenResult, CodegenError};
use crate::ast::*;
use std::fmt::Write;

/// Expression code generator
pub struct ExpressionGenerator<'a> {
    /// Generation context
    context: &'a mut CodegenContext,
    /// Code builder for temporary code
    temp_builder: CCodeBuilder,
}

impl<'a> ExpressionGenerator<'a> {
    /// Create a new expression generator
    pub fn new(context: &'a mut CodegenContext) -> Self {
        Self {
            context,
            temp_builder: CCodeBuilder::new(),
        }
    }
    
    /// Generate C code for an expression
    pub fn generate_expression(&mut self, expr: &Expr) -> CodegenResult<String> {
        match expr {
            Expr::Literal { literal, .. } => {
                self.generate_literal(literal)
            },
            Expr::Identifier { name, .. } => {
                Ok(self.format_identifier(name))
            },
            Expr::Path { segments, .. } => {
                self.generate_path(segments)
            },
            Expr::Binary { left, op, right, .. } => {
                self.generate_binary_op(left, *op, right)
            },
            Expr::Unary { op, expr, .. } => {
                self.generate_unary_op(*op, expr)
            },
            Expr::Call { callee, args, .. } => {
                self.generate_function_call(callee, args)
            },
            Expr::MethodCall { receiver, method, args, .. } => {
                self.generate_method_call(receiver, method, args)
            },
            Expr::FieldAccess { object, field, .. } => {
                self.generate_field_access(object, field)
            },
            Expr::Index { object, index, .. } => {
                self.generate_array_index(object, index)
            },
            Expr::Cast { expr, target_type, .. } => {
                self.generate_cast(expr, target_type)
            },
            Expr::Parenthesized { expr, .. } => {
                let inner = self.generate_expression(expr)?;
                Ok(format!("({})", inner))
            },
            Expr::Array { elements, .. } => {
                self.generate_array_literal(elements)
            },
            Expr::Tuple { elements, .. } => {
                self.generate_tuple_literal(elements)
            },
            Expr::StructInit { path, fields, .. } => {
                self.generate_struct_init(path, fields)
            },
            Expr::Range { start, end, inclusive, .. } => {
                self.generate_range(start.as_deref(), end.as_deref(), *inclusive)
            },
            Expr::Block { statements, trailing_expr, .. } => {
                self.generate_block_expression(statements, trailing_expr.as_deref())
            },
            Expr::If { condition, then_block, else_block, .. } => {
                self.generate_if_expression(condition, then_block, else_block.as_deref())
            },
            Expr::Match { expr, arms, .. } => {
                self.generate_match_expression(expr, arms)
            },
            Expr::Return { value, .. } => {
                self.generate_return(value.as_deref())
            },
            Expr::Break { label, value, .. } => {
                self.generate_break(label, value.as_deref())
            },
            Expr::Continue { label, .. } => {
                self.generate_continue(label)
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    format!("Expression type not yet supported: {:?}", expr)
                ))
            }
        }
    }
    
    /// Generate literal value
    fn generate_literal(&mut self, literal: &Literal) -> CodegenResult<String> {
        match literal {
            Literal::Integer { value, base, suffix } => {
                let mut result = value.clone();
                
                // Handle different bases
                match base {
                    crate::lexer::token::NumberBase::Binary => {
                        if value.starts_with("0b") || value.starts_with("0B") {
                            // Convert binary to decimal for C
                            let binary_str = &value[2..];
                            if let Ok(decimal) = i64::from_str_radix(binary_str, 2) {
                                result = decimal.to_string();
                            }
                        }
                    },
                    crate::lexer::token::NumberBase::Octal => {
                        // C supports octal literals natively
                        if !value.starts_with("0") && !value.starts_with("0o") {
                            result = format!("0{}", value);
                        }
                    },
                    crate::lexer::token::NumberBase::Hexadecimal => {
                        // C supports hex literals natively
                        if !value.starts_with("0x") && !value.starts_with("0X") {
                            result = format!("0x{}", value);
                        }
                    },
                    crate::lexer::token::NumberBase::Decimal => {
                        // Already in correct format
                    }
                }
                
                // Add suffix if present
                if let Some(suffix) = suffix {
                    let suffix_str = self.format_identifier(suffix);
                    result.push_str(&suffix_str);
                }
                
                Ok(result)
            },
            Literal::Float { value, suffix } => {
                let mut result = value.clone();
                
                // Add suffix if present
                if let Some(suffix) = suffix {
                    let suffix_str = self.format_identifier(suffix);
                    result.push_str(&suffix_str);
                }
                
                Ok(result)
            },
            Literal::String { value, raw, raw_delimiter } => {
                // For now, generate a simple string literal
                // In a real implementation, this would handle escape sequences
                Ok(format!("\"{}\"", self.format_identifier(value)))
            },
            Literal::Char(c) => {
                Ok(format!("'{}'", c))
            },
            Literal::Bool(b) => {
                Ok(if *b { "true" } else { "false" }.to_string())
            },
            Literal::Null => {
                Ok("NULL".to_string())
            },
        }
    }
    
    /// Generate path expression
    fn generate_path(&mut self, segments: &[InternedString]) -> CodegenResult<String> {
        let path_str = segments.iter()
            .map(|s| self.format_identifier(s))
            .collect::<Vec<_>>()
            .join("_");
        Ok(path_str)
    }
    
    /// Generate binary operation
    fn generate_binary_op(&mut self, left: &Expr, op: BinaryOp, right: &Expr) -> CodegenResult<String> {
        let left_code = self.generate_expression(left)?;
        let right_code = self.generate_expression(right)?;
        
        let op_str = match op {
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Modulo => "%",
            BinaryOp::BitwiseAnd => "&",
            BinaryOp::BitwiseOr => "|",
            BinaryOp::BitwiseXor => "^",
            BinaryOp::LeftShift => "<<",
            BinaryOp::RightShift => ">>",
            BinaryOp::LogicalAnd => "&&",
            BinaryOp::LogicalOr => "||",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::Assign => "=",
        };
        
        Ok(format!("({} {} {})", left_code, op_str, right_code))
    }
    
    /// Generate unary operation
    fn generate_unary_op(&mut self, op: UnaryOp, expr: &Expr) -> CodegenResult<String> {
        let expr_code = self.generate_expression(expr)?;
        
        let result = match op {
            UnaryOp::Not => format!("!({})", expr_code),
            UnaryOp::Negate => format!("-({})", expr_code),
            UnaryOp::Plus => format!("+({})", expr_code),
            UnaryOp::BitwiseNot => format!("~({})", expr_code),
            UnaryOp::Dereference => format!("*({})", expr_code),
            UnaryOp::AddressOf => format!("&({})", expr_code),
            UnaryOp::MutableRef => format!("&({})", expr_code), // Same as AddressOf in C
        };
        
        Ok(result)
    }
    
    /// Generate function call
    fn generate_function_call(&mut self, callee: &Expr, args: &[Expr]) -> CodegenResult<String> {
        let callee_code = self.generate_expression(callee)?;
        
        let mut arg_codes = Vec::new();
        for arg in args {
            arg_codes.push(self.generate_expression(arg)?);
        }
        
        Ok(format!("{}({})", callee_code, arg_codes.join(", ")))
    }
    
    /// Generate method call
    fn generate_method_call(&mut self, receiver: &Expr, method: &InternedString, args: &[Expr]) -> CodegenResult<String> {
        let receiver_code = self.generate_expression(receiver)?;
        let method_name = self.format_identifier(method);
        
        let mut arg_codes = Vec::new();
        arg_codes.push(receiver_code); // First argument is the receiver
        for arg in args {
            arg_codes.push(self.generate_expression(arg)?);
        }
        
        Ok(format!("{}({})", method_name, arg_codes.join(", ")))
    }
    
    /// Generate field access
    fn generate_field_access(&mut self, object: &Expr, field: &InternedString) -> CodegenResult<String> {
        let object_code = self.generate_expression(object)?;
        let field_name = self.format_identifier(field);
        
        Ok(format!("{}.{}", object_code, field_name))
    }
    
    /// Generate array indexing
    fn generate_array_index(&mut self, object: &Expr, index: &Expr) -> CodegenResult<String> {
        let object_code = self.generate_expression(object)?;
        let index_code = self.generate_expression(index)?;
        
        Ok(format!("{}[{}]", object_code, index_code))
    }
    
    /// Generate type cast
    fn generate_cast(&mut self, expr: &Expr, target_type: &Type) -> CodegenResult<String> {
        let expr_code = self.generate_expression(expr)?;
        let type_name = self.generate_type_name(target_type)?;
        
        Ok(format!("(({}) {})", type_name, expr_code))
    }
    
    /// Generate array literal
    fn generate_array_literal(&mut self, elements: &[Expr]) -> CodegenResult<String> {
        let mut element_codes = Vec::new();
        for element in elements {
            element_codes.push(self.generate_expression(element)?);
        }
        
        Ok(format!("{{ {} }}", element_codes.join(", ")))
    }
    
    /// Generate tuple literal (as struct)
    fn generate_tuple_literal(&mut self, elements: &[Expr]) -> CodegenResult<String> {
        let mut element_codes = Vec::new();
        for element in elements {
            element_codes.push(self.generate_expression(element)?);
        }
        
        Ok(format!("{{ {} }}", element_codes.join(", ")))
    }
    
    /// Generate struct initialization
    fn generate_struct_init(&mut self, path: &[InternedString], fields: &[FieldInit]) -> CodegenResult<String> {
        let struct_name = path.iter()
            .map(|s| self.format_identifier(s))
            .collect::<Vec<_>>()
            .join("_");
        
        let mut field_codes = Vec::new();
        for field in fields {
            let field_name = self.format_identifier(&field.name);
            
            if let Some(value) = &field.value {
                let value_code = self.generate_expression(value)?;
                field_codes.push(format!(".{} = {}", field_name, value_code));
            } else {
                // Shorthand field initialization
                field_codes.push(format!(".{} = {}", field_name, field_name));
            }
        }
        
        Ok(format!("({}) {{ {} }}", struct_name, field_codes.join(", ")))
    }
    
    /// Generate range expression
    fn generate_range(&mut self, start: Option<&Expr>, end: Option<&Expr>, inclusive: bool) -> CodegenResult<String> {
        // For now, generate a simple range structure
        let start_code = if let Some(start) = start {
            self.generate_expression(start)?
        } else {
            "0".to_string()
        };
        
        let end_code = if let Some(end) = end {
            self.generate_expression(end)?
        } else {
            "0".to_string()
        };
        
        Ok(format!("prism_range({}, {}, {})", start_code, end_code, inclusive))
    }
    
    /// Generate block expression
    fn generate_block_expression(&mut self, statements: &[Stmt], trailing_expr: Option<&Expr>) -> CodegenResult<String> {
        // For block expressions, we need to use a statement expression (GCC extension)
        // or generate a temporary function
        let temp_var = self.context.temp_var();
        
        let mut code = format!("({{ ");
        
        // Generate statements
        for stmt in statements {
            // This would need to be implemented in statements.rs
            code.push_str("/* statement */; ");
        }
        
        // Generate trailing expression
        if let Some(expr) = trailing_expr {
            let expr_code = self.generate_expression(expr)?;
            code.push_str(&expr_code);
        } else {
            code.push_str("0"); // Default value
        }
        
        code.push_str(" })");
        Ok(code)
    }
    
    /// Generate if expression
    fn generate_if_expression(&mut self, condition: &Expr, then_block: &Expr, else_block: Option<&Expr>) -> CodegenResult<String> {
        let condition_code = self.generate_expression(condition)?;
        let then_code = self.generate_expression(then_block)?;
        
        if let Some(else_expr) = else_block {
            let else_code = self.generate_expression(else_expr)?;
            Ok(format!("({} ? {} : {})", condition_code, then_code, else_code))
        } else {
            Ok(format!("({} ? {} : 0)", condition_code, then_code))
        }
    }
    
    /// Generate match expression
    fn generate_match_expression(&mut self, expr: &Expr, arms: &[MatchArm]) -> CodegenResult<String> {
        let expr_code = self.generate_expression(expr)?;
        
        // For now, generate a simple switch-like structure
        // This is a simplified implementation
        let temp_var = self.context.temp_var();
        
        let mut code = format!("({{ ");
        code.push_str(&format!("typeof({}) {} = {}; ", expr_code, temp_var, expr_code));
        
        for (i, arm) in arms.iter().enumerate() {
            if i == 0 {
                code.push_str("(");
            } else {
                code.push_str(" : (");
            }
            
            // Generate pattern matching condition
            let pattern_code = self.generate_pattern_match(&arm.pattern, &temp_var)?;
            code.push_str(&pattern_code);
            code.push_str(") ? ");
            
            // Generate body
            let body_code = self.generate_expression(&arm.body)?;
            code.push_str(&body_code);
        }
        
        // Close all parentheses
        for _ in 0..arms.len() {
            code.push_str(")");
        }
        
        code.push_str(" })");
        Ok(code)
    }
    
    /// Generate return statement
    fn generate_return(&mut self, value: Option<&Expr>) -> CodegenResult<String> {
        if let Some(value) = value {
            let value_code = self.generate_expression(value)?;
            Ok(format!("return {}", value_code))
        } else {
            Ok("return".to_string())
        }
    }
    
    /// Generate break statement
    fn generate_break(&mut self, label: &Option<InternedString>, value: Option<&Expr>) -> CodegenResult<String> {
        if let Some(loop_ctx) = self.context.current_loop() {
            if let Some(value) = value {
                let value_code = self.generate_expression(value)?;
                Ok(format!("{{ break_value = {}; goto {}; }}", value_code, loop_ctx.break_label))
            } else {
                Ok(format!("goto {}", loop_ctx.break_label))
            }
        } else {
            Err(CodegenError::InternalError("Break outside of loop".to_string()))
        }
    }
    
    /// Generate continue statement
    fn generate_continue(&mut self, label: &Option<InternedString>) -> CodegenResult<String> {
        if let Some(loop_ctx) = self.context.current_loop() {
            Ok(format!("goto {}", loop_ctx.continue_label))
        } else {
            Err(CodegenError::InternalError("Continue outside of loop".to_string()))
        }
    }
    
    /// Generate pattern match condition
    fn generate_pattern_match(&mut self, pattern: &Pattern, expr_var: &str) -> CodegenResult<String> {
        match pattern {
            Pattern::Literal { literal, .. } => {
                let literal_code = self.generate_literal(literal)?;
                Ok(format!("{} == {}", expr_var, literal_code))
            },
            Pattern::Identifier { name, .. } => {
                let var_name = self.format_identifier(name);
                Ok(format!("({} = {}, true)", var_name, expr_var))
            },
            Pattern::Wildcard { .. } => {
                Ok("true".to_string())
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
    fn test_literal_generation() {
        let mut ctx = create_test_context();
        let mut gen = ExpressionGenerator::new(&mut ctx);
        
        let int_lit = Literal::Integer {
            value: "42".to_string(),
            base: crate::lexer::token::NumberBase::Decimal,
            suffix: None,
        };
        
        assert_eq!(gen.generate_literal(&int_lit).unwrap(), "42");
        
        let bool_lit = Literal::Bool(true);
        assert_eq!(gen.generate_literal(&bool_lit).unwrap(), "true");
    }
    
    #[test]
    fn test_binary_operation() {
        let mut ctx = create_test_context();
        let mut gen = ExpressionGenerator::new(&mut ctx);
        
        let left = Expr::Literal {
            literal: Literal::Integer {
                value: "5".to_string(),
                base: crate::lexer::token::NumberBase::Decimal,
                suffix: None,
            },
            span: dummy_span(),
        };
        
        let right = Expr::Literal {
            literal: Literal::Integer {
                value: "3".to_string(),
                base: crate::lexer::token::NumberBase::Decimal,
                suffix: None,
            },
            span: dummy_span(),
        };
        
        let result = gen.generate_binary_op(&left, BinaryOp::Add, &right).unwrap();
        assert_eq!(result, "(5 + 3)");
    }
    
    #[test]
    fn test_unary_operation() {
        let mut ctx = create_test_context();
        let mut gen = ExpressionGenerator::new(&mut ctx);
        
        let expr = Expr::Literal {
            literal: Literal::Integer {
                value: "42".to_string(),
                base: crate::lexer::token::NumberBase::Decimal,
                suffix: None,
            },
            span: dummy_span(),
        };
        
        let result = gen.generate_unary_op(UnaryOp::Negate, &expr).unwrap();
        assert_eq!(result, "-(42)");
    }
    
    #[test]
    fn test_function_call() {
        let mut ctx = create_test_context();
        let mut gen = ExpressionGenerator::new(&mut ctx);
        
        let callee = Expr::Identifier {
            name: InternedString::new(1),
            span: dummy_span(),
        };
        
        let args = vec![
            Expr::Literal {
                literal: Literal::Integer {
                    value: "42".to_string(),
                    base: crate::lexer::token::NumberBase::Decimal,
                    suffix: None,
                },
                span: dummy_span(),
            }
        ];
        
        let result = gen.generate_function_call(&callee, &args).unwrap();
        assert_eq!(result, "id_1(42)");
    }
} 