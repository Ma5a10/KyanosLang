// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Unified tests for Kyanos AST nodes and data structures.

use kyanos_ast::{
    Accessibility, AccessorKind, AnonymousStructField, Argument, ArrayElement, ArraySize,
    ArrowBody, AssignOp, Associativity, Attribute, AttributeArg, BinaryOp, BindingTarget, Block,
    ClassDecl, ClassElementName, ClassField, ClassMember, ClassMethod, ConstructorDecl, Decl,
    DeclKind, EnumDecl, EnumVariant, Expr, ExprKind, ExternBlockDecl, ExternFunctionDecl,
    ExternMember, ExternParameterList, ForInit, ForIterationKind, ForStmt, FunctionDecl,
    FunctionParam, FunctionType, FunctionTypeParam, GetterSetterDecl, Ident, IndexSignature,
    InterfaceDecl, InterfaceMember, InterfaceMethod, InterfaceProperty, Literal, MemberProperty,
    MethodDecl, ObjectMember, ObjectPatternProp, Parameter, ParameterList, Pattern, PatternKind,
    PostfixUnaryOp, PredefinedType, PrefixUnaryOp, PrivateIdent, PropertyName, Radix,
    RestParameter, ReturnType, SingleExternDecl, SizeofTarget, Span, Stmt, StmtKind, StringLiteral,
    StructDecl, StructField, StructFieldInit, StructFieldPattern, StructMember, SwitchCase,
    SwitchExprArm, TemplateStringPart, TuplePatternElement, Type, TypeAliasDecl, TypeArguments,
    TypeKind, TypeName, TypeParameter, TypeParameters, TypePredicate, TypeReference, VarModifier,
    VariableDecl,
};

// =============================================================================
// Ident Tests
// =============================================================================

#[test]
fn test_ident_creation() {
    let span = Span::new(0, 3);
    let id = Ident::new("foo", span);
    assert_eq!(id.as_str(), "foo");
    assert_eq!(id.span, span);
    assert!(!id.is_underscore());

    let wildcard = Ident::new("_", Span::new(4, 5));
    assert!(wildcard.is_underscore());
}

#[test]
fn test_private_ident() {
    let span = Span::new(0, 5);
    let pid = PrivateIdent::new("#name", span);
    assert_eq!(pid.as_str(), "#name");
    assert_eq!(pid.to_string(), "#name");
}

// =============================================================================
// Operator Tests
// =============================================================================

#[test]
fn test_binary_op_precedence_and_associativity() {
    assert_eq!(BinaryOp::Pow.associativity(), Associativity::Right);
    assert_eq!(BinaryOp::Add.associativity(), Associativity::Left);

    assert!(BinaryOp::Mul.precedence() > BinaryOp::Add.precedence());
    assert!(BinaryOp::Add.precedence() > BinaryOp::Eq.precedence());
    assert!(BinaryOp::Eq.precedence() > BinaryOp::LogicalAnd.precedence());
    assert!(BinaryOp::LogicalAnd.precedence() > BinaryOp::NullCoalescing.precedence());

    let (l_pow, r_pow) = BinaryOp::Pow.binding_power();
    assert!(l_pow > r_pow); // Right-associative: left_bp > right_bp

    let (l_add, r_add) = BinaryOp::Add.binding_power();
    assert!(l_add < r_add); // Left-associative: left_bp < right_bp
}

#[test]
fn test_assign_op_conversion() {
    assert_eq!(AssignOp::Assign.to_binary_op(), None);
    assert_eq!(AssignOp::AddAssign.to_binary_op(), Some(BinaryOp::Add));
    assert_eq!(AssignOp::PowAssign.to_binary_op(), Some(BinaryOp::Pow));
}

#[test]
fn test_unary_op_symbols() {
    assert_eq!(PrefixUnaryOp::Plus.as_str(), "+");
    assert_eq!(PrefixUnaryOp::Not.as_str(), "!");
    assert_eq!(PostfixUnaryOp::NonNull.as_str(), "!");
}

// =============================================================================
// Literal Tests
// =============================================================================

#[test]
fn test_literal_creation_and_display() {
    let span = Span::new(0, 4);
    let int_lit = Literal::integer("0xFF", 255, Radix::Hex, span);
    assert_eq!(int_lit.to_string(), "0xFF");
    assert_eq!(int_lit.span, span);

    let float_lit = Literal::float("3.14", 3.14, Span::new(5, 9));
    assert_eq!(float_lit.to_string(), "3.14");

    let str_lit = Literal::string("hello", "\"hello\"", Span::new(10, 17));
    assert_eq!(str_lit.to_string(), "\"hello\"");

    let bool_lit = Literal::bool(true, Span::new(18, 22));
    assert_eq!(bool_lit.to_string(), "true");

    let null_lit = Literal::null(Span::new(23, 27));
    assert_eq!(null_lit.to_string(), "null");
}

// =============================================================================
// Type Annotation Tests
// =============================================================================

#[test]
fn test_predefined_types() {
    let span = Span::new(0, 3);
    let int_type = Type::predefined(PredefinedType::Int, span);
    assert_eq!(int_type.span, span);
    assert_eq!(int_type.kind, TypeKind::Predefined(PredefinedType::Int));
    assert_eq!(PredefinedType::Int.as_str(), "int");
    assert_eq!(PredefinedType::Rawptr.as_str(), "rawptr");
    assert_eq!(PredefinedType::Double.as_str(), "double");
}

#[test]
fn test_pointer_and_reference_types() {
    let int_ty = Type::predefined(PredefinedType::Int, Span::new(1, 4));

    // Mutable raw pointer: *int
    let ptr_ty = Type::pointer(int_ty.clone(), false, Span::new(0, 4));
    assert_eq!(
        ptr_ty.kind,
        TypeKind::Pointer {
            is_const: false,
            elem: Box::new(int_ty.clone())
        }
    );

    // Immutable raw pointer: *const int
    let const_ptr_ty = Type::pointer(int_ty.clone(), true, Span::new(0, 10));
    assert_eq!(
        const_ptr_ty.kind,
        TypeKind::Pointer {
            is_const: true,
            elem: Box::new(int_ty.clone())
        }
    );

    // Mutable reference: &int
    let ref_ty = Type::reference_type(int_ty.clone(), false, Span::new(0, 4));
    assert_eq!(
        ref_ty.kind,
        TypeKind::ReferenceType {
            is_const: false,
            elem: Box::new(int_ty.clone())
        }
    );

    // Immutable reference: &const int
    let const_ref_ty = Type::reference_type(int_ty.clone(), true, Span::new(0, 10));
    assert_eq!(
        const_ref_ty.kind,
        TypeKind::ReferenceType {
            is_const: true,
            elem: Box::new(int_ty.clone())
        }
    );
}

#[test]
fn test_array_and_optional_types() {
    let elem_ty = Type::predefined(PredefinedType::Int, Span::new(0, 3));

    // Dynamic array: int[]
    let dyn_arr = Type::dynamic_array(elem_ty.clone(), Span::new(0, 5));
    assert_eq!(
        dyn_arr.kind,
        TypeKind::DynamicArray(Box::new(elem_ty.clone()))
    );

    // Fixed array: int[16]
    let fixed_arr = Type::fixed_array(elem_ty.clone(), ArraySize::Literal(16), Span::new(0, 7));
    assert_eq!(
        fixed_arr.kind,
        TypeKind::FixedArray {
            elem: Box::new(elem_ty.clone()),
            size: ArraySize::Literal(16)
        }
    );

    // Optional sugar: int?
    let opt_ty = Type::optional(elem_ty.clone(), Span::new(0, 4));
    assert_eq!(opt_ty.kind, TypeKind::Optional(Box::new(elem_ty)));
}

#[test]
fn test_type_reference_and_generics() {
    // std.collections.Map<string, int>
    let map_name = TypeName::new(
        vec![
            Ident::new("std", Span::new(0, 3)),
            Ident::new("collections", Span::new(4, 15)),
            Ident::new("Map", Span::new(16, 19)),
        ],
        Span::new(0, 19),
    );
    assert_eq!(map_name.to_string(), "std.collections.Map");

    let string_ty = Type::predefined(PredefinedType::String, Span::new(20, 26));
    let int_ty = Type::predefined(PredefinedType::Int, Span::new(28, 31));
    let args = TypeArguments::new(vec![string_ty.clone(), int_ty.clone()], Span::new(19, 32));

    let generic_ref = Type::reference(map_name.clone(), Some(args), Span::new(0, 32));
    assert_eq!(
        generic_ref.kind,
        TypeKind::Reference(TypeReference::new(
            map_name,
            Some(TypeArguments::new(
                vec![string_ty, int_ty.clone()],
                Span::new(19, 32)
            )),
            Span::new(0, 32)
        ))
    );

    // Generic parameter definition: <T extends Object = int>
    let param = TypeParameter::new(
        Ident::new("T", Span::new(1, 2)),
        Some(Type::reference(
            TypeName::single(Ident::new("Object", Span::new(11, 17))),
            None,
            Span::new(11, 17),
        )),
        Some(int_ty),
        Span::new(1, 23),
    );
    let params = TypeParameters::new(vec![param], Span::new(0, 24));
    assert_eq!(params.params.len(), 1);
    assert_eq!(params.params[0].name.as_str(), "T");
    assert!(params.params[0].constraint.is_some());
    assert!(params.params[0].default.is_some());
}

#[test]
fn test_tuple_and_anonymous_struct_types() {
    let t1 = Type::predefined(PredefinedType::Int, Span::new(1, 4));
    let t2 = Type::predefined(PredefinedType::String, Span::new(6, 12));

    // Tuple: [int, string]
    let tuple_ty = Type::tuple(vec![t1.clone(), t2.clone()], Span::new(0, 13));
    assert_eq!(tuple_ty.kind, TypeKind::Tuple(vec![t1, t2.clone()]));

    // Anonymous Struct: { readonly x: string, y?: int }
    let fields = vec![
        AnonymousStructField::new(
            true,
            Ident::new("x", Span::new(11, 12)),
            false,
            t2,
            Span::new(2, 20),
        ),
        AnonymousStructField::new(
            false,
            Ident::new("y", Span::new(22, 23)),
            true,
            Type::predefined(PredefinedType::Int, Span::new(27, 30)),
            Span::new(22, 30),
        ),
    ];
    let anon_ty = Type::new(TypeKind::AnonymousStruct(fields), Span::new(0, 32));
    if let TypeKind::AnonymousStruct(fs) = anon_ty.kind {
        assert_eq!(fs.len(), 2);
        assert!(fs[0].is_readonly);
        assert!(!fs[0].is_optional);
        assert!(!fs[1].is_readonly);
        assert!(fs[1].is_optional);
    } else {
        panic!("expected TypeKind::AnonymousStruct");
    }
}

#[test]
fn test_function_type_and_type_predicate() {
    // (x: int, ...rest: string[]) => bool
    let param1 = FunctionTypeParam::new(
        Some(Ident::new("x", Span::new(1, 2))),
        false,
        false,
        false,
        Type::predefined(PredefinedType::Int, Span::new(4, 7)),
        Span::new(1, 7),
    );
    let param2 = FunctionTypeParam::new(
        Some(Ident::new("rest", Span::new(12, 16))),
        false,
        false,
        true,
        Type::dynamic_array(
            Type::predefined(PredefinedType::String, Span::new(18, 24)),
            Span::new(18, 26),
        ),
        Span::new(9, 26),
    );

    let fn_ty = FunctionType::new(
        None,
        vec![param1, param2],
        ReturnType::Type(Type::predefined(PredefinedType::Bool, Span::new(31, 35))),
        Span::new(0, 35),
    );
    assert_eq!(fn_ty.params.len(), 2);
    assert!(fn_ty.params[1].is_rest);

    // Type predicate: val is string
    let pred = TypePredicate::new(
        Ident::new("val", Span::new(0, 3)),
        Type::predefined(PredefinedType::String, Span::new(7, 13)),
        Span::new(0, 13),
    );
    let ret_pred = ReturnType::Predicate(pred);
    assert_eq!(ret_pred.span(), Span::new(0, 13));
}

// =============================================================================
// Expression Tests
// =============================================================================

#[test]
fn test_binary_and_unary_expressions() {
    let a = Expr::ident(Ident::new("a", Span::new(0, 1)));
    let b = Expr::ident(Ident::new("b", Span::new(4, 5)));

    // a + b
    let add_expr = Expr::binary(BinaryOp::Add, a.clone(), b.clone());
    assert_eq!(add_expr.span, Span::new(0, 5));
    if let ExprKind::Binary { op, left, right } = add_expr.kind {
        assert_eq!(op, BinaryOp::Add);
        assert_eq!(*left, a);
        assert_eq!(*right, b);
    } else {
        panic!("expected ExprKind::Binary");
    }

    // Prefix: -a, try res, await task
    let neg = Expr::unary_prefix(PrefixUnaryOp::Minus, a.clone(), Span::new(0, 2));
    assert_eq!(neg.span, Span::new(0, 2));

    let try_expr = Expr::unary_prefix(PrefixUnaryOp::Try, a.clone(), Span::new(0, 6));
    if let ExprKind::UnaryPrefix { op, target } = try_expr.kind {
        assert_eq!(op, PrefixUnaryOp::Try);
        assert_eq!(*target, a);
    } else {
        panic!("expected ExprKind::UnaryPrefix");
    }

    // Postfix: a++, a!
    let inc = Expr::unary_postfix(PostfixUnaryOp::PostInc, a.clone(), Span::new(0, 3));
    let non_null = Expr::unary_postfix(PostfixUnaryOp::NonNull, a, Span::new(0, 2));
    assert_eq!(inc.span, Span::new(0, 3));
    assert_eq!(non_null.span, Span::new(0, 2));
}

#[test]
fn test_call_and_member_expressions() {
    let callee = Expr::ident(Ident::new("compute", Span::new(0, 7)));
    let arg1 = Argument::normal(Expr::literal(Literal::integer(
        "10",
        10,
        Radix::Decimal,
        Span::new(8, 10),
    )));
    let arg2 = Argument::spread(
        Expr::ident(Ident::new("rest", Span::new(15, 19))),
        Span::new(12, 19),
    );

    // compute(10, ...rest)
    let call = Expr::call(callee.clone(), None, vec![arg1, arg2], Span::new(0, 20));
    if let ExprKind::Call {
        callee: c,
        type_arguments,
        args,
    } = call.kind
    {
        assert_eq!(*c, callee);
        assert!(type_arguments.is_none());
        assert_eq!(args.len(), 2);
        assert!(args[1].is_spread);
    } else {
        panic!("expected ExprKind::Call");
    }

    // obj.prop and obj.#privateProp
    let obj = Expr::ident(Ident::new("obj", Span::new(0, 3)));
    let member_pub = Expr::member(
        obj.clone(),
        MemberProperty::Ident(Ident::new("field", Span::new(4, 9))),
        Span::new(0, 9),
    );
    let member_priv = Expr::member(
        obj.clone(),
        MemberProperty::Private(PrivateIdent::new("#secret", Span::new(4, 11))),
        Span::new(0, 11),
    );

    assert_eq!(member_pub.span, Span::new(0, 9));
    assert_eq!(member_priv.span, Span::new(0, 11));

    // Index: arr[0] and optArr?.[0]
    let index_expr = Expr::literal(Literal::integer("0", 0, Radix::Decimal, Span::new(4, 5)));
    let idx = Expr::index(obj.clone(), index_expr.clone(), false, Span::new(0, 6));
    let opt_idx = Expr::index(obj, index_expr, true, Span::new(0, 7));

    if let ExprKind::Index { is_optional, .. } = idx.kind {
        assert!(!is_optional);
    }
    if let ExprKind::Index { is_optional, .. } = opt_idx.kind {
        assert!(is_optional);
    }
}

#[test]
fn test_literal_complex_expressions() {
    // Array: [1, ...rest, 3]
    let elem1 = ArrayElement::normal(Expr::literal(Literal::integer(
        "1",
        1,
        Radix::Decimal,
        Span::new(1, 2),
    )));
    let elem2 = ArrayElement::spread(
        Expr::ident(Ident::new("rest", Span::new(7, 11))),
        Span::new(4, 11),
    );
    let array_expr = Expr::new(ExprKind::Array(vec![elem1, elem2]), Span::new(0, 15));
    if let ExprKind::Array(elems) = array_expr.kind {
        assert_eq!(elems.len(), 2);
        assert!(elems[1].is_spread);
    } else {
        panic!("expected ExprKind::Array");
    }

    // Object: { x: 10, ...rest }
    let obj_prop = ObjectMember::Property {
        name: PropertyName::Ident(Ident::new("x", Span::new(2, 3))),
        value: Expr::literal(Literal::integer("10", 10, Radix::Decimal, Span::new(5, 7))),
        span: Span::new(2, 7),
    };
    let obj_spread = ObjectMember::Spread {
        expr: Expr::ident(Ident::new("rest", Span::new(12, 16))),
        span: Span::new(9, 16),
    };
    let obj_expr = Expr::new(
        ExprKind::Object(vec![obj_prop, obj_spread]),
        Span::new(0, 18),
    );
    if let ExprKind::Object(members) = obj_expr.kind {
        assert_eq!(members.len(), 2);
    } else {
        panic!("expected ExprKind::Object");
    }

    // Struct Instantiation: Point { x: 10, ...other }
    let struct_init = Expr::new(
        ExprKind::StructInstantiation {
            struct_name: TypeName::single(Ident::new("Point", Span::new(0, 5))),
            type_arguments: None,
            fields: vec![
                StructFieldInit::Field {
                    name: Ident::new("x", Span::new(8, 9)),
                    value: Some(Expr::literal(Literal::integer(
                        "10",
                        10,
                        Radix::Decimal,
                        Span::new(11, 13),
                    ))),
                    span: Span::new(8, 13),
                },
                StructFieldInit::Spread {
                    expr: Expr::ident(Ident::new("other", Span::new(18, 23))),
                    span: Span::new(15, 23),
                },
            ],
        },
        Span::new(0, 25),
    );
    if let ExprKind::StructInstantiation {
        struct_name,
        fields,
        ..
    } = struct_init.kind
    {
        assert_eq!(struct_name.to_string(), "Point");
        assert_eq!(fields.len(), 2);
    } else {
        panic!("expected ExprKind::StructInstantiation");
    }
}

#[test]
fn test_control_and_functional_expressions() {
    // Arrow Function: (x: int): int => x + 1
    let param = FunctionParam::simple(
        Ident::new("x", Span::new(1, 2)),
        Some(Type::predefined(PredefinedType::Int, Span::new(4, 7))),
        Span::new(1, 7),
    );
    let body = Expr::binary(
        BinaryOp::Add,
        Expr::ident(Ident::new("x", Span::new(19, 20))),
        Expr::literal(Literal::integer("1", 1, Radix::Decimal, Span::new(23, 24))),
    );
    let arrow = Expr::new(
        ExprKind::ArrowFunction {
            is_async: false,
            type_parameters: None,
            params: vec![param],
            return_type: Some(ReturnType::Type(Type::predefined(
                PredefinedType::Int,
                Span::new(10, 13),
            ))),
            body: ArrowBody::Expr(Box::new(body)),
        },
        Span::new(0, 24),
    );
    if let ExprKind::ArrowFunction {
        is_async, params, ..
    } = arrow.kind
    {
        assert!(!is_async);
        assert_eq!(params.len(), 1);
    } else {
        panic!("expected ExprKind::ArrowFunction");
    }

    // Switch Expression: switch (x) { case 1 => "one", default => "other" }
    let target = Expr::ident(Ident::new("x", Span::new(8, 9)));
    let arm1 = SwitchExprArm::Case {
        pattern: Pattern::literal(
            Literal::integer("1", 1, Radix::Decimal, Span::new(18, 19)),
            Span::new(18, 19),
        ),
        guard: None,
        body: Box::new(Expr::literal(Literal::string(
            "one",
            "\"one\"",
            Span::new(23, 28),
        ))),
        span: Span::new(13, 28),
    };
    let arm2 = SwitchExprArm::Default {
        body: Box::new(Expr::literal(Literal::string(
            "other",
            "\"other\"",
            Span::new(41, 48),
        ))),
        span: Span::new(30, 48),
    };
    let switch_expr = Expr::new(
        ExprKind::Switch {
            target: Box::new(target),
            arms: vec![arm1, arm2],
        },
        Span::new(0, 50),
    );
    if let ExprKind::Switch { arms, .. } = switch_expr.kind {
        assert_eq!(arms.len(), 2);
    } else {
        panic!("expected ExprKind::Switch");
    }

    // Ternary: cond ? a : b
    let cond = Expr::ident(Ident::new("cond", Span::new(0, 4)));
    let then_expr = Expr::ident(Ident::new("a", Span::new(7, 8)));
    let else_expr = Expr::ident(Ident::new("b", Span::new(11, 12)));
    let ternary = Expr::ternary(cond, then_expr, else_expr);
    assert_eq!(ternary.span, Span::new(0, 12));

    // Cast: x as int
    let x = Expr::ident(Ident::new("x", Span::new(0, 1)));
    let cast = Expr::cast(
        x,
        Type::predefined(PredefinedType::Int, Span::new(5, 8)),
        Span::new(0, 8),
    );
    assert_eq!(cast.span, Span::new(0, 8));

    // Assign: x += 10
    let target_var = Expr::ident(Ident::new("x", Span::new(0, 1)));
    let val = Expr::literal(Literal::integer("10", 10, Radix::Decimal, Span::new(5, 7)));
    let assign = Expr::assign(AssignOp::AddAssign, target_var, val);
    assert_eq!(assign.span, Span::new(0, 7));

    // Unsafe Block: unsafe { ... }
    let unsafe_expr = Expr::new(
        ExprKind::Unsafe(Block::empty(Span::new(7, 9))),
        Span::new(0, 9),
    );
    assert_eq!(unsafe_expr.span, Span::new(0, 9));

    // Sizeof: sizeof(int)
    let sizeof_expr = Expr::new(
        ExprKind::Sizeof(SizeofTarget::Type(Type::predefined(
            PredefinedType::Int,
            Span::new(7, 10),
        ))),
        Span::new(0, 11),
    );
    assert_eq!(sizeof_expr.span, Span::new(0, 11));

    // Template String: `Hello, ${name}!`
    let tmpl = Expr::new(
        ExprKind::TemplateString(vec![
            TemplateStringPart::String("Hello, ".to_string()),
            TemplateStringPart::Expr(Box::new(Expr::ident(Ident::new("name", Span::new(10, 14))))),
            TemplateStringPart::String("!".to_string()),
        ]),
        Span::new(0, 17),
    );
    if let ExprKind::TemplateString(parts) = tmpl.kind {
        assert_eq!(parts.len(), 3);
    } else {
        panic!("expected ExprKind::TemplateString");
    }
}

// =============================================================================
// Pattern Tests
// =============================================================================

#[test]
fn test_patterns() {
    // Wildcard: _
    let wildcard = Pattern::wildcard(Span::new(0, 1));
    assert_eq!(wildcard.kind, PatternKind::Wildcard);

    // Literal Pattern: -42
    let signed_lit = Pattern::signed_literal(
        PrefixUnaryOp::Minus,
        Literal::integer("42", 42, Radix::Decimal, Span::new(1, 3)),
        Span::new(0, 3),
    );
    if let PatternKind::Literal(lit_pat) = signed_lit.kind {
        assert_eq!(lit_pat.sign, Some(PrefixUnaryOp::Minus));
    } else {
        panic!("expected PatternKind::Literal");
    }

    // Var Pattern: let x
    let var_pat = Pattern::var(
        Ident::new("x", Span::new(4, 5)),
        Some(VarModifier::Let),
        Span::new(0, 5),
    );
    if let PatternKind::Var { modifier, name } = var_pat.kind {
        assert_eq!(modifier, Some(VarModifier::Let));
        assert_eq!(name.as_str(), "x");
    } else {
        panic!("expected PatternKind::Var");
    }

    // Struct Pattern: Point { x: 0, y: let y, ... }
    let struct_pat = Pattern::struct_pattern(
        TypeName::single(Ident::new("Point", Span::new(0, 5))),
        vec![
            StructFieldPattern::Field {
                name: Ident::new("x", Span::new(8, 9)),
                pattern: Pattern::literal(
                    Literal::integer("0", 0, Radix::Decimal, Span::new(11, 12)),
                    Span::new(11, 12),
                ),
                span: Span::new(8, 12),
            },
            StructFieldPattern::Binding {
                modifier: Some(VarModifier::Let),
                name: Ident::new("y", Span::new(18, 19)),
                span: Span::new(14, 19),
            },
            StructFieldPattern::Rest(Span::new(21, 24)),
        ],
        Span::new(0, 26),
    );
    if let PatternKind::Struct {
        struct_name,
        fields,
    } = struct_pat.kind
    {
        assert_eq!(struct_name.to_string(), "Point");
        assert_eq!(fields.len(), 3);
    } else {
        panic!("expected PatternKind::Struct");
    }

    // Variant Pattern: Option::Some(let val)
    let variant_pat = Pattern::variant(
        TypeName::new(
            vec![
                Ident::new("Option", Span::new(0, 6)),
                Ident::new("Some", Span::new(8, 12)),
            ],
            Span::new(0, 12),
        ),
        vec![Pattern::var(
            Ident::new("val", Span::new(17, 20)),
            Some(VarModifier::Let),
            Span::new(13, 20),
        )],
        Span::new(0, 21),
    );
    if let PatternKind::Variant {
        variant_name,
        payload,
    } = variant_pat.kind
    {
        assert_eq!(variant_name.to_string(), "Option.Some");
        assert_eq!(payload.len(), 1);
    } else {
        panic!("expected PatternKind::Variant");
    }

    // Tuple Pattern: [head, ...tail]
    let tuple_pat = Pattern::tuple(
        vec![
            TuplePatternElement::normal(Pattern::var(
                Ident::new("head", Span::new(1, 5)),
                None,
                Span::new(1, 5),
            )),
            TuplePatternElement::rest(
                Pattern::var(
                    Ident::new("tail", Span::new(10, 14)),
                    None,
                    Span::new(10, 14),
                ),
                Span::new(7, 14),
            ),
        ],
        Span::new(0, 15),
    );
    if let PatternKind::Tuple(elements) = tuple_pat.kind {
        assert_eq!(elements.len(), 2);
        assert!(elements[1].is_rest);
    } else {
        panic!("expected PatternKind::Tuple");
    }

    // Object Pattern: { x, y: target = 10, ...rest }
    let obj_target = BindingTarget::Object(vec![
        ObjectPatternProp::Property {
            name: Ident::new("x", Span::new(2, 3)),
            target: None,
            default: None,
            span: Span::new(2, 3),
        },
        ObjectPatternProp::Property {
            name: Ident::new("y", Span::new(5, 6)),
            target: Some(BindingTarget::Ident(Ident::new("target", Span::new(8, 14)))),
            default: Some(Box::new(Expr::literal(Literal::integer(
                "10",
                10,
                Radix::Decimal,
                Span::new(17, 19),
            )))),
            span: Span::new(5, 19),
        },
        ObjectPatternProp::Rest {
            target: Ident::new("rest", Span::new(24, 28)),
            span: Span::new(21, 28),
        },
    ]);
    assert_eq!(obj_target.span(), Span::new(2, 28));
}

// =============================================================================
// Statement Tests
// =============================================================================

#[test]
fn test_variable_and_resource_statements() {
    // let x: int = 10, y = 20;
    let decl1 = VariableDecl::new(
        BindingTarget::Ident(Ident::new("x", Span::new(4, 5))),
        Some(Type::predefined(PredefinedType::Int, Span::new(7, 10))),
        Some(Expr::literal(Literal::integer(
            "10",
            10,
            Radix::Decimal,
            Span::new(13, 15),
        ))),
        Span::new(4, 15),
    );
    let decl2 = VariableDecl::new(
        BindingTarget::Ident(Ident::new("y", Span::new(17, 18))),
        None,
        Some(Expr::literal(Literal::integer(
            "20",
            20,
            Radix::Decimal,
            Span::new(21, 23),
        ))),
        Span::new(17, 23),
    );
    let let_stmt = Stmt::variable(VarModifier::Let, vec![decl1, decl2], Span::new(0, 24));
    if let StmtKind::Variable(var_stmt) = let_stmt.kind {
        assert_eq!(var_stmt.modifier, VarModifier::Let);
        assert_eq!(var_stmt.declarations.len(), 2);
    } else {
        panic!("expected StmtKind::Variable");
    }

    // const [a, b] = tuple;
    let tuple_target = BindingTarget::Tuple(vec![
        TuplePatternElement::normal(Pattern::var(
            Ident::new("a", Span::new(7, 8)),
            None,
            Span::new(7, 8),
        )),
        TuplePatternElement::normal(Pattern::var(
            Ident::new("b", Span::new(10, 11)),
            None,
            Span::new(10, 11),
        )),
    ]);
    let const_destruct = Stmt::variable(
        VarModifier::Const,
        vec![VariableDecl::new(
            tuple_target,
            None,
            Some(Expr::ident(Ident::new("tuple", Span::new(16, 21)))),
            Span::new(6, 21),
        )],
        Span::new(0, 22),
    );
    if let StmtKind::Variable(var_stmt) = const_destruct.kind {
        assert_eq!(var_stmt.modifier, VarModifier::Const);
    } else {
        panic!("expected StmtKind::Variable");
    }

    // using file = openFile();
    let using_stmt = Stmt::using_stmt(
        Ident::new("file", Span::new(6, 10)),
        None,
        Expr::call(
            Expr::ident(Ident::new("openFile", Span::new(13, 21))),
            None,
            Vec::new(),
            Span::new(13, 23),
        ),
        Span::new(0, 24),
    );
    if let StmtKind::Using(u) = using_stmt.kind {
        assert_eq!(u.name.as_str(), "file");
    } else {
        panic!("expected StmtKind::Using");
    }
}

#[test]
fn test_control_flow_statements() {
    let cond = Expr::ident(Ident::new("condition", Span::new(4, 13)));
    let body = Stmt::block(Block::empty(Span::new(15, 17)));

    // if (cond) { ... } else { ... }
    let else_body = Stmt::block(Block::empty(Span::new(23, 25)));
    let if_stmt = Stmt::if_stmt(
        cond.clone(),
        body.clone(),
        Some(else_body),
        Span::new(0, 25),
    );
    if let StmtKind::If(i) = if_stmt.kind {
        assert!(i.else_branch.is_some());
    } else {
        panic!("expected StmtKind::If");
    }

    // while (cond) { ... }
    let while_stmt = Stmt::while_stmt(cond.clone(), body.clone(), Span::new(0, 17));
    if let StmtKind::While(w) = while_stmt.kind {
        assert_eq!(w.condition, cond);
    } else {
        panic!("expected StmtKind::While");
    }

    // do { ... } while (cond);
    let do_while = Stmt::do_while(body.clone(), cond.clone(), Span::new(0, 25));
    if let StmtKind::DoWhile(dw) = do_while.kind {
        assert_eq!(dw.condition, cond);
    } else {
        panic!("expected StmtKind::DoWhile");
    }

    // Standard for: for (let i = 0; i < 10; i += 1) { ... }
    let for_init = ForInit::Let(vec![VariableDecl::new(
        BindingTarget::Ident(Ident::new("i", Span::new(9, 10))),
        None,
        Some(Expr::literal(Literal::integer(
            "0",
            0,
            Radix::Decimal,
            Span::new(13, 14),
        ))),
        Span::new(9, 14),
    )]);
    let for_stmt = Stmt::for_standard(
        Some(for_init),
        Some(Expr::binary(
            BinaryOp::Lt,
            Expr::ident(Ident::new("i", Span::new(16, 17))),
            Expr::literal(Literal::integer(
                "10",
                10,
                Radix::Decimal,
                Span::new(20, 22),
            )),
        )),
        Some(vec![Expr::assign(
            AssignOp::AddAssign,
            Expr::ident(Ident::new("i", Span::new(24, 25))),
            Expr::literal(Literal::integer("1", 1, Radix::Decimal, Span::new(29, 30))),
        )]),
        body.clone(),
        Span::new(0, 35),
    );
    if let StmtKind::For(ForStmt::Standard {
        init, test, update, ..
    }) = for_stmt.kind
    {
        assert!(init.is_some());
        assert!(test.is_some());
        assert_eq!(update.unwrap().len(), 1);
    } else {
        panic!("expected Standard ForStmt");
    }

    // for (let item of array) { ... }
    let for_of = Stmt::for_iteration(
        VarModifier::Let,
        BindingTarget::Ident(Ident::new("item", Span::new(9, 13))),
        ForIterationKind::Of,
        Expr::ident(Ident::new("array", Span::new(17, 22))),
        body.clone(),
        Span::new(0, 27),
    );
    if let StmtKind::For(ForStmt::Iteration { kind, modifier, .. }) = for_of.kind {
        assert_eq!(kind, ForIterationKind::Of);
        assert_eq!(modifier, VarModifier::Let);
    } else {
        panic!("expected Iteration ForStmt");
    }
}

#[test]
fn test_switch_and_jump_statements() {
    // switch (val) { case 1: ... default: ... }
    let switch_target = Expr::ident(Ident::new("val", Span::new(8, 11)));
    let cases = vec![
        SwitchCase::Case {
            pattern: Pattern::literal(
                Literal::integer("1", 1, Radix::Decimal, Span::new(18, 19)),
                Span::new(18, 19),
            ),
            guard: None,
            body: vec![Stmt::break_stmt(None, Span::new(21, 27))],
            span: Span::new(13, 27),
        },
        SwitchCase::Default {
            body: vec![Stmt::return_stmt(None, Span::new(38, 45))],
            span: Span::new(29, 45),
        },
    ];
    let switch_stmt = Stmt::switch_stmt(switch_target, cases, Span::new(0, 48));
    if let StmtKind::Switch(s) = switch_stmt.kind {
        assert_eq!(s.cases.len(), 2);
    } else {
        panic!("expected StmtKind::Switch");
    }

    // Defer statement: defer cleanup();
    let defer_stmt = Stmt::defer_stmt(
        Stmt::expr(Expr::call(
            Expr::ident(Ident::new("cleanup", Span::new(6, 13))),
            None,
            Vec::new(),
            Span::new(6, 15),
        )),
        Span::new(0, 16),
    );
    if let StmtKind::Defer(d) = defer_stmt.kind {
        assert_eq!(d.span, Span::new(0, 16));
    } else {
        panic!("expected StmtKind::Defer");
    }

    // Unsafe statement: unsafe { ... }
    let unsafe_stmt = Stmt::unsafe_stmt(Block::empty(Span::new(7, 9)), Span::new(0, 9));
    assert_eq!(unsafe_stmt.span, Span::new(0, 9));

    // Jump statements: return, break, continue
    let ret = Stmt::return_stmt(
        Some(Expr::ident(Ident::new("res", Span::new(7, 10)))),
        Span::new(0, 11),
    );
    let brk = Stmt::break_stmt(
        Some(Ident::new("loopLabel", Span::new(6, 15))),
        Span::new(0, 16),
    );
    let cont = Stmt::continue_stmt(None, Span::new(0, 9));

    assert_eq!(ret.span, Span::new(0, 11));
    assert_eq!(brk.span, Span::new(0, 16));
    assert_eq!(cont.span, Span::new(0, 9));
}

// =============================================================================
// Declaration Tests
// =============================================================================

#[test]
fn test_function_and_struct_declarations() {
    // #[inline] async function fetch<T>(url: string, ...headers: string[]): Promise<T> { ... }
    let attr = Attribute::simple(Ident::new("inline", Span::new(2, 8)), Span::new(0, 9));
    let param1 = Parameter::simple(
        Ident::new("url", Span::new(26, 29)),
        Some(Type::predefined(PredefinedType::String, Span::new(31, 37))),
        Span::new(26, 37),
    );
    let rest = RestParameter::new(
        Ident::new("headers", Span::new(42, 49)),
        Some(Type::dynamic_array(
            Type::predefined(PredefinedType::String, Span::new(51, 57)),
            Span::new(51, 59),
        )),
        Span::new(39, 59),
    );
    let params = ParameterList::new(vec![param1], Some(rest), Span::new(25, 60));
    let t_param = TypeParameter::new(
        Ident::new("T", Span::new(23, 24)),
        None,
        None,
        Span::new(23, 24),
    );
    let type_params = TypeParameters::new(vec![t_param], Span::new(22, 25));

    let fn_decl = FunctionDecl::new(
        vec![attr],
        true,
        Ident::new("fetch", Span::new(16, 21)),
        Some(type_params),
        params,
        Some(ReturnType::Type(Type::reference(
            TypeName::single(Ident::new("Promise", Span::new(62, 69))),
            None,
            Span::new(62, 72),
        ))),
        Some(Block::empty(Span::new(73, 75))),
        Span::new(0, 75),
    );
    let decl = Decl::function(fn_decl);
    if let DeclKind::Function(f) = decl.kind {
        assert!(f.is_async);
        assert_eq!(f.attributes.len(), 1);
        assert_eq!(f.params.params.len(), 1);
        assert!(f.params.rest.is_some());
    } else {
        panic!("expected DeclKind::Function");
    }

    // #[repr(C)] struct Point { public readonly x: int; public constructor(x: int) { ... } }
    let repr_attr = Attribute::new(
        Ident::new("repr", Span::new(2, 6)),
        vec![AttributeArg::new(
            None,
            Expr::ident(Ident::new("C", Span::new(7, 8))),
            Span::new(7, 8),
        )],
        Span::new(0, 9),
    );
    let field = StructMember::Field(StructField {
        accessibility: Some(Accessibility::Public),
        is_static: false,
        is_readonly: true,
        name: Ident::new("x", Span::new(28, 29)),
        ty: Some(Type::predefined(PredefinedType::Int, Span::new(31, 34))),
        init: None,
        span: Span::new(12, 35),
    });
    let ctor = StructMember::Constructor(ConstructorDecl {
        accessibility: Some(Accessibility::Public),
        params: ParameterList::new(
            vec![Parameter::simple(
                Ident::new("x", Span::new(56, 57)),
                Some(Type::predefined(PredefinedType::Int, Span::new(59, 62))),
                Span::new(56, 62),
            )],
            None,
            Span::new(55, 63),
        ),
        body: Block::empty(Span::new(64, 66)),
        span: Span::new(36, 66),
    });
    let method = StructMember::Method(MethodDecl {
        accessibility: Some(Accessibility::Public),
        is_static: false,
        is_async: false,
        name: Ident::new("getX", Span::new(75, 79)),
        type_parameters: None,
        params: ParameterList::empty(Span::new(79, 81)),
        return_type: Some(ReturnType::Type(Type::predefined(
            PredefinedType::Int,
            Span::new(83, 86),
        ))),
        body: Block::empty(Span::new(87, 89)),
        span: Span::new(68, 89),
    });
    let struct_decl = StructDecl::new(
        vec![repr_attr],
        Ident::new("Point", Span::new(17, 22)),
        None,
        vec![field, ctor, method],
        Span::new(0, 91),
    );
    let decl_struct = Decl::struct_decl(struct_decl);
    if let DeclKind::Struct(s) = decl_struct.kind {
        assert_eq!(s.name.as_str(), "Point");
        assert_eq!(s.members.len(), 3);
    } else {
        panic!("expected DeclKind::Struct");
    }
}

#[test]
fn test_class_and_interface_declarations() {
    // class Node extends Base implements Disposable { private #id: int; get id(): int { ... } }
    let field = ClassMember::Field(ClassField {
        accessibility: Some(Accessibility::Private),
        is_static: false,
        is_readonly: false,
        name: ClassElementName::Private(PrivateIdent::new("#id", Span::new(57, 60))),
        ty: Some(Type::predefined(PredefinedType::Int, Span::new(62, 65))),
        init: None,
        span: Span::new(49, 66),
    });
    let getter = ClassMember::GetterSetter(GetterSetterDecl {
        accessibility: Some(Accessibility::Public),
        is_static: false,
        kind: AccessorKind::Get,
        name: ClassElementName::Ident(Ident::new("id", Span::new(76, 78))),
        params: ParameterList::empty(Span::new(78, 80)),
        return_type: Some(Type::predefined(PredefinedType::Int, Span::new(82, 85))),
        body: Block::empty(Span::new(86, 88)),
        span: Span::new(67, 88),
    });
    let method = ClassMember::Method(ClassMethod {
        accessibility: Some(Accessibility::Public),
        is_static: false,
        is_async: false,
        name: ClassElementName::Ident(Ident::new("dispose", Span::new(96, 103))),
        type_parameters: None,
        params: ParameterList::empty(Span::new(103, 105)),
        return_type: Some(ReturnType::Type(Type::predefined(
            PredefinedType::Void,
            Span::new(107, 111),
        ))),
        body: Block::empty(Span::new(112, 114)),
        span: Span::new(89, 114),
    });
    let class_decl = ClassDecl::new(
        Vec::new(),
        ClassElementName::Ident(Ident::new("Node", Span::new(6, 10))),
        None,
        Some(TypeReference::new(
            TypeName::single(Ident::new("Base", Span::new(19, 23))),
            None,
            Span::new(19, 23),
        )),
        vec![TypeReference::new(
            TypeName::single(Ident::new("Disposable", Span::new(35, 45))),
            None,
            Span::new(35, 45),
        )],
        vec![field, getter, method],
        Span::new(0, 116),
    );
    let decl_class = Decl::class_decl(class_decl);
    if let DeclKind::Class(c) = decl_class.kind {
        assert!(c.extends.is_some());
        assert_eq!(c.implements.len(), 1);
        assert_eq!(c.members.len(), 3);
    } else {
        panic!("expected DeclKind::Class");
    }

    // interface Collection { readonly count: int; [key: string]: int; clear(): void; }
    let iface_prop = InterfaceMember::Property(InterfaceProperty {
        is_readonly: true,
        name: Ident::new("count", Span::new(32, 37)),
        is_optional: false,
        ty: Type::predefined(PredefinedType::Int, Span::new(39, 42)),
        span: Span::new(23, 43),
    });
    let iface_index = InterfaceMember::Index(IndexSignature {
        name: Ident::new("key", Span::new(45, 48)),
        key_type: PredefinedType::String,
        value_type: Type::predefined(PredefinedType::Int, Span::new(58, 61)),
        span: Span::new(44, 62),
    });
    let iface_method = InterfaceMember::Method(InterfaceMethod {
        name: Ident::new("clear", Span::new(64, 69)),
        is_optional: false,
        type_parameters: None,
        params: ParameterList::empty(Span::new(69, 71)),
        return_type: ReturnType::Type(Type::predefined(PredefinedType::Void, Span::new(73, 77))),
        span: Span::new(64, 78),
    });
    let iface_decl = InterfaceDecl::new(
        Ident::new("Collection", Span::new(10, 20)),
        None,
        Vec::new(),
        vec![iface_prop, iface_index, iface_method],
        Span::new(0, 80),
    );
    let decl_iface = Decl::interface_decl(iface_decl);
    if let DeclKind::Interface(i) = decl_iface.kind {
        assert_eq!(i.name.as_str(), "Collection");
        assert_eq!(i.members.len(), 3);
    } else {
        panic!("expected DeclKind::Interface");
    }
}

#[test]
fn test_enum_type_alias_and_ffi_declarations() {
    // enum Event { KeyPress(char), Quit = 0 }
    let v1 = EnumVariant::Tuple {
        name: Ident::new("KeyPress", Span::new(13, 21)),
        payload: ParameterList::new(
            vec![Parameter::simple(
                Ident::new("ch", Span::new(22, 24)),
                Some(Type::predefined(PredefinedType::Char, Span::new(26, 30))),
                Span::new(22, 30),
            )],
            None,
            Span::new(21, 31),
        ),
        span: Span::new(13, 31),
    };
    let v2 = EnumVariant::Value {
        name: Ident::new("Quit", Span::new(33, 37)),
        value: Some(Expr::literal(Literal::integer(
            "0",
            0,
            Radix::Decimal,
            Span::new(40, 41),
        ))),
        span: Span::new(33, 41),
    };
    let enum_decl = EnumDecl::new(
        Vec::new(),
        Ident::new("Event", Span::new(5, 10)),
        None,
        None,
        vec![v1, v2],
        Span::new(0, 43),
    );
    let decl_enum = Decl::enum_decl(enum_decl);
    if let DeclKind::Enum(e) = decl_enum.kind {
        assert_eq!(e.name.as_str(), "Event");
        assert_eq!(e.variants.len(), 2);
    } else {
        panic!("expected DeclKind::Enum");
    }

    // type Handler<T> = (event: T) => void;
    let type_alias = TypeAliasDecl::new(
        Vec::new(),
        Ident::new("Handler", Span::new(5, 12)),
        Some(TypeParameters::new(
            vec![TypeParameter::new(
                Ident::new("T", Span::new(13, 14)),
                None,
                None,
                Span::new(13, 14),
            )],
            Span::new(12, 15),
        )),
        Type::predefined(PredefinedType::Void, Span::new(33, 37)),
        Span::new(0, 38),
    );
    let decl_alias = Decl::type_alias(type_alias);
    if let DeclKind::TypeAlias(t) = decl_alias.kind {
        assert_eq!(t.name.as_str(), "Handler");
    } else {
        panic!("expected DeclKind::TypeAlias");
    }

    // extern "C" { function puts(s: rawptr): int; }
    let extern_func = ExternMember::Function(ExternFunctionDecl {
        attributes: Vec::new(),
        name: Ident::new("puts", Span::new(24, 28)),
        type_parameters: None,
        params: ExternParameterList::new(
            vec![Parameter::simple(
                Ident::new("s", Span::new(29, 30)),
                Some(Type::predefined(PredefinedType::Rawptr, Span::new(32, 38))),
                Span::new(29, 38),
            )],
            false,
            None,
            Span::new(28, 39),
        ),
        return_type: Some(Type::predefined(PredefinedType::Int, Span::new(41, 44))),
        span: Span::new(15, 45),
    });
    let extern_block = ExternBlockDecl::new(
        Vec::new(),
        StringLiteral::new("C", "\"C\""),
        vec![extern_func],
        Span::new(0, 47),
    );
    let decl_extern = Decl::extern_block(extern_block);
    if let DeclKind::ExternBlock(eb) = decl_extern.kind {
        assert_eq!(eb.abi.value, "C");
        assert_eq!(eb.members.len(), 1);
    } else {
        panic!("expected DeclKind::ExternBlock");
    }

    // extern "C" function printf(fmt: rawptr, ...): int;
    let single_extern = SingleExternDecl::new(
        Vec::new(),
        StringLiteral::new("C", "\"C\""),
        Ident::new("printf", Span::new(20, 26)),
        None,
        ExternParameterList::new(
            vec![Parameter::simple(
                Ident::new("fmt", Span::new(27, 30)),
                Some(Type::predefined(PredefinedType::Rawptr, Span::new(32, 38))),
                Span::new(27, 38),
            )],
            true,
            None,
            Span::new(26, 43),
        ),
        Some(Type::predefined(PredefinedType::Int, Span::new(46, 49))),
        None,
        Span::new(0, 50),
    );
    let decl_single_extern = Decl::single_extern(single_extern);
    if let DeclKind::SingleExtern(se) = decl_single_extern.kind {
        assert!(se.params.is_variadic);
        assert_eq!(se.name.as_str(), "printf");
    } else {
        panic!("expected DeclKind::SingleExtern");
    }
}
