//! Function compilation for Cranelift
//!
//! This module handles function signature generation, calling conventions,
//! and function body compilation.

use crate::ast::{Item, Stmt, Expr, Type as AstType, Parameter, Pattern};
use super::{CodegenResult, CodegenError, utils, expressions};
use cranelift::prelude::{types as ctypes, Type, Value, InstBuilder, AbiParam, Signature};
use cranelift_codegen::ir::StackSlot;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Module as CraneliftModule, Linkage};
use cranelift_codegen::Context;
use std::collections::HashMap;

/// Local variable information for compilation
#[derive(Debug, Clone)]
pub struct LocalVariable {
    pub stack_slot: StackSlot,
    pub cranelift_type: Type,
    pub name: String, // For debugging
}

/// Variable context for function compilation
pub struct VariableContext {
    pub variables: HashMap<u32, LocalVariable>, // InternedString ID -> Variable info
    pub next_slot_id: u32,
}

impl VariableContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            next_slot_id: 0,
        }
    }

    pub fn declare_variable(
        &mut self,
        builder: &mut FunctionBuilder,
        name_id: u32,
        cranelift_type: Type,
        name: String,
    ) -> CodegenResult<StackSlot> {
        // Create stack slot for the variable
        let stack_slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            utils::type_size(cranelift_type) as u32,
        ));

        let local_var = LocalVariable {
            stack_slot,
            cranelift_type,
            name: name.clone(),
        };

        self.variables.insert(name_id, local_var);
        Ok(stack_slot)
    }

    pub fn get_variable(&self, name_id: u32) -> Option<&LocalVariable> {
        self.variables.get(&name_id)
    }
}

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
    _name: &crate::ast::InternedString,
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
    
    // Get function name as string - use ID for now since we don't have access to interner
    let func_name_string;
    let func_name = if params.is_empty() && return_type.is_some() {
        "main" // Assume main function for functions with no params and return type
    } else {
        // Use function ID as fallback for other functions
        func_name_string = format!("fn_{}", _name.id);
        &func_name_string
    };
    
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
    
    // Add block parameters to match function signature
    for param in params {
        if let Some(param_type) = &param.type_annotation {
            let cranelift_type = ast_type_to_cranelift_type(param_type)?;
            builder.append_block_param(entry_block, cranelift_type);
        }
    }
    
    builder.switch_to_block(entry_block);
    builder.seal_block(entry_block);
    
    // Initialize variable context
    let mut var_context = VariableContext::new();
    
    // Add function parameters as local variables
    let block_params: Vec<_> = builder.block_params(entry_block).to_vec();
    for (i, param) in params.iter().enumerate() {
        if let Pattern::Identifier { name, .. } = &param.pattern {
            let param_type = param.type_annotation.as_ref()
                .ok_or_else(|| CodegenError::InternalError("Parameter missing type annotation".to_string()))?;
            let cranelift_type = ast_type_to_cranelift_type(param_type)?;
            
            // Create stack slot for parameter
            let stack_slot = var_context.declare_variable(
                &mut builder,
                name.id,
                cranelift_type,
                format!("param_{}", i),
            )?;
            
            // Store the parameter value to the stack slot
            builder.ins().stack_store(block_params[i], stack_slot, 0);
        }
    }
    
    // Compile function body
    let result_value = compile_expression_with_variables(&mut builder, body, &mut var_context)?;
    
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

/// Compile an expression to a Cranelift value with variable context
fn compile_expression_with_variables(
    builder: &mut FunctionBuilder,
    expr: &Expr,
    var_context: &mut VariableContext,
) -> CodegenResult<Value> {
    match expr {
        Expr::Literal { literal, .. } => {
            expressions::compile_literal(builder, literal)
        }
        Expr::Identifier { name, .. } => {
            // Look up variable
            let var = var_context.get_variable(name.id).ok_or_else(|| {
                CodegenError::InternalError(format!("Undefined variable: {:?}", name))
            })?;
            
            // Load from stack slot
            Ok(builder.ins().stack_load(var.cranelift_type, var.stack_slot, 0))
        }
        Expr::Binary { left, op, right, .. } => {
            let left_val = compile_expression_with_variables(builder, left, var_context)?;
            let right_val = compile_expression_with_variables(builder, right, var_context)?;
            
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
                _ => Err(CodegenError::UnsupportedFeature(
                    format!("Binary operator not supported: {:?}", op)
                )),
            }
        }
        Expr::Block { statements, trailing_expr, .. } => {
            // Compile all statements
            for stmt in statements {
                compile_statement_with_variables(builder, stmt, var_context)?;
            }
            
            // Return trailing expression or unit
            if let Some(trailing) = trailing_expr {
                compile_expression_with_variables(builder, trailing, var_context)
            } else {
                // Return unit/void - for now return 0
                Ok(builder.ins().iconst(ctypes::I32, 0))
            }
        }
        Expr::Return { value, .. } => {
            if let Some(value_expr) = value {
                let value = compile_expression_with_variables(builder, value_expr, var_context)?;
                builder.ins().return_(&[value]);
                // This is unreachable, but we need to return a value
                Ok(builder.ins().iconst(ctypes::I32, 0))
            } else {
                builder.ins().return_(&[]);
                Ok(builder.ins().iconst(ctypes::I32, 0))
            }
        }
        Expr::Parenthesized { expr, .. } => {
            // Parentheses are just for grouping - compile the inner expression
            compile_expression_with_variables(builder, expr, var_context)
        }
        Expr::Call { callee, args, .. } => {
            // Handle function calls
            compile_function_call_with_variables(builder, callee, args, var_context)
        }
        Expr::If { condition, then_block, else_block, .. } => {
            // Handle if expressions
            compile_if_expression_with_variables(builder, condition, then_block, else_block, var_context)
        }
        Expr::Unary { op, expr, .. } => {
            // Handle unary operations
            let operand_val = compile_expression_with_variables(builder, expr, var_context)?;
            
            use crate::ast::UnaryOp;
            match op {
                UnaryOp::Negate => {
                    // Negate: 0 - operand
                    let zero = builder.ins().iconst(ctypes::I32, 0);
                    Ok(builder.ins().isub(zero, operand_val))
                }
                UnaryOp::Not => {
                    // Logical not: operand == 0
                    let zero = builder.ins().iconst(ctypes::I32, 0);
                    Ok(builder.ins().icmp(cranelift::prelude::IntCC::Equal, operand_val, zero))
                }
                _ => Err(CodegenError::UnsupportedFeature(
                    format!("Unary operator not supported: {:?}", op)
                )),
            }
        }
        _ => {
            // Use the expressions module for other expression types
            expressions::compile_expression(builder, expr)
        }
    }
}

/// Compile a single statement with variable context
fn compile_statement_with_variables(
    builder: &mut FunctionBuilder,
    statement: &Stmt,
    var_context: &mut VariableContext,
) -> CodegenResult<()> {
    match statement {
        Stmt::Return { expr, .. } => {
            if let Some(expr) = expr {
                let value = compile_expression_with_variables(builder, expr, var_context)?;
                builder.ins().return_(&[value]);
            } else {
                builder.ins().return_(&[]);
            }
            Ok(())
        }
        Stmt::Expression { expr, .. } => {
            // Evaluate expression but ignore result
            compile_expression_with_variables(builder, expr, var_context)?;
            Ok(())
        }
        Stmt::Let { pattern, type_annotation, initializer, .. } => {
            // Handle variable declaration
            compile_let_statement(builder, pattern, type_annotation, initializer, var_context)
        }
        _ => Err(CodegenError::UnsupportedFeature(
            format!("Statement not yet supported: {:?}", statement)
        )),
    }
}

/// Compile a let statement (variable declaration)
fn compile_let_statement(
    builder: &mut FunctionBuilder,
    pattern: &Pattern,
    type_annotation: &Option<AstType>,
    initializer: &Option<Expr>,
    var_context: &mut VariableContext,
) -> CodegenResult<()> {
    match pattern {
        Pattern::Identifier { name, .. } => {
            // Determine variable type
            let var_type = if let Some(type_ann) = type_annotation {
                ast_type_to_cranelift_type(type_ann)?
            } else {
                // Infer type from initializer (for now, assume i32)
                ctypes::I32
            };
            
            // Create stack slot for variable
            let stack_slot = var_context.declare_variable(
                builder, 
                name.id, 
                var_type, 
                format!("var_{}", name.id) // Placeholder name
            )?;
            
            // Compile initializer if present
            if let Some(init_expr) = initializer {
                let init_value = compile_expression_with_variables(builder, init_expr, var_context)?;
                
                // Store initial value in stack slot
                builder.ins().stack_store(init_value, stack_slot, 0);
            }
            
            Ok(())
        }
        _ => Err(CodegenError::UnsupportedFeature(
            "Only identifier patterns supported for let statements".to_string()
        )),
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

/// Compile an expression to a Cranelift value (legacy version without variables)
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

/// Compile a function call with variable context
fn compile_function_call_with_variables(
    builder: &mut FunctionBuilder,
    callee: &Expr,
    args: &[Expr],
    var_context: &mut VariableContext,
) -> CodegenResult<Value> {
    // For now, only handle simple identifier function calls
    if let Expr::Identifier { name, .. } = callee {
        // Create function signature for the call
        let mut sig = Signature::new(cranelift_codegen::isa::CallConv::SystemV);
        
        // Add parameters (assume all i32 for now)
        for _ in args {
            sig.params.push(AbiParam::new(ctypes::I32));
        }
        
        // Add return type (assume i32)
        sig.returns.push(AbiParam::new(ctypes::I32));
        
        // Import the signature
        let sig_ref = builder.func.import_signature(sig);
        
        // Create external function reference
        let func_ref = builder.import_function(cranelift_codegen::ir::ExtFuncData {
            name: cranelift_codegen::ir::ExternalName::testcase(&format!("fn_{}", name.id)),
            signature: sig_ref,
            colocated: true,
        });
        
        // Compile arguments
        let mut arg_values = Vec::new();
        for arg in args {
            let arg_value = compile_expression_with_variables(builder, arg, var_context)?;
            arg_values.push(arg_value);
        }
        
        // Generate the call
        let call_inst = builder.ins().call(func_ref, &arg_values);
        let results = builder.inst_results(call_inst);
        
        if results.is_empty() {
            // No return value, return unit (0)
            Ok(builder.ins().iconst(ctypes::I32, 0))
        } else {
            // Return the first result
            Ok(results[0])
        }
    } else {
        Err(CodegenError::UnsupportedFeature(
            "Only simple identifier function calls are supported".to_string()
        ))
    }
}

/// Compile an if expression with variable context
fn compile_if_expression_with_variables(
    builder: &mut FunctionBuilder,
    condition: &Expr,
    then_block: &Expr,
    else_block: &Option<Box<Expr>>,
    var_context: &mut VariableContext,
) -> CodegenResult<Value> {
    // Compile the condition
    let condition_val = compile_expression_with_variables(builder, condition, var_context)?;
    
    // Create blocks for then, else, and merge
    let then_bb = builder.create_block();
    let else_bb = builder.create_block();
    let merge_bb = builder.create_block();
    
    // Add a parameter to the merge block for the result value
    let merge_param = builder.append_block_param(merge_bb, ctypes::I32);
    
    // Branch based on condition (non-zero means true)
    let zero = builder.ins().iconst(ctypes::I32, 0);
    let is_true = builder.ins().icmp(cranelift::prelude::IntCC::NotEqual, condition_val, zero);
    builder.ins().brif(is_true, then_bb, &[], else_bb, &[]);
    
    // Compile then block
    builder.switch_to_block(then_bb);
    let then_val = compile_expression_with_variables(builder, then_block, var_context)?;
    builder.ins().jump(merge_bb, &[then_val]);
    
    // Compile else block
    builder.switch_to_block(else_bb);
    let else_val = if let Some(else_expr) = else_block {
        compile_expression_with_variables(builder, else_expr, var_context)?
    } else {
        // No else block, return unit (0)
        builder.ins().iconst(ctypes::I32, 0)
    };
    builder.ins().jump(merge_bb, &[else_val]);
    
    // Switch to merge block and seal all blocks
    builder.switch_to_block(merge_bb);
    builder.seal_block(then_bb);
    builder.seal_block(else_bb);
    builder.seal_block(merge_bb);
    
    // Return the merged value
    Ok(merge_param)
}

// Make the literal compilation function available for expressions.rs
pub use expressions::compile_literal; 