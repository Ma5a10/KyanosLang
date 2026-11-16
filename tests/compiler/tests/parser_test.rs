// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use kyanos_ast::{
    ArrowBody, AssignOp, BinaryOp, BindingTarget, Expr, ExprKind, LiteralKind, MemberProperty,
    NodeId, ObjectMember, PatternKind, PostfixUnaryOp, PredefinedType, PrefixUnaryOp, PropertyName,
    Radix, SizeofTarget, Span, StmtKind, StructFieldInit, SwitchExprArm, TemplateStringPart,
    TypeKind,
};
use kyanos_lexer::TokenKind;
use kyanos_parse::{ParseError, ParseErrorKind, Parser};

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
