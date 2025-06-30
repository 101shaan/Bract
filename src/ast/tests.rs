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
    fn test_unary_operators() {
        let ops = [
            UnaryOp::Not, UnaryOp::Negate, UnaryOp::Plus, UnaryOp::BitwiseNot,
            UnaryOp::Dereference, UnaryOp::AddressOf, UnaryOp::MutableRef,
        ];
        
        for op in &ops {
            assert_eq!(*op, *op);
        }
    }

    #[test]
    fn test_literal_creation() {
        // Integer literal
        let int_lit = Literal::Integer {
            value: "42".to_string(),
            base: crate::lexer::token::NumberBase::Decimal,
            suffix: None,
        };
        
        // Float literal
        let float_lit = Literal::Float {
            value: "3.14".to_string(),
            suffix: Some(dummy_interned_string(1)),
        };
        
        // String literal
        let str_lit = Literal::String {
            value: dummy_interned_string(2),
            raw: false,
            raw_delimiter: None,
        };
        
        // Character literal
        let char_lit = Literal::Char('a');
        
        // Boolean literal
        let bool_lit = Literal::Bool(true);
        
        // Null literal
        let null_lit = Literal::Null;
        
        // Test that they're all different
        assert_ne!(int_lit, float_lit);
        assert_ne!(str_lit, char_lit);
        assert_ne!(bool_lit, null_lit);
    }

    #[test]
    fn test_expression_span_retrieval() {
        let span = dummy_span();
        
        // Test literal expression
        let lit_expr = Expr::Literal {
            literal: Literal::Bool(true),
            span,
        };
        assert_eq!(lit_expr.span(), span);
        
        // Test identifier expression
        let id_expr = Expr::Identifier {
            name: dummy_interned_string(1),
            span,
        };
        assert_eq!(id_expr.span(), span);
        
        // Test binary expression
        let binary_expr = Expr::Binary {
            left: Box::new(lit_expr.clone()),
            op: BinaryOp::Add,
            right: Box::new(id_expr.clone()),
            span,
        };
        assert_eq!(binary_expr.span(), span);
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
    fn test_statement_creation() {
        let span = dummy_span();
        
        // Expression statement
        let expr_stmt = Stmt::Expression {
            expr: Expr::Literal {
                literal: Literal::Bool(true),
                span,
            },
            span,
        };
        assert_eq!(expr_stmt.span(), span);
        
        // Let statement
        let let_stmt = Stmt::Let {
            pattern: Pattern::Identifier {
                name: dummy_interned_string(1),
                is_mutable: false,
                span,
            },
            type_annotation: Some(Type::Primitive {
                kind: PrimitiveType::Bool,
                span,
            }),
            initializer: Some(Expr::Literal {
                literal: Literal::Bool(true),
                span,
            }),
            is_mutable: false,
            span,
        };
        assert_eq!(let_stmt.span(), span);
    }

    #[test]
    fn test_pattern_creation() {
        let span = dummy_span();
        
        // Wildcard pattern
        let wildcard = Pattern::Wildcard { span };
        assert_eq!(wildcard.span(), span);
        assert!(!wildcard.binds_variables());
        
        // Identifier pattern
        let id_pattern = Pattern::Identifier {
            name: dummy_interned_string(1),
            is_mutable: false,
            span,
        };
        assert_eq!(id_pattern.span(), span);
        assert!(id_pattern.binds_variables());
        
        // Tuple pattern
        let tuple_pattern = Pattern::Tuple {
            patterns: vec![wildcard.clone(), id_pattern.clone()],
            span,
        };
        assert_eq!(tuple_pattern.span(), span);
        assert!(tuple_pattern.binds_variables()); // Contains identifier pattern
    }

    #[test]
    fn test_type_creation() {
        let span = dummy_span();
        
        // Primitive type
        let prim_type = Type::Primitive {
            kind: PrimitiveType::I32,
            span,
        };
        assert_eq!(prim_type.span(), span);
        assert!(prim_type.is_primitive());
        assert!(!prim_type.is_reference());
        
        // Reference type
        let ref_type = Type::Reference {
            is_mutable: false,
            target_type: Box::new(prim_type.clone()),
            span,
        };
        assert_eq!(ref_type.span(), span);
        assert!(!ref_type.is_primitive());
        assert!(ref_type.is_reference());
        
        // Array type
        let array_type = Type::Array {
            element_type: Box::new(prim_type.clone()),
            size: Box::new(Expr::Literal {
                literal: Literal::Integer {
                    value: "10".to_string(),
                    base: crate::lexer::token::NumberBase::Decimal,
                    suffix: None,
                },
                span,
            }),
            span,
        };
        assert_eq!(array_type.span(), span);
    }

    #[test]
    fn test_item_creation() {
        let span = dummy_span();
        
        // Function item
        let func_item = Item::Function {
            visibility: Visibility::Public,
            name: dummy_interned_string(1),
            generics: vec![],
            params: vec![Parameter {
                pattern: Pattern::Identifier {
                    name: dummy_interned_string(2),
                    is_mutable: false,
                    span,
                },
                type_annotation: Some(Type::Primitive {
                    kind: PrimitiveType::I32,
                    span,
                }),
                span,
            }],
            return_type: Some(Type::Primitive {
                kind: PrimitiveType::I32,
                span,
            }),
            body: Some(Expr::Block {
                statements: vec![],
                trailing_expr: Some(Box::new(Expr::Literal {
                    literal: Literal::Integer {
                        value: "42".to_string(),
                        base: crate::lexer::token::NumberBase::Decimal,
                        suffix: None,
                    },
                    span,
                })),
                span,
            }),
            is_extern: false,
            span,
        };
        
        // Struct item
        let struct_item = Item::Struct {
            visibility: Visibility::Private,
            name: dummy_interned_string(3),
            generics: vec![],
            fields: StructFields::Named(vec![StructField {
                visibility: Visibility::Public,
                name: dummy_interned_string(4),
                field_type: Type::Primitive {
                    kind: PrimitiveType::I32,
                    span,
                },
                span,
            }]),
            span,
        };
        
        // Test that they're different
        assert_ne!(func_item, struct_item);
    }

    #[test]
    fn test_struct_fields() {
        let span = dummy_span();
        
        // Named fields
        let named_fields = StructFields::Named(vec![
            StructField {
                visibility: Visibility::Public,
                name: dummy_interned_string(1),
                field_type: Type::Primitive {
                    kind: PrimitiveType::I32,
                    span,
                },
                span,
            },
            StructField {
                visibility: Visibility::Private,
                name: dummy_interned_string(2),
                field_type: Type::Primitive {
                    kind: PrimitiveType::Bool,
                    span,
                },
                span,
            },
        ]);
        
        // Tuple fields
        let tuple_fields = StructFields::Tuple(vec![
            Type::Primitive {
                kind: PrimitiveType::I32,
                span,
            },
            Type::Primitive {
                kind: PrimitiveType::Bool,
                span,
            },
        ]);
        
        // Unit struct
        let unit_fields = StructFields::Unit;
        
        assert_ne!(named_fields, tuple_fields);
        assert_ne!(tuple_fields, unit_fields);
    }

    #[test]
    fn test_primitive_types() {
        let types = [
            PrimitiveType::I8, PrimitiveType::I16, PrimitiveType::I32, PrimitiveType::I64,
            PrimitiveType::I128, PrimitiveType::ISize,
            PrimitiveType::U8, PrimitiveType::U16, PrimitiveType::U32, PrimitiveType::U64,
            PrimitiveType::U128, PrimitiveType::USize,
            PrimitiveType::F32, PrimitiveType::F64,
            PrimitiveType::Bool, PrimitiveType::Char, PrimitiveType::Str, PrimitiveType::Unit,
        ];
        
        for ty in &types {
            assert_eq!(*ty, *ty);
        }
    }

    #[test]
    fn test_visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Private);
    }

    #[test]
    fn test_module_creation() {
        let span = dummy_span();
        
        let module = Module {
            items: vec![
                Item::Function {
                    visibility: Visibility::Public,
                    name: dummy_interned_string(1),
                    generics: vec![],
                    params: vec![],
                    return_type: None,
                    body: Some(Expr::Block {
                        statements: vec![],
                        trailing_expr: None,
                        span,
                    }),
                    is_extern: false,
                    span,
                },
            ],
            span,
        };
        
        assert_eq!(module.items.len(), 1);
        assert_eq!(module.span, span);
    }

    #[test]
    fn test_complex_expression_tree() {
        let span = dummy_span();
        
        // Create a complex expression: (x + y) * 2
        let x = Expr::Identifier {
            name: dummy_interned_string(1),
            span,
        };
        
        let y = Expr::Identifier {
            name: dummy_interned_string(2),
            span,
        };
        
        let two = Expr::Literal {
            literal: Literal::Integer {
                value: "2".to_string(),
                base: crate::lexer::token::NumberBase::Decimal,
                suffix: None,
            },
            span,
        };
        
        let add = Expr::Binary {
            left: Box::new(x),
            op: BinaryOp::Add,
            right: Box::new(y),
            span,
        };
        
        let parenthesized = Expr::Parenthesized {
            expr: Box::new(add),
            span,
        };
        
        let multiply = Expr::Binary {
            left: Box::new(parenthesized),
            op: BinaryOp::Multiply,
            right: Box::new(two),
            span,
        };
        
        assert_eq!(multiply.span(), span);
        assert!(!multiply.has_side_effects());
    }

    #[test]
    fn test_match_expression() {
        let span = dummy_span();
        
        let match_expr = Expr::Match {
            expr: Box::new(Expr::Identifier {
                name: dummy_interned_string(1),
                span,
            }),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal {
                        literal: Literal::Integer {
                            value: "1".to_string(),
                            base: crate::lexer::token::NumberBase::Decimal,
                            suffix: None,
                        },
                        span,
                    },
                    guard: None,
                    body: Expr::Literal {
                        literal: Literal::String {
                            value: dummy_interned_string(2),
                            raw: false,
                            raw_delimiter: None,
                        },
                        span,
                    },
                    span,
                },
                MatchArm {
                    pattern: Pattern::Wildcard { span },
                    guard: Some(Expr::Binary {
                        left: Box::new(Expr::Identifier {
                            name: dummy_interned_string(3),
                            span,
                        }),
                        op: BinaryOp::Greater,
                        right: Box::new(Expr::Literal {
                            literal: Literal::Integer {
                                value: "10".to_string(),
                                base: crate::lexer::token::NumberBase::Decimal,
                                suffix: None,
                            },
                            span,
                        }),
                        span,
                    }),
                    body: Expr::Literal {
                        literal: Literal::String {
                            value: dummy_interned_string(4),
                            raw: false,
                            raw_delimiter: None,
                        },
                        span,
                    },
                    span,
                },
            ],
            span,
        };
        
        assert_eq!(match_expr.arms.len(), 2);
        assert_eq!(match_expr.span(), span);
    }
} 