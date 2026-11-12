// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pattern matching AST representations.

use crate::{Ident, Literal, Span, TypeName};

/// A field pattern in a struct pattern: `{ x: pattern, y, ... }`.
#[derive(Debug, Clone, PartialEq)]
pub enum StructFieldPattern {
    Field {
        name: Ident,
        pattern: Option<Pattern>,
        span: Span,
    },
    Rest(Span),
}

/// A property pattern in an object pattern: `{ x: pattern = default, ...rest }`.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectPatternProp {
    Prop {
        name: Ident,
        target: Option<Pattern>,
        default: Option<Box<crate::Expr>>,
        span: Span,
    },
    Rest {
        target: Ident,
        span: Span,
    },
}

/// An element in a tuple pattern: `[p1, ...rest, p2]`.
#[derive(Debug, Clone, PartialEq)]
pub struct TuplePatternElement {
    pub is_rest: bool,
    pub pattern: Pattern,
    pub span: Span,
}

/// Specific variant of a destructuring or match pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    /// Wildcard / discard: `_`.
    Wildcard,
    /// Literal pattern: `42`, `"hello"`, `true`, `null`.
    Literal(Literal),
    /// Variable binding pattern: `x`, `let x`, `const x`.
    Binding { is_const: bool, name: Ident },
    /// Struct pattern: `Point { x, y }`.
    Struct {
        struct_name: TypeName,
        fields: Vec<StructFieldPattern>,
    },
    /// Variant pattern for tagged unions/enums: `Option::Some(x)`, `None`.
    Variant {
        variant_name: TypeName,
        payload: Vec<Pattern>,
    },
    /// Tuple pattern: `[a, b, ...rest]`.
    Tuple(Vec<TuplePatternElement>),
    /// Object pattern: `{ a, b: c = 10, ...rest }`.
    Object(Vec<ObjectPatternProp>),
}

/// A complete pattern node carrying its kind and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    pub fn new(kind: PatternKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn wildcard(span: Span) -> Self {
        Self::new(PatternKind::Wildcard, span)
    }

    pub fn literal(lit: Literal, span: Span) -> Self {
        Self::new(PatternKind::Literal(lit), span)
    }

    pub fn binding(name: Ident, is_const: bool, span: Span) -> Self {
        Self::new(PatternKind::Binding { is_const, name }, span)
    }
}
