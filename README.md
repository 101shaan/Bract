# Bract Programming Language

**Experimental systems programming language exploring memory strategy integration and performance contracts.**

![Bract Logo](Bract.png)

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Phase](https://img.shields.io/badge/Phase%203-In%20Progress-yellow)]()
[![Language](https://img.shields.io/badge/language-Rust-orange)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

⚠️ **Early Development**: Bract is a research language currently in active development. Not ready for production use.

## Current Status

**What works:**
- ✅ Basic lexer and parser for Bract syntax
- ✅ Memory strategy annotations (`@memory`, `@performance`)
- ✅ AST generation and semantic analysis foundation
- ✅ Native code generation via Cranelift (object emission + linking in CLI)
- ✅ Test suite green (unit + integration; some features still WIP)

**What's being worked on:**
- 🔄 Performance optimization (currently ~7ms compilation times)
- 🔄 Complete runtime system implementation
- 🔄 Memory strategy runtime integration
- 🔄 Error handling and diagnostics improvement

**What doesn't work yet:**
- ❌ Performance is not competitive with production languages
- ❌ Memory strategies are parsed but not fully implemented in codegen
- ❌ No standard library or real-world programs yet
- ❌ Many advanced language features are placeholders

## Vision

Bract aims to explore integrating memory management strategies directly into the type system:

```bract
@performance(max_cost = 1000, max_memory = 1024)
fn example(data: &[i32]) -> i32 {
    let sum = 0;
    for item in data {
        sum = sum + item;
    }
    sum
}
```

The goal is to allow different memory strategies:
- **Stack**: Zero-cost local allocation
- **Linear**: Move-only semantics  
- **Region**: Bulk allocation with fast cleanup
- **SmartPtr**: Reference counting
- **Manual**: Explicit memory control

## Current Capabilities

### Basic Compilation Pipeline (Cranelift)
```bash
# Compile a Bract program to a native executable via Cranelift
cargo run --bin bract_cranelift -- examples/hello_world.bract -v
```

### Supported Syntax
- Basic functions, variables, and control flow
- Memory strategy annotations (parsed, partial implementation)
- Performance contracts (parsed, validation TODO)
- Arrays, basic data types

### Example Programs
See `examples/` directory for current working programs:
- `hello_world.bract` - Basic function compilation
- `comprehensive_validation.bract` - Language features demo
- `system_showcase.bract` - Complex algorithm examples

## Installation & Usage

```bash
git clone https://github.com/bract-lang/bract.git
cd bract
cargo build --release

# Run tests
cargo test --lib

# Compile example
cargo run --bin bract_compile_simple -- examples/hello_world.bract
```

## Development Status

### Phase 1: ✅ Complete
- Basic language infrastructure
- Lexer, parser, AST
- Semantic analysis framework
- Code generation pipeline

### Phase 2: ✅ Complete  
- Memory strategy syntax parsing
- Performance annotation support
- Wrapper type parsing (`LinearPtr<T>`, etc.)

### Phase 3: 🔄 In Progress
- Performance optimization
- Runtime system completion
- Real-world validation examples
- Comprehensive testing

### Phase 4: 📋 Planned
- Tooling (formatter, LSP, profiler)
- Standard library
- Documentation

## Architecture

```
Bract Source → Lexer → Parser → AST → Semantic Analysis → C Generation → Native Code
```

**Current pipeline:**
- Bract syntax parsed to AST
- Basic semantic analysis
- AST lowered directly to Cranelift and emitted as native object code
- CLI links object into an executable (platform-specific)

**Planned improvements:**
- Richer native code generation coverage (struct returns, complex control flow)
- Memory strategy runtime integration  
- Performance contract verification
- Optimization passes

## Contributing

**Bract is a research project.** Contributions welcome but expect:
- Frequent breaking changes
- Incomplete features
- Performance issues being worked on
- Documentation gaps

Current focus areas:
1. **Performance**: Improve compilation speed and output quality
2. **Runtime**: Complete memory strategy implementation
3. **Testing**: More comprehensive validation
4. **Documentation**: Keep docs updated with reality

### Development Setup
```bash
git clone https://github.com/bract-lang/bract.git
cd bract
cargo test --lib          # Run test suite
cargo check               # Verify compilation
./cleanup_build_artifacts.ps1  # Clean build files
```

## Performance Goals vs Reality

**Goals:**
- Sub-millisecond compilation for small programs
- Competitive performance with C/Rust
- Zero-overhead memory strategy abstractions

**Current Reality:**
- ~7ms compilation for basic programs (needs improvement)
- Cranelift backend active; some language features not fully lowered yet (e.g., certain struct returns)
- Memory strategies parsed but runtime/codegen integration is partial

**Honest Assessment:**
This is experimental language research. Performance claims are aspirational. Current implementation focuses on correctness over speed.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built with:
- **Rust** - Implementation language
- **Cranelift** - Code generation framework (planned)
- **C** - Current compilation target

---

**Bract: speed, safety, and an exceptional developer experience — a promise, not a compromise.**

*Interested in language research? Check out the [examples](examples/) to see current capabilities.*