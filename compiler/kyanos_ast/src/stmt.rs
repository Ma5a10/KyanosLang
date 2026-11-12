// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Statement AST representations for Kyanos.

use crate::{
    pattern::{BindingTarget, Pattern, VarModifier},
    Expr, Ident, Span, Type,
};

/// A single variable binding in a `let` or `const` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub target: BindingTarget,
    pub ty: Option<Type>,
    pub init: Option<Expr>,
    pub span: Span,
}

impl VariableDecl {
    pub fn new(target: BindingTarget, ty: Option<Type>, init: Option<Expr>, span: Span) -> Self {
        Self {
            target,
            ty,
            init,
            span,
        }
    }
}

/// Variable declaration statement: `let x: int = 1, y = 2;` or `const [a, b] = tuple;`.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableStmt {
    pub modifier: VarModifier,
    pub declarations: Vec<VariableDecl>,
    pub span: Span,
}

impl VariableStmt {
    pub fn new(modifier: VarModifier, declarations: Vec<VariableDecl>, span: Span) -> Self {
        Self {
            modifier,
            declarations,
            span,
        }
    }
}

/// Resource-bound declaration: `using file = openFile("data.txt");`.
/// Automatically calls `.dispose()` upon scope exit via RAII.
#[derive(Debug, Clone, PartialEq)]
pub struct UsingStmt {
    pub name: Ident,
    pub ty: Option<Type>,
    pub init: Expr,
    pub span: Span,
}

impl UsingStmt {
    pub fn new(name: Ident, ty: Option<Type>, init: Expr, span: Span) -> Self {
        Self {
            name,
            ty,
            init,
            span,
        }
    }
}

/// Conditional branching statement: `if (cond) { ... } else { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Box<Stmt>,
    pub else_branch: Option<Box<Stmt>>,
    pub span: Span,
}

impl IfStmt {
    pub fn new(condition: Expr, then_branch: Stmt, else_branch: Option<Stmt>, span: Span) -> Self {
        Self {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
            span,
        }
    }
}

/// Do-while loop statement: `do { ... } while (cond);`.
#[derive(Debug, Clone, PartialEq)]
pub struct DoWhileStmt {
    pub body: Box<Stmt>,
    pub condition: Expr,
    pub span: Span,
}

impl DoWhileStmt {
    pub fn new(body: Stmt, condition: Expr, span: Span) -> Self {
        Self {
            body: Box::new(body),
            condition,
            span,
        }
    }
}

/// While loop statement: `while (cond) { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Box<Stmt>,
    pub span: Span,
}

impl WhileStmt {
    pub fn new(condition: Expr, body: Stmt, span: Span) -> Self {
        Self {
            condition,
            body: Box::new(body),
            span,
        }
    }
}

/// Initializer clause in a standard C-style for loop.
#[derive(Debug, Clone, PartialEq)]
pub enum ForInit {
    Let(Vec<VariableDecl>),
    Const(Vec<VariableDecl>),
    Expr(Vec<Expr>),
}

/// Iteration flavor for `for..of` and `for..in` loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForIterationKind {
    /// Collection iteration over items: `for (let item of array)`.
    Of,
    /// Key iteration over dictionaries/objects: `for (let key in dict)`.
    In,
}

/// Loop statement variants for `for`.
#[derive(Debug, Clone, PartialEq)]
pub enum ForStmt {
    /// C-style standard loop: `for (let i = 0; i < 10; i += 1) { ... }`.
    Standard {
        init: Option<ForInit>,
        test: Option<Expr>,
        update: Option<Vec<Expr>>,
        body: Box<Stmt>,
    },
    /// Iteration loop: `for (let item of list)` or `for (const key in dict)`.
    Iteration {
        modifier: VarModifier,
        target: BindingTarget,
        kind: ForIterationKind,
        iterable: Expr,
        body: Box<Stmt>,
    },
}

/// A branch arm in a `switch` statement.
#[derive(Debug, Clone, PartialEq)]
pub enum SwitchCase {
    Case {
        pattern: Pattern,
        guard: Option<Expr>,
        body: Vec<Stmt>,
        span: Span,
    },
    Default {
        body: Vec<Stmt>,
        span: Span,
    },
}

impl SwitchCase {
    pub fn span(&self) -> Span {
        match self {
            Self::Case { span, .. } | Self::Default { span, .. } => *span,
        }
    }
}

/// Pattern-matching switch statement: `switch (val) { case 1: ... default: ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStmt {
    pub target: Expr,
    pub cases: Vec<SwitchCase>,
    pub span: Span,
}

impl SwitchStmt {
    pub fn new(target: Expr, cases: Vec<SwitchCase>, span: Span) -> Self {
        Self {
            target,
            cases,
            span,
        }
    }
}

/// Return statement: `return value;` or `return;`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

impl ReturnStmt {
    pub fn new(value: Option<Expr>, span: Span) -> Self {
        Self { value, span }
    }
}

/// Break statement: `break;` or `break label;`.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakStmt {
    pub label: Option<Ident>,
    pub span: Span,
}

impl BreakStmt {
    pub fn new(label: Option<Ident>, span: Span) -> Self {
        Self { label, span }
    }
}

/// Continue statement: `continue;` or `continue label;`.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinueStmt {
    pub label: Option<Ident>,
    pub span: Span,
}

impl ContinueStmt {
    pub fn new(label: Option<Ident>, span: Span) -> Self {
        Self { label, span }
    }
}

/// Deferred execution block: `defer { cleanup(); }` or `defer close(file);`.
/// Executed in LIFO order upon scope exit.
#[derive(Debug, Clone, PartialEq)]
pub struct DeferStmt {
    pub statement: Box<Stmt>,
    pub span: Span,
}

impl DeferStmt {
    pub fn new(statement: Stmt, span: Span) -> Self {
        Self {
            statement: Box::new(statement),
            span,
        }
    }
}

/// Low-level unsafe block: `unsafe { *raw_ptr = 42; }`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnsafeStmt {
    pub block: Block,
    pub span: Span,
}

impl UnsafeStmt {
    pub fn new(block: Block, span: Span) -> Self {
        Self { block, span }
    }
}

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
    /// A nested block statement: `{ ... }`.
    Block(Block),
    /// Variable binding statement: `let x = 1;` or `const y = 2;`.
    Variable(VariableStmt),
    /// RAII disposable binding: `using res = init();`.
    Using(UsingStmt),
    /// Conditional branch: `if (cond) { ... } else { ... }`.
    If(IfStmt),
    /// Post-test loop: `do { ... } while (cond);`.
    DoWhile(DoWhileStmt),
    /// Pre-test loop: `while (cond) { ... }`.
    While(WhileStmt),
    /// For loop: standard C-style or iteration (`of`/`in`).
    For(ForStmt),
    /// Pattern-matching switch statement: `switch (v) { ... }`.
    Switch(SwitchStmt),
    /// Return statement: `return expr;`.
    Return(ReturnStmt),
    /// Break statement: `break;`.
    Break(BreakStmt),
    /// Continue statement: `continue;`.
    Continue(ContinueStmt),
    /// Deferred cleanup: `defer expr;`.
    Defer(DeferStmt),
    /// Unsafe execution block: `unsafe { ... }`.
    Unsafe(UnsafeStmt),
    /// Debugger breakpoint: `debugger;`.
    Debugger,
    /// Expression statement: `expr;`.
    Expr(Expr),
    /// Empty statement (redundant semicolon): `;`.
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

    pub fn block(block: Block) -> Self {
        let span = block.span;
        Self::new(StmtKind::Block(block), span)
    }

    pub fn variable(modifier: VarModifier, declarations: Vec<VariableDecl>, span: Span) -> Self {
        Self::new(
            StmtKind::Variable(VariableStmt::new(modifier, declarations, span)),
            span,
        )
    }

    pub fn using_stmt(name: Ident, ty: Option<Type>, init: Expr, span: Span) -> Self {
        Self::new(StmtKind::Using(UsingStmt::new(name, ty, init, span)), span)
    }

    pub fn if_stmt(
        condition: Expr,
        then_branch: Stmt,
        else_branch: Option<Stmt>,
        span: Span,
    ) -> Self {
        Self::new(
            StmtKind::If(IfStmt::new(condition, then_branch, else_branch, span)),
            span,
        )
    }

    pub fn while_stmt(condition: Expr, body: Stmt, span: Span) -> Self {
        Self::new(StmtKind::While(WhileStmt::new(condition, body, span)), span)
    }

    pub fn do_while(body: Stmt, condition: Expr, span: Span) -> Self {
        Self::new(
            StmtKind::DoWhile(DoWhileStmt::new(body, condition, span)),
            span,
        )
    }

    pub fn for_standard(
        init: Option<ForInit>,
        test: Option<Expr>,
        update: Option<Vec<Expr>>,
        body: Stmt,
        span: Span,
    ) -> Self {
        Self::new(
            StmtKind::For(ForStmt::Standard {
                init,
                test,
                update,
                body: Box::new(body),
            }),
            span,
        )
    }

    pub fn for_iteration(
        modifier: VarModifier,
        target: BindingTarget,
        kind: ForIterationKind,
        iterable: Expr,
        body: Stmt,
        span: Span,
    ) -> Self {
        Self::new(
            StmtKind::For(ForStmt::Iteration {
                modifier,
                target,
                kind,
                iterable,
                body: Box::new(body),
            }),
            span,
        )
    }

    pub fn switch_stmt(target: Expr, cases: Vec<SwitchCase>, span: Span) -> Self {
        Self::new(StmtKind::Switch(SwitchStmt::new(target, cases, span)), span)
    }

    pub fn return_stmt(value: Option<Expr>, span: Span) -> Self {
        Self::new(StmtKind::Return(ReturnStmt::new(value, span)), span)
    }

    pub fn break_stmt(label: Option<Ident>, span: Span) -> Self {
        Self::new(StmtKind::Break(BreakStmt::new(label, span)), span)
    }

    pub fn continue_stmt(label: Option<Ident>, span: Span) -> Self {
        Self::new(StmtKind::Continue(ContinueStmt::new(label, span)), span)
    }

    pub fn defer_stmt(statement: Stmt, span: Span) -> Self {
        Self::new(StmtKind::Defer(DeferStmt::new(statement, span)), span)
    }

    pub fn unsafe_stmt(block: Block, span: Span) -> Self {
        Self::new(StmtKind::Unsafe(UnsafeStmt::new(block, span)), span)
    }

    pub fn debugger(span: Span) -> Self {
        Self::new(StmtKind::Debugger, span)
    }

    pub fn expr(expr: Expr) -> Self {
        let span = expr.span;
        Self::new(StmtKind::Expr(expr), span)
    }

    pub fn empty(span: Span) -> Self {
        Self::new(StmtKind::Empty, span)
    }
}
