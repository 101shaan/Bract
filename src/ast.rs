//! Abstract Syntax Tree (AST) definitions for the Prism programming language
//!
//! This module defines all AST node types following the grammar specification.
//! The design prioritizes:
//! - Memory efficiency (arena allocation ready)
//! - Fast traversal (compact enums)
//! - Position tracking (for error reporting)
//! - Type safety (proper Rust enums/structs)

use crate::lexer::{Position, TokenType};

/// Unique identifier for AST nodes (used for arena allocation)
pub type NodeId = u32;

/// Unique identifier for symbols
pub type SymbolId = u32;

/// File identifier for source location tracking
pub type FileId = usize;

/// Source span for error reporting and debugging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
    
    pub fn single(pos: Position) -> Self {
        Self { start: pos, end: pos }
    }
    
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: if self.start.offset < other.start.offset { self.start } else { other.start },
            end: if self.end.offset > other.end.offset { self.end } else { other.end },
        }
    }
}

/// Interned string for efficient storage of identifiers and literals
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternedString {
    pub id: u32,
}

impl InternedString {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,        // +
    Subtract,   // -
    Multiply,   // *
    Divide,     // /
    Modulo,     // %
    
    // Bitwise
    BitwiseAnd, // &
    BitwiseOr,  // |
    BitwiseXor, // ^
    LeftShift,  // <<
    RightShift, // >>
    
    // Logical
    LogicalAnd, // &&
    LogicalOr,  // ||
    
    // Comparison
    Equal,      // ==
    NotEqual,   // !=
    Less,       // <
    LessEqual,  // <=
    Greater,    // >
    GreaterEqual, // >=
    
    // Assignment (handled separately in statements)
    Assign,     // =
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,        // !
    Negate,     // -
    Plus,       // +
    BitwiseNot, // ~
    Dereference, // *
    AddressOf,  // &
    MutableRef, // &mut
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer {
        value: String,
        base: crate::lexer::token::NumberBase,
        suffix: Option<InternedString>,
    },
    Float {
        value: String,
        suffix: Option<InternedString>,
    },
    String {
        value: InternedString,
        raw: bool,
        raw_delimiter: Option<usize>,
    },
    Char(char),
    Bool(bool),
    Null,
}

/// Expression nodes - the core of the AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal values
    Literal {
        literal: Literal,
        span: Span,
    },
    
    /// Identifiers and paths
    Identifier {
        name: InternedString,
        span: Span,
    },
    
    /// Path expressions (module::item)
    Path {
        segments: Vec<InternedString>,
        span: Span,
    },
    
    /// Binary operations
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    
    /// Unary operations
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Function calls
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    
    /// Method calls (syntactic sugar for function calls)
    MethodCall {
        receiver: Box<Expr>,
        method: InternedString,
        args: Vec<Expr>,
        span: Span,
    },
    
    /// Field access
    FieldAccess {
        object: Box<Expr>,
        field: InternedString,
        span: Span,
    },
    
    /// Array/slice indexing
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    
    /// Type casting
    Cast {
        expr: Box<Expr>,
        target_type: Type,
        span: Span,
    },
    
    /// Parenthesized expressions
    Parenthesized {
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Array literals
    Array {
        elements: Vec<Expr>,
        span: Span,
    },
    
    /// Tuple expressions
    Tuple {
        elements: Vec<Expr>,
        span: Span,
    },
    
    /// Struct initialization
    StructInit {
        path: Vec<InternedString>,
        fields: Vec<FieldInit>,
        span: Span,
    },
    
    /// Range expressions
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
        span: Span,
    },
    
    /// Closure expressions
    Closure {
        is_move: bool,
        params: Vec<Parameter>,
        body: Box<Expr>,
        span: Span,
    },
    
    /// Block expressions
    Block {
        statements: Vec<Stmt>,
        trailing_expr: Option<Box<Expr>>,
        span: Span,
    },
    
    /// If expressions
    If {
        condition: Box<Expr>,
        then_block: Box<Expr>,
        else_block: Option<Box<Expr>>,
        span: Span,
    },
    
    /// Match expressions
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    
    /// Loop expressions
    Loop {
        label: Option<InternedString>,
        body: Box<Expr>,
        span: Span,
    },
    
    /// While expressions
    While {
        condition: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    
    /// For expressions
    For {
        pattern: Pattern,
        iterator: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    
    /// Break expressions
    Break {
        label: Option<InternedString>,
        value: Option<Box<Expr>>,
        span: Span,
    },
    
    /// Continue expressions
    Continue {
        label: Option<InternedString>,
        span: Span,
    },
    
    /// Return expressions
    Return {
        value: Option<Box<Expr>>,
        span: Span,
    },
    
    /// Box expressions (heap allocation)
    Box {
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Reference expressions
    Reference {
        is_mutable: bool,
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Dereference expressions
    Dereference {
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Question mark operator (error propagation)
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Await expressions (async)
    Await {
        expr: Box<Expr>,
        span: Span,
    },
    
    /// Macro invocations
    Macro {
        name: InternedString,
        args: Vec<TokenType>, // Raw tokens for macro expansion
        span: Span,
    },
}

/// Field initialization in struct literals
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInit {
    pub name: InternedString,
    pub value: Option<Expr>, // None for shorthand syntax
    pub span: Span,
}

/// Match arms for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// Statement AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Expression statement
    Expression {
        expr: Expr,
        span: Span,
    },
    /// Let binding: let [mut] pattern [: type] [= expr];
    Let {
        pattern: Pattern,
        type_annotation: Option<Type>,
        initializer: Option<Expr>,
        is_mutable: bool,
        span: Span,
    },
    /// Assignment: lvalue = expr;
    Assignment {
        target: Expr,
        value: Expr,
        span: Span,
    },
    /// Compound assignment: lvalue op= expr;
    CompoundAssignment {
        target: Expr,
        op: BinaryOp,
        value: Expr,
        span: Span,
    },
    /// If statement: if expr block [else block]
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Box<Stmt>>,
        span: Span,
    },
    /// While loop: while expr block
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    /// For loop: for pattern in expr block
    For {
        pattern: Pattern,
        iterable: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    /// Infinite loop: [label:] loop block
    Loop {
        label: Option<InternedString>,
        body: Vec<Stmt>,
        span: Span,
    },
    /// Match statement: match expr { arms }
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// Break statement: break [label] [expr];
    Break {
        label: Option<InternedString>,
        expr: Option<Expr>,
        span: Span,
    },
    /// Continue statement: continue [label];
    Continue {
        label: Option<InternedString>,
        span: Span,
    },
    /// Return statement: return [expr];
    Return {
        expr: Option<Expr>,
        span: Span,
    },
    /// Block statement: { statements... }
    Block {
        statements: Vec<Stmt>,
        span: Span,
    },
    /// Item declaration
    Item {
        item: Item,
        span: Span,
    },
    /// Empty statement
    Empty {
        span: Span,
    },
}

/// Top-level items (declarations)
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Function definitions
    Function {
        visibility: Visibility,
        name: InternedString,
        generics: Vec<GenericParam>,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Option<Expr>, // None for extern functions
        is_extern: bool,
        span: Span,
    },
    
    /// Struct definitions
    Struct {
        visibility: Visibility,
        name: InternedString,
        generics: Vec<GenericParam>,
        fields: StructFields,
        span: Span,
    },
    
    /// Enum definitions
    Enum {
        visibility: Visibility,
        name: InternedString,
        generics: Vec<GenericParam>,
        variants: Vec<EnumVariant>,
        span: Span,
    },
    
    /// Type aliases
    TypeAlias {
        visibility: Visibility,
        name: InternedString,
        generics: Vec<GenericParam>,
        target_type: Type,
        span: Span,
    },
    
    /// Constant declarations
    Const {
        visibility: Visibility,
        name: InternedString,
        type_annotation: Type,
        value: Expr,
        span: Span,
    },
    
    /// Module declarations
    Module {
        visibility: Visibility,
        name: InternedString,
        items: Option<Vec<Item>>, // None for external modules
        span: Span,
    },
    
    /// Implementation blocks
    Impl {
        generics: Vec<GenericParam>,
        target_type: Type,
        trait_ref: Option<Type>, // For trait implementations
        items: Vec<ImplItem>,
        span: Span,
    },
    
    /// Use declarations (imports)
    Use {
        path: Vec<InternedString>,
        alias: Option<InternedString>,
        span: Span,
    },
}

/// Struct field definitions
#[derive(Debug, Clone, PartialEq)]
pub enum StructFields {
    /// Named fields: struct Foo { x: i32, y: i32 }
    Named(Vec<StructField>),
    /// Tuple fields: struct Foo(i32, i32);
    Tuple(Vec<Type>),
    /// Unit struct: struct Foo;
    Unit,
}

/// Named struct field
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub visibility: Visibility,
    pub name: InternedString,
    pub field_type: Type,
    pub span: Span,
}

/// Enum variant definition
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: InternedString,
    pub fields: StructFields,
    pub discriminant: Option<Expr>, // For explicit discriminant values
    pub span: Span,
}

/// Items that can appear in impl blocks
#[derive(Debug, Clone, PartialEq)]
pub enum ImplItem {
    /// Method definitions
    Function {
        visibility: Visibility,
        name: InternedString,
        generics: Vec<GenericParam>,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Option<Expr>,
        span: Span,
    },
    
    /// Associated type definitions
    Type {
        visibility: Visibility,
        name: InternedString,
        generics: Vec<GenericParam>,
        target_type: Type,
        span: Span,
    },
    
    /// Associated constants
    Const {
        visibility: Visibility,
        name: InternedString,
        type_annotation: Type,
        value: Option<Expr>, // None for trait declarations
        span: Span,
    },
}

/// Function parameters
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub pattern: Pattern,
    pub type_annotation: Option<Type>,
    pub span: Span,
}

/// Generic parameters
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: InternedString,
    pub bounds: Vec<Type>, // Trait bounds
    pub default: Option<Type>,
    pub span: Span,
}

/// Visibility modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

/// Pattern matching patterns
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard pattern (_)
    Wildcard {
        span: Span,
    },
    
    /// Identifier patterns
    Identifier {
        name: InternedString,
        is_mutable: bool,
        span: Span,
    },
    
    /// Literal patterns
    Literal {
        literal: Literal,
        span: Span,
    },
    
    /// Tuple patterns
    Tuple {
        patterns: Vec<Pattern>,
        span: Span,
    },
    
    /// Array patterns
    Array {
        patterns: Vec<Pattern>,
        span: Span,
    },
    
    /// Struct patterns
    Struct {
        path: Vec<InternedString>,
        fields: Vec<FieldPattern>,
        rest: bool, // true if pattern ends with ..
        span: Span,
    },
    
    /// Enum patterns
    Enum {
        path: Vec<InternedString>,
        patterns: Option<Vec<Pattern>>,
        span: Span,
    },
    
    /// Reference patterns
    Reference {
        is_mutable: bool,
        pattern: Box<Pattern>,
        span: Span,
    },
    
    /// Range patterns
    Range {
        start: Option<Box<Pattern>>,
        end: Option<Box<Pattern>>,
        inclusive: bool,
        span: Span,
    },
    
    /// Or patterns (pattern1 | pattern2)
    Or {
        patterns: Vec<Pattern>,
        span: Span,
    },
}

/// Field patterns in struct patterns
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPattern {
    pub name: InternedString,
    pub pattern: Option<Pattern>, // None for shorthand syntax
    pub span: Span,
}

/// Type system representation
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Primitive types
    Primitive {
        kind: PrimitiveType,
        span: Span,
    },
    
    /// Path types (user-defined types)
    Path {
        segments: Vec<InternedString>,
        generics: Vec<Type>,
        span: Span,
    },
    
    /// Array types [T; N]
    Array {
        element_type: Box<Type>,
        size: Box<Expr>, // Constant expression
        span: Span,
    },
    
    /// Slice types &[T]
    Slice {
        element_type: Box<Type>,
        span: Span,
    },
    
    /// Tuple types
    Tuple {
        types: Vec<Type>,
        span: Span,
    },
    
    /// Function types
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        is_variadic: bool,
        span: Span,
    },
    
    /// Reference types
    Reference {
        is_mutable: bool,
        target_type: Box<Type>,
        span: Span,
    },
    
    /// Pointer types
    Pointer {
        is_mutable: bool,
        target_type: Box<Type>,
        span: Span,
    },
    
    /// Generic type parameters
    Generic {
        name: InternedString,
        span: Span,
    },
    
    /// Inferred types (type holes)
    Inferred {
        span: Span,
    },
    
    /// Never type (!)
    Never {
        span: Span,
    },
}

/// Primitive type kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    // Integers
    I8, I16, I32, I64, I128, ISize,
    U8, U16, U32, U64, U128, USize,
    
    // Floats
    F32, F64,
    
    // Other primitives
    Bool,
    Char,
    Str,
    Unit, // ()
}

/// Root AST node representing a complete source file
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub items: Vec<Item>,
    pub span: Span,
}

/// AST visitor trait for traversing the tree
pub trait AstVisitor<T> {
    fn visit_module(&mut self, module: &Module) -> T;
    fn visit_item(&mut self, item: &Item) -> T;
    fn visit_stmt(&mut self, stmt: &Stmt) -> T;
    fn visit_expr(&mut self, expr: &Expr) -> T;
    fn visit_pattern(&mut self, pattern: &Pattern) -> T;
    fn visit_type(&mut self, ty: &Type) -> T;
}

/// Mutable AST visitor for transforming the tree
pub trait AstVisitorMut<T> {
    fn visit_module_mut(&mut self, module: &mut Module) -> T;
    fn visit_item_mut(&mut self, item: &mut Item) -> T;
    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) -> T;
    fn visit_expr_mut(&mut self, expr: &mut Expr) -> T;
    fn visit_pattern_mut(&mut self, pattern: &mut Pattern) -> T;
    fn visit_type_mut(&mut self, ty: &mut Type) -> T;
}

impl Expr {
    /// Get the span of any expression
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Identifier { span, .. } => *span,
            Expr::Path { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::MethodCall { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Cast { span, .. } => *span,
            Expr::Parenthesized { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Tuple { span, .. } => *span,
            Expr::StructInit { span, .. } => *span,
            Expr::Range { span, .. } => *span,
            Expr::Closure { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Loop { span, .. } => *span,
            Expr::While { span, .. } => *span,
            Expr::For { span, .. } => *span,
            Expr::Break { span, .. } => *span,
            Expr::Continue { span, .. } => *span,
            Expr::Return { span, .. } => *span,
            Expr::Box { span, .. } => *span,
            Expr::Reference { span, .. } => *span,
            Expr::Dereference { span, .. } => *span,
            Expr::Try { span, .. } => *span,
            Expr::Await { span, .. } => *span,
            Expr::Macro { span, .. } => *span,
        }
    }
    
    /// Check if expression is a literal
    pub fn is_literal(&self) -> bool {
        matches!(self, Expr::Literal { .. })
    }
    
    /// Check if expression is a simple identifier
    pub fn is_identifier(&self) -> bool {
        matches!(self, Expr::Identifier { .. })
    }
    
    /// Check if expression has side effects
    pub fn has_side_effects(&self) -> bool {
        match self {
            Expr::Literal { .. } | Expr::Identifier { .. } | Expr::Path { .. } => false,
            Expr::Call { .. } | Expr::MethodCall { .. } => true,
            Expr::Binary { left, right, .. } => left.has_side_effects() || right.has_side_effects(),
            Expr::Unary { expr, .. } => expr.has_side_effects(),
            _ => true, // Conservative default
        }
    }
}

impl Stmt {
    /// Get the span of this statement
    pub fn span(&self) -> Span {
        match self {
            Stmt::Expression { span, .. } => *span,
            Stmt::Let { span, .. } => *span,
            Stmt::Assignment { span, .. } => *span,
            Stmt::CompoundAssignment { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::Loop { span, .. } => *span,
            Stmt::Match { span, .. } => *span,
            Stmt::Break { span, .. } => *span,
            Stmt::Continue { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::Block { span, .. } => *span,
            Stmt::Item { span, .. } => *span,
            Stmt::Empty { span, .. } => *span,
        }
    }
    
    /// Check if this statement is an expression statement
    pub fn is_expression(&self) -> bool {
        matches!(self, Stmt::Expression { .. })
    }
    
    /// Check if this statement has side effects
    pub fn has_side_effects(&self) -> bool {
        match self {
            Stmt::Expression { expr, .. } => expr.has_side_effects(),
            Stmt::Let { .. } => true,
            Stmt::Assignment { .. } => true,
            Stmt::CompoundAssignment { .. } => true,
            Stmt::If { condition, then_block, else_block, .. } => {
                condition.has_side_effects() ||
                then_block.iter().any(|s| s.has_side_effects()) ||
                else_block.as_ref().map_or(false, |s| s.has_side_effects())
            },
            Stmt::While { condition, body, .. } => {
                condition.has_side_effects() || body.iter().any(|s| s.has_side_effects())
            },
            Stmt::For { iterable, body, .. } => {
                iterable.has_side_effects() || body.iter().any(|s| s.has_side_effects())
            },
            Stmt::Loop { body, .. } => body.iter().any(|s| s.has_side_effects()),
            Stmt::Match { expr, arms, .. } => {
                expr.has_side_effects() || arms.iter().any(|arm| {
                    arm.guard.as_ref().map_or(false, |g| g.has_side_effects()) ||
                    arm.body.has_side_effects()
                })
            },
            Stmt::Break { expr, .. } => expr.as_ref().map_or(false, |e| e.has_side_effects()),
            Stmt::Continue { .. } => false,
            Stmt::Return { expr, .. } => expr.as_ref().map_or(false, |e| e.has_side_effects()),
            Stmt::Block { statements, .. } => statements.iter().any(|s| s.has_side_effects()),
            Stmt::Item { .. } => true,
            Stmt::Empty { .. } => false,
        }
    }
}

impl Type {
    /// Get the span of any type
    pub fn span(&self) -> Span {
        match self {
            Type::Primitive { span, .. } => *span,
            Type::Path { span, .. } => *span,
            Type::Array { span, .. } => *span,
            Type::Slice { span, .. } => *span,
            Type::Tuple { span, .. } => *span,
            Type::Function { span, .. } => *span,
            Type::Reference { span, .. } => *span,
            Type::Pointer { span, .. } => *span,
            Type::Generic { span, .. } => *span,
            Type::Inferred { span, .. } => *span,
            Type::Never { span, .. } => *span,
        }
    }
    
    /// Check if type is primitive
    pub fn is_primitive(&self) -> bool {
        matches!(self, Type::Primitive { .. })
    }
    
    /// Check if type is a reference
    pub fn is_reference(&self) -> bool {
        matches!(self, Type::Reference { .. })
    }
}

impl Pattern {
    /// Get the span of any pattern
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard { span, .. } => *span,
            Pattern::Identifier { span, .. } => *span,
            Pattern::Literal { span, .. } => *span,
            Pattern::Tuple { span, .. } => *span,
            Pattern::Array { span, .. } => *span,
            Pattern::Struct { span, .. } => *span,
            Pattern::Enum { span, .. } => *span,
            Pattern::Reference { span, .. } => *span,
            Pattern::Range { span, .. } => *span,
            Pattern::Or { span, .. } => *span,
        }
    }
    
    /// Check if pattern binds any variables
    pub fn binds_variables(&self) -> bool {
        match self {
            Pattern::Identifier { .. } => true,
            Pattern::Tuple { patterns, .. } => patterns.iter().any(|p| p.binds_variables()),
            Pattern::Array { patterns, .. } => patterns.iter().any(|p| p.binds_variables()),
            Pattern::Struct { fields, .. } => fields.iter().any(|f| {
                f.pattern.as_ref().map_or(true, |p| p.binds_variables())
            }),
            Pattern::Reference { pattern, .. } => pattern.binds_variables(),
            Pattern::Or { patterns, .. } => patterns.iter().any(|p| p.binds_variables()),
            _ => false,
        }
    }
}

/// Default implementations for common cases
impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;

    fn dummy_position() -> Position {
        Position::new(1, 1, 0, 0)
    }

    fn dummy_span() -> Span {
        Span::single(dummy_position())
    }

    fn dummy_interned_string(id: u32) -> InternedString {
        InternedString::new(id)
    }

    #[test]
    fn test_span_creation_and_merging() {
        let pos1 = Position::new(1, 1, 0, 0);
        let pos2 = Position::new(1, 10, 9, 0);
        
        let span1 = Span::new(pos1, pos2);
        assert_eq!(span1.start, pos1);
        assert_eq!(span1.end, pos2);
        
        let span2 = Span::single(pos1);
        assert_eq!(span2.start, pos1);
        assert_eq!(span2.end, pos1);
        
        let pos3 = Position::new(2, 5, 20, 0);
        let span3 = Span::single(pos3);
        let merged = span1.merge(span3);
        
        assert_eq!(merged.start, pos1); // Earlier position
        assert_eq!(merged.end, pos3);   // Later position
    }

    #[test]
    fn test_interned_string() {
        let str1 = InternedString::new(42);
        let str2 = InternedString::new(42);
        let str3 = InternedString::new(43);
        
        assert_eq!(str1, str2);
        assert_ne!(str1, str3);
        assert_eq!(str1.id, 42);
    }

    #[test]
    fn test_binary_operators() {
        // Test that all binary operators are defined
        let ops = [
            BinaryOp::Add, BinaryOp::Subtract, BinaryOp::Multiply, BinaryOp::Divide,
            BinaryOp::Modulo, BinaryOp::BitwiseAnd, BinaryOp::BitwiseOr, BinaryOp::BitwiseXor,
            BinaryOp::LeftShift, BinaryOp::RightShift, BinaryOp::LogicalAnd, BinaryOp::LogicalOr,
            BinaryOp::Equal, BinaryOp::NotEqual, BinaryOp::Less, BinaryOp::LessEqual,
            BinaryOp::Greater, BinaryOp::GreaterEqual, BinaryOp::Assign,
        ];
        
        for op in &ops {
            // Just ensure they can be created and compared
            assert_eq!(*op, *op);
        }
    }

    #[test]
    fn test_expression_predicates() {
        let span = dummy_span();
        
        // Literal expression
        let lit_expr = Expr::Literal {
            literal: Literal::Bool(true),
            span,
        };
        assert!(lit_expr.is_literal());
        assert!(!lit_expr.is_identifier());
        assert!(!lit_expr.has_side_effects());
        
        // Identifier expression
        let id_expr = Expr::Identifier {
            name: dummy_interned_string(1),
            span,
        };
        assert!(!id_expr.is_literal());
        assert!(id_expr.is_identifier());
        assert!(!id_expr.has_side_effects());
        
        // Function call expression (has side effects)
        let call_expr = Expr::Call {
            callee: Box::new(id_expr.clone()),
            args: vec![lit_expr.clone()],
            span,
        };
        assert!(!call_expr.is_literal());
        assert!(!call_expr.is_identifier());
        assert!(call_expr.has_side_effects());
    }

    #[test]
    fn test_pattern_variable_binding() {
        let span = dummy_span();
        
        // Wildcard pattern
        let wildcard = Pattern::Wildcard { span };
        assert!(!wildcard.binds_variables());
        
        // Identifier pattern
        let id_pattern = Pattern::Identifier {
            name: dummy_interned_string(1),
            is_mutable: false,
            span,
        };
        assert!(id_pattern.binds_variables());
        
        // Tuple pattern with identifier
        let tuple_pattern = Pattern::Tuple {
            patterns: vec![wildcard.clone(), id_pattern.clone()],
            span,
        };
        assert!(tuple_pattern.binds_variables());
    }

    #[test]
    fn test_type_predicates() {
        let span = dummy_span();
        
        // Primitive type
        let prim_type = Type::Primitive {
            kind: PrimitiveType::I32,
            span,
        };
        assert!(prim_type.is_primitive());
        assert!(!prim_type.is_reference());
        
        // Reference type
        let ref_type = Type::Reference {
            is_mutable: false,
            target_type: Box::new(prim_type.clone()),
            span,
        };
        assert!(!ref_type.is_primitive());
        assert!(ref_type.is_reference());
    }
} 