// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pattern matching and destructuring AST representations.

use std::fmt;

use crate::{Expr, Ident, Literal, NodeId, Span, TypeName};

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
    Tuple {
        elements: Vec<TuplePatternElement>,
        span: Span,
    },
    /// Object destructuring: `{ a, b: c = 10, ...rest }`.
    Object {
        props: Vec<ObjectPatternProp>,
        span: Span,
    },
}

impl BindingTarget {
    pub fn ident(id: Ident) -> Self {
        Self::Ident(id)
    }

    pub fn wildcard(span: Span) -> Self {
        Self::Wildcard(span)
    }

    pub fn tuple(elements: Vec<TuplePatternElement>, span: Span) -> Self {
        Self::Tuple { elements, span }
    }

    pub fn object(props: Vec<ObjectPatternProp>, span: Span) -> Self {
        Self::Object { props, span }
    }

    pub fn span(&self) -> Span {
        match self {
            Self::Ident(id) => id.span,
            Self::Wildcard(span) | Self::Tuple { span, .. } | Self::Object { span, .. } => *span,
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

/// Sign prefix allowed on numeric literal patterns (`+` or `-`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralSign {
    Plus,
    Minus,
}

impl LiteralSign {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
        }
    }
}

impl fmt::Display for LiteralSign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Literal pattern with optional sign: `+1`, `-3.14`, `"ok"`, `true`, `null`.
#[derive(Debug, Clone, PartialEq)]
pub struct LiteralPattern {
    pub sign: Option<LiteralSign>,
    pub literal: Literal,
    pub span: Span,
}

impl LiteralPattern {
    pub fn new(sign: Option<LiteralSign>, literal: Literal, span: Span) -> Self {
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
    /// Object / anonymous struct pattern: `{ x: 0, y: let y, ...rest }`.
    Object(Vec<ObjectPatternProp>),
    /// Placeholder produced by fault-tolerant error recovery in the parser.
    Error,
}

/// A complete pattern node carrying its node ID, kind, and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub id: NodeId,
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    pub fn new(kind: PatternKind, span: Span) -> Self {
        Self {
            id: NodeId::DUMMY,
            kind,
            span,
        }
    }

    /// Assigns an explicit `NodeId` to this pattern node.
    pub const fn with_id(mut self, id: NodeId) -> Self {
        self.id = id;
        self
    }

    /// Constructs a synthetic error recovery pattern node.
    pub fn error(span: Span) -> Self {
        Self::new(PatternKind::Error, span)
    }

    pub fn wildcard(span: Span) -> Self {
        Self::new(PatternKind::Wildcard, span)
    }

    pub fn literal(lit: Literal, span: Span) -> Self {
        Self::new(PatternKind::Literal(LiteralPattern::unsigned(lit)), span)
    }

    pub fn signed_literal(sign: LiteralSign, lit: Literal, span: Span) -> Self {
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

    pub fn object(props: Vec<ObjectPatternProp>, span: Span) -> Self {
        Self::new(PatternKind::Object(props), span)
    }
}
