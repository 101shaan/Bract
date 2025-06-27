# Prism Compiler Architecture

> **Version:** 0.1 - Initial Design
>
> This document outlines the architecture of the Prism compiler, with a focus on achieving blazingly fast compilation speeds while maintaining memory safety and generating efficient code.

## Table of Contents

1. [Overview](#1-overview)
2. [Compiler Pipeline](#2-compiler-pipeline)
3. [AST & IR Data Structures](#3-ast--ir-data-structures)
4. [Symbol Table & Scope Management](#4-symbol-table--scope-management)
5. [Type System](#5-type-system)
6. [Error Reporting & Diagnostics](#6-error-reporting--diagnostics)
7. [Code Generation](#7-code-generation)
8. [Optimization Passes](#8-optimization-passes)
9. [Memory Management](#9-memory-management)
10. [Incremental Compilation](#10-incremental-compilation)
11. [Parallel Compilation](#11-parallel-compilation)
12. [Codebase Organization](#12-codebase-organization)

---

## 1. Overview

The Prism compiler (`prismc`) is designed with the following primary goals:

- **Speed**: Achieve sub-second compilation times for most projects
- **Safety**: Enforce memory safety without garbage collection
- **Modularity**: Support incremental and parallel compilation
- **Extensibility**: Enable easy addition of language features and optimizations
- **Diagnostics**: Provide clear, actionable error messages

The compiler is implemented in Rust to leverage its memory safety guarantees and performance characteristics.

---

## 2. Compiler Pipeline

### 2.1 Pipeline Stages

The compilation process follows these stages:

1. **Source Management**
   - File loading and caching
   - Encoding validation (UTF-8)
   - Source map generation

2. **Lexical Analysis**
   - Token generation
   - Comment handling (including doc comments)
   - Automatic semicolon insertion
   - Source location tracking

3. **Parsing**
   - Recursive descent parsing
   - Abstract Syntax Tree (AST) construction
   - Syntax error recovery
   - Macro expansion (future)

4. **Name Resolution**
   - Symbol table construction
   - Module/import resolution
   - Visibility checking
   - Basic name binding

5. **High-Level IR (HIR) Generation**
   - Desugaring complex syntax
   - Pattern matching compilation
   - Control flow normalization

6. **Type Checking**
   - Type inference
   - Type compatibility verification
   - Generic instantiation
   - Trait checking (future)

7. **Borrow Checking**
   - Ownership validation
   - Lifetime analysis
   - Aliasing rule enforcement

8. **Mid-Level IR (MIR) Generation**
   - Control flow graph construction
   - Memory management insertion
   - Optimization preparation

9. **Optimization**
   - Function-level optimizations
   - Inlining
   - Constant propagation
   - Dead code elimination

10. **Backend Code Generation**
    - Initial: C code generation
    - Final: LLVM IR generation
    - Target-specific optimizations

11. **Linking**
    - Library resolution
    - Binary generation

### 2.2 Intermediate Representations

The compiler uses multiple IRs to progressively lower the abstraction level:

- **AST**: Direct representation of source syntax
- **HIR**: Resolved names, desugared syntax
- **MIR**: Control flow graph, explicit memory operations
- **LLVM IR**: Low-level representation for code generation

### 2.3 Pipeline Coordination

- Each stage communicates via well-defined interfaces
- Stages can run in parallel where dependencies allow
- Incremental compilation skips unchanged components

---

## 3. AST & IR Data Structures

### 3.1 AST Node Design

AST nodes are designed for memory efficiency and fast traversal:

```rust
// Example AST node design
enum Expr {
    Literal(LiteralExpr),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    // ... other expression types
}

struct BinaryExpr {
    left: Box<Expr>,
    operator: BinaryOp,
    right: Box<Expr>,
    span: Span,
}

// Span tracks source location for error reporting
struct Span {
    start: BytePos,
    end: BytePos,
    file_id: FileId,
}
```

### 3.2 Memory-Efficient AST

- **Arena allocation**: Nodes allocated in typed arenas
- **Interning**: Strings and common structures are interned
- **Compact representation**: Bit-packed enums where appropriate

### 3.3 Expression Nodes

- Literals (integer, float, string, char, boolean, null)
- Binary operations (arithmetic, logical, comparison)
- Unary operations (negation, not, dereference, address-of)
- Function calls and method calls
- Field access and indexing
- Closures and anonymous functions
- Pattern matching expressions
- Range expressions
- Macro invocations
- Async/await expressions

### 3.4 Statement Nodes

- Variable declarations
- Assignments
- Control flow (if, while, for, loop)
- Block statements
- Return, break, continue
- Expression statements

### 3.5 Declaration Nodes

- Functions
- Structs and enums
- Type aliases
- Constants
- Modules
- Implementation blocks

### 3.6 HIR Structure

HIR transforms the AST by:
- Resolving all identifiers to unique symbols
- Expanding syntactic sugar
- Normalizing control flow
- Making implicit coercions explicit

### 3.7 MIR Structure

MIR uses a control flow graph (CFG) representation:

```rust
struct MirFunction {
    blocks: Vec<BasicBlock>,
    locals: Vec<LocalVar>,
    params: Vec<Parameter>,
}

struct BasicBlock {
    id: BlockId,
    statements: Vec<Statement>,
    terminator: Terminator,
}

enum Terminator {
    Return(Option<Operand>),
    Branch(BlockId),
    ConditionalBranch {
        condition: Operand,
        true_block: BlockId,
        false_block: BlockId,
    },
    // ... other terminators
}
```

---

## 4. Symbol Table & Scope Management

### 4.1 Symbol Table Design

The symbol table uses a hierarchical structure to represent nested scopes:

```rust
struct SymbolTable {
    scopes: Vec<Scope>,
    current_scope_id: ScopeId,
}

struct Scope {
    id: ScopeId,
    parent_id: Option<ScopeId>,
    symbols: HashMap<Identifier, Symbol>,
    children: Vec<ScopeId>,
}

struct Symbol {
    id: SymbolId,
    kind: SymbolKind,
    ty: Option<Type>,
    visibility: Visibility,
    span: Span,
}

enum SymbolKind {
    Variable { is_mutable: bool },
    Function,
    Type,
    Module,
    // ... other symbol kinds
}
```

### 4.2 Name Resolution Strategy

- **Two-pass approach**:
  1. First pass: Collect declarations and build scope structure
  2. Second pass: Resolve references to declarations

- **Import resolution**:
  - Resolve module dependencies first
  - Build a global symbol map for public items
  - Handle cyclic imports through forward declarations

### 4.3 Visibility Checking

- Public/private distinction enforced during name resolution
- Module hierarchy respected for visibility rules
- Re-exports handled through symbol aliasing

### 4.4 Efficient Lookup

- Hash-based symbol tables for O(1) lookups
- Caching frequently accessed symbols
- Pre-computing common lookups during compilation

---

## 5. Type System

### 5.1 Type Representation

Types are represented using a recursive structure:

```rust
enum Type {
    Primitive(PrimitiveType),
    Array(ArrayType),
    Slice(SliceType),
    Tuple(TupleType),
    Function(FunctionType),
    Struct(StructType),
    Enum(EnumType),
    Reference(ReferenceType),
    Generic(GenericType),
    // ... other type kinds
}

struct FunctionType {
    params: Vec<Type>,
    return_type: Box<Type>,
    is_variadic: bool,
}

struct GenericType {
    base: Box<Type>,
    args: Vec<Type>,
}
```

### 5.2 Type Inference Algorithm

The type inference system uses a modified Hindley-Milner algorithm:

1. **Constraint generation**:
   - Walk the AST and generate type constraints
   - Handle explicit type annotations
   - Generate placeholder types for inference variables

2. **Constraint solving**:
   - Unify types according to constraints
   - Resolve type variables
   - Handle subtyping and coercions

3. **Type completion**:
   - Fill in inferred types in the AST
   - Validate that all types are fully resolved
   - Generate errors for unresolvable constraints

### 5.3 Generic Instantiation

- Monomorphization approach for generics
- Type parameters replaced with concrete types
- Specialized versions generated for each unique instantiation

### 5.4 Type Checking

- Compatibility checks between expected and actual types
- Coercion insertion where allowed
- Trait bound verification (future)

### 5.5 Borrow Checking

- Region-based lifetime analysis
- Ownership tracking
- Mutable/immutable borrow validation
- Move semantics enforcement

---

## 6. Error Reporting & Diagnostics

### 6.1 Diagnostic System Design

```rust
struct Diagnostic {
    level: DiagnosticLevel,
    code: DiagnosticCode,
    message: String,
    spans: Vec<LabeledSpan>,
    notes: Vec<String>,
    suggestions: Vec<Suggestion>,
}

enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Help,
}

struct LabeledSpan {
    span: Span,
    label: String,
}

struct Suggestion {
    span: Span,
    replacement: String,
    message: String,
}
```

### 6.2 Error Recovery

- **Syntax error recovery**:
  - Skip to synchronization points (e.g., statement boundaries)
  - Insert missing tokens where unambiguous
  - Continue parsing to find more errors

- **Semantic error recovery**:
  - Use placeholder types for unresolved expressions
  - Continue type checking with best-guess types
  - Mark erroneous code but continue analysis

### 6.3 Error Presentation

- Colorized output with source context
- Precise error location highlighting
- Suggestions for fixes where possible
- Explanatory notes for complex errors

### 6.4 IDE Integration

- LSP-compatible error format
- Incremental error reporting
- Quick-fix suggestions

---

## 7. Code Generation

### 7.1 Initial Backend: C Transpilation

The first code generation backend will transpile Prism to C:

- **Advantages**:
  - Faster initial implementation
  - Portable to any platform with a C compiler
  - Leverages existing C optimization infrastructure

- **Implementation**:
  - Generate human-readable C code
  - Map Prism constructs to equivalent C patterns
  - Use macros and inline functions for language features

### 7.2 Final Backend: LLVM IR

The long-term backend will generate LLVM IR:

- **Advantages**:
  - More optimization opportunities
  - Direct control over code generation
  - Access to LLVM's extensive tooling

- **Implementation**:
  - Map MIR to LLVM IR constructs
  - Leverage LLVM's optimization passes
  - Generate debug information

### 7.3 Memory Model Implementation

- Stack allocation by default
- RAII pattern for resource management
- Explicit heap allocation via `box`
- Ownership transfer through moves

### 7.4 ABI Compatibility

- C-compatible FFI
- Platform-specific calling conventions
- Struct layout control via attributes

### 7.5 Cross-Compilation Support

- **Target triple handling**:
  - Parse and validate target triples (e.g., `x86_64-pc-windows-msvc`)
  - Configure toolchain based on target platform
  - Support cross-compilation from any host to any target

- **Platform-specific code generation**:
  - ABI adaptations for different platforms
  - OS-specific system calls and libraries
  - Architecture-specific optimizations

- **Standard library conditional compilation**:
  - Platform-specific implementations via `#[cfg(target_os = "...")]`
  - Feature detection and capability probing
  - Fallback implementations for portability

---

## 8. Optimization Passes

### 8.1 MIR-Level Optimizations

- **Constant propagation and folding**
- **Dead code elimination**
- **Common subexpression elimination**
- **Inlining**
- **Loop optimizations**:
  - Loop invariant code motion
  - Loop unrolling for small loops
- **Tail call optimization**

### 8.2 LLVM-Level Optimizations

- Leverage existing LLVM passes
- Custom passes for Prism-specific patterns
- Optimization level selection

### 8.3 Whole-Program Optimization

- Cross-module inlining
- Devirtualization
- Global dead code elimination
- Link-time optimization (LTO)

### 8.4 Optimization Pipeline

```
MIR Generation
  ↓
MIR Simplification
  ↓
Constant Propagation
  ↓
Dead Code Elimination
  ↓
Inlining
  ↓
Loop Optimization
  ↓
LLVM IR Generation
  ↓
LLVM Optimization Passes
  ↓
Target Code Generation
```

---

## 9. Memory Management

### 9.1 Compiler Memory Strategy

- **Arena allocation** for most compiler data structures
- **Region-based memory management** for compilation phases
- **Reference counting** for shared structures
- **Memory pools** for frequently allocated/deallocated objects

### 9.2 Memory-Efficient Data Structures

- **String interning** for identifiers and literals
- **Compact representations** for common patterns
- **Copy-on-write** for shared immutable data
- **Flyweight pattern** for repeated structures

### 9.3 Memory Profiling

- Built-in memory usage tracking
- Allocation hot-spot identification
- Peak memory usage optimization

---

## 10. Incremental Compilation

### 10.1 Dependency Tracking

- **Fine-grained dependency graph**:
  - Track dependencies at the function/item level
  - Record which items depend on which definitions
  - Track type dependencies separately from value dependencies

- **Fingerprinting**:
  - Compute content hashes for each item
  - Include all transitive dependencies in hash
  - Detect when recompilation is needed

### 10.2 Caching Strategy

- **On-disk cache**:
  - Store compiled artifacts in `.prism/cache`
  - Index by content hash
  - Versioned by compiler revision

- **In-memory cache**:
  - Keep recent compilation results in memory
  - Prioritize frequently used items
  - Share between related compilations

### 10.3 Minimal Recompilation

- Recompile only changed items and their dependents
- Preserve type information across compilations
- Reuse previous code generation where possible

### 10.4 Cross-Module Incremental Compilation

- Track dependencies across module boundaries
- Cache monomorphized generic instantiations
- Share compiled artifacts between related projects

---

## 11. Parallel Compilation

### 11.1 Task-Based Parallelism

- **Work stealing scheduler**:
  - Split compilation into independent tasks
  - Dynamically balance work across threads
  - Prioritize critical path tasks

- **Task types**:
  - Parsing
  - Type checking
  - Code generation
  - Optimization

### 11.2 Pipeline Parallelism

- Process multiple files simultaneously
- Pipeline different compilation stages
- Overlap I/O with computation

### 11.3 Dependency-Aware Scheduling

- Build dependency graph before scheduling
- Schedule independent tasks first
- Minimize blocking on dependencies

### 11.4 Thread Pool Management

- Adaptive thread count based on:
  - Available CPU cores
  - Memory constraints
  - I/O bottlenecks

### 11.5 Synchronization Strategy

- Lock-free data structures where possible
- Immutable shared data to minimize contention
- Batched updates to shared state

---

## 12. Codebase Organization

### 12.1 Module Structure

```
src/
├── main.rs                 # Compiler entry point
├── driver/                 # Compilation coordination
│   ├── mod.rs
│   ├── compiler.rs         # Main compiler driver
│   ├── options.rs          # Command-line options
│   ├── session.rs          # Compilation session
│   ├── project.rs          # Prism.toml handling
│   └── packages.rs         # Package manager integration
├── frontend/               # Front-end processing
│   ├── mod.rs
│   ├── source.rs           # Source file management
│   ├── lexer/              # Lexical analysis
│   ├── parser/             # Syntax analysis
│   ├── ast/                # Abstract Syntax Tree
│   ├── diagnostics/        # Error reporting
│   └── macros/             # Macro expansion (Phase 4)
│       ├── mod.rs
│       └── expand.rs
├── middle/                 # Middle-end processing
│   ├── mod.rs
│   ├── hir/                # High-level IR
│   ├── mir/                # Mid-level IR
│   ├── resolve/            # Name resolution
│   ├── typeck/             # Type checking
│   └── borrow/             # Borrow checking
├── backend/                # Back-end processing
│   ├── mod.rs
│   ├── c/                  # C code generation
│   ├── llvm/               # LLVM code generation
│   ├── optimize/           # Optimization passes
│   └── target/             # Cross-compilation support
│       ├── mod.rs
│       ├── triple.rs       # Target triple handling
│       └── platform.rs     # Platform-specific code gen
├── utils/                  # Shared utilities
│   ├── mod.rs
│   ├── arena.rs            # Memory arenas
│   ├── interner.rs         # String interning
│   └── parallel.rs         # Parallelism utilities
└── build/                  # Build system integration
    ├── mod.rs
    ├── cache.rs            # Incremental compilation
    └── deps.rs             # Dependency tracking
├── lsp/                    # Language Server Protocol
│   ├── mod.rs
│   ├── server.rs           # LSP server implementation
│   └── features.rs         # IDE features (hover, completion)
```

### 12.2 Library Design

- Core compiler functionality exposed as libraries
- Clear separation of concerns
- Well-defined interfaces between components
- Minimal dependencies between modules

### 12.3 Testing Strategy

- **Unit tests** for individual components
- **Integration tests** for compiler stages
- **End-to-end tests** for complete compilation
- **Snapshot testing** for generated code
- **Fuzz testing** for parser and type checker
- **Performance benchmarks** for compilation speed

### 12.4 Documentation

- Inline documentation for all public APIs
- Architecture documentation for major subsystems
- Contributor guides for common tasks
- Design rationale for key decisions

### 12.5 Build System Integration

- **Project manifest parsing**:
  - `Prism.toml` configuration and metadata
  - Dependency specification and resolution
  - Build customization options

- **Package management**:
  - Version resolution and compatibility checking
  - Remote package fetching and caching
  - Lockfile generation for reproducible builds

- **Build profiles**:
  - Debug, release, and custom configurations
  - Conditional compilation flags
  - Environment-specific settings

### 12.6 LSP Integration

- **Language Server Protocol implementation**:
  - Real-time error reporting
  - Code completion and navigation
  - Hover information and documentation

- **Incremental analysis**:
  - Partial reanalysis of changed files
  - Background type checking
  - Symbol indexing for fast lookups

- **IDE features**:
  - Code actions and quick fixes
  - Refactoring support
  - Semantic highlighting

### 12.7 Macro System Design

- **Declarative macros**:
  - Pattern-based syntax transformation
  - Hygiene preservation
  - Expansion tracing for error reporting

- **Procedural macros** (future):
  - Custom syntax extensions
  - Code generation from attributes
  - Token stream manipulation

- **Compile-time evaluation**:
  - Constant expression evaluation
  - Type-level computation
  - Static assertions

---

## Implementation Priorities

1. **Phase 1**: Basic Compilation Pipeline
   - Lexer and parser
   - Basic AST
   - Simple type checking
   - C code generation

2. **Phase 2**: Core Language Features
   - Full type system
   - Borrow checker
   - Basic optimizations
   - Incremental compilation foundation

3. **Phase 3**: Performance Optimizations
   - Parallel compilation
   - Advanced incremental compilation
   - Memory usage optimization
   - LLVM backend

4. **Phase 4**: Advanced Features
   - Macros
   - Traits
   - Advanced optimizations
   - IDE integration

---

This architecture document serves as a blueprint for implementing the Prism compiler. It prioritizes compilation speed while maintaining the language's safety guarantees and code quality. The design allows for incremental development, starting with a simpler C backend and evolving toward a full LLVM-based compiler with advanced optimization capabilities. 