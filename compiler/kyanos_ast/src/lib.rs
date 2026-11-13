// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! AST (Abstract Syntax Tree) definitions for the Kyanos programming language.

pub mod decl;
pub mod expr;
pub mod ident;
pub mod literal;
pub mod op;
pub mod pattern;
pub mod stmt;
pub mod types;

pub use decl::{
    Accessibility, AccessorKind, Attribute, AttributeArg, AttributeInput, ClassDecl,
    ClassElementName, ClassField, ClassMember, ClassMethod, ConstructorDecl, Decl, DeclKind,
    EnumDecl, EnumVariant, ExternBlockDecl, ExternFunctionDecl, ExternMember, ExternParameterList,
    ExternVariableDecl, FunctionDecl, GetterSetterDecl, IndexSignature, InterfaceDecl,
    InterfaceMember, InterfaceMethod, InterfaceProperty, MethodDecl, Parameter, ParameterList,
    RestParameter, SingleExternDecl, StructDecl, StructField, StructMember, TypeAliasDecl,
};

pub use expr::{
    Argument, ArrayElement, ArrowBody, Expr, ExprKind, MemberProperty, ObjectMember, ObjectMethod,
    PropertyName, SizeofTarget, StructFieldInit, SwitchExprArm, TemplateStringPart,
};
pub use ident::{Ident, PrivateIdent};
pub use literal::{
    BigIntLiteral, CStringLiteral, CharLiteral, FloatLiteral, IntegerLiteral, Literal, LiteralKind,
    Radix, StringLiteral,
};
pub use op::{precedence, AssignOp, Associativity, BinaryOp, PostfixUnaryOp, PrefixUnaryOp};
pub use pattern::{
    BindingTarget, LiteralPattern, LiteralSign, ObjectPatternProp, Pattern, PatternKind,
    StructFieldPattern, TuplePatternElement, VarModifier,
};
pub use stmt::{
    Block, BreakStmt, ContinueStmt, DeferStmt, DoWhileStmt, ForInit, ForIterationKind, ForStmt,
    IfStmt, LabeledStmt, ReturnStmt, Stmt, StmtKind, SwitchCase, SwitchStmt, UnsafeStmt, UsingStmt,
    VariableDecl, VariableStmt, WhileStmt,
};
pub use types::{
    AnonymousStructField, ArraySize, FunctionType, FunctionTypeParam, PredefinedType, ReturnType,
    Type, TypeArguments, TypeKind, TypeName, TypeParameter, TypeParameters, TypePredicate,
    TypeReference,
};

/// Unique identifier for an AST node within a module/compilation unit.
///
/// Constructed with `NodeId::DUMMY` by default and populated sequentially
/// (`0, 1, 2, ...`) during parsing or prior to semantic analysis, enabling
/// `O(1)` side-table lookups in `kyanos_sema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Sentinel / uninitialized node ID (`u32::MAX`).
    pub const DUMMY: Self = Self(u32::MAX);

    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub const fn is_dummy(self) -> bool {
        self.0 == Self::DUMMY.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::DUMMY
    }
}

/// Represents a source code span [start, end) byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Dummy / uninitialized span (`0..0`).
    pub const DUMMY: Self = Self { start: 0, end: 0 };

    pub const fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "Span start must not exceed end");
        Self { start, end }
    }

    /// Returns `true` if this span is the dummy/default span (`0..0`).
    pub const fn is_dummy(&self) -> bool {
        self.start == 0 && self.end == 0
    }

    /// Returns `true` if this span covers zero bytes (`start >= end`).
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Returns the byte length of the span (`end - start`, saturating at 0).
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns a new span that covers both `self` and `other`.
    /// If either span is `Span::DUMMY` (`0..0`), returns the other span to prevent offset-0 pollution.
    pub fn merge(self, other: Self) -> Self {
        if self.is_dummy() {
            return other;
        }
        if other.is_dummy() {
            return self;
        }
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

// =============================================================================
// Module & Root AST (ES Modules Style)
// =============================================================================

/// An imported symbol with an optional local alias: `Foo` or `Bar as Baz`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportSpecifier {
    pub name: Ident,
    pub alias: Option<Ident>,
    pub span: Span,
}

impl ImportSpecifier {
    pub fn new(name: Ident, alias: Option<Ident>, span: Span) -> Self {
        Self { name, alias, span }
    }

    pub fn simple(name: Ident) -> Self {
        let span = name.span;
        Self::new(name, None, span)
    }
}

/// The specific form of an import clause.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportClause {
    /// Named bindings: `import { Foo, Bar as Baz } from "./module";`.
    Named(Vec<ImportSpecifier>),
    /// Namespace wildcard: `import * as Math from "./math";`.
    Namespace(Ident),
    /// Default import with optional named bindings: `import Item, { Foo } from "./item";`.
    Default {
        default_name: Ident,
        named: Vec<ImportSpecifier>,
    },
}

/// An import statement: `import ... from "path";` or `import "side_effect";`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportStmt {
    pub clause: Option<ImportClause>,
    pub source: StringLiteral,
    pub span: Span,
}

impl ImportStmt {
    pub fn new(clause: Option<ImportClause>, source: StringLiteral, span: Span) -> Self {
        Self {
            clause,
            source,
            span,
        }
    }

    pub fn side_effect(source: StringLiteral, span: Span) -> Self {
        Self::new(None, source, span)
    }
}

/// An exported symbol with an optional exported alias: `Foo` or `Bar as Baz`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExportSpecifier {
    pub name: Ident,
    pub alias: Option<Ident>,
    pub span: Span,
}

impl ExportSpecifier {
    pub fn new(name: Ident, alias: Option<Ident>, span: Span) -> Self {
        Self { name, alias, span }
    }

    pub fn simple(name: Ident) -> Self {
        let span = name.span;
        Self::new(name, None, span)
    }
}

/// Specific variant of an export statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportKind {
    /// Exporting a declaration: `export function add() {}` or `export struct Point {}`.
    Decl(Decl),
    /// Named export list or re-export: `export { Foo, Bar as Baz } from "./module";`.
    Named {
        specifiers: Vec<ExportSpecifier>,
        source: Option<StringLiteral>,
    },
    /// Default expression export: `export default expr;`.
    Default(Expr),
}

/// An export statement: `export ...` with optional attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportStmt {
    pub attributes: Vec<Attribute>,
    pub kind: ExportKind,
    pub span: Span,
}

impl ExportStmt {
    pub fn new(attributes: Vec<Attribute>, kind: ExportKind, span: Span) -> Self {
        Self {
            attributes,
            kind,
            span,
        }
    }

    pub fn decl(attributes: Vec<Attribute>, decl: Decl, span: Span) -> Self {
        Self::new(attributes, ExportKind::Decl(decl), span)
    }

    pub fn named(
        specifiers: Vec<ExportSpecifier>,
        source: Option<StringLiteral>,
        span: Span,
    ) -> Self {
        Self::new(Vec::new(), ExportKind::Named { specifiers, source }, span)
    }

    pub fn default_expr(expr: Expr, span: Span) -> Self {
        Self::new(Vec::new(), ExportKind::Default(expr), span)
    }
}

/// A top-level source element within a Kyanos file / module.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceElement {
    Import(ImportStmt),
    Export(ExportStmt),
    Decl(Decl),
    Stmt(Stmt),
}

impl SourceElement {
    pub fn span(&self) -> Span {
        match self {
            Self::Import(i) => i.span,
            Self::Export(e) => e.span,
            Self::Decl(d) => d.span,
            Self::Stmt(s) => s.span,
        }
    }
}

/// Root AST node representing an entire Kyanos module / compilation unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub elements: Vec<SourceElement>,
    pub span: Span,
}

impl Program {
    pub fn new(elements: Vec<SourceElement>, span: Span) -> Self {
        Self { elements, span }
    }

    pub fn empty(span: Span) -> Self {
        Self {
            elements: Vec::new(),
            span,
        }
    }
}
