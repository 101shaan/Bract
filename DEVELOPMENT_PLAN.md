# Bract Language Development Plan
## Getting the Basics Right First

### Current Status Assessment
- ✅ **Lexer**: Fully functional, handles all token types
- ⚠️ **Parser**: Partially functional, can't handle real programs
- ⚠️ **Semantic Analysis**: Structure exists but incomplete
- ❌ **Codegen**: Mostly stubs and TODO comments  
- ❌ **Runtime**: Non-existent
- ❌ **End-to-End**: Cannot compile and run simple programs

### Core Problem
The language has grown complex with advanced features (hybrid memory management, performance contracts) while basic functionality is broken. **We need to get simple programs working end-to-end before adding advanced features.**

## Phase 1: Foundation (Weeks 1-2)
**Goal: Compile and run simple arithmetic programs**

### 1.1 Fix Parser File Handling
- **Branch**: `features/fix-parser-file-handling`
- **Task**: Fix `bract_parse` binary to read files properly
- **Acceptance**: Can parse real `.bract` files, not just strings

### 1.2 Complete Basic Parser
- **Branch**: `features/basic-parser-completion`  
- **Tasks**:
  - Fix multi-line parsing
  - Complete function declarations with parameters
  - Complete variable declarations (`let` statements)
  - Complete basic expressions (arithmetic, comparisons)
  - Complete basic statements (assignment, return)
- **Acceptance**: Can parse simple function programs

### 1.3 Implement Basic Codegen
- **Branch**: `features/basic-cranelift-codegen`
- **Tasks**:
  - Complete literal compilation (integers, booleans)
  - Complete variable compilation (stack slots)
  - Complete arithmetic operations
  - Complete function calls
  - Complete basic control flow (if/else)
- **Acceptance**: Can generate machine code for simple programs

### 1.4 Create Minimal Runtime
- **Branch**: `features/minimal-runtime`
- **Tasks**:
  - Remove complex memory management system temporarily
  - Create simple stack-based allocation
  - Implement basic program entry/exit
  - Fix linking issues
- **Acceptance**: Can run compiled programs

**Phase 1 Milestone**: Compile and run this program:
```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() -> i32 {
    let x = 10;
    let y = 20;
    let result = add(x, y);
    result
}
```

## Phase 2: Core Language Features (Weeks 3-4)
**Goal: Support essential language constructs**

### 2.1 Control Flow
- **Branch**: `features/control-flow`
- **Tasks**: while loops, if/else expressions, basic pattern matching

### 2.2 Data Structures  
- **Branch**: `features/basic-data-structures`
- **Tasks**: Arrays, basic structs (no methods yet)

### 2.3 Standard Library Basics
- **Branch**: `features/basic-stdlib`
- **Tasks**: Print functions, basic I/O

**Phase 2 Milestone**: Compile and run programs with loops, arrays, and structs

## Phase 3: Advanced Features (Weeks 5-6)
**Goal: Add back advanced features on solid foundation**

### 3.1 Memory Management
- **Branch**: `features/memory-management`
- **Tasks**: Re-implement memory strategies on working foundation

### 3.2 Performance Features
- **Branch**: `features/performance-contracts`
- **Tasks**: Re-implement performance annotations

### 3.3 Module System
- **Branch**: `features/modules`
- **Tasks**: Proper module support, imports/exports

## Development Workflow

### Git Workflow
```bash
# For each feature:
git checkout development
git pull origin development
git checkout -b features/<feature-name>

# Work on feature, commit regularly:
git add .
git commit -m "implement: <specific change>"

# When feature complete:
git checkout development
git merge features/<feature-name>
git branch -d features/<feature-name>

# For releases:
git checkout main
git merge development
git tag v0.x.0
```

### Commit Standards
- `implement: <description>` - New functionality
- `fix: <description>` - Bug fixes  
- `refactor: <description>` - Code restructuring
- `docs: <description>` - Documentation updates
- `test: <description>` - Test additions/fixes

### Clean Build Policy
**ALWAYS** clean build artifacts before commits:
```bash
cargo clean
# Remove any generated files
rm -rf output/ output_struct/ *.o *.exe
git add .
git commit -m "<message>"
```

## Success Metrics

### Phase 1 Success
- [ ] Can compile `examples/simple_function.bract` 
- [ ] Generated executable runs and returns correct result
- [ ] No linker errors
- [ ] Clean compilation with minimal warnings

### Phase 2 Success  
- [ ] Can compile programs with arrays and structs
- [ ] Can compile programs with loops and conditionals
- [ ] Basic standard library functions work

### Phase 3 Success
- [ ] Advanced memory management works
- [ ] Performance contracts functional
- [ ] Module system working

## Anti-Goals (For Now)
❌ **Complex memory management** - Focus on basic stack allocation first
❌ **Performance optimization** - Focus on correctness first  
❌ **Advanced type system** - Basic types first
❌ **Macro system** - Core language first
❌ **Concurrency** - Sequential execution first

## Documentation Requirements
- Update README.md after each phase
- Update LANGUAGE_SPEC.md with implemented features only
- Keep ARCHITECTURE.md current with actual implementation
- Remove or clearly mark unimplemented features in docs

---
*This plan prioritizes getting a working, compilable language over advanced features. Every phase must result in demonstrably working code.*
