// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pattern matching and destructuring AST representations.

use crate::{Expr, Ident, Literal, PrefixUnaryOp, Span, TypeName};

/// Variable declaration modifier (`let` or `const`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarModifier {
    Let,
    Const,
}

impl VarModifier {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Let => "let",
            Self::Const => "const",
        }
    }
}

/// A target binding used in variable declarations and destructuring:
/// `Identifier | Underscore | tuplePattern | objectPattern`.
#[derive(Debug, Clone, PartialEq)]
pub enum BindingTarget {
    /// Single variable binding: `x`.
    Ident(Ident),
    /// Wildcard / discard: `_`.
    Wildcard(Span),
    /// Tuple destructuring: `[a, b, ...rest]`.
    Tuple(Vec<TuplePatternElement>),
    /// Object destructuring: `{ a, b: c = 10, ...rest }`.
    Object(Vec<ObjectPatternProp>),
}

impl BindingTarget {
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(id) => id.span,
            Self::Wildcard(span) => *span,
            Self::Tuple(elems) => {
                if let (Some(first), Some(last)) = (elems.first(), elems.last()) {
                    first.span.merge(last.span)
                } else {
                    Span::default()
                }
            }
            Self::Object(props) => {
                if let (Some(first), Some(last)) = (props.first(), props.last()) {
                    first.span().merge(last.span())
                } else {
                    Span::default()
                }
            }
        }
    }
}

/// Property pattern in object destructuring: `{ x: target = default, ...rest }`.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectPatternProp {
    Property {
        name: Ident,
        target: Option<BindingTarget>,
        default: Option<Box<Expr>>,
        span: Span,
    },
    Rest {
        target: Ident,
        span: Span,
    },
}

impl ObjectPatternProp {
    pub fn span(&self) -> Span {
        match self {
            Self::Property { span, .. } | Self::Rest { span, .. } => *span,
        }
    }
}

/// Element in a tuple pattern: `[p1, ...rest, p2]`.
#[derive(Debug, Clone, PartialEq)]
pub struct TuplePatternElement {
    pub is_rest: bool,
    pub pattern: Pattern,
    pub span: Span,
}

impl TuplePatternElement {
    pub fn new(is_rest: bool, pattern: Pattern, span: Span) -> Self {
        Self {
            is_rest,
            pattern,
            span,
        }
    }

    pub fn normal(pattern: Pattern) -> Self {
        let span = pattern.span;
        Self::new(false, pattern, span)
    }

    pub fn rest(pattern: Pattern, span: Span) -> Self {
        Self::new(true, pattern, span)
    }
}

/// A field pattern inside a struct destructuring pattern: `Point { x: 10, y, ... }`.
#[derive(Debug, Clone, PartialEq)]
pub enum StructFieldPattern {
    /// Explicit field match: `field: pattern`.
    Field {
        name: Ident,
        pattern: Pattern,
        span: Span,
    },
    /// Field binding shorthand with optional modifier: `let x`, `const x`, or `x`.
    Binding {
        modifier: Option<VarModifier>,
        name: Ident,
        span: Span,
    },
    /// Rest placeholder: `...`.
    Rest(Span),
}

impl StructFieldPattern {
    pub fn span(&self) -> Span {
        match self {
            Self::Field { span, .. } | Self::Binding { span, .. } | Self::Rest(span) => *span,
        }
    }
}

/// Literal pattern with optional sign: `+1`, `-3.14`, `"ok"`, `true`, `null`.
#[derive(Debug, Clone, PartialEq)]
pub struct LiteralPattern {
    pub sign: Option<PrefixUnaryOp>,
    pub literal: Literal,
    pub span: Span,
}

impl LiteralPattern {
    pub fn new(sign: Option<PrefixUnaryOp>, literal: Literal, span: Span) -> Self {
        Self {
            sign,
            literal,
            span,
        }
    }

    pub fn unsigned(literal: Literal) -> Self {
        let span = literal.span;
        Self::new(None, literal, span)
    }
}

/// Specific variant of a pattern used in `switch` and destructuring.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    /// Wildcard placeholder `_` matching any value without binding.
    Wildcard,
    /// Literal match: `42`, `-1`, `"hello"`, `true`, `null`.
    Literal(LiteralPattern),
    /// Variable binding pattern with optional modifier: `let x`, `const x`, or `x`.
    Var {
        modifier: Option<VarModifier>,
        name: Ident,
    },
    /// Struct pattern: `Point { x: 0, y: let y, ... }`.
    Struct {
        struct_name: TypeName,
        fields: Vec<StructFieldPattern>,
    },
    /// Variant pattern for tagged unions/ADTs: `Ok(let v)`, `KeyPress('q')`, `None`.
    Variant {
        variant_name: TypeName,
        payload: Vec<Pattern>,
    },
    /// Tuple pattern: `[a, ...rest, b]`.
    Tuple(Vec<TuplePatternElement>),
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
        Self::new(PatternKind::Literal(LiteralPattern::unsigned(lit)), span)
    }

    pub fn signed_literal(sign: PrefixUnaryOp, lit: Literal, span: Span) -> Self {
        Self::new(
            PatternKind::Literal(LiteralPattern::new(Some(sign), lit, span)),
            span,
        )
    }

    pub fn var(name: Ident, modifier: Option<VarModifier>, span: Span) -> Self {
        Self::new(PatternKind::Var { modifier, name }, span)
    }

    pub fn struct_pattern(
        struct_name: TypeName,
        fields: Vec<StructFieldPattern>,
        span: Span,
    ) -> Self {
        Self::new(
            PatternKind::Struct {
                struct_name,
                fields,
            },
            span,
        )
    }

    pub fn variant(variant_name: TypeName, payload: Vec<Pattern>, span: Span) -> Self {
        Self::new(
            PatternKind::Variant {
                variant_name,
                payload,
            },
            span,
        )
    }

    pub fn tuple(elements: Vec<TuplePatternElement>, span: Span) -> Self {
        Self::new(PatternKind::Tuple(elements), span)
    }
}
