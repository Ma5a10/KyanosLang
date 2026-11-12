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
    Accessibility, AccessorKind, Attribute, AttributeArg, ClassDecl, ClassElementName, ClassField,
    ClassMember, ClassMethod, ConstructorDecl, Decl, DeclKind, EnumDecl, EnumVariant,
    ExternBlockDecl, ExternFunctionDecl, ExternMember, ExternParameterList, ExternVariableDecl,
    FunctionDecl, GetterSetterDecl, IndexSignature, InterfaceDecl, InterfaceMember,
    InterfaceMethod, InterfaceProperty, MethodDecl, Parameter, ParameterList, RestParameter,
    SingleExternDecl, StructDecl, StructField, StructMember, TypeAliasDecl,
};

pub use expr::{
    Argument, ArrayElement, ArrowBody, Expr, ExprKind, FunctionParam, MemberProperty, ObjectMember,
    ObjectMethod, PropertyName, SizeofTarget, StructFieldInit, SwitchExprArm, TemplateStringPart,
};
pub use ident::{Ident, PrivateIdent};
pub use literal::{
    BigIntLiteral, CStringLiteral, CharLiteral, FloatLiteral, IntegerLiteral, Literal, LiteralKind,
    Radix, StringLiteral,
};
pub use op::{AssignOp, Associativity, BinaryOp, PostfixUnaryOp, PrefixUnaryOp};
pub use pattern::{
    BindingTarget, LiteralPattern, ObjectPatternProp, Pattern, PatternKind, StructFieldPattern,
    TuplePatternElement, VarModifier,
};
pub use stmt::{
    Block, BreakStmt, ContinueStmt, DeferStmt, DoWhileStmt, ForInit, ForIterationKind, ForStmt,
    IfStmt, ReturnStmt, Stmt, StmtKind, SwitchCase, SwitchStmt, UnsafeStmt, UsingStmt,
    VariableDecl, VariableStmt, WhileStmt,
};
pub use types::{
    AnonymousStructField, ArraySize, FunctionType, FunctionTypeParam, PredefinedType, ReturnType,
    Type, TypeArguments, TypeKind, TypeName, TypeParameter, TypeParameters, TypePredicate,
    TypeReference,
};

/// Represents a source code span [start, end) byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns a new span that covers both `self` and `other`.
    pub fn merge(self, other: Self) -> Self {
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
