//! Item Code Generation
//!
//! This module handles code generation for different AST item types including
//! functions, structs, enums, constants, and modules.

use crate::ast::*;
use crate::semantic::SymbolTable;
use super::{CodegenContext, CodegenResult, CodegenError, CCodeBuilder, CodegenMetrics};
use std::collections::HashMap;

/// Item generator handles code generation for top-level items
pub struct ItemGenerator<'a> {
    /// Generation context
    context: &'a mut CodegenContext,
    /// Performance metrics
    metrics: &'a mut CodegenMetrics,
}

impl<'a> ItemGenerator<'a> {
    /// Create a new item generator
    pub fn new(context: &'a mut CodegenContext, metrics: &'a mut CodegenMetrics) -> Self {
        Self {
            context,
            metrics,
        }
    }

    /// Generate code for an item
    pub fn generate_item(&mut self, item: &Item, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        self.metrics.record_node();

        match item {
            Item::Function { .. } => {
                self.generate_function(item, builder)?;
            },
            Item::Struct { .. } => {
                self.generate_struct(item, builder)?;
            },
            Item::Enum { .. } => {
                self.generate_enum(item, builder)?;
            },
            Item::Const { .. } => {
                self.generate_const(item, builder)?;
            },
            Item::Module { .. } => {
                self.generate_nested_module(item, builder)?;
            },
            _ => {
                return Err(CodegenError::UnsupportedFeature(
                    format!("Item type not yet supported: {:?}", item)
                ));
            }
        }

        Ok(())
    }

    /// Generate forward declarations for items
    pub fn generate_forward_declarations(&mut self, items: &[Item], builder: &mut CCodeBuilder) -> CodegenResult<()> {
        builder.header_context();
        builder.comment("Forward declarations");

        for item in items {
            match item {
                Item::Function { name, .. } => {
                    // Function forward declarations will be generated during function processing
                },
                Item::Struct { name, .. } => {
                    let struct_name = self.format_identifier(name);
                    builder.line(&format!("typedef struct {} {};", struct_name, struct_name));
                },
                Item::Enum { name, .. } => {
                    let enum_name = self.format_identifier(name);
                    builder.line(&format!("typedef enum {} {};", enum_name, enum_name));
                },
                _ => {}
            }
        }

        builder.newline();
        builder.code_context();
        Ok(())
    }

    /// Generate a function definition
    fn generate_function(&mut self, item: &Item, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        if let Item::Function { name, params, return_type, body, .. } = item {
            let func_name = self.format_identifier(name);

            // Generate function signature
            let signature = self.generate_function_signature(name, params, return_type)?;

            // Generate function body
            self.context.current_function = Some(func_name.clone());
            self.context.enter_scope();

            if let Some(body_expr) = body {
                builder.function(&signature, |func_builder| {
                    if let Err(e) = self.generate_function_body(body_expr, func_builder) {
                        eprintln!("Error generating function body: {}", e);
                    }
                });
            } else {
                // External function declaration
                builder.header_context();
                builder.line(&format!("{};", signature));
                builder.code_context();
            }

            self.context.exit_scope();
            self.context.current_function = None;

            self.metrics.record_lines(10); // Estimate lines generated
        }

        Ok(())
    }

    /// Generate function signature
    fn generate_function_signature(&self, name: &InternedString, params: &[Parameter], return_type: &Option<Type>) -> CodegenResult<String> {
        let func_name = self.format_identifier(name);
        let mut signature = String::new();

        // Return type
        let ret_type = if let Some(ret_type) = return_type {
            self.generate_type_name(ret_type)?
        } else {
            "void".to_string()
        };
        signature.push_str(&ret_type);
        signature.push(' ');
        signature.push_str(&func_name);
        signature.push('(');

        // Parameters
        if params.is_empty() {
            signature.push_str("void");
        } else {
            for (i, param) in params.iter().enumerate() {
                if i > 0 {
                    signature.push_str(", ");
                }

                let param_type = if let Some(type_ann) = &param.type_annotation {
                    self.generate_type_name(type_ann)?
                } else {
                    return Err(CodegenError::TypeConversion(
                        "Function parameter must have explicit type".to_string()
                    ));
                };

                let param_name = self.generate_parameter_name(&param.pattern)?;
                signature.push_str(&format!("{} {}", param_type, param_name));
            }
        }
        signature.push(')');

        Ok(signature)
    }

    /// Generate function body
    fn generate_function_body(&mut self, body: &Expr, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        match body {
            Expr::Block { statements, trailing_expr, .. } => {
                // Generate statements
                for stmt in statements {
                    self.generate_statement(stmt, builder)?;
                }

                // Generate trailing expression as return
                if let Some(expr) = trailing_expr {
                    let expr_code = self.generate_expression(expr)?;
                    builder.line(&format!("return {};", expr_code));
                }
            },
            _ => {
                // Single expression body
                let expr_code = self.generate_expression(body)?;
                builder.line(&format!("return {};", expr_code));
            }
        }

        Ok(())
    }

    /// Generate a struct definition
    fn generate_struct(&mut self, item: &Item, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        if let Item::Struct { name, fields, .. } = item {
            let struct_name = self.format_identifier(name);

            builder.header_context();
            builder.line(&format!("struct {} {{", struct_name));
            builder.indent_inc();

            match fields {
                StructFields::Named(field_list) => {
                    for field in field_list {
                        let field_type = self.generate_type_name(&field.field_type)?;
                        let field_name = self.format_identifier(&field.name);
                        builder.line(&format!("{} {};", field_type, field_name));
                    }
                },
                StructFields::Tuple(types) => {
                    for (i, field_type) in types.iter().enumerate() {
                        let field_type_name = self.generate_type_name(field_type)?;
                        builder.line(&format!("{} field_{};", field_type_name, i));
                    }
                },
                StructFields::Unit => {
                    builder.line("char _unit_field; // Unit struct placeholder");
                }
            }

            builder.indent_dec();
            builder.line("};");
            builder.newline();
            builder.code_context();

            self.metrics.record_lines(5);
        }

        Ok(())
    }

    /// Generate an enum definition
    fn generate_enum(&mut self, item: &Item, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        if let Item::Enum { name, variants, .. } = item {
            let enum_name = self.format_identifier(name);

            builder.header_context();
            builder.comment(&format!("Enum: {}", enum_name));

            // Generate enum tag
            builder.line(&format!("typedef enum {} {{", format!("{}_Tag", enum_name)));
            builder.indent_inc();

            for (i, variant) in variants.iter().enumerate() {
                let variant_name = self.format_identifier(&variant.name);
                let tag_name = format!("{}_{}", enum_name.to_uppercase(), variant_name.to_uppercase());
                
                if i == variants.len() - 1 {
                    builder.line(&format!("{}", tag_name));
                } else {
                    builder.line(&format!("{},", tag_name));
                }
            }

            builder.indent_dec();
            builder.line(&format!("}} {}_Tag;", enum_name));
            builder.newline();

            // Generate enum union
            builder.line(&format!("typedef struct {} {{", enum_name));
            builder.indent_inc();
            builder.line(&format!("{}_Tag tag;", enum_name));
            builder.line("union {");
            builder.indent_inc();

            for variant in variants {
                let variant_name = self.format_identifier(&variant.name);
                
                match &variant.fields {
                    StructFields::Named(field_list) => {
                        builder.line(&format!("struct {{"));
                        builder.indent_inc();
                        for field in field_list {
                            let field_type = self.generate_type_name(&field.field_type)?;
                            let field_name = self.format_identifier(&field.name);
                            builder.line(&format!("{} {};", field_type, field_name));
                        }
                        builder.indent_dec();
                        builder.line(&format!("}} {};", variant_name));
                    },
                    StructFields::Tuple(types) => {
                        builder.line(&format!("struct {{"));
                        builder.indent_inc();
                        for (i, field_type) in types.iter().enumerate() {
                            let field_type_name = self.generate_type_name(field_type)?;
                            builder.line(&format!("{} field_{};", field_type_name, i));
                        }
                        builder.indent_dec();
                        builder.line(&format!("}} {};", variant_name));
                    },
                    StructFields::Unit => {
                        // No fields for unit variants
                    }
                }
            }

            builder.indent_dec();
            builder.line("} data;");
            builder.indent_dec();
            builder.line(&format!("}} {};", enum_name));
            builder.newline();
            builder.code_context();

            self.metrics.record_lines(15);
        }

        Ok(())
    }

    /// Generate a constant definition
    fn generate_const(&mut self, item: &Item, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        if let Item::Const { name, value, type_annotation, .. } = item {
            let const_name = self.format_identifier(name);

            let type_name = self.generate_type_name(type_annotation)?;

            let value_code = self.generate_expression(value)?;

            builder.header_context();
            builder.line(&format!("static const {} {} = {};", type_name, const_name, value_code));
            builder.code_context();

            self.metrics.record_lines(1);
        }

        Ok(())
    }

    /// Generate a nested module
    fn generate_nested_module(&mut self, item: &Item, builder: &mut CCodeBuilder) -> CodegenResult<()> {
        if let Item::Module { name, items: Some(items), .. } = item {
            let module_name = self.format_identifier(name);

            builder.comment(&format!("Module: {}", module_name));
            builder.newline();

            // Generate forward declarations for module items
            self.generate_forward_declarations(items, builder)?;

            // Generate module items
            for item in items {
                self.generate_item(item, builder)?;
            }

            builder.newline();
        }

        Ok(())
    }

    /// Generate a statement (delegated to statement generator)
    fn generate_statement(&mut self, _stmt: &Stmt, _builder: &mut CCodeBuilder) -> CodegenResult<()> {
        // This would delegate to the statement generator
        // For now, return an error to indicate it needs to be implemented
        Err(CodegenError::UnsupportedFeature(
            "Statement generation should be delegated to StatementGenerator".to_string()
        ))
    }

    /// Generate an expression (delegated to expression generator)
    fn generate_expression(&mut self, _expr: &Expr) -> CodegenResult<String> {
        // This would delegate to the expression generator
        // For now, return an error to indicate it needs to be implemented
        Err(CodegenError::UnsupportedFeature(
            "Expression generation should be delegated to ExpressionGenerator".to_string()
        ))
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
                    PrimitiveType::Str => "prism_str_t",
                    PrimitiveType::Unit => "void",
                };
                Ok(type_name.to_string())
            },
            Type::Array { element_type, .. } => {
                let elem_type = self.generate_type_name(element_type)?;
                Ok(format!("prism_array_t /* {} */", elem_type))
            },
            Type::Reference { target_type, .. } => {
                let target = self.generate_type_name(target_type)?;
                Ok(format!("{}*", target))
            },
            Type::Function { params, return_type, .. } => {
                let ret_type = self.generate_type_name(return_type)?;
                let mut param_types = Vec::new();
                
                for param in params {
                    param_types.push(self.generate_type_name(param)?);
                }
                
                if param_types.is_empty() {
                    Ok(format!("{}(*)(void)", ret_type))
                } else {
                    Ok(format!("{}(*)( {} )", ret_type, param_types.join(", ")))
                }
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

    /// Generate parameter name from pattern
    fn generate_parameter_name(&self, pattern: &Pattern) -> CodegenResult<String> {
        match pattern {
            Pattern::Identifier { name, .. } => {
                Ok(self.format_identifier(name))
            },
            _ => {
                Err(CodegenError::UnsupportedFeature(
                    "Only identifier patterns supported for parameters".to_string()
                ))
            }
        }
    }

    /// Format an identifier for C code
    fn format_identifier(&self, name: &InternedString) -> String {
        // Convert the interned string to a C-safe identifier
        // For now, just use the ID as a placeholder
        format!("prism_symbol_{}", name.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;
    use crate::semantic::SymbolTable;

    fn dummy_position() -> Position {
        Position::new(1, 1, 0, 0)
    }

    fn dummy_span() -> Span {
        Span::single(dummy_position())
    }

    #[test]
    fn test_item_generator_creation() {
        let symbol_table = SymbolTable::new();
        let mut context = CodegenContext::new(symbol_table);
        let mut metrics = CodegenMetrics::new();
        let _generator = ItemGenerator::new(&mut context, &mut metrics);
    }

    #[test]
    fn test_function_signature_generation() {
        let symbol_table = SymbolTable::new();
        let mut context = CodegenContext::new(symbol_table);
        let mut metrics = CodegenMetrics::new();
        let generator = ItemGenerator::new(&mut context, &mut metrics);

        let name = InternedString::new(42);
        let params = vec![];
        let return_type = Some(Type::Primitive { 
            kind: PrimitiveType::I32, 
            span: dummy_span() 
        });

        let signature = generator.generate_function_signature(&name, &params, &return_type).unwrap();
        assert!(signature.contains("int32_t"));
        assert!(signature.contains("prism_symbol_42"));
        assert!(signature.contains("void"));
    }

    #[test]
    fn test_type_name_generation() {
        let symbol_table = SymbolTable::new();
        let mut context = CodegenContext::new(symbol_table);
        let mut metrics = CodegenMetrics::new();
        let generator = ItemGenerator::new(&mut context, &mut metrics);

        let int_type = Type::Primitive { 
            kind: PrimitiveType::I32, 
            span: dummy_span() 
        };
        let type_name = generator.generate_type_name(&int_type).unwrap();
        assert_eq!(type_name, "int32_t");

        let str_type = Type::Primitive { 
            kind: PrimitiveType::Str, 
            span: dummy_span() 
        };
        let type_name = generator.generate_type_name(&str_type).unwrap();
        assert_eq!(type_name, "prism_str_t");
    }
} 