//! Expression compilation for Cranelift
//!
//! This module handles expression code generation for all Bract expressions.

use crate::ast::{Expr, Literal};
use super::{CodegenResult, CodegenError, utils};
use cranelift::prelude::{types as ctypes, Type, Value, InstBuilder};
use cranelift_frontend::FunctionBuilder;

/// Compile an expression to Cranelift IR
pub fn compile_expression(
    builder: &mut FunctionBuilder,
    expression: &Expr,
) -> CodegenResult<Value> {
    match expression {
        Expr::Literal { literal, .. } => compile_literal(builder, literal),
        Expr::Identifier { name, .. } => compile_variable(builder, name),
        Expr::Binary { left, op, right, .. } => compile_binary_op(builder, left, op, right),
        Expr::Unary { op, expr, .. } => compile_unary_op(builder, op, expr),
        Expr::Call { callee, args, .. } => compile_function_call(builder, callee, args),
        Expr::Index { object, index, .. } => compile_array_access(builder, object, index),
        Expr::FieldAccess { object, field, .. } => compile_field_access(builder, object, field),
        Expr::Array { elements, .. } => compile_array_literal(builder, elements),
        Expr::Reference { expr, .. } => compile_address_of(builder, expr),
        Expr::StructInit { path, fields, .. } => {
            compile_struct_construction(builder, path, fields)
        }
        Expr::MethodCall { receiver, method, args, .. } => {
            compile_method_call(builder, receiver, method, args)
        }
        Expr::Parenthesized { expr, .. } => {
            // Parentheses are just for grouping - compile the inner expression
            compile_expression(builder, expr)
        }
        _ => Err(CodegenError::UnsupportedFeature(
            format!("Expression not yet supported: {:?}", expression)
        )),
    }
}

/// Compile a literal value
pub fn compile_literal(builder: &mut FunctionBuilder, literal: &Literal) -> CodegenResult<Value> {
    match literal {
        Literal::Integer { value, .. } => {
            // Parse the string value to get the actual integer
            let int_value: i64 = value.parse().map_err(|_| {
                CodegenError::InternalError(format!("Invalid integer literal: {}", value))
            })?;
            // For now, assume i32 for integer literals
            Ok(builder.ins().iconst(ctypes::I32, int_value))
        }
        Literal::Float { value, .. } => {
            // Parse the string value to get the actual float
            let float_value: f64 = value.parse().map_err(|_| {
                CodegenError::InternalError(format!("Invalid float literal: {}", value))
            })?;
            // For now, assume f64 for float literals
            Ok(builder.ins().f64const(float_value))
        }
        Literal::String { .. } => {
            // String literals require more complex handling (heap allocation)
            Err(CodegenError::UnsupportedFeature(
                "String literals not yet implemented".to_string()
            ))
        }
        Literal::Bool(value) => {
            // Booleans are represented as i8 (0 for false, 1 for true)
            let int_value = if *value { 1 } else { 0 };
            Ok(builder.ins().iconst(ctypes::I8, int_value))
        }
        Literal::Char(value) => {
            // Characters are represented as i8 (ASCII value)
            Ok(builder.ins().iconst(ctypes::I8, *value as i64))
        }
        Literal::Null => {
            // Null pointer
            Ok(builder.ins().iconst(ctypes::I64, 0))
        }
    }
}

/// Compile a variable reference
fn compile_variable(builder: &mut FunctionBuilder, _name: &crate::ast::InternedString) -> CodegenResult<Value> {
    // TODO: Implement variable lookup from symbol table
    Err(CodegenError::UnsupportedFeature(
        "Variable references not yet implemented".to_string()
    ))
}

/// Compile a binary operation
fn compile_binary_op(
    builder: &mut FunctionBuilder,
    left: &Expr,
    op: &crate::ast::BinaryOp,
    right: &Expr,
) -> CodegenResult<Value> {
    let left_val = compile_expression(builder, left)?;
    let right_val = compile_expression(builder, right)?;
    
    use crate::ast::BinaryOp;
    
    // For now, assume i32 arithmetic
    match op {
        BinaryOp::Add => Ok(builder.ins().iadd(left_val, right_val)),
        BinaryOp::Subtract => Ok(builder.ins().isub(left_val, right_val)),
        BinaryOp::Multiply => Ok(builder.ins().imul(left_val, right_val)),
        BinaryOp::Divide => Ok(builder.ins().sdiv(left_val, right_val)),
        BinaryOp::Modulo => Ok(builder.ins().srem(left_val, right_val)),
        BinaryOp::Equal => Ok(builder.ins().icmp(cranelift::prelude::IntCC::Equal, left_val, right_val)),
        BinaryOp::NotEqual => Ok(builder.ins().icmp(cranelift::prelude::IntCC::NotEqual, left_val, right_val)),
        BinaryOp::Less => Ok(builder.ins().icmp(cranelift::prelude::IntCC::SignedLessThan, left_val, right_val)),
        BinaryOp::LessEqual => Ok(builder.ins().icmp(cranelift::prelude::IntCC::SignedLessThanOrEqual, left_val, right_val)),
        BinaryOp::Greater => Ok(builder.ins().icmp(cranelift::prelude::IntCC::SignedGreaterThan, left_val, right_val)),
        BinaryOp::GreaterEqual => Ok(builder.ins().icmp(cranelift::prelude::IntCC::SignedGreaterThanOrEqual, left_val, right_val)),
        BinaryOp::LogicalAnd => Ok(builder.ins().band(left_val, right_val)),
        BinaryOp::LogicalOr => Ok(builder.ins().bor(left_val, right_val)),
        _ => Err(CodegenError::UnsupportedFeature(
            format!("Binary operator not supported: {:?}", op)
        )),
    }
}

/// Compile a unary operation
fn compile_unary_op(
    builder: &mut FunctionBuilder,
    op: &crate::ast::UnaryOp,
    operand: &Expr,
) -> CodegenResult<Value> {
    let operand_val = compile_expression(builder, operand)?;
    
    use crate::ast::UnaryOp;
    
    match op {
        UnaryOp::Negate => {
            // Negate: 0 - operand
            let zero = builder.ins().iconst(ctypes::I32, 0);
            Ok(builder.ins().isub(zero, operand_val))
        }
        UnaryOp::Not => {
            // Logical not: operand == 0
            let zero = builder.ins().iconst(ctypes::I8, 0);
            Ok(builder.ins().icmp(cranelift::prelude::IntCC::Equal, operand_val, zero))
        }
        _ => Err(CodegenError::UnsupportedFeature(
            format!("Unary operator not supported: {:?}", op)
        )),
    }
}

/// Compile a function call
fn compile_function_call(
    _builder: &mut FunctionBuilder,
    _function: &Expr,
    _args: &[Expr],
) -> CodegenResult<Value> {
    // TODO: Implement function calls
    Err(CodegenError::UnsupportedFeature(
        "Function calls not yet implemented".to_string()
    ))
}

/// Compile array access
fn compile_array_access(
    _builder: &mut FunctionBuilder,
    _array: &Expr,
    _index: &Expr,
) -> CodegenResult<Value> {
    // TODO: Implement array access
    Err(CodegenError::UnsupportedFeature(
        "Array access not yet implemented".to_string()
    ))
}

/// Compile field access
fn compile_field_access(
    _builder: &mut FunctionBuilder,
    _object: &Expr,
    _field: &crate::ast::InternedString,
) -> CodegenResult<Value> {
    // TODO: Implement field access
    Err(CodegenError::UnsupportedFeature(
        "Field access not yet implemented".to_string()
    ))
}

/// Compile array literal
fn compile_array_literal(
    _builder: &mut FunctionBuilder,
    _elements: &[Expr],
) -> CodegenResult<Value> {
    // TODO: Implement array literals
    Err(CodegenError::UnsupportedFeature(
        "Array literals not yet implemented".to_string()
    ))
}

/// Compile address-of operation
fn compile_address_of(
    _builder: &mut FunctionBuilder,
    _expr: &Expr,
) -> CodegenResult<Value> {
    // TODO: Implement address-of operation
    Err(CodegenError::UnsupportedFeature(
        "Address-of operation not yet implemented".to_string()
    ))
}

/// Compile struct construction
fn compile_struct_construction(
    _builder: &mut FunctionBuilder,
    _struct_name: &[crate::ast::InternedString],
    _fields: &[crate::ast::FieldInit],
) -> CodegenResult<Value> {
    // TODO: Implement struct construction
    Err(CodegenError::UnsupportedFeature(
        "Struct construction not yet implemented".to_string()
    ))
}

/// Compile method call
fn compile_method_call(
    _builder: &mut FunctionBuilder,
    _object: &Expr,
    _method: &crate::ast::InternedString,
    _args: &[Expr],
) -> CodegenResult<Value> {
    // TODO: Implement method calls
    Err(CodegenError::UnsupportedFeature(
        "Method calls not yet implemented".to_string()
    ))
} 