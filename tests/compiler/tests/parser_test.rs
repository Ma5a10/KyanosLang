// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use kyanos_ast::{
    AccessorKind, AnonymousStructMember, ArraySize, ArrowBody, AssignOp, AttributeInput, BinaryOp,
    BindingTarget, Block, ClassElementName, ClassMember, DeclKind, EnumVariant, ExportKind, Expr,
    ExprKind, ExternMember, ForInit, ForIterationKind, ForStmt, ImportClause, InterfaceMember,
    LiteralKind, LiteralSign, MemberProperty, NodeId, ObjectMember, ObjectPatternProp, Pattern,
    PatternKind, PostfixUnaryOp, PredefinedType, PrefixUnaryOp, PropertyName, Radix, ReturnType,
    SizeofTarget, SourceElement, Span, Stmt, StmtKind, StructFieldInit, StructFieldPattern,
    StructMember, SwitchCase, SwitchExprArm, TemplateStringPart, Type, TypeKind, VarModifier,
};
use kyanos_lexer::TokenKind;
use kyanos_parse::{parse, parse_expr, ParseError, ParseErrorKind, ParseOutput, Parser};

fn parse_expr_ok(source: &str) -> Expr {
    let mut parser = Parser::new(source);
    let expr = parser.parse_expr().expect("expression should parse");
    assert!(
        parser.errors().is_empty(),
        "unexpected parse errors for `{}`: {:?}",
        source,
        parser.errors()
    );
    assert!(
        parser.is_eof(),
        "expected EOF after `{}`, found {:?}",
        source,
        parser.current()
    );
    expr
}

#[test]
fn test_trivia_filtering_and_leading_newline_across_comments() {
    let source = "let a = 1 // trailing comment\n/* block */\nlet b = 2 /* inline */ let c = 3";
    let mut parser = Parser::new(source);

    // First statement tokens: `let`, `a`, `=`, `1`
    assert_eq!(parser.bump().kind, TokenKind::Let);
    assert_eq!(parser.bump().kind, TokenKind::Identifier);
    assert_eq!(parser.bump().kind, TokenKind::Assign);
    assert_eq!(parser.bump().kind, TokenKind::IntegerLiteral);

    // Comments are skipped; `let` on the new line keeps `has_leading_newline = true`.
    let tok_let_b = parser.current();
    assert_eq!(tok_let_b.kind, TokenKind::Let);
    assert!(tok_let_b.has_leading_newline);
    parser.bump(); // let
    assert_eq!(parser.current_slice(), "b");
    parser.bump(); // b
    parser.bump(); // =
    parser.bump(); // 2

    // Inline block comment without a newline keeps `has_leading_newline = false`.
    let tok_let_c = parser.current();
    assert_eq!(tok_let_c.kind, TokenKind::Let);
    assert!(!tok_let_c.has_leading_newline);
}

#[test]
fn test_cursor_lookahead_and_span_merge() {
    let mut parser = Parser::new("get value");

    assert!(parser.at_ident_str("get"));
    assert!(!parser.at_ident_str("set"));
    assert_eq!(parser.peek_kind(1), TokenKind::Identifier);
    assert_eq!(parser.peek_kind(2), TokenKind::Eof);

    let first = parser.expect(TokenKind::Identifier, "accessor").unwrap();
    let second = parser
        .expect(TokenKind::Identifier, "property name")
        .unwrap();
    let full = parser.span_from(first.span);

    assert_eq!(full, Span::new(first.span.start, second.span.end));
    assert_eq!(parser.slice(full), "get value");
    assert!(parser.is_eof());
}

#[test]
fn test_checkpoint_and_restore_rolls_back_cursor_and_errors() {
    let mut parser = Parser::new("a + b");
    let cp = parser.checkpoint();

    assert_eq!(parser.bump().kind, TokenKind::Identifier);
    assert_eq!(parser.bump().kind, TokenKind::Plus);
    parser.report_error(ParseError::syntax(
        "speculative error",
        parser.current_span(),
    ));
    assert_eq!(parser.errors().len(), 1);
    assert_eq!(parser.current_slice(), "b");

    parser.restore(cp);
    assert_eq!(parser.current_slice(), "a");
    assert!(parser.errors().is_empty());
}

#[test]
fn test_generic_gt_splitting_for_nested_generics() {
    // 1. `>>` splits into two `>` tokens.
    let mut parser = Parser::new("Array<Array<int>>");
    assert_eq!(parser.bump().kind, TokenKind::Identifier); // Array
    assert_eq!(parser.bump().kind, TokenKind::LessThan); // <
    assert_eq!(parser.bump().kind, TokenKind::Identifier); // Array
    assert_eq!(parser.bump().kind, TokenKind::LessThan); // <
    assert_eq!(parser.bump().kind, TokenKind::TypeInt); // int

    let inner_gt = parser.expect_generic_gt("'>'").unwrap();
    assert_eq!(parser.slice(inner_gt), ">");
    assert_eq!(parser.current_kind(), TokenKind::GreaterThan);

    let outer_gt = parser.expect_generic_gt("'>'").unwrap();
    assert_eq!(parser.slice(outer_gt), ">");
    assert!(parser.is_eof());

    // 2. `>>>` splits into three `>` tokens, and `>=` splits into `>` + `=`.
    let mut parser2 = Parser::new("Box<Box<Box<int>>>=1");
    for _ in 0..7 {
        parser2.bump(); // Box < Box < Box < int
    }
    let gt1 = parser2.expect_generic_gt("'>'").unwrap();
    let gt2 = parser2.expect_generic_gt("'>'").unwrap();
    let gt3 = parser2.expect_generic_gt("'>'").unwrap();
    assert_eq!(parser2.slice(gt1), ">");
    assert_eq!(parser2.slice(gt2), ">");
    assert_eq!(parser2.slice(gt3), ">");
    assert_eq!(parser2.bump().kind, TokenKind::Assign);
    assert_eq!(parser2.bump().kind, TokenKind::IntegerLiteral);
    assert!(parser2.is_eof());

    // 3. `Checkpoint` restores unsplit `>>` after speculative generic split.
    let mut parser3 = Parser::new("a >> b");
    parser3.bump(); // a
    let cp = parser3.checkpoint();
    assert!(parser3.eat_generic_gt().is_some());
    assert_eq!(parser3.current_kind(), TokenKind::GreaterThan);
    parser3.restore(cp);
    assert_eq!(parser3.current_kind(), TokenKind::ShiftRight);
}

#[test]
fn test_parse_eos_swift_style_rules() {
    // 1. Explicit semicolon is consumed.
    let mut p1 = Parser::new("1; 2");
    p1.bump(); // 1
    assert!(p1.parse_eos().is_ok());
    assert_eq!(p1.current_slice(), "2");

    // 2. Newline terminates statement without consuming the next token.
    let mut p2 = Parser::new("1\n2");
    p2.bump(); // 1
    assert!(p2.parse_eos().is_ok());
    assert_eq!(p2.current_slice(), "2");

    // 3. Close brace and EOF terminate statement without consuming.
    let mut p3 = Parser::new("{ 1 }");
    p3.bump(); // {
    p3.bump(); // 1
    assert!(p3.parse_eos().is_ok());
    assert_eq!(p3.current_kind(), TokenKind::CloseBrace);

    let mut p4 = Parser::new("1");
    p4.bump(); // 1
    assert!(p4.parse_eos().is_ok());
    assert!(p4.is_eof());

    // 4. Two statements on the same line without ';' trigger ExpectedEos.
    let mut p5 = Parser::new("1 2");
    p5.bump(); // 1
    let err = p5.parse_eos().expect_err("expected EOS error on same line");
    assert_eq!(
        err.kind,
        ParseErrorKind::ExpectedEos {
            found: TokenKind::IntegerLiteral
        }
    );
}

#[test]
fn test_error_recovery_synchronize_and_node_ids() {
    let mut parser = Parser::new("let = ???;\nlet ok = 1\n}\nreturn");

    assert_eq!(parser.alloc_id(), NodeId::new(0));
    assert_eq!(parser.alloc_id(), NodeId::new(1));
    assert_eq!(parser.next_node_id_raw(), 2);

    // Note: `???` produces '?' tokens, and `synchronize` skips until `;`.
    parser.bump(); // let
    parser.synchronize();
    assert_eq!(parser.current_kind(), TokenKind::Let);
    assert_eq!(parser.peek(1).kind, TokenKind::Identifier);

    // Test advance_and_synchronize guarantees progress even on a sync keyword.
    parser.advance_and_synchronize();
    assert_eq!(parser.current_kind(), TokenKind::CloseBrace);
}

#[test]
fn test_with_offset_and_lexer_error_propagation() {
    // Embedded expression slice starting at byte offset 10 with NodeId starting at 5.
    let mut parser = Parser::with_offset("a + @ b", 10, 5);
    assert_eq!(parser.alloc_id(), NodeId::new(5));

    // Lexer error on '@' was captured at collection time without halting token stream.
    assert_eq!(parser.errors().len(), 1);
    assert!(matches!(
        parser.errors()[0].kind,
        ParseErrorKind::LexerError(_)
    ));

    let a = parser.bump();
    assert_eq!(a.span, Span::new(10, 11));
    assert_eq!(parser.slice(a.span), "a");
    assert_eq!(parser.bump().kind, TokenKind::Plus);
    let b = parser.bump();
    assert_eq!(parser.slice(b.span), "b");
    assert_eq!(b.span, Span::new(16, 17));
}

// =============================================================================
// Phase 2: Literal Decoding & Pratt Expression Parser Tests
// =============================================================================

#[test]
fn test_literal_decoding_all_kinds() {
    let hex = parse_expr_ok("0xFF_00");
    assert!(matches!(
        hex.kind,
        ExprKind::Literal(ref l) if matches!(&l.kind, LiteralKind::Integer(i) if i.value == 0xFF00 && i.radix == Radix::Hex)
    ));

    let oct = parse_expr_ok("0o755");
    assert!(matches!(
        oct.kind,
        ExprKind::Literal(ref l) if matches!(&l.kind, LiteralKind::Integer(i) if i.value == 0o755 && i.radix == Radix::Octal)
    ));

    let bin = parse_expr_ok("0b1010_0101");
    assert!(matches!(
        bin.kind,
        ExprKind::Literal(ref l) if matches!(&l.kind, LiteralKind::Integer(i) if i.value == 0b10100101 && i.radix == Radix::Binary)
    ));

    let flt = parse_expr_ok("3.14_15");
    assert!(matches!(
        flt.kind,
        ExprKind::Literal(ref l) if matches!(&l.kind, LiteralKind::Float(f) if (f.value - 3.1415).abs() < 1e-10)
    ));

    let big = parse_expr_ok("9007199254740991n");
    assert!(matches!(
        big.kind,
        ExprKind::Literal(ref l) if matches!(&l.kind, LiteralKind::BigInt(b) if b.raw == "9007199254740991n")
    ));

    let s = parse_expr_ok(r#""hi\n\u{3042}""#);
    assert!(matches!(
        s.kind,
        ExprKind::Literal(ref l) if matches!(&l.kind, LiteralKind::String(st) if st.value == "hi\nあ")
    ));

    let cs = parse_expr_ok(r#"c"hello\t""#);
    assert!(matches!(
        cs.kind,
        ExprKind::Literal(ref l) if matches!(&l.kind, LiteralKind::CString(c) if c.value == "hello\t")
    ));

    let ch = parse_expr_ok(r"'\u3042'");
    assert!(matches!(
        ch.kind,
        ExprKind::Literal(ref l) if matches!(&l.kind, LiteralKind::Char(c) if c.value == 'あ')
    ));

    let unit = parse_expr_ok("()");
    assert!(matches!(
        unit.kind,
        ExprKind::Literal(ref l) if l.kind == LiteralKind::Unit
    ));

    // Overflow beyond u64::MAX reports InvalidLiteral.
    let mut overflow_parser = Parser::new("99999999999999999999999");
    let err = overflow_parser.parse_expr().unwrap_err();
    assert!(matches!(err.kind, ParseErrorKind::InvalidLiteral(_)));
}

#[test]
fn test_template_string_interpolation_and_spans() {
    let src = "`Hi ${name}, sum=${1 + 2}, nested=${`x=${y}`}`";
    let expr = parse_expr_ok(src);

    let ExprKind::TemplateString(parts) = expr.kind else {
        panic!("expected TemplateString, got {:?}", expr.kind);
    };
    assert_eq!(parts.len(), 6);
    assert_eq!(parts[0], TemplateStringPart::String("Hi ".to_string()));
    match &parts[1] {
        TemplateStringPart::Expr(inner) => {
            assert!(matches!(&inner.kind, ExprKind::Ident(id) if id.as_str() == "name"));
            assert_eq!(&src[inner.span.start..inner.span.end], "name");
        }
        other => panic!("expected Expr part, got {:?}", other),
    }
    assert_eq!(parts[2], TemplateStringPart::String(", sum=".to_string()));
    match &parts[3] {
        TemplateStringPart::Expr(inner) => {
            assert!(matches!(
                inner.kind,
                ExprKind::Binary {
                    op: BinaryOp::Add,
                    ..
                }
            ));
            assert_eq!(&src[inner.span.start..inner.span.end], "1 + 2");
        }
        other => panic!("expected Expr part, got {:?}", other),
    }
    assert_eq!(
        parts[4],
        TemplateStringPart::String(", nested=".to_string())
    );
    match &parts[5] {
        TemplateStringPart::Expr(inner) => {
            assert!(matches!(inner.kind, ExprKind::TemplateString(_)));
        }
        other => panic!("expected nested TemplateString, got {:?}", other),
    }
}

#[test]
fn test_pratt_precedence_and_associativity() {
    // 1. a + b * c ** d ** e  =>  a + (b * (c ** (d ** e)))
    let arith = parse_expr_ok("a + b * c ** d ** e");
    let ExprKind::Binary {
        op: BinaryOp::Add,
        right: mul,
        ..
    } = arith.kind
    else {
        panic!("expected Add at root");
    };
    let ExprKind::Binary {
        op: BinaryOp::Mul,
        right: pow1,
        ..
    } = mul.kind
    else {
        panic!("expected Mul on right of Add");
    };
    let ExprKind::Binary {
        op: BinaryOp::Pow,
        right: pow2,
        ..
    } = pow1.kind
    else {
        panic!("expected right-associative Pow");
    };
    assert!(matches!(
        pow2.kind,
        ExprKind::Binary {
            op: BinaryOp::Pow,
            ..
        }
    ));

    // 2. Right-associative assignment and ternary: a = b += c ? d : e ? f : g
    let assign = parse_expr_ok("a = b += c ? d : e ? f : g");
    let ExprKind::Assign {
        op: AssignOp::Assign,
        value: rhs_assign,
        ..
    } = assign.kind
    else {
        panic!("expected outer Assign");
    };
    let ExprKind::Assign {
        op: AssignOp::AddAssign,
        value: ternary,
        ..
    } = rhs_assign.kind
    else {
        panic!("expected inner AddAssign");
    };
    let ExprKind::Ternary { else_expr, .. } = ternary.kind else {
        panic!("expected Ternary");
    };
    assert!(matches!(else_expr.kind, ExprKind::Ternary { .. }));

    // 3. Prefix < Postfix, Cast < Prefix, Additive < Cast: -a as int + 1, *p++
    let cast_expr = parse_expr_ok("-a as int + 1");
    let ExprKind::Binary {
        op: BinaryOp::Add,
        left: cast_node,
        ..
    } = cast_expr.kind
    else {
        panic!("expected Add around Cast");
    };
    let ExprKind::Cast { expr: neg_node, ty } = cast_node.kind else {
        panic!("expected Cast");
    };
    assert_eq!(ty.kind, TypeKind::Predefined(PredefinedType::Int));
    assert!(matches!(
        neg_node.kind,
        ExprKind::UnaryPrefix {
            op: PrefixUnaryOp::Minus,
            ..
        }
    ));

    let deref_inc = parse_expr_ok("*p++");
    let ExprKind::UnaryPrefix {
        op: PrefixUnaryOp::Deref,
        target,
    } = deref_inc.kind
    else {
        panic!("expected Deref wrapping PostInc");
    };
    assert!(matches!(
        target.kind,
        ExprKind::UnaryPostfix {
            op: PostfixUnaryOp::PostInc,
            ..
        }
    ));

    // 4. Null-coalescing < LogicalOr < LogicalAnd, and TypeTest > Equality
    let logic = parse_expr_ok("a ?? b || c && d");
    assert!(matches!(
        logic.kind,
        ExprKind::Binary {
            op: BinaryOp::NullCoalescing,
            ..
        }
    ));

    let type_test = parse_expr_ok("x is int == true");
    let ExprKind::Binary {
        op: BinaryOp::Eq,
        left,
        ..
    } = type_test.kind
    else {
        panic!("expected Eq above TypeTest");
    };
    assert!(matches!(left.kind, ExprKind::TypeTest { .. }));

    // 5. Invalid assignment target produces diagnostic.
    let mut bad_assign = Parser::new("a + b = c");
    let _ = bad_assign.parse_expr().unwrap();
    assert_eq!(bad_assign.errors().len(), 1);
    assert_eq!(
        bad_assign.errors()[0].kind,
        ParseErrorKind::InvalidAssignmentTarget
    );
}

#[test]
fn test_postfix_chains_generics_and_new() {
    // Member, private member, optional chain, optional index, call, spread, non-null
    let chain = parse_expr_ok("obj.prop.#secret?.opt?.[0](1, ...rest)!");
    let ExprKind::UnaryPostfix {
        op: PostfixUnaryOp::NonNull,
        target: call,
    } = chain.kind
    else {
        panic!("expected NonNull");
    };
    let ExprKind::Call { callee, args, .. } = call.kind else {
        panic!("expected Call");
    };
    assert_eq!(args.len(), 2);
    assert!(!args[0].is_spread);
    assert!(args[1].is_spread);
    let ExprKind::Index {
        target: opt_mem,
        is_optional: true,
        ..
    } = callee.kind
    else {
        panic!("expected optional Index");
    };
    let ExprKind::Member {
        target: priv_mem,
        is_optional: true,
        ..
    } = opt_mem.kind
    else {
        panic!("expected optional Member");
    };
    assert!(matches!(
        priv_mem.kind,
        ExprKind::Member {
            property: MemberProperty::Private(_),
            is_optional: false,
            ..
        }
    ));

    // Generic call vs relational '<'
    let gen_call = parse_expr_ok("f<int, Array<string>>(1, 2)");
    let ExprKind::Call {
        type_arguments: Some(ta),
        args,
        ..
    } = gen_call.kind
    else {
        panic!("expected generic Call");
    };
    assert_eq!(ta.args.len(), 2);
    assert_eq!(args.len(), 2);

    let cmp = parse_expr_ok("f < intVal && b > (c)");
    assert!(matches!(
        cmp.kind,
        ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            ..
        }
    ));

    // Predefined type static member
    let from_cstr = parse_expr_ok("string.fromCString(ptr)");
    let ExprKind::Call { callee, .. } = from_cstr.kind else {
        panic!("expected Call on PredefinedTypeMember");
    };
    assert!(matches!(
        callee.kind,
        ExprKind::PredefinedTypeMember {
            ty: PredefinedType::String,
            ..
        }
    ));

    // New expression: args belong to `New`, not inner `Call`.
    let new_box = parse_expr_ok(r#"new Box<int>("Kyanos")"#);
    let ExprKind::New {
        callee,
        type_arguments: Some(ta),
        args: Some(args),
    } = new_box.kind
    else {
        panic!("expected New with type args and args");
    };
    assert!(matches!(callee.kind, ExprKind::Ident(ref id) if id.as_str() == "Box"));
    assert_eq!(ta.args.len(), 1);
    assert_eq!(args.len(), 1);

    let new_bare = parse_expr_ok("new std.Vec");
    assert!(matches!(
        new_bare.kind,
        ExprKind::New {
            type_arguments: None,
            args: None,
            ..
        }
    ));
}

#[test]
fn test_composite_literals_struct_init_and_arrow_functions() {
    // Array literal with spread & trailing comma
    let arr = parse_expr_ok("[1, ...rest, 3,]");
    let ExprKind::Array(elems) = arr.kind else {
        panic!("expected Array");
    };
    assert_eq!(elems.len(), 3);
    assert!(elems[1].is_spread);

    // Object literal: ident key, string key, numeric key, computed key, spread, shorthand, method
    let obj = parse_expr_ok(
        r#"{ x: 10, "host": "localhost", 0: true, [k]: v, ...other, shorthand, async run(a: int): int { return a } }"#,
    );
    let ExprKind::Object(members) = obj.kind else {
        panic!("expected Object");
    };
    assert_eq!(members.len(), 7);
    assert!(matches!(
        members[0],
        ObjectMember::Property {
            name: PropertyName::Ident(_),
            ..
        }
    ));
    assert!(matches!(
        members[1],
        ObjectMember::Property {
            name: PropertyName::String(_),
            ..
        }
    ));
    assert!(matches!(
        members[2],
        ObjectMember::Property {
            name: PropertyName::Numeric(_),
            ..
        }
    ));
    assert!(matches!(
        members[3],
        ObjectMember::Property {
            name: PropertyName::Computed(_),
            ..
        }
    ));
    assert!(matches!(members[4], ObjectMember::Spread { .. }));
    assert!(matches!(members[5], ObjectMember::Shorthand { .. }));
    assert!(matches!(
        members[6],
        ObjectMember::Method(ref m) if m.is_async && m.name.as_str() == "run" && m.body.statements.len() == 1
    ));

    // Struct instantiation (plain, dotted, and generic)
    let st = parse_expr_ok(r#"std.Pair<int, string> { first: 1, second, ...base }"#);
    let ExprKind::StructInstantiation {
        struct_name,
        type_arguments: Some(ta),
        fields,
    } = st.kind
    else {
        panic!("expected StructInstantiation");
    };
    assert_eq!(struct_name.to_string(), "std.Pair");
    assert_eq!(ta.args.len(), 2);
    assert_eq!(fields.len(), 3);
    assert!(matches!(
        fields[1],
        StructFieldInit::Field { value: None, .. }
    ));

    // Arrow functions & parenthesized vs sequence
    let arrow0 = parse_expr_ok("() => 1");
    assert!(matches!(
        arrow0.kind,
        ExprKind::ArrowFunction {
            is_async: false,
            ref params,
            body: ArrowBody::Expr(_),
            ..
        } if params.params.is_empty()
    ));

    let arrow_block = parse_expr_ok("(): int => { return 1 }");
    assert!(matches!(
        arrow_block.kind,
        ExprKind::ArrowFunction {
            return_type: Some(_),
            body: ArrowBody::Block(ref b),
            ..
        } if b.statements.len() == 1
    ));

    let arrow_single = parse_expr_ok("x => x * 2");
    assert!(matches!(
        arrow_single.kind,
        ExprKind::ArrowFunction {
            ref params,
            ..
        } if params.params.len() == 1
    ));

    let arrow_typed = parse_expr_ok("(x: int, ...rest: int[]): int => x + 1");
    let ExprKind::ArrowFunction { params, .. } = arrow_typed.kind else {
        panic!("expected ArrowFunction");
    };
    assert_eq!(params.params.len(), 1);
    assert!(matches!(params.params[0].target, BindingTarget::Ident(_)));
    assert!(params.rest.is_some());

    let arrow_generic = parse_expr_ok("async <T>(x: T) => await x");
    assert!(matches!(
        arrow_generic.kind,
        ExprKind::ArrowFunction {
            is_async: true,
            type_parameters: Some(_),
            ..
        }
    ));

    assert!(matches!(
        parse_expr_ok("(a + b)").kind,
        ExprKind::Parenthesized(_)
    ));
    assert!(matches!(
        parse_expr_ok("(a, b, c)").kind,
        ExprKind::Sequence(ref s) if s.len() == 3
    ));
}

#[test]
fn test_switch_unsafe_and_sizeof_expressions() {
    let sw = parse_expr_ok(r#"switch (res) { case Ok(let v) when v > 0 => v, default => 0 }"#);
    let ExprKind::Switch { arms, .. } = sw.kind else {
        panic!("expected Switch expression");
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(
        &arms[0],
        SwitchExprArm::Case {
            pattern,
            guard: Some(_),
            ..
        } if matches!(pattern.kind, PatternKind::Variant { .. })
    ));
    assert!(matches!(arms[1], SwitchExprArm::Default { .. }));

    let uns = parse_expr_ok("unsafe { *ptr }");
    assert!(matches!(
        uns.kind,
        ExprKind::Unsafe(ref b) if matches!(b.statements.first().map(|s| &s.kind), Some(StmtKind::Expr(_)))
    ));

    let sz_type = parse_expr_ok("sizeof(int)");
    assert!(matches!(
        sz_type.kind,
        ExprKind::Sizeof(SizeofTarget::Type(_))
    ));

    let sz_expr = parse_expr_ok("sizeof(x + 1)");
    assert!(matches!(
        sz_expr.kind,
        ExprKind::Sizeof(SizeofTarget::Expr(_))
    ));
}

#[test]
fn test_expression_newline_boundaries_and_continuations() {
    // 1. `foo\n{ ... }` must NOT parse `{ ... }` as struct instantiation.
    let mut p1 = Parser::new("foo\n{ 1 }");
    let e1 = p1.parse_expr().unwrap();
    assert!(matches!(e1.kind, ExprKind::Ident(_)));
    assert_eq!(p1.current_kind(), TokenKind::OpenBrace);

    // 2. `a\n(b)` and `a\n*ptr` must stop at `a` so next line starts a new statement.
    let mut p2 = Parser::new("a\n(b)");
    let e2 = p2.parse_expr().unwrap();
    assert!(matches!(e2.kind, ExprKind::Ident(_)));
    assert_eq!(p2.current_kind(), TokenKind::OpenParen);

    let mut p3 = Parser::new("a\n*ptr");
    let e3 = p3.parse_expr().unwrap();
    assert!(matches!(e3.kind, ExprKind::Ident(_)));
    assert_eq!(p3.current_kind(), TokenKind::Star);

    // 3. `a +\nb` and `arr\n.map(x => x)` and `(a\n + b)` MUST continue across newlines.
    let add_cont = parse_expr_ok("a +\nb");
    assert!(matches!(
        add_cont.kind,
        ExprKind::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));

    let chain_cont = parse_expr_ok("arr\n  .map(x => x)\n  .filter(x => x > 0)");
    assert!(matches!(chain_cont.kind, ExprKind::Call { .. }));

    let paren_cont = parse_expr_ok("(a\n + b)");
    assert!(matches!(paren_cont.kind, ExprKind::Parenthesized(_)));
}

// =============================================================================
// Phase 3: Type, Generics, and Pattern Parser Tests
// =============================================================================

fn parse_type_ok(source: &str) -> Type {
    let mut parser = Parser::new(source);
    let ty = parser.parse_type().expect("type should parse");
    assert!(
        parser.errors().is_empty(),
        "unexpected parse errors for type `{}`: {:?}",
        source,
        parser.errors()
    );
    assert!(
        parser.is_eof(),
        "expected EOF after type `{}`, found {:?}",
        source,
        parser.current()
    );
    ty
}

fn parse_pattern_ok(source: &str) -> Pattern {
    let mut parser = Parser::new(source);
    let pat = parser.parse_pattern().expect("pattern should parse");
    assert!(
        parser.errors().is_empty(),
        "unexpected parse errors for pattern `{}`: {:?}",
        source,
        parser.errors()
    );
    assert!(
        parser.is_eof(),
        "expected EOF after pattern `{}`, found {:?}",
        source,
        parser.current()
    );
    pat
}

fn parse_binding_ok(source: &str) -> BindingTarget {
    let mut parser = Parser::new(source);
    let target = parser
        .parse_binding_target()
        .expect("binding target should parse");
    assert!(
        parser.errors().is_empty(),
        "unexpected parse errors for binding `{}`: {:?}",
        source,
        parser.errors()
    );
    assert!(
        parser.is_eof(),
        "expected EOF after binding `{}`, found {:?}",
        source,
        parser.current()
    );
    target
}

#[test]
fn test_pointer_reference_array_optional_and_tuple_types() {
    // *const int[]?  =>  Pointer { is_const: true, elem: Optional(DynamicArray(Int)) }
    let ptr_ty = parse_type_ok("*const int[]?");
    let TypeKind::Pointer {
        is_const: true,
        elem: opt_ty,
    } = ptr_ty.kind
    else {
        panic!("expected const Pointer, got {:?}", ptr_ty.kind);
    };
    let TypeKind::Optional(arr_ty) = opt_ty.kind else {
        panic!("expected Optional");
    };
    assert!(matches!(
        arr_ty.kind,
        TypeKind::DynamicArray(ref inner) if inner.kind == TypeKind::Predefined(PredefinedType::Int)
    ));

    // &int and &const string
    assert!(matches!(
        parse_type_ok("&int").kind,
        TypeKind::ReferenceType {
            is_const: false,
            ..
        }
    ));
    assert!(matches!(
        parse_type_ok("&const string").kind,
        TypeKind::ReferenceType { is_const: true, .. }
    ));

    // (*int)[]  =>  DynamicArray(Parenthesized(Pointer(Int)))
    let arr_of_ptr = parse_type_ok("(*int)[]");
    let TypeKind::DynamicArray(paren_ptr) = arr_of_ptr.kind else {
        panic!("expected DynamicArray");
    };
    assert!(matches!(paren_ptr.kind, TypeKind::Parenthesized(_)));

    // Fixed arrays: literal size & expression size
    let fixed_lit = parse_type_ok("int[16]");
    assert!(matches!(
        fixed_lit.kind,
        TypeKind::FixedArray {
            size: ArraySize::Literal(16),
            ..
        }
    ));

    let fixed_expr = parse_type_ok("double[SIZE + 1]");
    assert!(matches!(
        fixed_expr.kind,
        TypeKind::FixedArray {
            size: ArraySize::Expr(_),
            ..
        }
    ));

    // Tuple type including `this`
    let tuple_ty = parse_type_ok("[int, string, this]");
    let TypeKind::Tuple(elems) = tuple_ty.kind else {
        panic!("expected Tuple");
    };
    assert_eq!(elems.len(), 3);
    assert_eq!(elems[2].kind, TypeKind::This);
}

#[test]
fn test_anonymous_struct_types() {
    // Comma-separated with readonly, optional fields, and method signature
    let anon1 = parse_type_ok("{ readonly x: string, y?: int, onClick(x: int, y: int): void }");
    let TypeKind::AnonymousStruct(members1) = anon1.kind else {
        panic!("expected AnonymousStruct");
    };
    assert_eq!(members1.len(), 3);
    assert!(matches!(
        &members1[0],
        AnonymousStructMember::Field(f) if f.is_readonly && !f.is_optional
    ));
    assert!(matches!(
        &members1[1],
        AnonymousStructMember::Field(f) if !f.is_readonly && f.is_optional
    ));
    assert!(matches!(
        &members1[2],
        AnonymousStructMember::Method(m) if m.name.as_str() == "onClick" && m.params.params.len() == 2
    ));

    // Semicolon and newline-separated fields and generic optional method
    let anon2 = parse_type_ok("{\n a: int;\n b: bool\n compute?<T>(v: T): T\n}");
    let TypeKind::AnonymousStruct(members2) = anon2.kind else {
        panic!("expected AnonymousStruct");
    };
    assert_eq!(members2.len(), 3);
    assert!(matches!(
        &members2[2],
        AnonymousStructMember::Method(m) if m.is_optional && m.type_parameters.is_some()
    ));
}

#[test]
fn test_function_types_and_nested_generics() {
    // 1. Zero-param function type
    let fn0 = parse_type_ok("() => void");
    assert!(matches!(
        fn0.kind,
        TypeKind::Function(ref f) if f.params.is_empty() && f.type_parameters.is_none()
    ));

    // 2. Unnamed parameter types
    let fn_unnamed = parse_type_ok("(int, string) => bool");
    let TypeKind::Function(f_unnamed) = fn_unnamed.kind else {
        panic!("expected Function type");
    };
    assert_eq!(f_unnamed.params.len(), 2);
    assert!(f_unnamed.params[0].name.is_none());
    assert!(f_unnamed.params[1].name.is_none());

    // 3. Named, readonly, optional, rest params + type predicate return
    let fn_pred = parse_type_ok("(readonly x?: int, ...rest: string[]) => val is string");
    let TypeKind::Function(f_pred) = fn_pred.kind else {
        panic!("expected Function type");
    };
    assert_eq!(f_pred.params.len(), 2);
    assert!(f_pred.params[0].is_readonly);
    assert!(f_pred.params[0].is_optional);
    assert_eq!(
        f_pred.params[0].name.as_ref().map(|i| i.as_str()),
        Some("x")
    );
    assert!(f_pred.params[1].is_rest);
    assert!(matches!(*f_pred.return_type, ReturnType::Predicate(_)));

    // 4. Generic function type with constraint and default
    let fn_gen = parse_type_ok("<T extends Object = int>(x: T) => T");
    let TypeKind::Function(f_gen) = fn_gen.kind else {
        panic!("expected generic Function type");
    };
    let tp = f_gen.type_parameters.expect("type parameters");
    assert_eq!(tp.params.len(), 1);
    assert!(tp.params[0].constraint.is_some());
    assert!(tp.params[0].default.is_some());

    // 5. Optional parenthesized function type
    let opt_fn = parse_type_ok("((int) => void)?");
    assert!(matches!(opt_fn.kind, TypeKind::Optional(_)));

    // 6. Nested generics with `>>>` splitting
    let nested = parse_type_ok("std.collections.Map<string, Array<Box<int>>>");
    let TypeKind::Reference(outer_ref) = nested.kind else {
        panic!("expected TypeReference");
    };
    assert_eq!(outer_ref.name.to_string(), "std.collections.Map");
    let outer_args = outer_ref.type_arguments.expect("outer type args");
    assert_eq!(outer_args.args.len(), 2);
    assert!(matches!(outer_args.args[1].kind, TypeKind::Reference(_)));
}

#[test]
fn test_patterns_and_binding_targets() {
    // Wildcard, signed numeric literal, string literal, var patterns
    assert_eq!(parse_pattern_ok("_").kind, PatternKind::Wildcard);

    let signed = parse_pattern_ok("-42");
    assert!(matches!(
        signed.kind,
        PatternKind::Literal(ref lp) if lp.sign == Some(LiteralSign::Minus)
    ));

    let const_var = parse_pattern_ok("const item");
    assert!(matches!(
        const_var.kind,
        PatternKind::Var {
            modifier: Some(VarModifier::Const),
            ref name
        } if name.as_str() == "item"
    ));

    // Struct pattern: Point { x: 0, y: let y, z, const w, ... }
    let struct_pat = parse_pattern_ok("Point { x: 0, y: let y, z, const w, ... }");
    let PatternKind::Struct {
        struct_name,
        fields,
    } = struct_pat.kind
    else {
        panic!("expected Struct pattern");
    };
    assert_eq!(struct_name.to_string(), "Point");
    assert_eq!(fields.len(), 5);
    assert!(matches!(fields[0], StructFieldPattern::Field { .. }));
    assert!(matches!(
        fields[1],
        StructFieldPattern::Field {
            pattern: Pattern {
                kind: PatternKind::Var {
                    modifier: Some(VarModifier::Let),
                    ..
                },
                ..
            },
            ..
        }
    ));
    assert!(matches!(
        fields[2],
        StructFieldPattern::Binding { modifier: None, .. }
    ));
    assert!(matches!(
        fields[3],
        StructFieldPattern::Binding {
            modifier: Some(VarModifier::Const),
            ..
        }
    ));
    assert!(matches!(fields[4], StructFieldPattern::Rest(_)));

    // Variant patterns: qualified unit variant and payload variant
    let unit_variant = parse_pattern_ok("FileError.NotFound");
    assert!(matches!(
        unit_variant.kind,
        PatternKind::Variant {
            ref variant_name,
            ref payload
        } if variant_name.to_string() == "FileError.NotFound" && payload.is_empty()
    ));

    let payload_variant = parse_pattern_ok("Option.Some(let val)");
    assert!(matches!(
        payload_variant.kind,
        PatternKind::Variant {
            ref variant_name,
            ref payload
        } if variant_name.to_string() == "Option.Some" && payload.len() == 1
    ));

    // Tuple pattern with nesting and rest
    let tuple_pat = parse_pattern_ok("[head, [a, -2], ...tail]");
    let PatternKind::Tuple(elems) = tuple_pat.kind else {
        panic!("expected Tuple pattern");
    };
    assert_eq!(elems.len(), 3);
    assert!(!elems[0].is_rest);
    assert!(matches!(elems[1].pattern.kind, PatternKind::Tuple(_)));
    assert!(elems[2].is_rest);

    // Object pattern in both Pattern and BindingTarget
    let obj_pat = parse_pattern_ok("{ x, y: [a, b] = [1, 2], ...rest }");
    let PatternKind::Object(props) = obj_pat.kind else {
        panic!("expected Object pattern");
    };
    assert_eq!(props.len(), 3);
    assert!(matches!(
        props[1],
        ObjectPatternProp::Property {
            target: Some(BindingTarget::Tuple { .. }),
            default: Some(_),
            ..
        }
    ));
    assert!(matches!(props[2], ObjectPatternProp::Rest { .. }));

    // BindingTarget tuple & nested object destructuring
    let bt = parse_binding_ok("[first, { a, b: renamed = 10 }, ...others]");
    assert!(matches!(
        bt,
        BindingTarget::Tuple { ref elements, .. } if elements.len() == 3
    ));
}

// =============================================================================
// Phase 4: Statement & Control-Flow Parser Tests
// =============================================================================

fn parse_stmt_ok(source: &str) -> Stmt {
    let mut parser = Parser::new(source);
    let stmt = parser.parse_stmt().expect("statement should parse");
    assert!(
        parser.errors().is_empty(),
        "unexpected parse errors for stmt `{}`: {:?}",
        source,
        parser.errors()
    );
    assert!(
        parser.is_eof(),
        "expected EOF after stmt `{}`, found {:?}",
        source,
        parser.current()
    );
    stmt
}

fn parse_block_ok(source: &str) -> Block {
    let mut parser = Parser::new(source);
    let block = parser.parse_block().expect("block should parse");
    assert!(
        parser.errors().is_empty(),
        "unexpected parse errors for block `{}`: {:?}",
        source,
        parser.errors()
    );
    assert!(
        parser.is_eof(),
        "expected EOF after block `{}`, found {:?}",
        source,
        parser.current()
    );
    block
}

#[test]
fn test_variable_and_using_statements() {
    let block = parse_block_ok(
        "{\n let x: int = 10, y = 20\n let uninit\n const [a, b] = tuple; const { k, v: renamed } = obj\n using db: SafeDb = openDb(\"test.db\")\n}",
    );
    assert_eq!(block.statements.len(), 5);

    assert!(matches!(
        &block.statements[0].kind,
        StmtKind::Variable(v) if v.modifier == VarModifier::Let && v.declarations.len() == 2
    ));
    assert!(matches!(
        &block.statements[1].kind,
        StmtKind::Variable(v) if v.declarations[0].init.is_none()
    ));
    assert!(matches!(
        &block.statements[2].kind,
        StmtKind::Variable(v) if v.modifier == VarModifier::Const && matches!(v.declarations[0].target, BindingTarget::Tuple { .. })
    ));
    assert!(matches!(
        &block.statements[3].kind,
        StmtKind::Variable(v) if v.modifier == VarModifier::Const && matches!(v.declarations[0].target, BindingTarget::Object { .. })
    ));
    assert!(matches!(
        &block.statements[4].kind,
        StmtKind::Using(u) if u.name.as_str() == "db" && u.ty.is_some()
    ));

    // `const` without initializer emits MissingInitializer while returning partial VariableStmt.
    let mut missing_const = Parser::new("const bad");
    let stmt = missing_const.parse_stmt().unwrap();
    assert!(matches!(stmt.kind, StmtKind::Variable(_)));
    assert_eq!(missing_const.errors().len(), 1);
    assert_eq!(
        missing_const.errors()[0].kind,
        ParseErrorKind::MissingInitializer
    );
}

#[test]
fn test_if_dangling_else_while_and_do_while() {
    // Dangling else binds to the innermost `if`.
    let nested_if = parse_stmt_ok("if (a) if (b) x = 1 else x = 2");
    let StmtKind::If(outer) = nested_if.kind else {
        panic!("expected outer If");
    };
    assert!(outer.else_branch.is_none());
    let StmtKind::If(inner) = outer.then_branch.kind else {
        panic!("expected inner If");
    };
    assert!(inner.else_branch.is_some());

    let while_stmt = parse_stmt_ok("while (cond) { work() }");
    assert!(matches!(while_stmt.kind, StmtKind::While(_)));

    let do_while = parse_stmt_ok("do { x += 1 } while (x < 10)");
    assert!(matches!(do_while.kind, StmtKind::DoWhile(_)));

    let braceless_do = parse_stmt_ok("do x += 1 while (x < 10)");
    assert!(matches!(braceless_do.kind, StmtKind::DoWhile(_)));

    // Outside `if`, `else` on the same line still triggers ExpectedEos.
    let mut bad_else = Parser::new("x = 1 else");
    let _ = bad_else.parse_stmt().unwrap();
    assert_eq!(bad_else.errors().len(), 1);
    assert_eq!(
        bad_else.errors()[0].kind,
        ParseErrorKind::ExpectedEos {
            found: TokenKind::Else
        }
    );
}

#[test]
fn test_for_standard_and_iteration_statements() {
    let std_for = parse_stmt_ok("for (let i = 0; i < 10; i += 1) { sum += i }");
    assert!(matches!(
        std_for.kind,
        StmtKind::For(ForStmt::Standard {
            init: Some(ForInit::Let(_)),
            test: Some(_),
            update: Some(_),
            ..
        })
    ));

    let empty_for = parse_stmt_ok("for (;;) { break }");
    assert!(matches!(
        empty_for.kind,
        StmtKind::For(ForStmt::Standard {
            init: None,
            test: None,
            update: None,
            ..
        })
    ));

    let for_of = parse_stmt_ok("for (const [k, v] of pairs) { usePair(k, v) }");
    assert!(matches!(
        for_of.kind,
        StmtKind::For(ForStmt::Iteration {
            modifier: VarModifier::Const,
            target: BindingTarget::Tuple { .. },
            kind: ForIterationKind::Of,
            ..
        })
    ));

    let for_in = parse_stmt_ok("for (let key in dict) { println(key) }");
    assert!(matches!(
        for_in.kind,
        StmtKind::For(ForStmt::Iteration {
            modifier: VarModifier::Let,
            kind: ForIterationKind::In,
            ..
        })
    ));
}

#[test]
fn test_switch_statement_with_patterns_and_guards() {
    let sw = parse_stmt_ok(
        "switch (pt) {\n  case Point { x: 0, y: 0 }:\n    println(\"origin\")\n  case Point { x: 0, y: let y } when y > 0:\n    println(y)\n  case _:\n  default:\n    break\n}",
    );
    let StmtKind::Switch(s) = sw.kind else {
        panic!("expected Switch statement");
    };
    assert_eq!(s.cases.len(), 4);
    assert!(matches!(
        &s.cases[0],
        SwitchCase::Case {
            guard: None,
            body,
            ..
        } if body.len() == 1
    ));
    assert!(matches!(
        &s.cases[1],
        SwitchCase::Case {
            guard: Some(_),
            body,
            ..
        } if body.len() == 1
    ));
    assert!(matches!(
        &s.cases[2],
        SwitchCase::Case { body, .. } if body.is_empty()
    ));
    assert!(matches!(&s.cases[3], SwitchCase::Default { body, .. } if body.len() == 1));
}

#[test]
fn test_jump_defer_unsafe_labeled_newlines_and_recovery() {
    // 1. Newline after `return` / `break` splits into separate statements.
    let block = parse_block_ok(
        "{\n  return\n  x + 1\n  break\n  nextStmt\n  outer: for (let i = 0; i < 3; i += 1) { continue outer }\n  defer unsafe { free(p) }\n  debugger;\n  ;\n  async (x) => await x\n}",
    );
    assert_eq!(block.statements.len(), 9);
    assert!(matches!(
        block.statements[0].kind,
        StmtKind::Return(ref r) if r.value.is_none()
    ));
    assert!(matches!(block.statements[1].kind, StmtKind::Expr(_)));
    assert!(matches!(
        block.statements[2].kind,
        StmtKind::Break(ref b) if b.label.is_none()
    ));
    assert!(matches!(block.statements[3].kind, StmtKind::Expr(_)));
    assert!(matches!(
        block.statements[4].kind,
        StmtKind::Labeled(ref l) if l.label.as_str() == "outer"
    ));
    assert!(matches!(
        block.statements[5].kind,
        StmtKind::Defer(ref d) if matches!(d.statement.kind, StmtKind::Unsafe(_))
    ));
    assert_eq!(block.statements[6].kind, StmtKind::Debugger);
    assert_eq!(block.statements[7].kind, StmtKind::Empty);
    assert!(matches!(
        block.statements[8].kind,
        StmtKind::Expr(Expr {
            kind: ExprKind::ArrowFunction { is_async: true, .. },
            ..
        })
    ));

    // 2. Error recovery inside block continues parsing subsequent valid statements.
    let mut bad_block = Parser::new("{\n  let = 123\n  let ok = 42\n}");
    let recovered = bad_block.parse_block().unwrap();
    assert!(!bad_block.errors().is_empty());
    assert_eq!(recovered.statements.len(), 2);
    assert_eq!(recovered.statements[0].kind, StmtKind::Error);
    assert!(matches!(
        recovered.statements[1].kind,
        StmtKind::Variable(_)
    ));
}

// =============================================================================
// Phase 5 & 6: Declarations, Attributes, C-FFI, Modules & Integration Tests
// =============================================================================

fn parse_program_ok(source: &str) -> ParseOutput {
    let out = parse(source);
    assert!(
        !out.has_errors(),
        "unexpected parse errors in program:\n{}\nerrors: {:?}",
        source,
        out.errors
    );
    out
}

#[test]
fn test_import_and_export_statements() {
    let out = parse_program_ok(
        r#"
import { Foo, Bar as Baz, } from "./mod"
import * as Math from "./math"
import DefaultItem from "./item"
import Item, { Sub as Alias } from "./combo"
import "./side_effect"

export { Foo, Bar as OutBar } from "./reexport"
export default (x: int) => x * 2
#[export_name = "MyLib_Multiply"]
export extern "C" function multiply(a: double, b: double): double {
    return a * b
}
"#,
    );

    assert_eq!(out.program.elements.len(), 8);
    assert!(matches!(
        &out.program.elements[0],
        SourceElement::Import(i) if matches!(i.clause, Some(ImportClause::Named(ref specs)) if specs.len() == 2)
    ));
    assert!(matches!(
        &out.program.elements[1],
        SourceElement::Import(i) if matches!(i.clause, Some(ImportClause::Namespace(ref id)) if id.as_str() == "Math")
    ));
    assert!(matches!(
        &out.program.elements[2],
        SourceElement::Import(i) if matches!(i.clause, Some(ImportClause::Default { ref named, .. }) if named.is_empty())
    ));
    assert!(matches!(
        &out.program.elements[3],
        SourceElement::Import(i) if matches!(i.clause, Some(ImportClause::Default { ref named, .. }) if named.len() == 1)
    ));
    assert!(matches!(
        &out.program.elements[4],
        SourceElement::Import(i) if i.clause.is_none()
    ));
    assert!(matches!(
        &out.program.elements[5],
        SourceElement::Export(e) if matches!(e.kind, ExportKind::Named { source: Some(_), .. })
    ));
    assert!(matches!(
        &out.program.elements[6],
        SourceElement::Export(e) if matches!(e.kind, ExportKind::Default(_))
    ));
    assert!(matches!(
        &out.program.elements[7],
        SourceElement::Export(e) if e.attributes.len() == 1 && matches!(e.kind, ExportKind::Decl(_))
    ));
}

#[test]
fn test_function_struct_class_and_interface_declarations() {
    let out = parse_program_ok(
        r#"
async function fetch<T extends Comparable>(a: T, ...rest: T[]): Promise<T> {
    function localHelper(x: int): int { return x + 1 }
    return a
}

function forwardDecl(x: int): void

#[repr(C)]
#[repr(align(16))]
struct Vector2<T> {
    public readonly x: T
    private static count: int = 0
    constructor(x: T) {
        this.x = x
    }
    public length(): double {
        return 0.0
    }
}

class AdminUser extends User implements Disposable, Serializable {
    private #token: string
    get token(): string { return this.#token }
    set #token(v: string) { this.#token = v }
    get(): int { return 1 }
    constructor(id: ulong) {
        super(id)
    }
}

interface Collection<T> extends Iterable<T> {
    readonly count: int
    [key: string]: T
    clear?(): void
}
"#,
    );

    assert_eq!(out.program.elements.len(), 5);

    // Function with nested local function declaration inside block
    let SourceElement::Decl(fn_decl) = &out.program.elements[0] else {
        panic!("expected Decl");
    };
    let DeclKind::Function(f) = &fn_decl.kind else {
        panic!("expected Function");
    };
    assert!(f.is_async);
    assert!(f.type_parameters.is_some());
    assert!(f.params.rest.is_some());
    let body = f.body.as_ref().expect("function body");
    assert!(matches!(body.statements[0].kind, StmtKind::Decl(_)));

    // Bodyless function declaration
    assert!(matches!(
        &out.program.elements[1],
        SourceElement::Decl(d) if matches!(d.kind, DeclKind::Function(ref fd) if fd.body.is_none())
    ));

    // Struct with attributes, fields, constructor, method
    let SourceElement::Decl(st_decl) = &out.program.elements[2] else {
        panic!("expected Decl");
    };
    let DeclKind::Struct(st) = &st_decl.kind else {
        panic!("expected Struct");
    };
    assert_eq!(st.attributes.len(), 2);
    assert!(matches!(st.attributes[0].input, AttributeInput::Args(_)));
    assert_eq!(st.members.len(), 4);
    assert!(matches!(st.members[0], StructMember::Field(_)));
    assert!(matches!(st.members[1], StructMember::Field(ref f) if f.is_static));
    assert!(matches!(st.members[2], StructMember::Constructor(_)));
    assert!(matches!(st.members[3], StructMember::Method(_)));

    // Class with private `#token`, `get`/`set` accessors, AND regular `get()` method
    let SourceElement::Decl(cls_decl) = &out.program.elements[3] else {
        panic!("expected Decl");
    };
    let DeclKind::Class(cls) = &cls_decl.kind else {
        panic!("expected Class");
    };
    assert!(cls.extends.is_some());
    assert_eq!(cls.implements.len(), 2);
    assert_eq!(cls.members.len(), 5);
    assert!(matches!(
        &cls.members[0],
        ClassMember::Field(f) if matches!(f.name, ClassElementName::Private(_))
    ));
    assert!(matches!(
        &cls.members[1],
        ClassMember::GetterSetter(gs) if gs.kind == AccessorKind::Get
    ));
    assert!(matches!(
        &cls.members[2],
        ClassMember::GetterSetter(gs) if gs.kind == AccessorKind::Set && matches!(gs.name, ClassElementName::Private(_))
    ));
    assert!(matches!(
        &cls.members[3],
        ClassMember::Method(m) if matches!(m.name, ClassElementName::Ident(ref id) if id.as_str() == "get")
    ));

    // Interface
    let SourceElement::Decl(if_decl) = &out.program.elements[4] else {
        panic!("expected Decl");
    };
    let DeclKind::Interface(iface) = &if_decl.kind else {
        panic!("expected Interface");
    };
    assert_eq!(iface.extends.len(), 1);
    assert_eq!(iface.members.len(), 3);
    assert!(matches!(iface.members[0], InterfaceMember::Property(_)));
    assert!(matches!(iface.members[1], InterfaceMember::Index(_)));
    assert!(matches!(iface.members[2], InterfaceMember::Method(ref m) if m.is_optional));
}

#[test]
fn test_enum_type_alias_and_ffi_declarations() {
    let out = parse_program_ok(
        r#"
enum WebEvent<T> {
    PageLoad,
    KeyPress(char),
    Click(x: int, y: int),
    Message(string),
    Custom(T),
}

enum Flags : uint {
    Read = 1,
    Write = 2,
}

type Handler<T = int> = (event: T) => void

#[link(name = "sqlite3")]
extern "C" {
    #[link_name = "sqlite3_open"]
    function sqliteOpen(filename: *const ubyte, ppDb: *rawptr): int
    function printf(fmt: *const ubyte, ...): int
    let errno: int
    const stdout: rawptr
}

export extern "C" function kyanos_sum_ints(count: int, ...args: VaList): int {
    return count
}

extern "C" function puts(s: *const ubyte): int
"#,
    );

    assert_eq!(out.program.elements.len(), 6);

    // Enum ADT with unnamed and named payloads
    let SourceElement::Decl(e1_decl) = &out.program.elements[0] else {
        panic!("expected Decl");
    };
    let DeclKind::Enum(e1) = &e1_decl.kind else {
        panic!("expected Enum");
    };
    assert_eq!(e1.variants.len(), 5);
    assert!(matches!(
        &e1.variants[0],
        EnumVariant::Value { value: None, .. }
    ));
    assert!(matches!(
        &e1.variants[1],
        EnumVariant::Tuple { payload, .. } if payload.len() == 1 && payload[0].name.is_none()
    ));
    assert!(matches!(
        &e1.variants[2],
        EnumVariant::Tuple { payload, .. } if payload.len() == 2 && payload[0].name.is_some()
    ));
    assert!(matches!(
        &e1.variants[4],
        EnumVariant::Tuple { payload, .. } if payload.len() == 1 && payload[0].name.is_none()
    ));

    // Tag-typed numeric enum
    assert!(matches!(
        &out.program.elements[1],
        SourceElement::Decl(d) if matches!(d.kind, DeclKind::Enum(ref e) if e.tag_type == Some(PredefinedType::Uint))
    ));

    // Type alias
    assert!(matches!(
        &out.program.elements[2],
        SourceElement::Decl(d) if matches!(d.kind, DeclKind::TypeAlias(_))
    ));

    // Extern block with variadic `...` and variables
    let SourceElement::Decl(ext_blk_decl) = &out.program.elements[3] else {
        panic!("expected Decl");
    };
    let DeclKind::ExternBlock(blk) = &ext_blk_decl.kind else {
        panic!("expected ExternBlock");
    };
    assert_eq!(blk.abi.value, "C");
    assert_eq!(blk.members.len(), 4);
    assert!(matches!(
        &blk.members[1],
        ExternMember::Function(f) if f.params.is_variadic && f.params.rest.is_none()
    ));
    assert!(matches!(
        &blk.members[2],
        ExternMember::Variable(v) if !v.is_const
    ));
    assert!(matches!(
        &blk.members[3],
        ExternMember::Variable(v) if v.is_const
    ));

    // Exported extern function with `...args: VaList`
    let SourceElement::Export(exp) = &out.program.elements[4] else {
        panic!("expected Export");
    };
    assert!(matches!(
        &exp.kind,
        ExportKind::Decl(d) if matches!(
            d.kind,
            DeclKind::SingleExtern(ref se) if !se.params.is_variadic && se.params.rest.is_some() && se.body.is_some()
        )
    ));

    // Bodyless single extern declaration
    assert!(matches!(
        &out.program.elements[5],
        SourceElement::Decl(d) if matches!(d.kind, DeclKind::SingleExtern(ref se) if se.body.is_none())
    ));
}

#[test]
fn test_end_to_end_kyanos_showcase_and_recovery() {
    let showcase = r#"
import { println } from "std/io"

struct Point {
    x: double
    y: double
}

export function main(): int {
    const origin = Point { x: 0.0, y: 0.0 }
    let config = { "host": "localhost", "port": "8080" }
    let host: string = config.host ?? "127.0.0.1"
    let label = `Host=${host}, X=${origin.x}`

    let nums: int[] = [1, 2, 3]
    let doubled = nums.map((x) => x * 2)

    let status = switch (doubled[0]) {
        case 2 => "ok",
        default => "other",
    }

    println(`${label}: ${status}`)
    return 0
}
"#;
    let out = parse_program_ok(showcase);
    assert_eq!(out.program.elements.len(), 3);

    // Top-level `parse_expr` helper checks trailing tokens.
    let (expr, errs) = parse_expr("1 + 2 * 3");
    assert!(errs.is_empty());
    assert!(matches!(expr.kind, ExprKind::Binary { .. }));

    let (_, trailing_errs) = parse_expr("1 + 2 foo");
    assert_eq!(trailing_errs.len(), 1);

    // Top-level recovery across stray `}` and broken declaration preserves later functions.
    let broken = "}\nfunction broken( {\n}\nfunction recovered(): int {\n    return 42\n}\n";
    let res = parse(broken);
    assert!(res.has_errors());
    assert!(res.program.elements.iter().any(|el| matches!(
        el,
        SourceElement::Decl(d) if matches!(d.kind, DeclKind::Function(ref f) if f.name.as_str() == "recovered")
    )));
}
