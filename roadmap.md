STEP 1: Language Specification Design
Cursor Request:
I'm creating a new programming language called Prism that needs to be:
- Blazingly fast compilation to native machine code
- Memory safe without garbage collection  
- Readable syntax (not cryptic)
- Modern features like pattern matching, type inference
- Zero-cost abstractions

Design the complete language specification including:
1. Basic syntax for variables, functions, control flow
2. Type system (static typing with inference)
3. Memory management approach (ownership/borrowing or ref counting)
4. Module system and imports
5. Error handling mechanism
6. Basic operators and expressions
7. Function definitions and calls
8. Data structures (structs, enums, arrays)
9. Pattern matching syntax
10. Comments and documentation

Create a comprehensive LANGUAGE_SPEC.md file with examples for each feature. Make it readable but not verbose. Focus on clarity and performance-oriented design choices.


STEP 2: Formal Grammar Definition
Cursor Request:

Based on the Prism language specification, create a complete formal grammar in EBNF (Extended Backus-Naur Form) notation that covers:

1. Lexical grammar (tokens, keywords, operators, literals)
2. Expression grammar (arithmetic, logical, function calls, method calls)
3. Statement grammar (assignments, control flow, declarations)
4. Type grammar (basic types, generics, function types)
5. Module grammar (imports, exports, module declarations)
6. Pattern matching grammar
7. Function and struct definitions
8. Comments and whitespace handling

Save this as GRAMMAR.md. Make sure the grammar is unambiguous and suitable for recursive descent parsing. Include precedence rules for operators and associativity.


STEP 3: Compiler Architecture Design
Cursor Request:

Design the complete architecture for the Prism compiler optimized for fast compilation speeds. Create ARCHITECTURE.md that includes:

1. Overall compiler pipeline (lexer → parser → semantic analysis → codegen → linking)
2. Data structures for AST nodes (all expression types, statement types, declarations)
3. Symbol table design and scope management
4. Type checking and inference system architecture
5. Error reporting and diagnostics system
6. Code generation strategy (initially transpile to C, later LLVM IR)
7. Optimization passes to include
8. Memory management for compiler itself
9. Incremental compilation strategy
10. Parallel compilation approach
11. File organization and module structure for the compiler codebase

Make it detailed enough that we can implement each component systematically. Focus on performance and maintainability.

PHASE 2: LEXER & TOKENIZER
STEP 4: Complete Lexer Implementation
Cursor Request:
Implement a complete lexer/tokenizer for the Prism language in Rust. Based on the grammar we defined, create:

1. Complete Token enum with all token types (keywords, operators, literals, identifiers, etc.)
2. Lexer struct with position tracking (line, column, file)
3. Tokenization methods for:
   - All keywords (fn, let, if, else, match, struct, enum, etc.)
   - All operators (+, -, *, /, ==, !=, &&, ||, etc.)
   - String literals (with escape sequences)
   - Number literals (integers, floats, different bases)
   - Identifiers and type names
   - Comments (single-line and multi-line)
   - Whitespace handling
4. Error handling for invalid tokens
5. Position tracking for error reporting
6. Comprehensive test suite covering all token types
7. Performance optimizations (efficient string handling)

Create src/lexer.rs with full implementation. Make it robust and fast - this is the foundation of compilation speed.

STEP 5: Lexer Testing & Refinement
Cursor Request:
Create comprehensive tests for the Prism lexer and fix any issues. Generate:

1. Unit tests for each token type
2. Integration tests with real Prism code samples
3. Error handling tests (invalid characters, unterminated strings, etc.)
4. Performance benchmarks for large files
5. Edge case tests (Unicode identifiers, very long tokens, etc.)
6. Position tracking accuracy tests

Also create a simple CLI tool that can tokenize Prism files and output the token stream for debugging. Put tests in tests/lexer_tests.rs and the CLI tool in src/bin/prism_lex.rs.

PHASE 3: PARSER & AST
STEP 6: AST Node Definitions ✅ **COMPLETED**
Cursor Request:
Define all AST (Abstract Syntax Tree) node types for Prism in Rust. Create src/ast.rs with:

1. Expression nodes:
   - Literals (numbers, strings, booleans)
   - Identifiers and paths
   - Binary operations (arithmetic, logical, comparison)
   - Unary operations
   - Function calls
   - Method calls
   - Array/struct access
   - Pattern matching expressions

2. Statement nodes:
   - Variable declarations (let bindings)
   - Assignments
   - Expression statements
   - Control flow (if/else, loops, match)
   - Return statements
   - Block statements

3. Declaration nodes:
   - Function definitions
   - Struct definitions
   - Enum definitions
   - Type aliases
   - Module declarations

4. Type nodes:
   - Basic types
   - Function types
   - Generic types
   - Array types

Each node should include source position information for error reporting. Use proper Rust enums and structs. Include Debug, Clone traits where appropriate.

STEP 7: Recursive Descent Parser
Cursor Request:
Implement a complete recursive descent parser for Prism that converts tokens to AST. Create src/parser.rs with:

1. Parser struct that takes a token stream from the lexer
2. Parsing methods for each AST node type:
   - parse_expression() with proper precedence handling
   - parse_statement()
   - parse_declaration()
   - parse_type()
   - parse_pattern()

3. Operator precedence parsing for expressions
4. Error recovery mechanisms (don't stop on first error)
5. Detailed error messages with source positions
6. Look-ahead and backtracking where needed
7. Helper methods for common patterns (expect_token, peek, advance)

8. Integration with lexer to create a complete parsing pipeline
9. Comprehensive error reporting with suggestions

The parser should be robust and produce helpful error messages. Handle all syntax from the grammar specification.


STEP 8: Parser Testing & Integration
Cursor Request:
Create comprehensive tests for the Prism parser and integrate it with the lexer. Generate:

1. Unit tests for each parsing method
2. Integration tests parsing complete Prism programs
3. Error handling tests (syntax errors, recovery testing)
4. AST structure validation tests
5. Performance benchmarks for parsing large files

Also create a CLI tool that can parse Prism files and output the AST structure for debugging. Create tests/parser_tests.rs and src/bin/prism_parse.rs.

Include example Prism programs in examples/ directory that demonstrate all language features and can be used for testing.

PHASE 4: SEMANTIC ANALYSIS
STEP 9: Symbol Table & Scope Management
Cursor Request:
Implement symbol table and scope management for Prism semantic analysis. Create src/semantic/mod.rs and src/semantic/symbols.rs with:

1. Symbol table implementation:
   - Variable symbols (name, type, mutability, position)
   - Function symbols (name, parameters, return type, body)
   - Type symbols (structs, enums, type aliases)
   - Module symbols and imports

2. Scope management:
   - Scope stack for nested scopes
   - Symbol resolution across scopes
   - Shadowing rules
   - Module-level scope handling

3. Symbol table builder that walks the AST:
   - First pass: collect all declarations
   - Handle forward references
   - Check for duplicate definitions
   - Build dependency graph

4. Name resolution:
   - Resolve identifiers to symbols
   - Handle qualified names (module::function)
   - Check accessibility/visibility

Include comprehensive error reporting for name conflicts, undefined symbols, etc.

STEP 10: Type System & Type Checking
Cursor Request:
Implement the complete type system and type checker for Prism. Create src/semantic/types.rs and src/semantic/typechecker.rs with:

1. Type representation:
   - Basic types (int, float, bool, string)
   - Compound types (arrays, structs, enums)
   - Function types
   - Generic types and constraints
   - Type variables for inference

2. Type inference engine:
   - Hindley-Milner style inference where possible
   - Constraint generation and solving
   - Unification algorithm
   - Error reporting for type mismatches

3. Type checking for all AST nodes:
   - Expression type checking
   - Statement type checking
   - Function signature validation
   - Pattern matching exhaustiveness
   - Generic instantiation

4. Advanced features:
   - Trait/interface system (if planned)
   - Ownership/borrowing type checking (if using ownership model)
   - Lifetime analysis

Make the type system expressive but efficient. Provide clear error messages with suggestions.


PHASE 5: CODE GENERATION
STEP 11: C Code Generation (Initial Target)
Cursor Request:
Implement C code generation for Prism (as stepping stone to native compilation). Create src/codegen/c_gen.rs with:

1. C code generator that walks the typed AST:
   - Expression translation to C expressions
   - Statement translation to C statements
   - Function definitions to C functions
   - Struct definitions to C structs
   - Memory management code generation

2. Runtime system in C:
   - Memory allocation/deallocation
   - String handling
   - Array operations
   - Error handling (panics, exceptions)

3. Standard library bindings:
   - I/O operations (print, file operations)
   - Basic data structures
   - Math functions

4. Build system integration:
   - Generate Makefile or use cc crate
   - Link with runtime system
   - Optimization flags

5. Name mangling for Prism symbols to avoid C naming conflicts

Create working C output that can be compiled with GCC/Clang to produce fast executables.


STEP 12: Compiler Driver & CLI
Cursor Request:
Create the main Prism compiler driver and command-line interface. Create src/main.rs and src/driver.rs with:

1. Command-line argument parsing:
   - Input files
   - Output file/directory
   - Optimization levels
   - Debug information
   - Verbose output
   - Help and version info

2. Compilation pipeline orchestration:
   - File reading and preprocessing
   - Lexing → Parsing → Semantic Analysis → Code Generation
   - Error aggregation and reporting
   - Progress reporting for large projects

3. Build system features:
   - Dependency tracking
   - Incremental compilation
   - Parallel compilation of modules
   - Caching of intermediate results

4. Integration with system tools:
   - Invoke C compiler for final compilation
   - Handle linker flags and libraries
   - Support cross-compilation

5. Comprehensive error reporting:
   - Colored output
   - Source code highlighting
   - Suggestions for fixes
   - Multiple error display

Make it professional-quality with excellent UX. This is what users will interact with daily.

PHASE 6: TOOLING & ECOSYSTEM
STEP 13: VS Code Extension
Cursor Request:
Create a complete VS Code extension for Prism language support. Generate all necessary files:

1. package.json with extension metadata and dependencies
2. Syntax highlighting (TextMate grammar in syntaxes/prism.tmLanguage.json)
3. Language configuration (language-configuration.json) with:
   - Comment definitions
   - Bracket definitions
   - Auto-closing pairs
   - Indentation rules

4. Snippets for common Prism patterns (snippets/prism.json)
5. Basic language server integration preparation
6. Extension activation and commands
7. Build and packaging scripts

Create the extension in tools/vscode-extension/ directory. Make syntax highlighting comprehensive and visually appealing. Include installation instructions.


STEP 14: Language Server Protocol (LSP)
Cursor Request:
Implement a Language Server Protocol server for Prism to provide IDE features. Create src/lsp/mod.rs with:

1. LSP server implementation:
   - Initialize and shutdown handlers
   - Document synchronization (open, change, close)
   - Diagnostic publishing (errors, warnings)
   - Completion provider
   - Hover information
   - Go to definition
   - Find references
   - Document symbols

2. Integration with Prism compiler:
   - Incremental parsing and analysis
   - Error reporting with ranges
   - Symbol information extraction
   - Type information for hover

3. Performance optimizations:
   - Caching of analysis results
   - Incremental updates
   - Background processing
   - Memory management

4. Client communication:
   - JSON-RPC protocol handling
   - Async request processing
   - Progress reporting

Create src/bin/prism_lsp.rs as the LSP server binary. Make it responsive and reliable for smooth IDE experience.


PHASE 7: OPTIMIZATION & POLISH
STEP 15: Standard Library Foundation
Cursor Request:
Design and implement the core Prism standard library. Create stdlib/ directory with:

1. Core module (stdlib/core.prism):
   - Basic types and operations
   - Memory management utilities
   - Panic/error handling
   - Debugging utilities

2. Collections module (stdlib/collections.prism):
   - Dynamic arrays/vectors
   - Hash maps/dictionaries
   - Sets
   - Linked lists, queues, stacks

3. I/O module (stdlib/io.prism):
   - File operations
   - Console I/O
   - Network I/O basics
   - Serialization/deserialization

4. String module (stdlib/string.prism):
   - String manipulation
   - Regular expressions
   - Unicode handling
   - Formatting

5. Math module (stdlib/math.prism):
   - Basic math functions
   - Random number generation
   - Statistical functions

Implement these first in Prism syntax, then ensure the compiler can handle them. Focus on the most commonly needed functionality.


STEP 16: Testing Infrastructure & Examples
Cursor Request:
Create comprehensive testing infrastructure and example programs for Prism. Generate:

1. Test framework for Prism itself:
   - Unit testing macros/functions
   - Integration testing support
   - Benchmarking utilities
   - Test runner

2. Comprehensive test suite:
   - Language feature tests
   - Standard library tests
   - Performance regression tests
   - Cross-platform compatibility tests

3. Example programs in examples/:
   - Hello World and basic syntax
   - Data structures and algorithms
   - File I/O and text processing
   - Simple games or utilities
   - Performance showcase programs

4. Documentation and tutorials:
   - Getting started guide
   - Language reference
   - Standard library documentation
   - Best practices guide

5. Continuous integration setup:
   - GitHub Actions workflow
   - Multiple platform testing
   - Performance tracking
   - Documentation generation

Make examples engaging and demonstrate Prism's strengths clearly.



