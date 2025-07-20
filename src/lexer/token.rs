use std::fmt;
use crate::lexer::position::Position;

/// Token type for the Bract language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // End of file
    Eof,

    // Literals
    Identifier(String),
    Integer {
        value: String,
        base: NumberBase,
        suffix: Option<String>,
    },
    Float {
        value: String,
        suffix: Option<String>,
    },
    String {
        value: String,
        raw: bool,
        raw_delimiter: Option<usize>,
    },
    Char(char),
    Bool(bool),
    Null,

    // Keywords
    Abort,
    Break,
    Box,
    Const,
    Continue,
    Do,
    Else,
    Enum,
    Extern,
    False,
    Fn,
    For,
    If,
    Impl,
    In,
    Let,
    Loop,
    Match,
    Mod,
    Move,
    Mut,
    Pub,
    Return,
    Struct,
    True,
    Type,
    Use,
    While,
    Async,
    Await,
    
    // Reserved keywords (not yet implemented)
    Trait,
    Try,

    // Operators
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %
    Caret,          // ^
    And,            // &
    Or,             // |
    Tilde,          // ~
    Not,            // !
    Less,           // <
    Greater,        // >
    Equal,          // =
    Question,       // ?
    Colon,          // :
    
    // Compound operators
    DoubleColon,    // ::
    Arrow,          // ->
    FatArrow,       // =>
    LogicalAnd,     // &&
    LogicalOr,      // ||
    Eq,             // ==
    NotEq,          // !=
    LessEq,         // <=
    GreaterEq,      // >=
    LeftShift,      // <<
    RightShift,     // >>
    
    // Assignment operators
    PlusEq,         // +=
    MinusEq,        // -=
    StarEq,         // *=
    SlashEq,        // /=
    PercentEq,      // %=
    AndEq,          // &=
    OrEq,           // |=
    CaretEq,        // ^=
    LeftShiftEq,    // <<=
    RightShiftEq,   // >>=
    
    // Punctuation
    LeftParen,      // (
    RightParen,     // )
    LeftBracket,    // [
    RightBracket,   // ]
    LeftBrace,      // {
    RightBrace,     // }
    Comma,          // ,
    Dot,            // .
    Semicolon,      // ;
    DotDot,         // ..
    At,             // @ (for memory annotations)
    
    // Comments (usually filtered out, but kept for documentation tools)
    LineComment(String),
    BlockComment(String),
    DocLineComment(String),
    DocBlockComment(String),
}

/// Represents the base of a numeric literal
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberBase {
    Decimal,
    Hexadecimal,
    Octal,
    Binary,
}

/// A token in the Bract language
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The type of token
    pub token_type: TokenType,
    /// The position in the source code
    pub position: Position,
}

impl Token {
    /// Create a new token
    pub fn new(token_type: TokenType, position: Position) -> Self {
        Self { token_type, position }
    }
    
    /// Returns true if the token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::Abort
                | TokenType::Break
                | TokenType::Box
                | TokenType::Const
                | TokenType::Continue
                | TokenType::Do
                | TokenType::Else
                | TokenType::Enum
                | TokenType::Extern
                | TokenType::False
                | TokenType::Fn
                | TokenType::For
                | TokenType::If
                | TokenType::Impl
                | TokenType::In
                | TokenType::Let
                | TokenType::Loop
                | TokenType::Match
                | TokenType::Mod
                | TokenType::Move
                | TokenType::Mut
                | TokenType::Pub
                | TokenType::Return
                | TokenType::Struct
                | TokenType::True
                | TokenType::Type
                | TokenType::Use
                | TokenType::While
                | TokenType::Async
                | TokenType::Await
                | TokenType::Trait
                | TokenType::Try
        )
    }
    
    /// Returns true if the token is an operator
    pub fn is_operator(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::Plus
                | TokenType::Minus
                | TokenType::Star
                | TokenType::Slash
                | TokenType::Percent
                | TokenType::Caret
                | TokenType::And
                | TokenType::Or
                | TokenType::Tilde
                | TokenType::Not
                | TokenType::Less
                | TokenType::Greater
                | TokenType::Equal
                | TokenType::Question
                | TokenType::Colon
                | TokenType::DoubleColon
                | TokenType::Arrow
                | TokenType::FatArrow
                | TokenType::LogicalAnd
                | TokenType::LogicalOr
                | TokenType::Eq
                | TokenType::NotEq
                | TokenType::LessEq
                | TokenType::GreaterEq
                | TokenType::LeftShift
                | TokenType::RightShift
                | TokenType::PlusEq
                | TokenType::MinusEq
                | TokenType::StarEq
                | TokenType::SlashEq
                | TokenType::PercentEq
                | TokenType::AndEq
                | TokenType::OrEq
                | TokenType::CaretEq
                | TokenType::LeftShiftEq
                | TokenType::RightShiftEq
                | TokenType::DotDot
        )
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::Eof => write!(f, "EOF"),
            TokenType::Identifier(s) => write!(f, "Identifier({})", s),
            TokenType::Integer { value, base, suffix } => {
                let base_str = match base {
                    NumberBase::Decimal => "decimal",
                    NumberBase::Hexadecimal => "hex",
                    NumberBase::Octal => "octal",
                    NumberBase::Binary => "binary",
                };
                
                match suffix {
                    Some(s) => write!(f, "Integer({}, {}, {})", value, base_str, s),
                    None => write!(f, "Integer({}, {})", value, base_str),
                }
            }
            TokenType::Float { value, suffix } => {
                match suffix {
                    Some(s) => write!(f, "Float({}, {})", value, s),
                    None => write!(f, "Float({})", value),
                }
            }
            TokenType::String { value, raw, raw_delimiter } => {
                if *raw {
                    match raw_delimiter {
                        Some(n) => write!(f, "RawString({}, #{}) ", value, n),
                        None => write!(f, "RawString({}) ", value),
                    }
                } else {
                    write!(f, "String({})", value)
                }
            }
            TokenType::Char(c) => write!(f, "Char({})", c),
            TokenType::Bool(b) => write!(f, "Bool({})", b),
            TokenType::Null => write!(f, "Null"),
            
            // Keywords
            TokenType::Abort => write!(f, "abort"),
            TokenType::Break => write!(f, "break"),
            TokenType::Box => write!(f, "box"),
            TokenType::Const => write!(f, "const"),
            TokenType::Continue => write!(f, "continue"),
            TokenType::Do => write!(f, "do"),
            TokenType::Else => write!(f, "else"),
            TokenType::Enum => write!(f, "enum"),
            TokenType::Extern => write!(f, "extern"),
            TokenType::False => write!(f, "false"),
            TokenType::Fn => write!(f, "fn"),
            TokenType::For => write!(f, "for"),
            TokenType::If => write!(f, "if"),
            TokenType::Impl => write!(f, "impl"),
            TokenType::In => write!(f, "in"),
            TokenType::Let => write!(f, "let"),
            TokenType::Loop => write!(f, "loop"),
            TokenType::Match => write!(f, "match"),
            TokenType::Mod => write!(f, "mod"),
            TokenType::Move => write!(f, "move"),
            TokenType::Mut => write!(f, "mut"),
            TokenType::Pub => write!(f, "pub"),
            TokenType::Return => write!(f, "return"),
            TokenType::Struct => write!(f, "struct"),
            TokenType::True => write!(f, "true"),
            TokenType::Type => write!(f, "type"),
            TokenType::Use => write!(f, "use"),
            TokenType::While => write!(f, "while"),
            TokenType::Async => write!(f, "async"),
            TokenType::Await => write!(f, "await"),
            TokenType::Trait => write!(f, "trait"),
            TokenType::Try => write!(f, "try"),
            
            // Operators
            TokenType::Plus => write!(f, "+"),
            TokenType::Minus => write!(f, "-"),
            TokenType::Star => write!(f, "*"),
            TokenType::Slash => write!(f, "/"),
            TokenType::Percent => write!(f, "%"),
            TokenType::Caret => write!(f, "^"),
            TokenType::And => write!(f, "&"),
            TokenType::Or => write!(f, "|"),
            TokenType::Tilde => write!(f, "~"),
            TokenType::Not => write!(f, "!"),
            TokenType::Less => write!(f, "<"),
            TokenType::Greater => write!(f, ">"),
            TokenType::Equal => write!(f, "="),
            TokenType::Question => write!(f, "?"),
            TokenType::Colon => write!(f, ":"),
            
            // Compound operators
            TokenType::DoubleColon => write!(f, "::"),
            TokenType::Arrow => write!(f, "->"),
            TokenType::FatArrow => write!(f, "=>"),
            TokenType::LogicalAnd => write!(f, "&&"),
            TokenType::LogicalOr => write!(f, "||"),
            TokenType::Eq => write!(f, "=="),
            TokenType::NotEq => write!(f, "!="),
            TokenType::LessEq => write!(f, "<="),
            TokenType::GreaterEq => write!(f, ">="),
            TokenType::LeftShift => write!(f, "<<"),
            TokenType::RightShift => write!(f, ">>"),
            
            // Assignment operators
            TokenType::PlusEq => write!(f, "+="),
            TokenType::MinusEq => write!(f, "-="),
            TokenType::StarEq => write!(f, "*="),
            TokenType::SlashEq => write!(f, "/="),
            TokenType::PercentEq => write!(f, "%="),
            TokenType::AndEq => write!(f, "&="),
            TokenType::OrEq => write!(f, "|="),
            TokenType::CaretEq => write!(f, "^="),
            TokenType::LeftShiftEq => write!(f, "<<="),
            TokenType::RightShiftEq => write!(f, ">>="),
            
            // Punctuation
            TokenType::LeftParen => write!(f, "("),
            TokenType::RightParen => write!(f, ")"),
            TokenType::LeftBracket => write!(f, "["),
            TokenType::RightBracket => write!(f, "]"),
            TokenType::LeftBrace => write!(f, "{{"),
            TokenType::RightBrace => write!(f, "}}"),
            TokenType::Comma => write!(f, ","),
            TokenType::Dot => write!(f, "."),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::DotDot => write!(f, ".."),
            TokenType::At => write!(f, "@"),
            
            // Comments
            TokenType::LineComment(s) => write!(f, "LineComment({})", s),
            TokenType::BlockComment(s) => write!(f, "BlockComment({})", s),
            TokenType::DocLineComment(s) => write!(f, "DocLineComment({})", s),
            TokenType::DocBlockComment(s) => write!(f, "DocBlockComment({})", s),
        }
    }
} 
