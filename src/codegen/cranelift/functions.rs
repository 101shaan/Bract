//! Function compilation for Cranelift
//!
//! This module handles function signature generation, calling conventions,
//! and function body compilation.

use crate::ast::{Item, Stmt, Expr, Type as AstType, Parameter};
use super::{CodegenResult, CodegenError, utils, expressions};
use cranelift::prelude::{types as ctypes, Type, Value, InstBuilder, AbiParam};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Module as CraneliftModule, Linkage};
use cranelift_codegen::Context;

/// Compile a function from Item::Function to Cranelift IR
pub fn compile_function_item(
    module: &mut dyn CraneliftModule,
    item: &Item,
    builder_context: &mut FunctionBuilderContext,
) -> CodegenResult<()> {
    match item {
        Item::Function { 
            name, 
            params, 
            return_type, 
            body, 
            is_extern,
            .. 
        } => {
            if *is_extern {
                // External functions just need declaration
                return Ok(());
            }
            
            let body_expr = body.as_ref().ok_or_else(|| {
                CodegenError::InternalError("Function body is missing".to_string())
            })?;
            
            compile_function_with_body(module, name, params, return_type, body_expr, builder_context)
        }
        _ => Err(CodegenError::InternalError("Expected function item".to_string())),
    }
}

/// Compile a function with its body
fn compile_function_with_body(
    module: &mut dyn CraneliftModule,
    name: &crate::ast::InternedString,
    params: &[Parameter],
    return_type: &Option<AstType>,
    body: &Expr,
    builder_context: &mut FunctionBuilderContext,
) -> CodegenResult<()> {
    // Create function signature
    let mut sig = module.make_signature();
    
    // Add parameters
    for param in params {
        if let Some(param_type) = &param.type_annotation {
            let cranelift_type = ast_type_to_cranelift_type(param_type)?;
            sig.params.push(AbiParam::new(cranelift_type));
        } else {
            return Err(CodegenError::InternalError("Parameter missing type annotation".to_string()));
        }
    }
    
    // Add return type
    if let Some(return_type) = return_type {
        let ret_type = ast_type_to_cranelift_type(return_type)?;
        sig.returns.push(AbiParam::new(ret_type));
    }
    
    // Get function name as string - for now just use "main" as placeholder
    let func_name = "main"; // TODO: properly convert InternedString to &str
    
    // Declare function
    let linkage = if func_name == "main" {
        Linkage::Export
    } else {
        Linkage::Local
    };
    
    let func_id = module.declare_function(func_name, linkage, &sig)
        .map_err(|e| CodegenError::InternalError(format!("Failed to declare function '{}': {}", func_name, e)))?;
    
    // Create function context
    let mut ctx = Context::new();
    ctx.func.signature = sig;
    
    // Create function builder
    let mut builder = FunctionBuilder::new(&mut ctx.func, builder_context);
    
    // Create entry block
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    builder.seal_block(entry_block);
    
    // Add function parameters as local variables
    // TODO: Implement parameter handling
    
    // Compile function body
    let result_value = compile_expression_to_value(&mut builder, body)?;
    
    // Return the result
    if return_type.is_some() {
        builder.ins().return_(&[result_value]);
    } else {
        builder.ins().return_(&[]);
    }
    
    // Finalize function
    builder.finalize();
    
    // Define function in module
    module.define_function(func_id, &mut ctx)
        .map_err(|e| CodegenError::InternalError(format!("Failed to define function '{}': {}", func_name, e)))?;
    
    Ok(())
}

/// Compile an expression to a Cranelift value
fn compile_expression_to_value(
    builder: &mut FunctionBuilder,
    expr: &Expr,
) -> CodegenResult<Value> {
    match expr {
        Expr::Literal { literal, .. } => {
            expressions::compile_literal(builder, literal)
        }
        Expr::Block { statements, trailing_expr, .. } => {
            // Compile all statements
            for stmt in statements {
                compile_statement(builder, stmt)?;
            }
            
            // Return trailing expression or unit
            if let Some(trailing) = trailing_expr {
                compile_expression_to_value(builder, trailing)
            } else {
                // Return unit/void - for now return 0
                Ok(builder.ins().iconst(ctypes::I32, 0))
            }
        }
        Expr::Return { value, .. } => {
            if let Some(value_expr) = value {
                let value = compile_expression_to_value(builder, value_expr)?;
                builder.ins().return_(&[value]);
                // This is unreachable, but we need to return a value
                Ok(builder.ins().iconst(ctypes::I32, 0))
            } else {
                builder.ins().return_(&[]);
                Ok(builder.ins().iconst(ctypes::I32, 0))
            }
        }
        _ => {
            // Use the expressions module for other expression types
            expressions::compile_expression(builder, expr)
        }
    }
}

/// Compile a single statement
fn compile_statement(
    builder: &mut FunctionBuilder,
    statement: &Stmt,
) -> CodegenResult<()> {
    match statement {
        Stmt::Return { expr, .. } => {
            if let Some(expr) = expr {
                let value = compile_expression_to_value(builder, expr)?;
                builder.ins().return_(&[value]);
            } else {
                builder.ins().return_(&[]);
            }
            Ok(())
        }
        Stmt::Expression { expr, .. } => {
            // Evaluate expression but ignore result
            compile_expression_to_value(builder, expr)?;
            Ok(())
        }
        _ => Err(CodegenError::UnsupportedFeature(
            format!("Statement not yet supported: {:?}", statement)
        )),
    }
}

/// Convert AST type to Cranelift type
fn ast_type_to_cranelift_type(ast_type: &AstType) -> CodegenResult<Type> {
    match ast_type {
        AstType::Primitive { kind, .. } => {
            use crate::ast::PrimitiveType;
            match kind {
                PrimitiveType::I8 => Ok(ctypes::I8),
                PrimitiveType::I16 => Ok(ctypes::I16),
                PrimitiveType::I32 => Ok(ctypes::I32),
                PrimitiveType::I64 => Ok(ctypes::I64),
                PrimitiveType::U8 => Ok(ctypes::I8),
                PrimitiveType::U16 => Ok(ctypes::I16),
                PrimitiveType::U32 => Ok(ctypes::I32),
                PrimitiveType::U64 => Ok(ctypes::I64),
                PrimitiveType::F32 => Ok(ctypes::F32),
                PrimitiveType::F64 => Ok(ctypes::F64),
                PrimitiveType::Bool => Ok(ctypes::I8),
                PrimitiveType::Char => Ok(ctypes::I8),
                PrimitiveType::Str => Ok(ctypes::I64), // String pointer
                PrimitiveType::Unit => Ok(ctypes::I32), // Unit type as i32 for now
                _ => Ok(ctypes::I64), // Default to pointer size
            }
        }
        AstType::Path { .. } => Ok(ctypes::I64), // Custom types as pointers
        AstType::Array { .. } => Ok(ctypes::I64), // Arrays as pointers
        AstType::Reference { .. } => Ok(ctypes::I64), // References as pointers
        AstType::Pointer { .. } => Ok(ctypes::I64), // Pointers
        AstType::Function { .. } => Ok(ctypes::I64), // Function pointers
        _ => Err(CodegenError::UnsupportedFeature(
            format!("Type not yet supported: {:?}", ast_type)
        )),
    }
}

// Make the literal compilation function available for expressions.rs
pub use expressions::compile_literal; 