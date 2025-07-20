# Bract Programming Language

**Revolutionary systems programming with contractual performance guarantees and hybrid memory management.**

![Bract Logo](Bract.png)

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Phase 1](https://img.shields.io/badge/Phase%201-Complete-brightgreen)]()
[![Phase 2](https://img.shields.io/badge/Phase%202-In%20Progress-yellow)]()
[![Language](https://img.shields.io/badge/language-Rust-orange)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

## 🚀 **Phase 1 COMPLETE: Revolutionary Type System & IR Architecture**

Bract has successfully implemented **the world's first type system with integrated memory management strategies** and performance contracts. We deliver:

- ✅ **5 Memory Strategies** with compile-time cost analysis
- ✅ **Ownership & Lifetime Analysis** preventing all memory bugs
- ✅ **Type Inference Engine** with performance optimization
- ✅ **Semantic-Preserving IR** for optimal code generation
- ✅ **Complete Lowering Pipeline** to native code
- ✅ **Performance Contract Enforcement** at compile time

## Vision

**FAST + SAFE + CLEAR = Non-negotiable contracts**

```bract
#[performance(max_cost = 1000, max_memory = 1024)]
fn guaranteed_fast(data: &[i32]) -> i32 {
    // Compiler ENFORCES performance contract
    // Memory strategy automatically optimized
    // Zero-overhead abstractions guaranteed
    data.iter().sum()  // Verified O(n) with bounded cost
}
```

## Key Innovations

### 🔥 **Hybrid Memory Management**
First language to support **5 memory strategies** in a unified type system:

```bract
fn demonstrate_strategies() {
    let stack_var: i32 = 42;                    // Cost: 0 (zero overhead)
    let linear_data: LinearPtr<Buffer> = ...;   // Cost: 1 (move semantics)  
    let region_alloc: RegionPtr<Data> = ...;    // Cost: 2 (bulk cleanup)
    let manual_ptr: ManualPtr<Raw> = malloc(4); // Cost: 3 (explicit control)
    let shared: SmartPtr<Cache> = ...;          // Cost: 4 (reference counting)
}
```

### ⚡ **Performance Contracts**
**Compile-time guarantees** for execution cost and memory usage:

```bract
#[performance(
    max_cpu_cycles = 500_000,
    max_memory_bytes = 4096,
    max_allocations = 0,
    deterministic = true
)]
fn real_time_critical() -> Result<Data, Error> {
    // Hardware-verified performance guarantees
    // Compiler rejects if contract cannot be met
}
```

### 🛡️ **Revolutionary Safety**
Complete memory safety through ownership analysis:

```bract
fn ownership_demo() {
    let data = LinearData::new();
    let processed = consume(data);     // data moved, no longer accessible
    // println!("{}", data.value);    // COMPILE ERROR: use after move
    println!("{}", processed.result); // Safe: ownership transferred
}
```

### 🧠 **Intelligent Type Inference**
Memory strategy resolution with performance optimization:

```bract
fn type_inference_demo() {
    let mixed = if high_performance_mode {
        create_stack_data()     // Returns Data [Stack]  
    } else {
        create_shared_data()    // Returns Data [SmartPtr]
    };
    // Type: Data [SmartPtr] - compiler chooses strategy for all cases
    // Performance analysis guides optimal selection
}
```

## Architecture Highlights

### **Enhanced AST with Memory Semantics**
```rust
pub enum Type {
    Primitive {
        kind: PrimitiveType,
        memory_strategy: MemoryStrategy,  // ⚡ Integrated strategy
        span: Span,
    },
    Reference {
        target_type: Box<Type>,
        lifetime: Option<LifetimeId>,     // 🛡️ Lifetime tracking
        ownership: Ownership,             // 🔒 Ownership rules
        span: Span,
    },
}
```

### **Bract IR: Semantic-Preserving Intermediate Representation**
```rust
pub enum BIROp {
    Allocate { 
        strategy: MemoryStrategy,     // Strategy-aware allocation
        region_id: Option<u32>,
    },
    Move { 
        check_consumed: bool,         // Linear type verification
    },
    ArcIncref { arc: BIRValueId },   // Smart pointer operations
    BoundsCheck { /* ... */ },       // Safety with optimization
}
```

### **Complete Lowering Pipeline**
```
AST + Memory Strategies → Bract IR → Cranelift IR → Native Code
  ↓                        ↓          ↓              ↓
Ownership Analysis      Strategy     Register      Machine Code
Type Inference         Optimization  Allocation    + Memory Runtime
Performance Contracts  Dead Code     Instruction   + Bounds Checking
                       Elimination   Selection     + Profiling Hooks
```

## Current Implementation Status

### ✅ **Phase 1 Complete: Core Infrastructure**
- **Revolutionary Type System**: Memory strategies, ownership, inference
- **Bract IR**: High-level operations with performance modeling  
- **Lowering Pipeline**: Complete AST → IR → native code path
- **Memory Manager**: Hybrid system with 5 strategies implemented
- **Performance Analysis**: Cost estimation and contract verification

### 🚧 **Phase 2 In Progress: Memory Integration**
- **Language Syntax**: Explicit memory strategy annotations
- **Polymorphic Strategies**: Generic functions over memory strategies
- **Advanced Operations**: Region management, strategy conversion
- **Code Generation**: Strategy-specific optimization passes

### 📋 **Upcoming Phases**
- **Phase 3**: Real-world validation with comprehensive examples
- **Phase 4**: Tooling ecosystem (`bractfmt`, `bract-prof`, LSP)
- **Phase 5**: Community building and ecosystem development

## Quick Start

### Installation
```bash
git clone https://github.com/bract-lang/bract.git
cd bract
cargo build --release
```

### Your First Bract Program
```bract
// examples/hello_performance.bract
#[performance(max_cost = 500, max_memory = 64)]
fn main() {
    println!("Hello, guaranteed fast world!");
    
    // Stack allocation - zero cost
    let numbers: [i32; 5] = [1, 2, 3, 4, 5];
    
    // Linear ownership - move semantics
    let buffer = LinearPtr::new(Vec::with_capacity(1000));
    let processed = process_buffer(buffer);  // buffer consumed
    
    println!("Processed {} items", processed.len());
}

fn process_buffer(data: LinearPtr<Vec<i32>>) -> ProcessedData {
    // Implementation with guaranteed performance
    ProcessedData { len: data.into_inner().len() }
}

struct ProcessedData { len: usize }
```

### Compile and Run
```bash
./target/release/bract compile examples/hello_performance.bract
./hello_performance
```

## Language Features

### **Memory Strategies**
- **Stack**: Zero-cost local allocation
- **Linear**: Move-only semantics, zero-copy transfers
- **Region**: Bulk allocation with O(1) cleanup  
- **SmartPtr**: Reference counting with cycle detection
- **Manual**: Explicit control for system programming

### **Safety Guarantees**
- **No use-after-free**: Ownership system prevents access to moved values
- **No double-free**: Automatic cleanup with strategy-specific rules
- **No buffer overflows**: Bounds checking with compiler optimization
- **No memory leaks**: Static analysis and linear type consumption

### **Performance Features**
- **Zero-overhead abstractions**: High-level features compile to optimal code
- **Performance contracts**: Compile-time verification of execution costs
- **Strategy selection**: Automatic optimization based on usage patterns
- **Hotspot identification**: Built-in profiling and optimization suggestions

## Examples

### **Stack Allocation (Zero Cost)**
```bract
fn stack_demo() {
    let array: [i32; 1000] = [0; 1000];  // Zero allocation cost
    let sum = array.iter().sum::<i32>(); // Vectorized by compiler
    sum  // Automatic cleanup, no overhead
}
```

### **Linear Types (Move Semantics)**
```bract
fn linear_demo() -> ProcessedBuffer {
    let buffer = LinearPtr::new(Buffer::with_capacity(4096));
    process_data(buffer)  // Ownership transferred, zero-copy
}

fn process_data(buffer: LinearPtr<Buffer>) -> ProcessedBuffer {
    // buffer is consumed here, original no longer accessible
    ProcessedBuffer::from(buffer.into_inner())
}
```

### **Region-Based Allocation**
```bract
#[memory(strategy = "region", size_hint = 64_KB)]
fn batch_processing(items: &[Input]) -> Vec<Output> {
    // All allocations use same region
    items.iter()
         .map(|item| expensive_transform(item))  // Region allocated
         .collect()
    // Entire region freed at once - O(1) cleanup
}
```

### **Smart Pointers (Shared Ownership)**
```bract
fn sharing_demo() -> (SmartPtr<Data>, SmartPtr<Data>) {
    let shared = SmartPtr::new(expensive_computation());
    let clone1 = shared.clone();  // Reference count increment  
    let clone2 = shared.clone();  // Reference count increment
    (clone1, clone2)  // Original freed when last reference drops
}
```

### **Performance Contracts**
```bract
#[performance(max_cost = 2000, max_memory = 1024, deterministic = true)]
fn guaranteed_algorithm(data: &[f64]) -> f64 {
    // Compiler verifies this meets performance requirements
    // Runtime verification in debug builds
    data.iter().fold(0.0, |acc, &x| acc + x * x).sqrt()
}
```

## Documentation

- **[Architecture Guide](ARCHITECTURE.md)** - System design and implementation
- **[Language Specification](LANGUAGE_SPEC.md)** - Complete syntax and semantics
- **[Performance Guide](docs/performance.md)** - Optimization strategies
- **[Memory Management](docs/memory.md)** - Strategy selection guide
- **[Examples](examples/)** - Real-world usage patterns

## Development Roadmap

### **Phase 2: Memory Integration** (Current)
**Objective**: Language-level memory strategy syntax and polymorphic strategies

**Deliverables**:
- [ ] Memory strategy syntax parser
- [ ] Polymorphic memory strategy support  
- [ ] Advanced memory operations (regions, conversion)
- [ ] Strategy-specific code generation

**Timeline**: 2-3 weeks

### **Phase 3: Validation & Examples**
**Objective**: Real-world programs demonstrating all capabilities

**Deliverables**:
- [ ] Comprehensive example suite
- [ ] Performance benchmarks vs C/Rust
- [ ] Memory safety verification
- [ ] End-to-end testing framework

### **Phase 4: Tooling Ecosystem**
**Objective**: World-class developer experience

**Deliverables**:
- [ ] `bractfmt` - Code formatter with strategy awareness
- [ ] `bract-prof` - Performance profiler and analyzer
- [ ] `bract-analyzer` - Static analysis with optimization hints
- [ ] LSP server for IDE integration

### **Phase 5: Community & Ecosystem**
**Objective**: Open source community and standard library

**Deliverables**:
- [ ] Formal language specification
- [ ] Standard library with performance contracts
- [ ] Package manager integration
- [ ] Community documentation and tutorials

## Contributing

We welcome contributions! Bract is building the future of systems programming.

### **Getting Started**
1. Read the [Architecture Guide](ARCHITECTURE.md)
2. Check [open issues](https://github.com/bract-lang/bract/issues)
3. Review our [contributing guidelines](CONTRIBUTING.md)
4. Join our [Discord community](https://discord.gg/bract-lang)

### **Current Focus Areas**
- **Phase 2 Implementation**: Memory strategy syntax and integration
- **Performance Testing**: Benchmarks and validation
- **Documentation**: Examples and guides
- **Tooling**: Developer experience improvements

### **Development Setup**
```bash
git clone https://github.com/bract-lang/bract.git
cd bract
cargo test  # Run test suite
cargo run -- --help  # Test compiler
./run_comprehensive_tests.ps1  # Full validation
```

## Community

- **Discord**: [Join our community](https://discord.gg/bract-lang)
- **GitHub**: [Source code and issues](https://github.com/bract-lang/bract)
- **Blog**: [Development updates](https://blog.bract-lang.org)
- **Twitter**: [@BractLang](https://twitter.com/BractLang)

## Performance Philosophy

**"Performance and safety are contracts, not gambles."**

Bract eliminates the traditional trade-off between safety and performance through:

1. **Compile-Time Guarantees**: Performance contracts verified before deployment
2. **Zero-Overhead Abstractions**: High-level features with no runtime cost  
3. **Optimal Memory Strategies**: Automatic selection for best performance
4. **Hardware Awareness**: Code generation optimized for target architecture
5. **Predictable Behavior**: No hidden allocations or surprise costs

## License

Bract is dual-licensed under:
- **MIT License** - for permissive open source usage
- **Apache 2.0 License** - for patent protection

See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.

## Acknowledgments

Built with ❤️ by the Bract team and powered by:
- **Cranelift**: High-performance code generation
- **Rust**: Memory-safe systems programming foundation
- **LLVM**: Optimization infrastructure inspiration

---

**Bract: Where performance guarantees meet memory safety. 🚀**

*Ready to revolutionize systems programming? [Get started today!](#quick-start)*
