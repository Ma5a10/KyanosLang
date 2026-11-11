// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Unified tests for Kyanos AST nodes and data structures.

use kyanos_ast::{
    AssignOp, Associativity, BinaryOp, Ident, Literal, PostfixUnaryOp, PrefixUnaryOp, PrivateIdent,
    Radix, Span,
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

