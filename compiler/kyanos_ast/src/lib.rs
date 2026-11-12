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
