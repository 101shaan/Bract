//! Main parser implementation for the Bract programming language

use crate::lexer::{Lexer, Token, TokenType, Position};
use crate::ast::{Module, Item, Expr, Stmt, Span, Visibility, Parameter, InternedString, Pattern, Type, MemoryStrategy};
use super::error::{
    ParseError, ParseResult, ParseContext, ExpectedToken, Suggestion, SuggestionCategory,
    suggest_similar_identifiers, suggest_for_context, UnclosedDelimiter
};
use std::collections::HashMap;

/// String interner for efficient string storage
pub struct StringInterner {
    strings: Vec<String>,
    map: HashMap<String, u32>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            map: HashMap::new(),
        }
    }
    
    pub fn intern(&mut self, s: &str) -> InternedString {
        if let Some(&id) = self.map.get(s) {
            InternedString::new(id)
        } else {
            let id = self.strings.len() as u32;
            self.strings.push(s.to_string());
            self.map.insert(s.to_string(), id);
            InternedString::new(id)
        }
    }
    
    pub fn get(&self, interned: &InternedString) -> Option<&str> {
        self.strings.get(interned.id as usize).map(|s| s.as_str())
    }
}

/// The main parser struct that converts tokens to AST
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    pub(super) current_token: Option<Token>,
    pub(super) interner: StringInterner,
    errors: Vec<ParseError>,
    /// Current parsing context for better error messages
    context_stack: Vec<ParseContext>,
    /// Delimiter stack for tracking unclosed delimiters
    delimiter_stack: Vec<(TokenType, Position, String)>,
    /// Keywords for similarity matching
    keywords: Vec<&'static str>,
}

impl<'a> Parser<'a> {
    /// Create a new parser from source code
    pub fn new(input: &'a str, file_id: usize) -> ParseResult<Self> {
        let mut lexer = Lexer::new(input, file_id);
        let current_token = match lexer.next_token() {
            Ok(token) => Some(token),
            Err(err) => return Err(ParseError::from(err)),
        };
        
        let keywords = vec![
            "fn", "struct", "enum", "impl", "trait", "type", "const", "static",
            "let", "mut", "if", "else", "while", "for", "loop", "match",
            "return", "break", "continue", "where", "use", "mod", "pub",
            "self", "Self", "super", "crate", "as", "in", "ref", "move",
            "true", "false", "null", "i8", "i16", "i32", "i64", "i128",
            "u8", "u16", "u32", "u64", "u128", "f32", "f64", "bool",
            "char", "str", "String", "Vec", "Option", "Result"
        ];
        
        Ok(Parser {
            lexer,
            current_token,
            interner: StringInterner::new(),
            errors: Vec::new(),
            context_stack: vec![ParseContext::TopLevel],
            delimiter_stack: Vec::new(),
            keywords,
        })
    }
    
    /// Get the current token without consuming it
    pub fn current_token(&self) -> Option<&Token> {
        self.current_token.as_ref()
    }
    
    /// Advance to the next token
    pub fn advance(&mut self) -> ParseResult<()> {
        match self.lexer.next_token() {
            Ok(token) => {
                self.current_token = Some(token);
                Ok(())
            }
            Err(err) => {
                let enhanced_error = ParseError::LexerError {
                    error: err,
                    suggestions: vec![
                        Suggestion::new("Check for invalid characters in source code", self.current_position())
                            .with_category(SuggestionCategory::Syntax)
                    ],
                    help: Some("Lexer errors often indicate invalid character sequences or encoding issues.".to_string()),
                };
                Err(enhanced_error)
            }
        }
    }
    
    /// Check if current token matches the expected type
    pub fn check(&self, token_type: &TokenType) -> bool {
        self.current_token
            .as_ref()
            .map(|t| std::mem::discriminant(&t.token_type) == std::mem::discriminant(token_type))
            .unwrap_or(false)
    }
    
    /// Check if we've reached the end of input
    pub fn is_at_end(&self) -> bool {
        self.current_token
            .as_ref()
            .map(|t| matches!(t.token_type, TokenType::Eof))
            .unwrap_or(true)
    }
    
    /// Consume a token if it matches the expected type
    pub fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance().unwrap_or(());
            true
        } else {
            false
        }
    }
    
    /// Enhanced expect method with context-aware error messages
    pub fn expect(&mut self, expected: TokenType, description: &str) -> ParseResult<Token> {
        if let Some(token) = &self.current_token {
            if std::mem::discriminant(&token.token_type) == std::mem::discriminant(&expected) {
                let token = token.clone();
                
                // Track delimiters
                self.track_delimiter(&token.token_type, token.position);
                
                self.advance()?;
                Ok(token)
            } else {
                let context = self.current_context().clone();
                let position = token.position;
                let found = token.token_type.clone();
                
                // Generate intelligent suggestions
                let mut suggestions = suggest_for_context(&context, &found);
                
                // Add similarity-based suggestions for identifiers
                if let TokenType::Identifier(ref name) = found {
                    let similar = suggest_similar_identifiers(name, &self.keywords);
                    for similar_word in similar {
                        suggestions.push(
                            Suggestion::new(&format!("Did you mean '{}'?", similar_word), position)
                                .with_replacement(&similar_word)
                                .with_category(SuggestionCategory::Syntax)
                                .with_confidence(0.7)
                        );
                    }
                }
                
                // Context-specific help
                let help = match (&expected, &context) {
                    (TokenType::Semicolon, ParseContext::Statement) => {
                        Some("Most statements in Bract must end with a semicolon (;)".to_string())
                    }
                    (TokenType::LeftBrace, ParseContext::FunctionBody) => {
                        Some("Function bodies must be enclosed in braces { }".to_string())
                    }
                    (TokenType::Colon, ParseContext::TypeAnnotation) => {
                        Some("Type annotations are specified with a colon (:) followed by the type".to_string())
                    }
                    _ => None,
                };
                
                Err(ParseError::UnexpectedToken {
                    expected: vec![ExpectedToken::new(&format!("{:?}", expected), description)],
                    found,
                    position,
                    context,
                    suggestions,
                    help,
                })
            }
        } else {
            let context = self.current_context().clone();
            let position = self.lexer.get_position();
            
            // Check for unclosed delimiters
            let unclosed_delimiters: Vec<UnclosedDelimiter> = self.delimiter_stack
                .iter()
                .map(|(delim, pos, ctx)| UnclosedDelimiter {
                    delimiter: delim.clone(),
                    open_position: *pos,
                    context: ctx.clone(),
                })
                .collect();
            
            let suggestions = if !unclosed_delimiters.is_empty() {
                vec![Suggestion::new("Close unclosed delimiters before end of file", position)
                    .with_category(SuggestionCategory::Syntax)]
            } else {
                vec![Suggestion::new(&format!("Add {} before end of file", description), position)
                    .with_category(SuggestionCategory::Syntax)]
            };
            
            Err(ParseError::UnexpectedEof {
                expected: vec![ExpectedToken::new(&format!("{:?}", expected), description)],
                position,
                context,
                unclosed_delimiters,
                suggestions,
            })
        }
    }
    
    /// Track opening/closing delimiters for better error messages
    fn track_delimiter(&mut self, token_type: &TokenType, position: Position) {
        match token_type {
            TokenType::LeftParen => {
                self.delimiter_stack.push((TokenType::RightParen, position, format!("{}", self.current_context())));
            }
            TokenType::LeftBrace => {
                self.delimiter_stack.push((TokenType::RightBrace, position, format!("{}", self.current_context())));
            }
            TokenType::LeftBracket => {
                self.delimiter_stack.push((TokenType::RightBracket, position, format!("{}", self.current_context())));
            }
            TokenType::RightParen | TokenType::RightBrace | TokenType::RightBracket => {
                if let Some((expected_closer, _, _)) = self.delimiter_stack.last() {
                    if std::mem::discriminant(token_type) == std::mem::discriminant(expected_closer) {
                        self.delimiter_stack.pop();
                    }
                }
            }
            _ => {}
        }
    }
    
    /// Get current parsing context
    pub fn current_context(&self) -> &ParseContext {
        self.context_stack.last().unwrap_or(&ParseContext::TopLevel)
    }
    
    /// Enter a new parsing context
    pub fn enter_context(&mut self, context: ParseContext) {
        self.context_stack.push(context);
    }
    
    /// Exit current parsing context
    pub fn exit_context(&mut self) {
        if self.context_stack.len() > 1 {
            self.context_stack.pop();
        }
    }
    
    /// Get current position for error reporting
    pub fn current_position(&self) -> Position {
        self.current_token
            .as_ref()
            .map(|t| t.position)
            .unwrap_or_else(|| self.lexer.get_position())
    }
    
    /// Add an error to the error list but continue parsing
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }
    
    /// Get all accumulated errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
    
    /// Extract the string interner (consumes the parser)
    pub fn take_interner(self) -> StringInterner {
        self.interner
    }
    
    /// Enhanced synchronize parser state after an error (error recovery)
    pub fn synchronize(&mut self) {
        let mut recovery_attempts = 0;
        const MAX_RECOVERY_ATTEMPTS: usize = 10;
        
        while !self.is_at_end() && recovery_attempts < MAX_RECOVERY_ATTEMPTS {
            if let Some(token) = &self.current_token {
                match &token.token_type {
                    // Primary recovery points
                    TokenType::Semicolon => {
                        self.advance().unwrap_or(());
                        return;
                    }
                    
                    // Item-level recovery points
                    TokenType::Fn | TokenType::Struct | TokenType::Enum | 
                    TokenType::Type | TokenType::Const | TokenType::Impl |
                    TokenType::Mod | TokenType::Use => {
                        // Reset to top-level context
                        self.context_stack.clear();
                        self.context_stack.push(ParseContext::TopLevel);
                        return;
                    }
                    
                    // Statement-level recovery points
                    TokenType::Let | TokenType::If | TokenType::While | 
                    TokenType::For | TokenType::Return | TokenType::Break |
                    TokenType::Continue => return,
                    
                    // Block boundaries
                    TokenType::RightBrace => {
                        self.advance().unwrap_or(());
                        return;
                    }
                    
                    _ => {
                        self.advance().unwrap_or(());
                        recovery_attempts += 1;
                    }
                }
            } else {
                break;
            }
        }
        
        // If we couldn't recover properly, clear contexts
        if recovery_attempts >= MAX_RECOVERY_ATTEMPTS {
            self.context_stack.clear();
            self.context_stack.push(ParseContext::TopLevel);
        }
    }
    
    /// Parse a complete module (top-level entry point) with enhanced error handling
    pub fn parse_module(&mut self) -> ParseResult<Module> {
        let start_pos = self.current_position();
        let mut items = Vec::new();
        let mut error_count = 0;
        const MAX_ERRORS_PER_MODULE: usize = 50;
        
        self.enter_context(ParseContext::TopLevel);
        
        while !self.is_at_end() && error_count < MAX_ERRORS_PER_MODULE {
            match self.parse_item() {
                Ok(item) => {
                    items.push(item);
                    // Reset error count on successful parse
                    error_count = 0;
                }
                Err(err) => {
                    self.add_error(err);
                    self.synchronize();
                    error_count += 1;
                    
                    // Prevent infinite error loops
                    if error_count >= 5 {
                        // Try a more aggressive recovery
                        while !self.is_at_end() {
                            if let Some(token) = &self.current_token {
                                if matches!(token.token_type, 
                                    TokenType::Fn | TokenType::Struct | TokenType::Enum |
                                    TokenType::Type | TokenType::Const | TokenType::Impl |
                                    TokenType::Mod | TokenType::Use
                                ) {
                                    break;
                                }
                                self.advance().unwrap_or(());
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        self.exit_context();
        
        // Report if we hit the error limit
        if error_count >= MAX_ERRORS_PER_MODULE {
            self.add_error(ParseError::InternalError {
                message: "Too many parse errors, stopping module parsing".to_string(),
                position: self.current_position(),
                debug_info: Some(format!("Reached maximum error limit of {}", MAX_ERRORS_PER_MODULE)),
            });
        }
        
        let end_pos = self.current_position();
        Ok(Module {
            items,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a top-level item (function, struct, etc.) with enhanced error handling
    pub fn parse_item(&mut self) -> ParseResult<Item> {
        let start_pos = self.current_position();
        
        // Parse visibility modifier
        let visibility = if self.match_token(&TokenType::Pub) {
            Visibility::Public
        } else {
            Visibility::Private
        };
        
        // Skip annotations for now (parse but don't use them yet)
        while self.check(&TokenType::At) {
            self.enter_context(ParseContext::MemoryAnnotation);
            // Skip annotation - for now just advance past it
            while !self.is_at_end() && !self.check(&TokenType::Fn) && !self.check(&TokenType::Struct) 
                && !self.check(&TokenType::Enum) && !self.check(&TokenType::Type) 
                && !self.check(&TokenType::Const) && !self.check(&TokenType::Mod) 
                && !self.check(&TokenType::Impl) && !self.check(&TokenType::Use) {
                self.advance()?;
            }
            self.exit_context();
        }
        
        if let Some(token) = &self.current_token {
            match &token.token_type {
                TokenType::Fn => {
                    self.enter_context(ParseContext::FunctionDeclaration);
                    let result = self.parse_function(visibility, start_pos);
                    self.exit_context();
                    result
                },
                TokenType::Struct => {
                    self.enter_context(ParseContext::StructDeclaration);
                    let result = self.parse_struct(visibility, start_pos);
                    self.exit_context();
                    result
                },
                TokenType::Enum => {
                    self.enter_context(ParseContext::EnumDeclaration);
                    let result = self.parse_enum(visibility, start_pos);
                    self.exit_context();
                    result
                },
                TokenType::Type => self.parse_type_alias(visibility, start_pos),
                TokenType::Const => self.parse_const(visibility, start_pos),
                TokenType::Mod => {
                    self.enter_context(ParseContext::ModuleDeclaration);
                    let result = self.parse_module_decl(visibility, start_pos);
                    self.exit_context();
                    result
                },
                TokenType::Impl => {
                    self.enter_context(ParseContext::ImplBlock);
                    let result = self.parse_impl_block(start_pos);
                    self.exit_context();
                    result
                },
                TokenType::Use => {
                    self.enter_context(ParseContext::UseDeclaration);
                    let result = self.parse_use_decl(start_pos);
                    self.exit_context();
                    result
                },
                _ => {
                    let context = self.current_context().clone();
                    let suggestions = vec![
                        Suggestion::new("Start with a declaration keyword (fn, struct, enum, etc.)", token.position)
                            .with_category(SuggestionCategory::Syntax),
                        Suggestion::new("Check if this should be inside a function body", token.position)
                            .with_category(SuggestionCategory::Semantics),
                    ];
                    
                    // Add keyword suggestions if it's an identifier
                    let mut enhanced_suggestions = suggestions;
                    if let TokenType::Identifier(ref name) = token.token_type {
                        let keywords = ["fn", "struct", "enum", "impl", "type", "const", "mod", "use"];
                        let similar = suggest_similar_identifiers(name, &keywords);
                        for similar_keyword in similar {
                            enhanced_suggestions.push(
                                Suggestion::new(&format!("Did you mean '{}'?", similar_keyword), token.position)
                                    .with_replacement(&similar_keyword)
                                    .with_category(SuggestionCategory::Syntax)
                                    .with_confidence(0.8)
                            );
                        }
                    }
                    
                    Err(ParseError::UnexpectedToken {
                        expected: vec![
                            ExpectedToken::new("fn", "function declaration").with_example("fn main() {}"),
                            ExpectedToken::new("struct", "structure declaration").with_example("struct Point { x: i32, y: i32 }"),
                            ExpectedToken::new("enum", "enumeration declaration").with_example("enum Option<T> { Some(T), None }"),
                            ExpectedToken::new("impl", "implementation block").with_example("impl SomeStruct { }"),
                            ExpectedToken::new("type", "type alias").with_example("type MyInt = i32;"),
                            ExpectedToken::new("const", "constant declaration").with_example("const PI: f64 = 3.14159;"),
                            ExpectedToken::new("mod", "module declaration").with_example("mod my_module { }"),
                            ExpectedToken::new("use", "use declaration").with_example("use std::collections::HashMap;"),
                        ],
                        found: token.token_type.clone(),
                        position: token.position,
                        context,
                        suggestions: enhanced_suggestions,
                        help: Some("Items are top-level declarations like functions, structs, and enums that form the building blocks of your program.".to_string()),
                    })
                }
            }
        } else {
            let context = self.current_context().clone();
            let suggestions = vec![
                Suggestion::new("Add a top-level declaration", self.current_position())
                    .with_category(SuggestionCategory::Syntax)
                    .with_replacement("fn main() {\n    // Your code here\n}"),
            ];
            
            Err(ParseError::UnexpectedEof {
                expected: vec![
                    ExpectedToken::new("item declaration", "function, struct, enum, or other top-level construct")
                ],
                position: self.current_position(),
                context,
                unclosed_delimiters: Vec::new(),
                suggestions,
            })
        }
    }
    
    /// Parse a function declaration
    fn parse_function(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Fn, "function declaration")?;
        
        // Function name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "function name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected function name".to_string(),
                position: name_token.position,
                context: self.current_context().clone(),
                suggestions: vec![
                    Suggestion::new("Use a valid identifier for the function name", name_token.position)
                        .with_category(SuggestionCategory::Syntax)
                ],
                help: Some("Function names must be valid identifiers starting with a letter or underscore".to_string()),
                related_errors: Vec::new(),
            });
        };
        
        // Generic parameters
        let generics = if self.match_token(&TokenType::Less) {
            let mut generic_params = Vec::new();
            
            if !self.check(&TokenType::Greater) {
                loop {
                    let param_start = self.current_position();
                    
                    // Parse generic parameter name
                    if let Some(token) = &self.current_token {
                        if let TokenType::Identifier(param_name) = &token.token_type {
                            let name = self.interner.intern(param_name);
                            self.advance()?;
                            
                            // TODO: Parse trait bounds and default values
                            generic_params.push(crate::ast::GenericParam {
                                name,
                                bounds: Vec::new(),
                                default: None,
                                span: Span::new(param_start, self.current_position()),
                            });
                        } else {
                            return Err(ParseError::InvalidSyntax {
                                message: "Expected generic parameter name".to_string(),
                                position: token.position,
                                context: ParseContext::GenericParameters,
                                suggestions: vec![
                                    Suggestion::new("Use a valid type parameter name", token.position)
                                        .with_category(SuggestionCategory::Syntax)
                                        .with_replacement("T")
                                ],
                                help: Some("Generic parameters should be valid identifiers, typically single capital letters like T, U, V".to_string()),
                                related_errors: Vec::new(),
                            });
                        }
                    }
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }
            
            self.expect(TokenType::Greater, "generic parameters")?;
            generic_params
        } else {
            Vec::new()
        };
        
        // Parameters
        self.expect(TokenType::LeftParen, "function parameters")?;
        let mut params = Vec::new();
        
        if !self.check(&TokenType::RightParen) {
            loop {
                let param_start = self.current_position();
                
                // Check for special self parameters: self, &self, &mut self
                let (pattern, type_annotation) = if self.is_self_parameter() {
                    self.parse_self_parameter()?
                } else {
                    let pattern = self.parse_pattern()?;
                    let type_annotation = if self.match_token(&TokenType::Colon) {
                        Some(self.parse_type()?)
                    } else {
                        None
                    };
                    (pattern, type_annotation)
                };
                
                params.push(Parameter {
                    pattern,
                    type_annotation,
                    span: Span::new(param_start, self.current_position()),
                });
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }
        
        self.expect(TokenType::RightParen, "function parameters")?;
        
        // Return type
        let return_type = if self.match_token(&TokenType::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        // Function body
        let body = if self.check(&TokenType::LeftBrace) {
            Some(self.parse_block_expression()?)
        } else {
            self.expect(TokenType::Semicolon, "function body or semicolon")?;
            None
        };
        
        let end_pos = self.current_position();
        Ok(Item::Function {
            visibility,
            name,
            generics,
            params,
            return_type,
            body,
            is_extern: false, // TODO: handle extern functions
            span: Span::new(start_pos, end_pos),
        })
    }
    
    // Implement struct parsing
    fn parse_struct(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Struct, "struct declaration")?;
        
        // Struct name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "struct name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected struct name".to_string(),
                position: name_token.position,
                context: self.current_context().clone(),
                suggestions: vec![
                    Suggestion::new("Use a valid identifier for the struct name", name_token.position)
                        .with_category(SuggestionCategory::Syntax)
                ],
                help: Some("Struct names must be valid identifiers starting with a letter or underscore".to_string()),
                related_errors: Vec::new(),
            });
        };
        
        // Generic parameters
        let generics = if self.match_token(&TokenType::Less) {
            let mut generic_params = Vec::new();
            
            if !self.check(&TokenType::Greater) {
                loop {
                    let param_start = self.current_position();
                    
                    // Parse generic parameter name
                    if let Some(token) = &self.current_token {
                        if let TokenType::Identifier(param_name) = &token.token_type {
                            let name = self.interner.intern(param_name);
                            self.advance()?;
                            
                            // TODO: Parse trait bounds and default values
                            generic_params.push(crate::ast::GenericParam {
                                name,
                                bounds: Vec::new(),
                                default: None,
                                span: Span::new(param_start, self.current_position()),
                            });
                        } else {
                            return Err(ParseError::InvalidSyntax {
                                message: "Expected generic parameter name".to_string(),
                                position: token.position,
                                context: ParseContext::GenericParameters,
                                suggestions: vec![
                                    Suggestion::new("Use a valid type parameter name", token.position)
                                        .with_category(SuggestionCategory::Syntax)
                                        .with_replacement("T")
                                ],
                                help: Some("Generic parameters should be valid identifiers, typically single capital letters like T, U, V".to_string()),
                                related_errors: Vec::new(),
                            });
                        }
                    }
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }
            
            self.expect(TokenType::Greater, "generic parameters")?;
            generic_params
        } else {
            Vec::new()
        };
        
        // Parse struct fields
        let fields = if self.match_token(&TokenType::LeftBrace) {
            // Named fields: struct Point { x: i32, y: i32 }
            let mut field_list = Vec::new();
            
            while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                let field_start = self.current_position();
                
                // Field visibility (default private)
                let field_visibility = if self.match_token(&TokenType::Pub) {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                
                // Field name
                let field_name_token = self.expect(TokenType::Identifier("".to_string()), "field name")?;
                let field_name = if let TokenType::Identifier(name_str) = field_name_token.token_type {
                    self.interner.intern(&name_str)
                } else {
                    return Err(ParseError::InvalidSyntax {
                        message: "Expected field name".to_string(),
                        position: field_name_token.position,
                        context: self.current_context().clone(),
                        suggestions: vec![
                            Suggestion::new("Use a valid identifier for the field name", field_name_token.position)
                                .with_category(SuggestionCategory::Syntax)
                        ],
                        help: Some("Field names must be valid identifiers starting with a letter or underscore".to_string()),
                        related_errors: Vec::new(),
                    });
                };
                
                self.expect(TokenType::Colon, "field type annotation")?;
                let field_type = self.parse_type()?;
                
                field_list.push(crate::ast::StructField {
                    visibility: field_visibility,
                    name: field_name,
                    field_type,
                    span: Span::new(field_start, self.current_position()),
                });
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
            
            self.expect(TokenType::RightBrace, "struct fields")?;
            crate::ast::StructFields::Named(field_list)
        } else if self.match_token(&TokenType::LeftParen) {
            // Tuple struct: struct Point(i32, i32);
            let mut types = Vec::new();
            
            while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                types.push(self.parse_type()?);
                
                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
            
            self.expect(TokenType::RightParen, "tuple struct fields")?;
            self.expect(TokenType::Semicolon, "tuple struct declaration")?;
            crate::ast::StructFields::Tuple(types)
        } else {
            // Unit struct: struct Unit;
            self.expect(TokenType::Semicolon, "unit struct declaration")?;
            crate::ast::StructFields::Unit
        };
        
        let end_pos = self.current_position();
        Ok(Item::Struct {
            visibility,
            name,
            generics,
            fields,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    // Implement enum parsing
    fn parse_enum(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Enum, "enum declaration")?;
        
        // Enum name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "enum name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected enum name".to_string(),
                position: name_token.position,
                context: self.current_context().clone(),
                suggestions: vec![
                    Suggestion::new("Use a valid identifier for the enum name", name_token.position)
                        .with_category(SuggestionCategory::Syntax)
                ],
                help: Some("Enum names must be valid identifiers starting with a letter or underscore".to_string()),
                related_errors: Vec::new(),
            });
        };
        
        // Generic parameters
        let generics = if self.match_token(&TokenType::Less) {
            let mut generic_params = Vec::new();
            
            if !self.check(&TokenType::Greater) {
                loop {
                    let param_start = self.current_position();
                    
                    // Parse generic parameter name
                    if let Some(token) = &self.current_token {
                        if let TokenType::Identifier(param_name) = &token.token_type {
                            let name = self.interner.intern(param_name);
                            self.advance()?;
                            
                            // TODO: Parse trait bounds and default values
                            generic_params.push(crate::ast::GenericParam {
                                name,
                                bounds: Vec::new(),
                                default: None,
                                span: Span::new(param_start, self.current_position()),
                            });
                        } else {
                            return Err(ParseError::InvalidSyntax {
                                message: "Expected generic parameter name".to_string(),
                                position: token.position,
                                context: ParseContext::GenericParameters,
                                suggestions: vec![
                                    Suggestion::new("Use a valid type parameter name", token.position)
                                        .with_category(SuggestionCategory::Syntax)
                                        .with_replacement("T")
                                ],
                                help: Some("Generic parameters should be valid identifiers, typically single capital letters like T, U, V".to_string()),
                                related_errors: Vec::new(),
                            });
                        }
                    }
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
            }
            
            self.expect(TokenType::Greater, "generic parameters")?;
            generic_params
        } else {
            Vec::new()
        };
        
        // Parse enum variants
        self.expect(TokenType::LeftBrace, "enum variants")?;
        let mut variants = Vec::new();
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            let variant_start = self.current_position();
            
            // Variant name
            let variant_name_token = self.expect(TokenType::Identifier("".to_string()), "variant name")?;
            let variant_name = if let TokenType::Identifier(name_str) = variant_name_token.token_type {
                self.interner.intern(&name_str)
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected variant name".to_string(),
                    position: variant_name_token.position,
                    context: self.current_context().clone(),
                    suggestions: vec![
                        Suggestion::new("Use a valid identifier for the variant name", variant_name_token.position)
                            .with_category(SuggestionCategory::Syntax)
                    ],
                    help: Some("Variant names must be valid identifiers starting with a letter or underscore".to_string()),
                    related_errors: Vec::new(),
                });
            };
            
            // Parse variant fields
            let fields = if self.match_token(&TokenType::LeftBrace) {
                // Named fields: Some { value: T }
                let mut field_list = Vec::new();
                
                while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                    let field_start = self.current_position();
                    
                    let field_name_token = self.expect(TokenType::Identifier("".to_string()), "field name")?;
                    let field_name = if let TokenType::Identifier(name_str) = field_name_token.token_type {
                        self.interner.intern(&name_str)
                    } else {
                        return Err(ParseError::InvalidSyntax {
                            message: "Expected field name".to_string(),
                            position: field_name_token.position,
                            context: self.current_context().clone(),
                            suggestions: vec![
                                Suggestion::new("Use a valid identifier for the field name", field_name_token.position)
                                    .with_category(SuggestionCategory::Syntax)
                            ],
                            help: Some("Field names must be valid identifiers starting with a letter or underscore".to_string()),
                            related_errors: Vec::new(),
                        });
                    };
                    
                    self.expect(TokenType::Colon, "field type")?;
                    let field_type = self.parse_type()?;
                    
                    field_list.push(crate::ast::StructField {
                        visibility: Visibility::Public, // Enum fields are always public
                        name: field_name,
                        field_type,
                        span: Span::new(field_start, self.current_position()),
                    });
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
                
                self.expect(TokenType::RightBrace, "variant fields")?;
                crate::ast::StructFields::Named(field_list)
            } else if self.match_token(&TokenType::LeftParen) {
                // Tuple fields: Some(T)
                let mut types = Vec::new();
                
                while !self.check(&TokenType::RightParen) && !self.is_at_end() {
                    types.push(self.parse_type()?);
                    
                    if !self.match_token(&TokenType::Comma) {
                        break;
                    }
                }
                
                self.expect(TokenType::RightParen, "variant fields")?;
                crate::ast::StructFields::Tuple(types)
            } else {
                // Unit variant: None
                crate::ast::StructFields::Unit
            };
            
            // Optional discriminant value
            let discriminant = if self.match_token(&TokenType::Equal) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            variants.push(crate::ast::EnumVariant {
                name: variant_name,
                fields,
                discriminant,
                span: Span::new(variant_start, self.current_position()),
            });
            
            if !self.match_token(&TokenType::Comma) {
                break;
            }
        }
        
        self.expect(TokenType::RightBrace, "enum variants")?;
        
        let end_pos = self.current_position();
        Ok(Item::Enum {
            visibility,
            name,
            generics,
            variants,
            span: Span::new(start_pos, end_pos),
        })
    }

    fn parse_type_alias(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Type, "type alias")?;
        
        // Type alias name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "type name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected type name".to_string(),
                position: name_token.position,
                context: self.current_context().clone(),
                suggestions: vec![
                    Suggestion::new("Use a valid identifier for the type name", name_token.position)
                        .with_category(SuggestionCategory::Syntax)
                ],
                help: Some("Type names must be valid identifiers starting with a letter or underscore".to_string()),
                related_errors: Vec::new(),
            });
        };
        
        // Generic parameters (placeholder for now)
        let generics = Vec::new();
        
        self.expect(TokenType::Equal, "type alias")?;
        let target_type = self.parse_type()?;
        self.expect(TokenType::Semicolon, "type alias")?;
        
        let end_pos = self.current_position();
        Ok(Item::TypeAlias {
            visibility,
            name,
            generics,
            target_type,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    fn parse_const(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Const, "const declaration")?;
        
        // Const name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "const name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected const name".to_string(),
                position: name_token.position,
                context: self.current_context().clone(),
                suggestions: vec![
                    Suggestion::new("Use a valid identifier for the const name", name_token.position)
                        .with_category(SuggestionCategory::Syntax)
                ],
                help: Some("Const names must be valid identifiers starting with a letter or underscore".to_string()),
                related_errors: Vec::new(),
            });
        };
        
        self.expect(TokenType::Colon, "const type")?;
        let type_annotation = self.parse_type()?;
        self.expect(TokenType::Equal, "const value")?;
        let value = self.parse_expression()?;
        self.expect(TokenType::Semicolon, "const declaration")?;
        
        let end_pos = self.current_position();
        Ok(Item::Const {
            visibility,
            name,
            type_annotation,
            value,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    fn parse_module_decl(&mut self, visibility: Visibility, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Mod, "module declaration")?;
        
        // Module name
        let name_token = self.expect(TokenType::Identifier("".to_string()), "module name")?;
        let name = if let TokenType::Identifier(name_str) = name_token.token_type {
            self.interner.intern(&name_str)
        } else {
            return Err(ParseError::InvalidSyntax {
                message: "Expected module name".to_string(),
                position: name_token.position,
                context: self.current_context().clone(),
                suggestions: vec![
                    Suggestion::new("Use a valid identifier for the module name", name_token.position)
                        .with_category(SuggestionCategory::Syntax)
                ],
                help: Some("Module names must be valid identifiers starting with a letter or underscore".to_string()),
                related_errors: Vec::new(),
            });
        };
        
        // Parse module body
        let items = if self.match_token(&TokenType::LeftBrace) {
            let mut module_items = Vec::new();
            
            while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
                match self.parse_item() {
                    Ok(item) => module_items.push(item),
                    Err(err) => {
                        self.add_error(err);
                        self.synchronize();
                    }
                }
            }
            
            self.expect(TokenType::RightBrace, "module body")?;
            Some(module_items)
        } else {
            // External module: mod foo;
            self.expect(TokenType::Semicolon, "module declaration")?;
            None
        };
        
        let end_pos = self.current_position();
        Ok(Item::Module {
            visibility,
            name,
            items,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    fn parse_impl_block(&mut self, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Impl, "impl block")?;
        
        // Generic parameters (placeholder for now)
        let generics = Vec::new();
        
        // Target type
        let target_type = self.parse_type()?;
        
        // Optional trait implementation
        let trait_ref = if self.match_token(&TokenType::For) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        self.expect(TokenType::LeftBrace, "impl block")?;
        let mut items = Vec::new();
        
        // Parse impl items (functions, types, consts)
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            let item_start = self.current_position();
            let item_visibility = if self.match_token(&TokenType::Pub) {
                Visibility::Public
            } else {
                Visibility::Private
            };
            
            if self.check(&TokenType::Fn) {
                // Parse method
                if let Ok(Item::Function { name, generics, params, return_type, body, is_extern: _, .. }) = self.parse_function(item_visibility, item_start) {
                    items.push(crate::ast::ImplItem::Function {
                        visibility: item_visibility,
                        name,
                        generics,
                        params,
                        return_type,
                        body,
                        span: Span::new(item_start, self.current_position()),
                    });
                }
            } else {
                // Skip unknown items for now
                self.synchronize();
            }
        }
        
        self.expect(TokenType::RightBrace, "impl block")?;
        
        let end_pos = self.current_position();
        Ok(Item::Impl {
            generics,
            target_type,
            trait_ref,
            items,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    fn parse_use_decl(&mut self, start_pos: Position) -> ParseResult<Item> {
        self.expect(TokenType::Use, "use declaration")?;
        
        // Parse use path
        let mut path = Vec::new();
        
        loop {
            let name_token = self.expect(TokenType::Identifier("".to_string()), "use path")?;
            if let TokenType::Identifier(name_str) = name_token.token_type {
                path.push(self.interner.intern(&name_str));
            }
            
            if !self.match_token(&TokenType::DoubleColon) {
                break;
            }
        }
        
        // TODO: Add alias support when 'as' keyword is implemented
        let alias = None;
        
        self.expect(TokenType::Semicolon, "use declaration")?;
        
        let end_pos = self.current_position();
        Ok(Item::Use {
            path,
            alias,
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Parse a block expression: { [statements...] [expr] }
    pub fn parse_block_expression(&mut self) -> ParseResult<Expr> {
        let start_pos = self.current_position();
        self.expect(TokenType::LeftBrace, "block expression")?;
        
        let mut statements = Vec::new();
        let mut trailing_expr = None;
        
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Try to parse as statement first
            if self.is_statement_start() {
                match self.parse_statement() {
                    Ok(stmt) => statements.push(stmt),
                    Err(err) => {
                        self.add_error(err);
                        self.synchronize();
                    }
                }
            } else {
                // Try to parse as expression
                let expr_start = self.current_position();
                let expr = self.parse_expression()?;
                
                // If there's no semicolon and we're at the end of the block,
                // this is the trailing expression
                if !self.check(&TokenType::Semicolon) && self.check(&TokenType::RightBrace) {
                    trailing_expr = Some(expr);
                    break;
                } else {
                    // It's an expression statement
                    self.expect(TokenType::Semicolon, "expression statement")?;
                    let end_pos = self.current_position();
                    statements.push(Stmt::Expression {
                        expr,
                        span: Span::new(expr_start, end_pos),
                    });
                }
            }
        }
        
        self.expect(TokenType::RightBrace, "block expression")?;
        let end_pos = self.current_position();
        
        Ok(Expr::Block {
            statements,
            trailing_expr: trailing_expr.map(Box::new),
            span: Span::new(start_pos, end_pos),
        })
    }
    
    /// Check if current token sequence represents a self parameter
    fn is_self_parameter(&self) -> bool {
        match &self.current_token {
            Some(token) => match &token.token_type {
                // Direct self parameter
                TokenType::Identifier(name) if name == "self" => true,
                // &self or &mut self
                TokenType::And => {
                    // We need to look ahead to see if this is &self or &mut self
                    // This is a simplified check - in a full implementation we'd need proper lookahead
                    true
                }
                _ => false,
            },
            None => false,
        }
    }
    
    /// Parse a self parameter and return the pattern and inferred type
    fn parse_self_parameter(&mut self) -> ParseResult<(Pattern, Option<Type>)> {
        let start_pos = self.current_position();
        
        if let Some(token) = &self.current_token {
            match &token.token_type {
                // Direct self parameter: self
                TokenType::Identifier(name) if name == "self" => {
                    self.advance()?;
                    let pattern = Pattern::Identifier {
                        name: self.interner.intern("self"),
                        is_mutable: false,
                        span: Span::new(start_pos, self.current_position()),
                    };
                    // Type will be inferred as the struct type during semantic analysis
                    let self_type = Type::Path {
                        segments: vec![self.interner.intern("Self")],
                        generics: Vec::new(),
                        memory_strategy: MemoryStrategy::Inferred,
                        span: Span::new(start_pos, self.current_position()),
                    };
                    Ok((pattern, Some(self_type)))
                }
                // Reference self parameter: &self or &mut self
                TokenType::And => {
                    self.advance()?; // consume &
                    let is_mutable = self.match_token(&TokenType::Mut);
                    
                    // Expect 'self' identifier
                    if let Some(token) = &self.current_token {
                        if let TokenType::Identifier(name) = &token.token_type {
                            if name == "self" {
                                self.advance()?;
                                let pattern = Pattern::Identifier {
                                    name: self.interner.intern("self"),
                                    is_mutable: false, // The reference itself is not mutable
                                    span: Span::new(start_pos, self.current_position()),
                                };
                                // Create reference type to Self
                                let self_type = Type::Reference {
                                    is_mutable,
                                    target_type: Box::new(Type::Path {
                                        segments: vec![self.interner.intern("Self")],
                                        generics: Vec::new(),
                                        memory_strategy: MemoryStrategy::Inferred,
                                        span: Span::new(start_pos, self.current_position()),
                                    }),
                                    lifetime: None,
                                    ownership: crate::ast::Ownership::Borrowed,
                                    span: Span::new(start_pos, self.current_position()),
                                };
                                Ok((pattern, Some(self_type)))
                            } else {
                                Err(ParseError::UnexpectedToken {
                                    expected: vec![ExpectedToken::new("self", "self parameter")],
                                    found: token.token_type.clone(),
                                    position: token.position,
                                    context: self.current_context().clone(),
                                    suggestions: vec![
                                        Suggestion::new("Use 'self' for the parameter name", token.position)
                                            .with_replacement("self")
                                            .with_category(SuggestionCategory::Syntax)
                                    ],
                                    help: Some("Method parameters can use 'self', '&self', or '&mut self'".to_string()),
                                })
                            }
                        } else {
                            Err(ParseError::UnexpectedToken {
                                expected: vec![ExpectedToken::new("self", "self parameter")],
                                found: token.token_type.clone(),
                                position: token.position,
                                context: self.current_context().clone(),
                                suggestions: vec![
                                    Suggestion::new("Use 'self' for the parameter name", token.position)
                                        .with_replacement("self")
                                        .with_category(SuggestionCategory::Syntax)
                                ],
                                help: Some("Method parameters can use 'self', '&self', or '&mut self'".to_string()),
                            })
                        }
                    } else {
                        Err(ParseError::UnexpectedEof {
                            expected: vec![ExpectedToken::new("self", "self parameter")],
                            position: self.current_position(),
                            context: self.current_context().clone(),
                            unclosed_delimiters: Vec::new(),
                            suggestions: vec![
                                Suggestion::new("Add 'self' parameter", self.current_position())
                                    .with_replacement("self")
                                    .with_category(SuggestionCategory::Syntax)
                            ],
                        })
                    }
                }
                _ => Err(ParseError::UnexpectedToken {
                    expected: vec![ExpectedToken::new("self parameter", "self, &self, or &mut self")],
                    found: token.token_type.clone(),
                    position: token.position,
                    context: self.current_context().clone(),
                    suggestions: vec![
                        Suggestion::new("Use a valid self parameter", token.position)
                            .with_replacement("self")
                            .with_category(SuggestionCategory::Syntax)
                    ],
                    help: Some("Self parameters can be 'self' (owned), '&self' (borrowed), or '&mut self' (mutable borrow)".to_string()),
                }),
            }
        } else {
            Err(ParseError::UnexpectedEof {
                expected: vec![ExpectedToken::new("self parameter", "self, &self, or &mut self")],
                position: self.current_position(),
                context: self.current_context().clone(),
                unclosed_delimiters: Vec::new(),
                suggestions: vec![
                    Suggestion::new("Add self parameter", self.current_position())
                        .with_replacement("self")
                        .with_category(SuggestionCategory::Syntax)
                ],
            })
        }
    }
} 
