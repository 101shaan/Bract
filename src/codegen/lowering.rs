//! Lowering Pipeline: AST → Bract IR → Cranelift IR
//!
//! This module implements the lowering pipeline that converts high-level AST
//! constructs to performance-optimized machine code while preserving memory
//! management semantics and ownership information.
//!
//! Pipeline stages:
//! 1. AST → Bract IR: Preserve ownership, memory strategies, performance contracts
//! 2. Bract IR Optimization: Memory strategy optimization, dead code elimination
//! 3. Bract IR → Cranelift IR: Lower to machine-level operations
//! 4. Cranelift Optimization: Register allocation, instruction selection

use crate::ast::{
    Module, Item, Expr, Stmt, Type, Literal, BinaryOp, UnaryOp, 
    InternedString, Span, MemoryStrategy, Ownership, LifetimeId, PrimitiveType
};
use crate::codegen::bract_ir::{
    BIRModule, BIRFunction, BIRBasicBlock, BIRInstruction, BIROp, BIRValue, BIRType,
    BIRValueId, BIRBlockId, BIRFunctionId, CallingConvention, MemoryOrder,
    PerformanceContract, MemoryRegion
};
use crate::semantic::types::{TypeChecker, TypeError};
use cranelift_codegen::ir::{Function as ClifFunction, InstBuilder, Type as ClifType};
use cranelift_codegen::Context as ClifContext;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use std::collections::HashMap;

/// Result type for lowering operations
pub type LoweringResult<T> = Result<T, LoweringError>;

/// Lowering errors
#[derive(Debug, Clone, PartialEq)]
pub enum LoweringError {
    /// Type checking failed during lowering
    TypeError(TypeError),
    /// Unsupported AST construct
    UnsupportedConstruct(String, Span),
    /// Memory strategy conflict
    MemoryStrategyConflict(MemoryStrategy, MemoryStrategy, Span),
    /// Performance contract violation
    PerformanceViolation(String, u64, u64, Span),
    /// Cranelift error
    CraneliftError(String),
    /// Internal lowering error
    InternalError(String),
}

impl From<TypeError> for LoweringError {
    fn from(error: TypeError) -> Self {
        LoweringError::TypeError(error)
    }
}

/// AST to Bract IR lowerer with memory management awareness
pub struct AstToBirLowerer {
    /// Current module being lowered
    current_module: BIRModule,
    /// Type checker for semantic analysis
    type_checker: TypeChecker,
    /// Next unique IDs
    next_value_id: BIRValueId,
    next_block_id: BIRBlockId,
    next_function_id: BIRFunctionId,
    /// Current function context
    current_function: Option<BIRFunctionId>,
    /// Current basic block
    current_block: Option<BIRBlockId>,
    /// Value mapping from expressions to BIR values
    value_map: HashMap<*const Expr, BIRValueId>,
    /// Symbol table for variables
    symbol_table: HashMap<InternedString, BIRValueId>,
    /// Performance tracking
    performance_budget: u64,
    performance_used: u64,
}

impl AstToBirLowerer {
    pub fn new(type_checker: TypeChecker) -> Self {
        Self {
            current_module: BIRModule::new(),
            type_checker,
            next_value_id: 0,
            next_block_id: 0,
            next_function_id: 0,
            current_function: None,
            current_block: None,
            value_map: HashMap::new(),
            symbol_table: HashMap::new(),
            performance_budget: 10000, // Default performance budget
            performance_used: 0,
        }
    }
    
    /// Lower a complete module from AST to Bract IR
    pub fn lower_module(&mut self, module: &Module) -> LoweringResult<BIRModule> {
        // First pass: Type check the module
        self.type_checker.check_module(module)?;
        
        // Second pass: Lower items to Bract IR
        for item in &module.items {
            self.lower_item(item)?;
        }
        
        // Analyze performance characteristics
        self.current_module.analyze_performance();
        
        // Validate performance contracts
        self.validate_performance_contracts()?;
        
        Ok(std::mem::replace(&mut self.current_module, BIRModule::new()))
    }
    
    /// Lower a top-level item
    fn lower_item(&mut self, item: &Item) -> LoweringResult<()> {
        match item {
            Item::Function { 
                name, params, return_type, body: Some(body), span, .. 
            } => {
                self.lower_function(*name, params, return_type.as_ref(), body, *span)
            }
            Item::Function { body: None, .. } => {
                // External function - just register in symbol table
                Ok(())
            }
            Item::Struct { .. } => {
                // TODO: Implement struct lowering
                Ok(())
            }
            _ => {
                // TODO: Implement other item types
                Ok(())
            }
        }
    }
    
    /// Lower a function with memory management integration
    fn lower_function(
        &mut self,
        name: InternedString,
        params: &[crate::ast::Parameter],
        return_type: Option<&Type>,
        body: &Expr,
        span: Span,
    ) -> LoweringResult<()> {
        let function_id = self.next_function_id;
        self.next_function_id += 1;
        self.current_function = Some(function_id);
        
        // Create entry basic block
        let entry_block_id = self.next_block_id;
        self.next_block_id += 1;
        self.current_block = Some(entry_block_id);
        
        let mut entry_block = BIRBasicBlock::new(entry_block_id);
        
        // Lower parameters to BIR values
        let mut bir_params = Vec::new();
        for (i, param) in params.iter().enumerate() {
            let param_id = self.next_value_id;
            self.next_value_id += 1;
            
            // Convert AST type to BIR type
            let param_type = if let Some(type_annotation) = &param.type_annotation {
                self.ast_type_to_bir_type(type_annotation)?
            } else {
                // Use type inference result
                if let Some(inferred_type) = self.type_checker.get_expression_type(body) {
                    self.ast_type_to_bir_type(inferred_type)?
                } else {
                    // Default to i32 for now
                    BIRType::Integer {
                        width: 32,
                        signed: true,
                        memory_strategy: MemoryStrategy::Stack,
                    }
                }
            };
            
            let param_value = BIRValue::new(
                param_id,
                param_type,
                Ownership::Owned, // Parameters are owned by default
                param.span,
            );
            
            bir_params.push(param_value);
            
            // Add parameter to symbol table if it's a named identifier
            if let crate::ast::Pattern::Identifier { name: param_name, .. } = &param.pattern {
                self.symbol_table.insert(*param_name, param_id);
            }
        }
        
        // Lower function body
        let body_value_id = self.lower_expr(body, &mut entry_block)?;
        
        // Add return instruction
        let return_inst = BIRInstruction::new(
            self.get_next_instruction_id(),
            BIROp::Return { value: Some(body_value_id) },
            span,
        );
        entry_block.add_instruction(return_inst);
        
        // Determine return type
        let bir_return_type = if let Some(ret_type) = return_type {
            self.ast_type_to_bir_type(ret_type)?
        } else {
            // Infer from body expression
            if let Some(body_type) = self.type_checker.get_expression_type(body) {
                self.ast_type_to_bir_type(body_type)?
            } else {
                BIRType::Integer {
                    width: 32,
                    signed: true,
                    memory_strategy: MemoryStrategy::Stack,
                }
            }
        };
        
        // Create BIR function
        let bir_function = BIRFunction {
            id: function_id,
            name,
            params: bir_params,
            return_type: bir_return_type,
            calling_convention: CallingConvention::Fast, // Default to fast calling convention
            memory_regions: Vec::new(), // TODO: Analyze and add memory regions
            performance_contract: self.infer_performance_contract(&entry_block),
            blocks: vec![entry_block],
            span,
        };
        
        // Add function to module
        self.current_module.add_function(bir_function);
        self.current_function = None;
        self.current_block = None;
        
        Ok(())
    }
    
    /// Lower an expression to Bract IR
    fn lower_expr(&mut self, expr: &Expr, block: &mut BIRBasicBlock) -> LoweringResult<BIRValueId> {
        let value_id = match expr {
            Expr::Literal { literal, span } => {
                self.lower_literal(literal, *span, block)?
            }
            
            Expr::Identifier { name, span } => {
                if let Some(&value_id) = self.symbol_table.get(name) {
                    value_id
                } else {
                    return Err(LoweringError::InternalError(
                        format!("Undefined variable: {}", name.id)
                    ));
                }
            }
            
            Expr::Binary { left, right, op, span } => {
                self.lower_binary_expr(left, right, *op, *span, block)?
            }
            
            Expr::Unary { expr: inner_expr, op, span } => {
                self.lower_unary_expr(inner_expr, *op, *span, block)?
            }
            
            Expr::Call { callee, args, span } => {
                self.lower_call_expr(callee, args, *span, block)?
            }
            
            Expr::Reference { expr: inner_expr, is_mutable, span } => {
                self.lower_reference_expr(inner_expr, *is_mutable, *span, block)?
            }
            
            Expr::Dereference { expr: inner_expr, span } => {
                self.lower_dereference_expr(inner_expr, *span, block)?
            }
            
            _ => {
                // TODO: Implement remaining expression types
                return Err(LoweringError::UnsupportedConstruct(
                    "Expression type not yet supported".to_string(),
                    expr.span(),
                ));
            }
        };
        
        // Cache the value mapping
        self.value_map.insert(expr as *const Expr, value_id);
        Ok(value_id)
    }
    
    /// Lower a literal expression
    fn lower_literal(
        &mut self,
        literal: &Literal,
        span: Span,
        block: &mut BIRBasicBlock,
    ) -> LoweringResult<BIRValueId> {
        let value_id = self.next_value_id;
        self.next_value_id += 1;
        
        let (bir_type, ownership) = match literal {
            Literal::Integer { .. } => (
                BIRType::Integer {
                    width: 32,
                    signed: true,
                    memory_strategy: MemoryStrategy::Stack,
                },
                Ownership::Owned,
            ),
            Literal::Float { .. } => (
                BIRType::Float {
                    width: 64,
                    memory_strategy: MemoryStrategy::Stack,
                },
                Ownership::Owned,
            ),
            Literal::Bool(_) => (
                BIRType::Bool {
                    memory_strategy: MemoryStrategy::Stack,
                },
                Ownership::Owned,
            ),
            _ => {
                return Err(LoweringError::UnsupportedConstruct(
                    "Literal type not yet supported".to_string(),
                    span,
                ));
            }
        };
        
        let value = BIRValue::new(value_id, bir_type, ownership, span);
        
        // For literals, we don't need explicit instructions in most cases
        // The value represents the compile-time constant
        
        Ok(value_id)
    }
    
    /// Lower a binary expression with memory strategy awareness
    fn lower_binary_expr(
        &mut self,
        left: &Expr,
        right: &Expr,
        op: BinaryOp,
        span: Span,
        block: &mut BIRBasicBlock,
    ) -> LoweringResult<BIRValueId> {
        let left_id = self.lower_expr(left, block)?;
        let right_id = self.lower_expr(right, block)?;
        
        let result_id = self.next_value_id;
        self.next_value_id += 1;
        
        let bir_op = match op {
            BinaryOp::Add => BIROp::Add { lhs: left_id, rhs: right_id },
            BinaryOp::Subtract => BIROp::Sub { lhs: left_id, rhs: right_id },
            BinaryOp::Multiply => BIROp::Mul { lhs: left_id, rhs: right_id },
            BinaryOp::Divide => BIROp::Div { lhs: left_id, rhs: right_id },
            BinaryOp::Equal => BIROp::Eq { lhs: left_id, rhs: right_id },
            BinaryOp::Less => BIROp::Lt { lhs: left_id, rhs: right_id },
            _ => {
                return Err(LoweringError::UnsupportedConstruct(
                    format!("Binary operator {:?} not yet supported", op),
                    span,
                ));
            }
        };
        
        // Infer result type based on operands
        let result_type = match op {
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual |
            BinaryOp::Greater | BinaryOp::GreaterEqual => {
                BIRType::Bool { memory_strategy: MemoryStrategy::Stack }
            }
            _ => {
                // For arithmetic operations, use the type of the left operand
                BIRType::Integer {
                    width: 32,
                    signed: true,
                    memory_strategy: MemoryStrategy::Stack,
                }
            }
        };
        
        let result_value = BIRValue::new(result_id, result_type, Ownership::Owned, span);
        
        let instruction = BIRInstruction::new(
            self.get_next_instruction_id(),
            bir_op,
            span,
        ).with_result(result_value);
        
        self.track_performance_cost(&instruction);
        block.add_instruction(instruction);
        
        Ok(result_id)
    }
    
    /// Lower a unary expression
    fn lower_unary_expr(
        &mut self,
        expr: &Expr,
        op: UnaryOp,
        span: Span,
        block: &mut BIRBasicBlock,
    ) -> LoweringResult<BIRValueId> {
        let expr_id = self.lower_expr(expr, block)?;
        
        match op {
            UnaryOp::Dereference => {
                // Dereference operation - load from memory
                let result_id = self.next_value_id;
                self.next_value_id += 1;
                
                let load_op = BIROp::Load {
                    address: expr_id,
                    memory_order: MemoryOrder::Relaxed,
                    bounds_check: true, // Always enable bounds checking for safety
                };
                
                let result_type = BIRType::Integer {
                    width: 32,
                    signed: true,
                    memory_strategy: MemoryStrategy::Stack,
                };
                
                let result_value = BIRValue::new(result_id, result_type, Ownership::Owned, span);
                
                let instruction = BIRInstruction::new(
                    self.get_next_instruction_id(),
                    load_op,
                    span,
                ).with_result(result_value);
                
                self.track_performance_cost(&instruction);
                block.add_instruction(instruction);
                
                Ok(result_id)
            }
            _ => {
                Err(LoweringError::UnsupportedConstruct(
                    format!("Unary operator {:?} not yet supported", op),
                    span,
                ))
            }
        }
    }
    
    /// Lower a function call expression
    fn lower_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
        block: &mut BIRBasicBlock,
    ) -> LoweringResult<BIRValueId> {
        // Lower arguments
        let mut arg_ids = Vec::new();
        for arg in args {
            let arg_id = self.lower_expr(arg, block)?;
            arg_ids.push(arg_id);
        }
        
        // For now, assume direct function calls
        // TODO: Handle function pointers and method calls
        let function_id = 0; // Placeholder
        
        let result_id = self.next_value_id;
        self.next_value_id += 1;
        
        let call_op = BIROp::Call {
            function: function_id,
            args: arg_ids,
            tail_call: false, // TODO: Implement tail call detection
        };
        
        let result_type = BIRType::Integer {
            width: 32,
            signed: true,
            memory_strategy: MemoryStrategy::Stack,
        };
        
        let result_value = BIRValue::new(result_id, result_type, Ownership::Owned, span);
        
        let instruction = BIRInstruction::new(
            self.get_next_instruction_id(),
            call_op,
            span,
        ).with_result(result_value);
        
        self.track_performance_cost(&instruction);
        block.add_instruction(instruction);
        
        Ok(result_id)
    }
    
    /// Lower a reference expression (&expr)
    fn lower_reference_expr(
        &mut self,
        expr: &Expr,
        is_mutable: bool,
        span: Span,
        block: &mut BIRBasicBlock,
    ) -> LoweringResult<BIRValueId> {
        let expr_id = self.lower_expr(expr, block)?;
        
        let result_id = self.next_value_id;
        self.next_value_id += 1;
        
        let borrow_op = BIROp::Borrow {
            source: expr_id,
            is_mutable,
            lifetime: LifetimeId::new(0), // TODO: Proper lifetime analysis
        };
        
        let result_type = BIRType::Reference {
            target_type: Box::new(BIRType::Integer {
                width: 32,
                signed: true,
                memory_strategy: MemoryStrategy::Stack,
            }),
            is_mutable,
            lifetime: LifetimeId::new(0),
            ownership: if is_mutable { Ownership::MutBorr } else { Ownership::Borrowed },
        };
        
        let result_value = BIRValue::new(result_id, result_type, Ownership::Borrowed, span);
        
        let instruction = BIRInstruction::new(
            self.get_next_instruction_id(),
            borrow_op,
            span,
        ).with_result(result_value);
        
        self.track_performance_cost(&instruction);
        block.add_instruction(instruction);
        
        Ok(result_id)
    }
    
    /// Lower a dereference expression (*expr)
    fn lower_dereference_expr(
        &mut self,
        expr: &Expr,
        span: Span,
        block: &mut BIRBasicBlock,
    ) -> LoweringResult<BIRValueId> {
        let expr_id = self.lower_expr(expr, block)?;
        
        let result_id = self.next_value_id;
        self.next_value_id += 1;
        
        let load_op = BIROp::Load {
            address: expr_id,
            memory_order: MemoryOrder::Relaxed,
            bounds_check: true,
        };
        
        let result_type = BIRType::Integer {
            width: 32,
            signed: true,
            memory_strategy: MemoryStrategy::Stack,
        };
        
        let result_value = BIRValue::new(result_id, result_type, Ownership::Owned, span);
        
        let instruction = BIRInstruction::new(
            self.get_next_instruction_id(),
            load_op,
            span,
        ).with_result(result_value);
        
        self.track_performance_cost(&instruction);
        block.add_instruction(instruction);
        
        Ok(result_id)
    }
    
    /// Convert AST type to BIR type with memory strategy preservation
    fn ast_type_to_bir_type(&self, ast_type: &Type) -> LoweringResult<BIRType> {
        match ast_type {
            Type::Primitive { kind, memory_strategy, .. } => {
                match kind {
                    PrimitiveType::I8 => Ok(BIRType::Integer { width: 8, signed: true, memory_strategy: *memory_strategy }),
                    PrimitiveType::I16 => Ok(BIRType::Integer { width: 16, signed: true, memory_strategy: *memory_strategy }),
                    PrimitiveType::I32 => Ok(BIRType::Integer { width: 32, signed: true, memory_strategy: *memory_strategy }),
                    PrimitiveType::I64 => Ok(BIRType::Integer { width: 64, signed: true, memory_strategy: *memory_strategy }),
                    PrimitiveType::U8 => Ok(BIRType::Integer { width: 8, signed: false, memory_strategy: *memory_strategy }),
                    PrimitiveType::U16 => Ok(BIRType::Integer { width: 16, signed: false, memory_strategy: *memory_strategy }),
                    PrimitiveType::U32 => Ok(BIRType::Integer { width: 32, signed: false, memory_strategy: *memory_strategy }),
                    PrimitiveType::U64 => Ok(BIRType::Integer { width: 64, signed: false, memory_strategy: *memory_strategy }),
                    PrimitiveType::F32 => Ok(BIRType::Float { width: 32, memory_strategy: *memory_strategy }),
                    PrimitiveType::F64 => Ok(BIRType::Float { width: 64, memory_strategy: *memory_strategy }),
                    PrimitiveType::Bool => Ok(BIRType::Bool { memory_strategy: *memory_strategy }),
                    _ => Err(LoweringError::UnsupportedConstruct(
                        format!("Primitive type {:?} not yet supported", kind),
                        ast_type.span(),
                    )),
                }
            }
            Type::Reference { target_type, is_mutable, lifetime, ownership, .. } => {
                Ok(BIRType::Reference {
                    target_type: Box::new(self.ast_type_to_bir_type(target_type)?),
                    is_mutable: *is_mutable,
                    lifetime: lifetime.unwrap_or(LifetimeId::new(0)),
                    ownership: ownership.clone(),
                })
            }
            Type::Pointer { target_type, is_mutable, memory_strategy, .. } => {
                Ok(BIRType::Pointer {
                    target_type: Box::new(self.ast_type_to_bir_type(target_type)?),
                    is_mutable: *is_mutable,
                    memory_strategy: *memory_strategy,
                    ownership: Ownership::Owned,
                })
            }
            _ => {
                Err(LoweringError::UnsupportedConstruct(
                    format!("AST type not yet supported: {:?}", ast_type),
                    ast_type.span(),
                ))
            }
        }
    }
    
    /// Infer performance contract for a function based on its body
    fn infer_performance_contract(&self, block: &BIRBasicBlock) -> Option<PerformanceContract> {
        let total_cost = block.total_cost();
        
        Some(PerformanceContract {
            max_allocation_cost: total_cost,
            max_stack_depth: 1024, // Conservative default
            max_execution_time: Some(total_cost * 100), // Rough estimate
            memory_bound: None,
        })
    }
    
    /// Track performance cost of an instruction
    fn track_performance_cost(&mut self, instruction: &BIRInstruction) {
        self.performance_used += instruction.cost_estimate;
    }
    
    /// Validate all performance contracts
    fn validate_performance_contracts(&self) -> LoweringResult<()> {
        if self.performance_used > self.performance_budget {
            return Err(LoweringError::PerformanceViolation(
                "Total performance cost exceeds budget".to_string(),
                self.performance_used,
                self.performance_budget,
                Span::single(crate::lexer::Position::start(0)),
            ));
        }
        Ok(())
    }
    
    /// Get next instruction ID
    fn get_next_instruction_id(&mut self) -> u32 {
        static mut INSTRUCTION_COUNTER: u32 = 0;
        unsafe {
            let id = INSTRUCTION_COUNTER;
            INSTRUCTION_COUNTER += 1;
            id
        }
    }
}

/// Bract IR to Cranelift IR lowerer with memory management integration
pub struct BirToClifLowerer {
    /// Cranelift context
    cranelift_context: ClifContext,
    /// Function builder context
    builder_context: FunctionBuilderContext,
    /// Value mapping from BIR to Cranelift
    value_map: HashMap<BIRValueId, cranelift_codegen::ir::Value>,
    /// Block mapping from BIR to Cranelift
    block_map: HashMap<BIRBlockId, cranelift_codegen::ir::Block>,
}

impl BirToClifLowerer {
    pub fn new() -> Self {
        Self {
            cranelift_context: ClifContext::new(),
            builder_context: FunctionBuilderContext::new(),
            value_map: HashMap::new(),
            block_map: HashMap::new(),
        }
    }
    
    /// Lower a BIR function to Cranelift IR
    pub fn lower_function(&mut self, bir_function: &BIRFunction) -> LoweringResult<ClifFunction> {
        // Create Cranelift function signature
        let mut signature = cranelift_codegen::ir::Signature::new(cranelift_codegen::isa::CallConv::SystemV);
        
        // Add parameters
        for param in &bir_function.params {
            let clif_type = param.value_type.to_cranelift_type()
                .map_err(|e| LoweringError::CraneliftError(e))?;
            signature.params.push(cranelift_codegen::ir::AbiParam::new(clif_type));
        }
        
        // Add return type
        let return_clif_type = bir_function.return_type.to_cranelift_type()
            .map_err(|e| LoweringError::CraneliftError(e))?;
        signature.returns.push(cranelift_codegen::ir::AbiParam::new(return_clif_type));
        
        // Create function
        let user_func_name = cranelift_codegen::ir::UserFuncName::user(0, bir_function.id);
        let mut function = ClifFunction::with_name_signature(user_func_name, signature);
        
        // Create function builder
        let mut builder = FunctionBuilder::new(&mut function, &mut self.builder_context);
        
        // Create entry block
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);
        
        // Lower basic blocks
        for (i, bir_block) in bir_function.blocks.iter().enumerate() {
            if i == 0 {
                // Entry block already created
                self.block_map.insert(bir_block.id, entry_block);
            } else {
                let clif_block = builder.create_block();
                self.block_map.insert(bir_block.id, clif_block);
            }
        }
        
        // Lower instructions
        for bir_block in &bir_function.blocks {
            let clif_block = self.block_map[&bir_block.id];
            builder.switch_to_block(clif_block);
            
            for instruction in &bir_block.instructions {
                // TODO: Implement instruction lowering
                // self.lower_instruction(instruction, &mut builder)?;
            }
            
            builder.seal_block(clif_block);
        }
        
        builder.finalize();
        Ok(function)
    }
    
    /// Lower a BIR instruction to Cranelift
    fn lower_instruction(
        &mut self,
        instruction: &BIRInstruction,
        builder: &mut FunctionBuilder,
    ) -> LoweringResult<()> {
        match &instruction.op {
            BIROp::Add { lhs, rhs } => {
                let lhs_val = self.value_map[lhs];
                let rhs_val = self.value_map[rhs];
                let result = builder.ins().iadd(lhs_val, rhs_val);
                
                if let Some(result_value) = &instruction.result {
                    self.value_map.insert(result_value.id, result);
                }
            }
            
            BIROp::Return { value } => {
                if let Some(value_id) = value {
                    let return_val = self.value_map[value_id];
                    builder.ins().return_(&[return_val]);
                } else {
                    builder.ins().return_(&[]);
                }
            }
            
            BIROp::Load { address, bounds_check, .. } => {
                let addr_val = self.value_map[address];
                
                // Add bounds check if enabled
                if *bounds_check {
                    // TODO: Implement runtime bounds checking
                }
                
                let loaded_val = builder.ins().load(
                    cranelift_codegen::ir::types::I32, // TODO: Use correct type
                    cranelift_codegen::ir::MemFlags::new(),
                    addr_val,
                    0,
                );
                
                if let Some(result_value) = &instruction.result {
                    self.value_map.insert(result_value.id, loaded_val);
                }
            }
            
            _ => {
                // TODO: Implement remaining instructions
                return Err(LoweringError::UnsupportedConstruct(
                    format!("BIR instruction not yet supported: {:?}", instruction.op),
                    instruction.span,
                ));
            }
        }
        
        Ok(())
    }
}

/// Complete lowering pipeline orchestrator
pub struct LoweringPipeline {
    ast_to_bir: AstToBirLowerer,
    bir_to_clif: BirToClifLowerer,
}

impl LoweringPipeline {
    pub fn new(type_checker: TypeChecker) -> Self {
        Self {
            ast_to_bir: AstToBirLowerer::new(type_checker),
            bir_to_clif: BirToClifLowerer::new(),
        }
    }
    
    /// Execute the complete lowering pipeline
    pub fn lower_module(&mut self, module: &Module) -> LoweringResult<HashMap<BIRFunctionId, ClifFunction>> {
        // Stage 1: AST → Bract IR
        let bir_module = self.ast_to_bir.lower_module(module)?;
        
        // Stage 2: BIR Optimizations (TODO: Implement optimization passes)
        
        // Stage 3: Bract IR → Cranelift IR
        let mut clif_functions = HashMap::new();
        for (function_id, bir_function) in &bir_module.functions {
            let clif_function = self.bir_to_clif.lower_function(bir_function)?;
            clif_functions.insert(*function_id, clif_function);
        }
        
        Ok(clif_functions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::symbols::SymbolTable;
    
    #[test]
    fn test_basic_lowering() {
        let symbol_table = SymbolTable::new();
        let type_checker = TypeChecker::new(symbol_table);
        let mut pipeline = LoweringPipeline::new(type_checker);
        
        // TODO: Create test AST and verify lowering
    }
} 