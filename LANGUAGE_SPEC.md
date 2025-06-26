# Prism Language Specification (v0.1 – Early Draft)

> **Status:** Draft for community review.  Out-of-date sections are marked ⓘ and will be revised as the implementation evolves.  Corrections and PRs welcome.

Prism is a _statically-typed, ahead-of-time (AOT) compiled_ systems programming language focused on blazing-fast builds, predictable performance, and _memory-safety without a garbage collector_.  Its design draws inspiration from C, Rust, Swift, and ML languages, while maintaining a syntax that reads naturally to programmers coming from modern C-style languages.

This document is the **normative reference** for the language—tooling, compilers, and linters _must_ follow it.  Examples are **non-normative** and shown in • **bold monospace** blocks.

---

## Contents

1.  [Lexical Structure](#lexical-structure)
2.  [Syntactic Grammar](#syntactic-grammar)
3.  [Types & Type System](#types--type-system)
4.  [Memory Model](#memory-model)
5.  [Variables, Bindings, & Constants](#variables-bindings--constants)
6.  [Functions & Closures](#functions--closures)
7.  [Statements & Control Flow](#statements--control-flow)
8.  [Expressions & Operators](#expressions--operators)
9.  [Data Declarations](#data-declarations)
10. [Pattern Matching](#pattern-matching)
11. [Modules & Packages](#modules--packages)
12. [Error Handling](#error-handling)
13. [Concurrency & Atomics](#concurrency--atomics)
14. [Attributes & Metadata](#attributes--metadata)
15. [Toolchain & Compilation Model](#toolchain--compilation-model)
16. [Standard Library Overview](#standard-library-overview)
17. [Future Directions](#future-directions)
18. [Appendix A: Complete Token Grammar (EBNF)](#appendix-a-complete-token-grammar-ebnf)
19. [Appendix B: Operator Precedence Table](#appendix-b-operator-precedence-table)

---

<a name="lexical-structure"></a>
## 1. Lexical Structure

### 1.1 Source Encoding
Prism source files **must** be valid _UTF-8_.  The compiler rejects any ill-formed sequences.

### 1.2 Line Terminators
Line terminators are `\u000A` LF (`\n`) or `\u000D` CRLF (`\r\n`).  They are interchangeable outside of string literals.

### 1.3 Whitespace & Semicolon Elision
Whitespace (space, tab, line terminator, FF) separates tokens.  **Semicolons** act as statement separators but _may be elided_ where a newline _unambiguously_ ends a statement (same rule as Go).  The lexer inserts a virtual semicolon at newline boundaries when the next token cannot be parsed as part of the current statement.

### 1.4 Comments
| Kind       | Introducer | Terminator | Nestable | Emits Doc? |
|------------|-----------|-----------|----------|------------|
| Line       | `//`      | end-of-line| —        | No |
| Block      | `/*`      | `*/`      | **Yes**  | No |
| Doc line   | `///`     | EOL       | —        | **Yes** |
| Doc block  | `/**`     | `*/`      | Yes      | **Yes** |

Doc comments become Markdown-based documentation emitted by tooling.

### 1.5 Identifiers
```
identifier  ::= IdentStart IdentContinue*
IdentStart  ::= XID_Start | '_'
IdentContinue ::= XID_Continue
```
Unicode is allowed, but mixing scripts in a single identifier is **warned** by default lints.

### 1.6 Keywords
```
abort break box const continue do else enum extern false fn for if impl in let loop match mod move mut pub return struct true type use while
```
`async`, `await`, `trait`, `try` are **reserved** for future use.

### 1.7 Literals
* **Integer:** `0`, `42`, `0xFF`, `0b1010`, suffixed (`123_u16`).  Defaults: unsuffixed integers start as `i32` then coerced.
* **Floating:** `3.14`, `2e10`, suffix `_f32`.  Default `f64`.
* **String:** `"escape\n"`, `r"raw string"`, `r#"raw #1"#` (up to 255 `#` levels).
* **Char:** `'A'`, `'\u{1F600}'`.
* **Boolean:** `true`, `false`.

---

<a name="syntactic-grammar"></a>
## 2. Syntactic Grammar

Prism uses an LL(*) grammar amenable to deterministic parsing with limited backtracking.
The grammar is provided here in abridged EBNF; Appendix A contains the full set.

```
Module      ::= { UseDecl | Item } EOF
Item        ::= FnDecl | StructDecl | EnumDecl | TypeAlias | ConstDecl | ModDecl | ImplBlock
```

#### 2.1 Blocks & Statements
```
Block       ::= '{' { Statement } '}'
Statement   ::= LetStmt | Item | ExprStmt | ControlStmt
```

Control statements include `if`, `while`, `for`, and `loop`.

---

<a name="types--type-system"></a>
## 3. Types & Type System

1. **Primitives:** `i8…i128`, `u8…u128`, `f32`, `f64`, `bool`, `char`, `void` (unit).
2. **Compound:** arrays `[T; N]`, slices `&[T]`, tuples `(T1, T2, …)`, function pointers `fn(T) -> U`.
3. **User-Defined:** `struct`, `enum`, opaque `type` alias.
4. **Generics:** parametric polymorphism with _monomorphisation_ at compile-time.
5. **Traits (planned):** compile-time interfaces ad-hoc.

### 3.1 Type Inference
A Hindley-Milner-style algorithm with local modifications: inference is _lexically-scoped_ and does not cross function boundaries.  All generic parameters must be resolvable at monomorphisation time.

### 3.2 Subtyping & Coercions
Prism has **no implicit subtyping** except:
* Numeric literals → any numeric type of sufficient width.
* `&mut T` → `&T` (readonly coerces).
* Arrays of fixed length `N` → slice `&[T]`.

All other conversions require explicit casts: `as` keyword or trait method.

---

<a name="memory-model"></a>
## 4. Memory Model

Prism adopts an **ownership/borrowing** scheme enforced at compile-time.

1. **Move Semantics** – binding assignment (`let a = b;`) _moves_ unless the type implements `Copy` marker trait.
2. **Borrowing** – `&T` (shared) / `&mut T` (exclusive) with lexical _lifetimes_ automatically inferred.
3. **Aliasing XOR Mutation** – At any program point, a value may have
   *many_ immutable readers **or** _one_ mutable writer, never both.
4. **Destructor – `drop`** – deterministic RAII cleanup when the owner goes out of scope.
5. **Unsafe Blocks (future)** – `unsafe` allows opting-out of borrow checks for FFI or manual memory manipulation.

Implementation uses stack allocation by default; `box` places objects on the heap managed by unique `Box<T>` owner.

---

<a name="variables-bindings--constants"></a>
## 5. Variables, Bindings, & Constants

| Keyword | Mutability | Lifetime               |
|---------|------------|------------------------|
| `let`   | immutable  | scope/block            |
| `let mut` | mutable  | scope/block            |
| `const` | immutable  | program (inlined)      |
| `static`| mutable*†  | program, single address|

† `static mut` is `unsafe`.

Shadowing is permitted; each `let` introduces a new binding.

Destructuring patterns allowed:
```prism
let (x, y) = point;
let Point { x, y: yy } = point;
```

---

<a name="functions--closures"></a>
## 6. Functions & Closures

```prism
fn max<T: Ord>(a: T, b: T) -> T {
    if a > b { a } else { b }
}
```

* **Visibility:** `pub` exports the item.
* **Generics:** after name: `fn foo<T, U>(t: T) -> U where T: Into<U>`.
* **Variadics:** C-ABI only `extern "C" fn printf(fmt: &str, ...);`.
* **Closures:** `|x| x + 1` capture by move default; explicit with `move |x| …`.
* **Tail-Call:** Compiler _may_ optimize last expressions.

---

<a name="statements--control-flow"></a>
## 7. Statements & Control Flow

* `if` / `else if` / `else` (expression condition of type `bool`).
* `match` – exhaustive (see §10).
* Loops: `loop`, `while`, `for PATTERN in EXPR` (desugars to iterator).
* Jump statements: `break`, `continue`, `return EXPR?`, labeled loops `label: loop { … }`.
* Defer (future): `defer { … }` executes on scope exit.

---

<a name="expressions--operators"></a>
## 8. Expressions & Operators

* All operators return a value (_expressions everywhere_).
* Evaluation order is **left-to-right**, except assignments evaluate RHS first.
* Short-circuit: `&&`, `||`.

See full precedence list in Appendix B.

---

<a name="data-declarations"></a>
## 9. Data Declarations

### 9.1 Structs
```prism
pub struct Rect {
    width:  u32,
    height: u32,
}
```
* Named, tuple `struct Color(u8, u8, u8);`, and unit `struct Marker;` varieties.
* Default constructor `Rect { width: 0, height: 0 }`.

### 9.2 Enums
```prism
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```
Enums may have variant payloads of differing types.

### 9.3 Unions (ⓘ experimental)
C-style unions for FFI behind `unsafe`.

### 9.4 Arrays, Slices, & Vec
Fixed length `[T; N]`, runtime growable `Vec<T>` provided by std.

---

<a name="pattern-matching"></a>
## 10. Pattern Matching

Patterns appear in `let`, `match`, `for`, and function parameters.

```
Pattern ::= '_' | Literal | IDENT | '&' Pattern | '&mut' Pattern | StructPat | EnumPat | ( PatternList )
```

Example:
```prism
match msg {
    Message::Move { x, y } => handle_move(x, y),
    Message::Quit         => quit(),
}
```

Refutability rules follow Rust’s model.

---

<a name="modules--packages"></a>
## 11. Modules & Packages

* Each file `foo.pri` is a module named `foo` unless it contains `mod NAME;`.
* Directory with `mod.rs` (`lib.pri` in future) forms a _package_ root.
* `use path::{item1, item2 as alias2};` inserts names into local scope.
* **Privacy:** Items are private to the module unless prefixed by `pub`.

Manifest file `Prism.toml` describes package metadata and dependencies.

---

<a name="error-handling"></a>
## 12. Error Handling

1. **Recoverable:** `Result<T, E>` enriched with `?` propagation operator.
2. **Unrecoverable:** `panic!(msg)` aborts (no unwinding) to ensure zero-cost in the common path.  The panic strategy can be switched to _unwind_ via compiler flag for tests.
3. **Option<T>** for presence/absence.

---

<a name="concurrency--atomics"></a>
## 13. Concurrency & Atomics

Prism’s standard library provides:
* `std::thread::spawn(closure) -> JoinHandle` – OS thread per call.
* `channel()` – multi-producer, single-consumer MPSC channel.
* `Mutex<T>`, `RwLock<T>`, `Atomic*` primitives.

The memory model follows **C++20** _sequenced-before_ rules with `Acquire`, `Release`, and `AcqRel` orderings.  Data races are UB and rejected by the borrow checker in safe code.

---

<a name="attributes--metadata"></a>
## 14. Attributes & Metadata

Attributes modify compilation semantics:
```prism
#[inline(always)]
fn fast() { … }

#[cfg(target_os = "windows")]
mod win;
```
Syntax: `#` `[` MetaItem `]` with nested key-value pairs.

---

<a name="toolchain--compilation-model"></a>
## 15. Toolchain & Compilation Model

* **Compiler:** `prismc` front-end → HIR → MIR → LLVM IR.
* **Incremental:** fine-grained hashing invalidates only affected crates.
* **Optimization levels:** `-O0`, `-O2` (default), `-O3`, `-Os`.
* **Linkage:** static by default, dynamic with `--shared`.
* **Target triples** follow LLVM naming (`x86_64-pc-windows-msvc`).
* **Build tool:** `prism build`, `prism test`, `prism run` (analogue to Cargo).

---

<a name="standard-library-overview"></a>
## 16. Standard Library Overview

| Module    | Purpose                        |
|-----------|--------------------------------|
| `core`    | language primitives, `Option`, `Result`, iterators |
| `alloc`   | heap collections `Vec`, `String`, `Box` |
| `std`     | IO, OS abstractions, threading, networking |
| `test`    | unit-test harness |

The **prelude** (`use std::prelude::*;`) is implicitly imported into every module.

---

<a name="future-directions"></a>
## 17. Future Directions

* Traits & trait objects
* Macros & compile-time eval (`const fn`, `constexpr` style)
* Async/await with cooperative green threading
* WASM backend & embedded targets
* Formal verification of borrow checker (research)

---

<a name="appendix-a-complete-token-grammar-ebnf"></a>
## Appendix A: Complete Token Grammar (EBNF)

```
DecimalLiteral   ::= ['0'..'9']['0'..'9' '_']*
HexLiteral       ::= '0x'['0'..'9' 'a'..'f' 'A'..'F' '_']+
OctalLiteral     ::= '0o'[0'..'7' '_']+
BinaryLiteral    ::= '0b'[ '0' | '1' | '_' ]+
FloatLiteral     ::= DecimalLiteral '.' DecimalLiteral [ Exponent ]
Exponent         ::= ('e' | 'E') ['+'|'-']? DecimalLiteral
StringLiteral    ::= '"' ( '\\'Any | ~('"'|'\\') )* '"'
RawStringLiteral ::= 'r' '#'* '"' .* '"' '#'*
CharLiteral      ::= '\'' ( '\\'Any | ~('\''|'\\') ) '\''
```

---

<a name="appendix-b-operator-precedence-table"></a>
## Appendix B: Operator Precedence Table

| Level (high→low) | Operators | Associativity |
|------------------|-----------|---------------|
| 14 | `() [] . ?`               | left |
| 13 | `!  ~  &*`                | right |
| 12 | `*  /  %`                 | left |
| 11 | `+  -`                    | left |
| 10 | `<<  >>`                  | left |
| 9  | `< <= > >=`               | left |
| 8  | `== !=`                   | left |
| 7  | `&`                       | left |
| 6  | `^`                       | left |
| 5  | `|`                       | left |
| 4  | `&&`                      | left |
| 3  | `||`                      | left |
| 2  | `?:` (ternary)            | right |
| 1  | `= += -= *= /= %= &= ^= |= <<= >>=` | right |

---

### End of Specification