// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Unified tests for Kyanos AST nodes and data structures.

use kyanos_ast::{
    AnonymousStructField, ArraySize, AssignOp, Associativity, BinaryOp, FunctionType,
    FunctionTypeParam, Ident, Literal, PostfixUnaryOp, PredefinedType, PrefixUnaryOp, PrivateIdent,
    Radix, ReturnType, Span, Type, TypeArguments, TypeKind, TypeName, TypeParameter,
    TypeParameters, TypePredicate, TypeReference,
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
