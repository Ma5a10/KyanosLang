// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Statement AST representations.

use crate::{Expr, Span};

/// A block of statements enclosed in braces: `{ stmt1; stmt2; }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

impl Block {
    pub fn new(statements: Vec<Stmt>, span: Span) -> Self {
        Self { statements, span }
    }

    pub fn empty(span: Span) -> Self {
        Self {
            statements: Vec::new(),
            span,
        }
    }
}

/// Specific variant of a statement.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// An expression evaluated as a statement: `expr;`.
    Expr(Expr),
    /// A nested block of statements: `{ ... }`.
    Block(Block),
    /// An empty statement (e.g. redundant semicolon): `;`.
    Empty,
}

/// A complete statement node carrying its kind and span.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn expr(expr: Expr) -> Self {
        let span = expr.span;
        Self::new(StmtKind::Expr(expr), span)
    }

    pub fn block(block: Block) -> Self {
        let span = block.span;
        Self::new(StmtKind::Block(block), span)
    }

    pub fn empty(span: Span) -> Self {
        Self::new(StmtKind::Empty, span)
    }
}

