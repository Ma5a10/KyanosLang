// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Expression AST representations for Kyanos.

use crate::{
    pattern::Pattern, stmt::Block, AssignOp, BinaryOp, Ident, Literal, NodeId, ParameterList,
    PostfixUnaryOp, PredefinedType, PrefixUnaryOp, PrivateIdent, ReturnType, Span, StringLiteral,
    Type, TypeArguments, TypeName, TypeParameters,
};

/// An argument passed to a function or constructor call.
#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub is_spread: bool,
    pub expr: Expr,
    pub span: Span,
}

impl Argument {
    pub fn new(is_spread: bool, expr: Expr, span: Span) -> Self {
        Self {
            is_spread,
            expr,
            span,
        }
    }

    pub fn normal(expr: Expr) -> Self {
        let span = expr.span;
        Self::new(false, expr, span)
    }

    pub fn spread(expr: Expr, span: Span) -> Self {
        Self::new(true, expr, span)
    }
}

/// An element in an array literal: `[1, ...items, 3]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayElement {
    pub is_spread: bool,
    pub expr: Expr,
    pub span: Span,
}

impl ArrayElement {
    pub fn new(is_spread: bool, expr: Expr, span: Span) -> Self {
        Self {
            is_spread,
            expr,
            span,
        }
    }

    pub fn normal(expr: Expr) -> Self {
        let span = expr.span;
        Self::new(false, expr, span)
    }

    pub fn spread(expr: Expr, span: Span) -> Self {
        Self::new(true, expr, span)
    }
}

/// Property key name in an object literal.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyName {
    Ident(Ident),
    String(StringLiteral),
    Numeric(Literal),
    Computed(Box<Expr>),
}

impl PropertyName {
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(id) => id.span,
            Self::String(s) => s.span,
            Self::Numeric(n) => n.span,
            Self::Computed(e) => e.span,
        }
    }
}

/// A method defined directly inside an object literal.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMethod {
    pub is_async: bool,
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub params: ParameterList,
    pub return_type: Option<ReturnType>,
    pub body: Block,
    pub span: Span,
}

/// A member inside an object literal `{ x: 10, ...rest, method() {} }`.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectMember {
    Property {
        name: PropertyName,
        value: Expr,
        span: Span,
    },
    Shorthand {
        name: Ident,
        span: Span,
    },
    Spread {
        expr: Expr,
        span: Span,
    },
    Method(ObjectMethod),
}

/// Struct instantiation field initializer: `Point { x: 10, y, ...rest }`.
#[derive(Debug, Clone, PartialEq)]
pub enum StructFieldInit {
    Field {
        name: Ident,
        value: Option<Expr>,
        span: Span,
    },
    Spread {
        expr: Expr,
        span: Span,
    },
}

/// Body of an arrow function: either a concise expression or a full block.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowBody {
    Expr(Box<Expr>),
    Block(Block),
}

impl ArrowBody {
    pub fn span(&self) -> Span {
        match self {
            Self::Expr(e) => e.span,
            Self::Block(b) => b.span,
        }
    }
}

/// Arm in a `switch` expression: `case pattern when guard => expr` or `default => expr`.
#[derive(Debug, Clone, PartialEq)]
pub enum SwitchExprArm {
    Case {
        pattern: Pattern,
        guard: Option<Box<Expr>>,
        body: Box<Expr>,
        span: Span,
    },
    Default {
        body: Box<Expr>,
        span: Span,
    },
}

impl SwitchExprArm {
    pub fn span(&self) -> Span {
        match self {
            Self::Case { span, .. } | Self::Default { span, .. } => *span,
        }
    }
}

/// Target of a `sizeof(...)` expression.
#[derive(Debug, Clone, PartialEq)]
pub enum SizeofTarget {
    Type(Type),
    Expr(Box<Expr>),
}

/// Member property for dot access (`obj.prop` or `obj.#privateProp`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemberProperty {
    Ident(Ident),
    Private(PrivateIdent),
}

impl MemberProperty {
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(id) => id.span,
            Self::Private(pid) => pid.span,
        }
    }
}

/// Part in a template string (`Hello, ${name}!`).
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateStringPart {
    String(String),
    Expr(Box<Expr>),
}

/// Specific variant of an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    // --- Primary Expressions ---
    /// Literal expression (e.g. `42`, `"hello"`, `true`, `null`).
    Literal(Literal),
    /// Variable or identifier reference (e.g. `foo`).
    Ident(Ident),
    /// The `this` self-reference.
    This,
    /// The `super` base-class reference.
    Super,
    /// Parenthesized expression: `(expr)`.
    Parenthesized(Box<Expr>),
    /// Comma-separated expression sequence: `(a, b, c)`.
    Sequence(Vec<Expr>),
    /// Array literal: `[1, 2, ...rest]`.
    Array(Vec<ArrayElement>),
    /// Object literal: `{ x: 1, "key": val, ...rest }`.
    Object(Vec<ObjectMember>),
    /// Struct instantiation: `Point { x: 10, y: 20 }`.
    StructInstantiation {
        struct_name: TypeName,
        type_arguments: Option<TypeArguments>,
        fields: Vec<StructFieldInit>,
    },
    /// Arrow function / lambda: `(x: int): int => x + 1`.
    ArrowFunction {
        is_async: bool,
        type_parameters: Option<TypeParameters>,
        params: ParameterList,
        return_type: Option<ReturnType>,
        body: ArrowBody,
    },
    /// Switch expression: `switch (val) { case 1 => "a", default => "b" }`.
    Switch {
        target: Box<Expr>,
        arms: Vec<SwitchExprArm>,
    },
    /// Unsafe block expression: `unsafe { ... }`.
    Unsafe(Block),
    /// Sizeof query: `sizeof(T)` or `sizeof(expr)`.
    Sizeof(SizeofTarget),
    /// Predefined primitive type static member access: `int.MAX_VALUE`.
    PredefinedTypeMember { ty: PredefinedType, member: Ident },
    /// Template string: `` `Hello, ${name}!` ``.
    TemplateString(Vec<TemplateStringPart>),

    // --- Postfix Expressions ---
    /// Index access: `arr[i]` or `optArr?.[i]`.
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
        is_optional: bool,
    },
    /// Member dot or optional-chain access: `obj.prop`, `obj.#privateProp`, `obj?.prop`, `obj?.#privateProp`.
    Member {
        target: Box<Expr>,
        property: MemberProperty,
        is_optional: bool,
    },
    /// Function or method call: `f(a, b)` or `f<T, U>(a, b)`.
    Call {
        callee: Box<Expr>,
        type_arguments: Option<TypeArguments>,
        args: Vec<Argument>,
    },
    /// Postfix increment/decrement/non-null assertion: `x++`, `x--`, `x!`.
    UnaryPostfix {
        op: PostfixUnaryOp,
        target: Box<Expr>,
    },

    // --- Prefix Unary Expressions ---
    /// Prefix unary operator: `+x`, `-x`, `!x`, `~x`, `++x`, `*ptr`, `&val`, `try expr`, `await expr`, etc.
    UnaryPrefix {
        op: PrefixUnaryOp,
        target: Box<Expr>,
    },
    /// Object allocation: `new ClassName<T>(arg1, arg2)`.
    New {
        callee: Box<Expr>,
        type_arguments: Option<TypeArguments>,
        args: Option<Vec<Argument>>,
    },

    // --- Type Operators ---
    /// Explicit type cast: `expr as Type`.
    Cast { expr: Box<Expr>, ty: Type },
    /// Runtime type test: `expr is Type`.
    TypeTest { expr: Box<Expr>, ty: Type },

    // --- Binary & Ternary Expressions ---
    /// Binary operator expression: `a + b`, `a && b`, `a ?? b`.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Conditional ternary operator: `cond ? then_expr : else_expr`.
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    // --- Assignment Expressions ---
    /// Assignment and compound assignment: `a = b`, `a += b`.
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },

    // --- Error Recovery ---
    /// Placeholder produced by fault-tolerant error recovery in the parser.
    Error,
}

/// A complete expression node carrying its node ID, kind, and span.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self {
            id: NodeId::DUMMY,
            kind,
            span,
        }
    }

    /// Assigns an explicit `NodeId` to this expression node.
    pub const fn with_id(mut self, id: NodeId) -> Self {
        self.id = id;
        self
    }

    /// Constructs a synthetic error recovery expression node.
    pub fn error(span: Span) -> Self {
        Self::new(ExprKind::Error, span)
    }

    pub fn literal(lit: Literal) -> Self {
        let span = lit.span;
        Self::new(ExprKind::Literal(lit), span)
    }

    pub fn ident(id: Ident) -> Self {
        let span = id.span;
        Self::new(ExprKind::Ident(id), span)
    }

    pub fn this(span: Span) -> Self {
        Self::new(ExprKind::This, span)
    }

    pub fn super_kw(span: Span) -> Self {
        Self::new(ExprKind::Super, span)
    }

    pub fn binary(op: BinaryOp, left: Expr, right: Expr) -> Self {
        let span = left.span.merge(right.span);
        Self::new(
            ExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        )
    }

    pub fn unary_prefix(op: PrefixUnaryOp, target: Expr, span: Span) -> Self {
        Self::new(
            ExprKind::UnaryPrefix {
                op,
                target: Box::new(target),
            },
            span,
        )
    }

    pub fn unary_postfix(op: PostfixUnaryOp, target: Expr, span: Span) -> Self {
        Self::new(
            ExprKind::UnaryPostfix {
                op,
                target: Box::new(target),
            },
            span,
        )
    }

    pub fn call(
        callee: Expr,
        type_arguments: Option<TypeArguments>,
        args: Vec<Argument>,
        span: Span,
    ) -> Self {
        Self::new(
            ExprKind::Call {
                callee: Box::new(callee),
                type_arguments,
                args,
            },
            span,
        )
    }

    pub fn member(target: Expr, property: MemberProperty, is_optional: bool, span: Span) -> Self {
        Self::new(
            ExprKind::Member {
                target: Box::new(target),
                property,
                is_optional,
            },
            span,
        )
    }

    pub fn index(target: Expr, index: Expr, is_optional: bool, span: Span) -> Self {
        Self::new(
            ExprKind::Index {
                target: Box::new(target),
                index: Box::new(index),
                is_optional,
            },
            span,
        )
    }

    pub fn assign(op: AssignOp, target: Expr, value: Expr) -> Self {
        let span = target.span.merge(value.span);
        Self::new(
            ExprKind::Assign {
                op,
                target: Box::new(target),
                value: Box::new(value),
            },
            span,
        )
    }

    pub fn cast(expr: Expr, ty: Type, span: Span) -> Self {
        Self::new(
            ExprKind::Cast {
                expr: Box::new(expr),
                ty,
            },
            span,
        )
    }

    pub fn ternary(condition: Expr, then_expr: Expr, else_expr: Expr) -> Self {
        let span = condition.span.merge(else_expr.span);
        Self::new(
            ExprKind::Ternary {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            },
            span,
        )
    }
}
