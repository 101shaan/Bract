# Prism Formal Grammar (EBNF)

> **Version:** aligned with Prism Language Specification v0.1 (Early Draft)
>
> This document contains the normative grammar for the Prism programming language. It is intentionally structured for efficient **recursive-descent / LL(*)** parsing and is free of left-recursion. Operator precedence and associativity are expressed through a layered expression grammar.
>
> • All non-terminals start with an uppercase letter.
> • *italic* names denote lexical tokens (terminals) that the lexer emits.
> • Optional elements are written `[ … ]`, repetitions `{ … }`, and alternatives with `|`.
> • The special token *SEMICOLON* is produced either by an explicit `;` character **or** by the _automatic semicolon insertion_ rule (§1.3 of the spec).

---

## 1  Lexical Grammar

```ebnf
// === Character classes ===
Digit              ::= "0"…"9"
HexDigit           ::= Digit | "a"…"f" | "A"…"F"
IdentifierStart    ::= XID_Start | "_"
IdentifierContinue ::= XID_Continue

// === Tokens ===
identifier        ::= IdentifierStart IdentifierContinue*
integerLiteral    ::= decimalLiteral | hexLiteral | octalLiteral | binaryLiteral
floatLiteral      ::= decimalLiteral "." decimalLiteral [exponent]
stringLiteral     ::= '"' … '"' | rawStringLiteral
charLiteral       ::= "'" … "'"
booleanLiteral    ::= "true" | "false"
nullLiteral       ::= "null"
literal           ::= integerLiteral | floatLiteral | stringLiteral |
                     charLiteral | booleanLiteral | nullLiteral

// (Full literal regexes appear in Appendix A of the spec.)

keyword           ::=
    "abort" | "break" | "box"   | "const" | "continue" | "do"    | "else" | "enum" |
    "extern"| "false"| "fn"    | "for"  | "if"   | "impl" | "in"   | "let"  |
    "loop"  | "match"| "mod"   | "move" | "mut"  | "pub"  | "return"|
    "struct"| "true" | "type" | "use"  | "while" | "async" | "await"

operator          ::= "::" | "->" | "=>" |
                     "&&" | "||" | "==" | "!=" | "<=" | ">=" |
                     "<<" | ">>" |
                     "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" |
                     "+" | "-"  | "*"  | "/"  | "%" | "&" | "|" | "^" | "!" | "~" | "=" | "<" | ">" | "?" | ":"

punctuation        ::= "(" | ")" | "[" | "]" | "{" | "}" | "," | "." | ";"

// === Comments & whitespace (ignored by parser) ===
lineComment       ::= "//" … <line-terminator>
blockComment      ::= "/*" ( blockComment | anyChar )* "*/"     // Nestable
whitespace        ::= " " | "\t" | "\r" | "\n" | "\f"
```

The **lexer** skips `whitespace`, `lineComment`, and `blockComment`, emitting `token`s to the syntactic analyzer.  Doc comments are processed by tooling but ignored by the grammar.

---

## 2  Operator Precedence & Associativity

| Level (high → low) | Operators | Associativity |
|--------------------|-----------|---------------|
| 14 | `()` `[]` `.` `?` | left |
| 13 | unary `! ~ &*`    | right |
| 12 | `* / %`           | left |
| 11 | `+ -`             | left |
| 10 | `<< >>`           | left |
| 9  | `< <= > >=`       | left |
| 8  | `== !=`           | left |
| 7  | `&`               | left |
| 6  | `^`               | left |
| 5  | `|`               | left |
| 4  | `&&`              | left |
| 3  | `||`              | left |
| 2  | ternary `?:`      | right |
| 1  | assignment group  | right |

The following expression grammar (Section 3) is layered to realise this table without left-recursion.

---

## 3  Expression Grammar

```ebnf
Expression            ::= AssignmentExpr

AssignmentExpr        ::= TernaryExpr
                          [ AssignmentOperator AssignmentExpr ]
AssignmentOperator    ::= "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="

TernaryExpr           ::= LogicalOrExpr [ "?" Expression ":" TernaryExpr ]

LogicalOrExpr         ::= LogicalAndExpr { "||" LogicalAndExpr }
LogicalAndExpr        ::= BitwiseOrExpr { "&&" BitwiseOrExpr }
BitwiseOrExpr         ::= BitwiseXorExpr { "|"  BitwiseXorExpr }
BitwiseXorExpr        ::= BitwiseAndExpr { "^"  BitwiseAndExpr }
BitwiseAndExpr        ::= EqualityExpr  { "&"  EqualityExpr  }
EqualityExpr          ::= RelationalExpr { ("==" | "!=") RelationalExpr }
RelationalExpr        ::= ShiftExpr      { ("<" | "<=" | ">" | ">=") ShiftExpr }
ShiftExpr             ::= AddExpr        { ("<<" | ">>") AddExpr }
AddExpr               ::= MulExpr        { ("+"  | "-")  MulExpr }
MulExpr               ::= UnaryExpr      { ("*"  | "/" | "%") UnaryExpr }

UnaryExpr             ::= [UnaryOperator] CastExpr
UnaryOperator         ::= "!" | "~" | "&" | "*" | "-" | "+"

CastExpr              ::= PostfixExpr [ "as" Type ]

PostfixExpr           ::= PrimaryExpr { PostfixOp }
PostfixOp             ::= CallOp | IndexOp | FieldOp | QuestionOp | AwaitOp
CallOp                ::= "(" [ ArgumentList ] ")"
IndexOp               ::= "[" Expression "]"
FieldOp               ::= "." identifier
QuestionOp            ::= "?"                        //  null-assert / option unwrap
AwaitOp               ::= "." "await"

PrimaryExpr           ::= literal
                       | identifier
                       | PathExpr
                       | "(" Expression ")"
                       | "&" ["mut"] Expression
                       | "box" Expression
                       | StructInit
                       | ClosureExpr
                       | ArrayLiteral
                       | RangeExpr
                       | MacroCall

PathExpr              ::= identifier { "::" identifier }
StructInit            ::= PathExpr "{" FieldInitList "}"
FieldInitList         ::= FieldInit { "," FieldInit } [ "," ]
FieldInit             ::= identifier ":" Expression | identifier   // shorthand

ClosureExpr           ::= ["move"] "|" ParameterList? "|" Expression
ArrayLiteral         ::= "[" [Expression { "," Expression }] "]"
RangeExpr            ::= Expression ".." [Expression] | ".." Expression
MacroCall            ::= identifier "!" "(" TokenStream? ")"
TokenStream          ::= /* implementation-defined sequence of tokens */
ArgumentList          ::= Expression { "," Expression } [ "," ]
ParameterList         ::= Parameter { "," Parameter } [ "," ]
Parameter             ::= Pattern [":" Type]
```

---

## 4  Statement Grammar

```ebnf
Block                ::= "{" { Statement } "}"
Statement            ::= DeclarationStmt
                       | Item
                       | ExpressionStmt
                       | ControlStmt
                       | SEMICOLON              // empty statement

DeclarationStmt      ::= "let" ["mut"] Pattern [":" Type] ["=" Expression] SEMICOLON
ExpressionStmt       ::= Expression SEMICOLON

// --- Control flow ---
ControlStmt          ::= IfStmt | WhileStmt | ForStmt | LoopStmt | MatchStmt
                       | BreakStmt | ContinueStmt | ReturnStmt

IfStmt               ::= "if" Expression Block { "else" IfTail }
IfTail               ::= IfStmt | Block

WhileStmt            ::= "while" Expression Block
ForStmt              ::= "for" Pattern "in" Expression Block
LoopStmt             ::= [identifier ":"] "loop" Block

BreakStmt            ::= "break" [identifier] SEMICOLON
ContinueStmt         ::= "continue" [identifier] SEMICOLON
ReturnStmt           ::= "return" [Expression] SEMICOLON

MatchStmt            ::= "match" Expression "{" MatchArmList "}"
MatchArmList         ::= MatchArm { "," MatchArm } [ "," ]
MatchArm             ::= Pattern [ "if" Expression ] "=>" ( Expression | Block )
```

---

## 5  Pattern Grammar

```ebnf
Pattern               ::= "_"
                       | literal
                       | identifier
                       | "&" ["mut"] Pattern
                       | StructPattern
                       | EnumPattern
                       | TuplePattern
                       | ArrayPattern

StructPattern        ::= PathExpr "{" FieldPatList "}"
FieldPatList         ::= FieldPattern { "," FieldPattern } [ "," ]
FieldPattern         ::= identifier ":" Pattern | identifier | ".."

EnumPattern          ::= PathExpr [ "(" PatternList? ")" ]
TuplePattern         ::= "(" PatternList? ")"
ArrayPattern         ::= "[" PatternList? "]"
PatternList          ::= Pattern { "," Pattern } [ "," ]
```

---

## 6  Type Grammar

```ebnf
Type                  ::= FunctionType
                       | SliceType
                       | ArrayType
                       | PointerType
                       | ReferenceType
                       | TupleType
                       | PathType

// --- Basics ---
PathType             ::= identifier { "::" identifier } [ GenericArgs ]
GenericArgs          ::= "<" TypeList ">"
TypeList             ::= Type { "," Type } [ "," ]

// --- Compound forms ---
FunctionType         ::= "fn" "(" ParamTypeList? ")" [ "->" Type ]
ParamTypeList        ::= Type { "," Type } [ "," ]

SliceType            ::= "&[" Type "]"
ArrayType            ::= "[" Type ";" Expression "]"
PointerType          ::= "*" ("const" | "mut") Type
ReferenceType        ::= "&" ["mut"] Type
TupleType            ::= "(" TypeList? ")"
```

---

## 7  Item & Module Grammar

```ebnf
Module                ::= { UseDecl | Item } EOF

// --- Imports/Exports ---
UseDecl              ::= "use" UsePath SEMICOLON
UsePath              ::= PathExpr [ "as" identifier ]

// --- Top-level items ---
Item                 ::= FnDef | StructDef | EnumDef | TypeAlias | ConstDecl | ModDecl | ImplBlock

// Functions
FnDef                ::= [Visibility] "fn" identifier GenericParamList?
                          "(" ParamList? ")" ReturnType? Block
ReturnType           ::= "->" Type
ParamList            ::= Parameter { "," Parameter } [ "," ]
GenericParamList     ::= "<" GenericParam { "," GenericParam } [ "," ] ">"
GenericParam         ::= identifier [":" Type] [WhereClause]
WhereClause          ::= "where" WherePredicateList
WherePredicateList   ::= WherePredicate { "," WherePredicate } [ "," ]
WherePredicate       ::= Type
Visibility           ::= "pub"

// Structs
StructDef            ::= [Visibility] "struct" identifier StructBody
StructBody           ::= VariantStruct | TupleStruct | UnitStruct
VariantStruct        ::= "{" FieldDeclList? "}"
TupleStruct          ::= "(" FieldDeclList? ")" SEMICOLON
UnitStruct           ::= SEMICOLON
FieldDeclList        ::= FieldDecl { "," FieldDecl } [ "," ]
FieldDecl            ::= [Visibility] identifier ":" Type

// Enums
EnumDef              ::= [Visibility] "enum" identifier GenericParamList?
                         "{" EnumVariantList "}"
EnumVariantList      ::= EnumVariant { "," EnumVariant } [ "," ]
EnumVariant          ::= identifier ( VariantStruct | TupleStruct | UnitStruct )?

// Type aliases & constants
TypeAlias            ::= [Visibility] "type" identifier "=" Type SEMICOLON
ConstDecl            ::= [Visibility] "const" identifier ":" Type "=" Expression SEMICOLON

// Sub-modules
ModDecl              ::= [Visibility] "mod" identifier ( SEMICOLON | Block )

// Impl blocks (methods & associated items)
ImplBlock            ::= "impl" [ GenericParamList ] PathType Block
```

---

## 8  Comments & Whitespace Handling

• The lexer **always** discards `whitespace`, `lineComment`, and `blockComment` tokens.
• New-line characters participate in **Automatic Semicolon Insertion (ASI)**:
  – After a token that can terminate a statement (`identifier`, `literal`, `)`, `]`, `}`),
    if the next non-trivia token begins a construct that cannot follow immediately
    (per §1.3), the lexer injects an implicit *SEMICOLON*.
• A *SEMICOLON* token in the grammar refers to either an explicit `;` character
  **or** an implicit one produced by ASI.

---

### End of Grammar 