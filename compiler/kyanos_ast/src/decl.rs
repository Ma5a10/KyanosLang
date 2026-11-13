// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Declaration AST representations for Kyanos.

use crate::{
    pattern::BindingTarget,
    stmt::{Block, UsingStmt, VariableStmt},
    Expr, Ident, NodeId, PredefinedType, PrivateIdent, ReturnType, Span, StringLiteral, Type,
    TypeParameters, TypeReference,
};

/// Member accessibility modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Accessibility {
    Public,
    Protected,
    Private,
}

impl Accessibility {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Private => "private",
        }
    }
}

/// Argument inside an attribute: `#[attr(arg)]` or `#[attr(key = val)]`.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeArg {
    pub name: Option<Ident>,
    pub value: Expr,
    pub span: Span,
}

impl AttributeArg {
    pub fn new(name: Option<Ident>, value: Expr, span: Span) -> Self {
        Self { name, value, span }
    }
}

/// Input form of an attribute annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeInput {
    /// Bare attribute without arguments: `#[inline]`.
    None,
    /// Parenthesized argument list: `#[repr(C)]`, `#[link(name = "sqlite3")]`.
    Args(Vec<AttributeArg>),
    /// Key-value assignment attribute: `#[link_name = "sqlite3_open"]`.
    Value(Expr),
}

/// An attribute annotation: `#[repr(C)]`, `#[link = "..."]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: Ident,
    pub input: AttributeInput,
    pub span: Span,
}

impl Attribute {
    pub fn new(name: Ident, args: Vec<AttributeArg>, span: Span) -> Self {
        Self {
            name,
            input: AttributeInput::Args(args),
            span,
        }
    }

    pub fn simple(name: Ident, span: Span) -> Self {
        Self {
            name,
            input: AttributeInput::None,
            span,
        }
    }

    pub fn with_value(name: Ident, value: Expr, span: Span) -> Self {
        Self {
            name,
            input: AttributeInput::Value(value),
            span,
        }
    }
}

/// Parameter definition in a function, constructor, or method.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub accessibility: Option<Accessibility>,
    pub is_readonly: bool,
    pub target: BindingTarget,
    pub is_optional: bool,
    pub ty: Option<Type>,
    pub default: Option<Expr>,
    pub span: Span,
}

impl Parameter {
    pub fn new(
        accessibility: Option<Accessibility>,
        is_readonly: bool,
        target: BindingTarget,
        is_optional: bool,
        ty: Option<Type>,
        default: Option<Expr>,
        span: Span,
    ) -> Self {
        Self {
            accessibility,
            is_readonly,
            target,
            is_optional,
            ty,
            default,
            span,
        }
    }

    pub fn simple(name: Ident, ty: Option<Type>, span: Span) -> Self {
        Self::new(
            None,
            false,
            BindingTarget::Ident(name),
            false,
            ty,
            None,
            span,
        )
    }
}

/// Rest parameter: `...args: int[]`.
#[derive(Debug, Clone, PartialEq)]
pub struct RestParameter {
    pub name: Ident,
    pub ty: Option<Type>,
    pub span: Span,
}

impl RestParameter {
    pub fn new(name: Ident, ty: Option<Type>, span: Span) -> Self {
        Self { name, ty, span }
    }
}

/// Parameter list including optional rest parameter: `(a: int, b: string, ...rest: double[])`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterList {
    pub params: Vec<Parameter>,
    pub rest: Option<RestParameter>,
    pub span: Span,
}

impl ParameterList {
    pub fn new(params: Vec<Parameter>, rest: Option<RestParameter>, span: Span) -> Self {
        Self { params, rest, span }
    }

    pub fn empty(span: Span) -> Self {
        Self {
            params: Vec::new(),
            rest: None,
            span,
        }
    }
}

/// Function declaration: `async? function name<T>(params): ret { body }`.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub attributes: Vec<Attribute>,
    pub is_async: bool,
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub params: ParameterList,
    pub return_type: Option<ReturnType>,
    pub body: Option<Block>,
    pub span: Span,
}

impl FunctionDecl {
    pub fn new(
        attributes: Vec<Attribute>,
        is_async: bool,
        name: Ident,
        type_parameters: Option<TypeParameters>,
        params: ParameterList,
        return_type: Option<ReturnType>,
        body: Option<Block>,
        span: Span,
    ) -> Self {
        Self {
            attributes,
            is_async,
            name,
            type_parameters,
            params,
            return_type,
            body,
            span,
        }
    }
}

/// A field member in a struct declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub accessibility: Option<Accessibility>,
    pub is_static: bool,
    pub is_readonly: bool,
    pub name: Ident,
    pub ty: Option<Type>,
    pub init: Option<Expr>,
    pub span: Span,
}

/// Constructor declaration inside a struct or class.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDecl {
    pub accessibility: Option<Accessibility>,
    pub params: ParameterList,
    pub body: Block,
    pub span: Span,
}

/// Method declaration in a struct.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub accessibility: Option<Accessibility>,
    pub is_static: bool,
    pub is_async: bool,
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub params: ParameterList,
    pub return_type: Option<ReturnType>,
    pub body: Block,
    pub span: Span,
}

/// A member inside a struct declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum StructMember {
    Field(StructField),
    Constructor(ConstructorDecl),
    Method(MethodDecl),
}

/// Struct declaration (stack-allocated nominal value type, C-ABI compatible).
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub attributes: Vec<Attribute>,
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub members: Vec<StructMember>,
    pub span: Span,
}

impl StructDecl {
    pub fn new(
        attributes: Vec<Attribute>,
        name: Ident,
        type_parameters: Option<TypeParameters>,
        members: Vec<StructMember>,
        span: Span,
    ) -> Self {
        Self {
            attributes,
            name,
            type_parameters,
            members,
            span,
        }
    }
}

/// Element name in a class (standard identifier or private `#ident`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClassElementName {
    Ident(Ident),
    Private(PrivateIdent),
}

impl ClassElementName {
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(id) => id.span,
            Self::Private(pid) => pid.span,
        }
    }
}

/// A field in a class declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassField {
    pub accessibility: Option<Accessibility>,
    pub is_static: bool,
    pub is_readonly: bool,
    pub name: ClassElementName,
    pub ty: Option<Type>,
    pub init: Option<Expr>,
    pub span: Span,
}

/// A method in a class declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassMethod {
    pub accessibility: Option<Accessibility>,
    pub is_static: bool,
    pub is_async: bool,
    pub name: ClassElementName,
    pub type_parameters: Option<TypeParameters>,
    pub params: ParameterList,
    pub return_type: Option<ReturnType>,
    pub body: Block,
    pub span: Span,
}

/// Getter or setter accessor kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessorKind {
    Get,
    Set,
}

/// Getter or setter declaration: `get x(): int { ... }` or `set x(val: int) { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct GetterSetterDecl {
    pub accessibility: Option<Accessibility>,
    pub is_static: bool,
    pub kind: AccessorKind,
    pub name: ClassElementName,
    pub params: ParameterList,
    pub return_type: Option<Type>,
    pub body: Block,
    pub span: Span,
}

/// A member inside a class declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field(ClassField),
    Constructor(ConstructorDecl),
    Method(ClassMethod),
    GetterSetter(GetterSetterDecl),
}

/// Class declaration (heap-allocated reference type, Nim-style ORC managed).
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub attributes: Vec<Attribute>,
    pub name: ClassElementName,
    pub type_parameters: Option<TypeParameters>,
    pub extends: Option<TypeReference>,
    pub implements: Vec<TypeReference>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

impl ClassDecl {
    pub fn new(
        attributes: Vec<Attribute>,
        name: ClassElementName,
        type_parameters: Option<TypeParameters>,
        extends: Option<TypeReference>,
        implements: Vec<TypeReference>,
        members: Vec<ClassMember>,
        span: Span,
    ) -> Self {
        Self {
            attributes,
            name,
            type_parameters,
            extends,
            implements,
            members,
            span,
        }
    }
}

/// A property signature in an interface.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceProperty {
    pub is_readonly: bool,
    pub name: Ident,
    pub is_optional: bool,
    pub ty: Type,
    pub span: Span,
}

/// A method signature in an interface.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    pub name: Ident,
    pub is_optional: bool,
    pub type_parameters: Option<TypeParameters>,
    pub params: ParameterList,
    pub return_type: ReturnType,
    pub span: Span,
}

/// An index signature in an interface: `[key: string]: int`.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexSignature {
    pub name: Ident,
    pub key_type: PredefinedType,
    pub value_type: Type,
    pub span: Span,
}

/// A member inside an interface declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceMember {
    Property(InterfaceProperty),
    Method(InterfaceMethod),
    Index(IndexSignature),
}

/// Interface declaration (structural duck typing).
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub extends: Vec<TypeReference>,
    pub members: Vec<InterfaceMember>,
    pub span: Span,
}

impl InterfaceDecl {
    pub fn new(
        name: Ident,
        type_parameters: Option<TypeParameters>,
        extends: Vec<TypeReference>,
        members: Vec<InterfaceMember>,
        span: Span,
    ) -> Self {
        Self {
            name,
            type_parameters,
            extends,
            members,
            span,
        }
    }
}

/// Variant of an enumeration or algebraic data type (ADT).
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariant {
    /// Variant with payload: `KeyPress(char)`, `Click(x: int, y: int)`.
    Tuple {
        name: Ident,
        payload: ParameterList,
        span: Span,
    },
    /// Simple value/unit variant: `PageLoad`, `Ok = 200`.
    Value {
        name: Ident,
        value: Option<Expr>,
        span: Span,
    },
}

impl EnumVariant {
    pub fn span(&self) -> Span {
        match self {
            Self::Tuple { span, .. } | Self::Value { span, .. } => *span,
        }
    }
}

/// Enum declaration (tagged union / algebraic data type or numeric enum).
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub attributes: Vec<Attribute>,
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub tag_type: Option<PredefinedType>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

impl EnumDecl {
    pub fn new(
        attributes: Vec<Attribute>,
        name: Ident,
        type_parameters: Option<TypeParameters>,
        tag_type: Option<PredefinedType>,
        variants: Vec<EnumVariant>,
        span: Span,
    ) -> Self {
        Self {
            attributes,
            name,
            type_parameters,
            tag_type,
            variants,
            span,
        }
    }
}

/// Type alias declaration: `type Point2D = [double, double];`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDecl {
    pub attributes: Vec<Attribute>,
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub ty: Type,
    pub span: Span,
}

impl TypeAliasDecl {
    pub fn new(
        attributes: Vec<Attribute>,
        name: Ident,
        type_parameters: Option<TypeParameters>,
        ty: Type,
        span: Span,
    ) -> Self {
        Self {
            attributes,
            name,
            type_parameters,
            ty,
            span,
        }
    }
}

/// External parameter list in FFI declarations, supporting C-style variadic `...`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternParameterList {
    pub params: Vec<Parameter>,
    pub is_variadic: bool,
    pub rest: Option<RestParameter>,
    pub span: Span,
}

impl ExternParameterList {
    pub fn new(
        params: Vec<Parameter>,
        is_variadic: bool,
        rest: Option<RestParameter>,
        span: Span,
    ) -> Self {
        Self {
            params,
            is_variadic,
            rest,
            span,
        }
    }
}

/// External C function declaration inside an `extern` block.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternFunctionDecl {
    pub attributes: Vec<Attribute>,
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub params: ExternParameterList,
    pub return_type: Option<Type>,
    pub span: Span,
}

/// External global variable declaration inside an `extern` block.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternVariableDecl {
    pub attributes: Vec<Attribute>,
    pub is_const: bool,
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}

/// A member inside an `extern "C" { ... }` block.
#[derive(Debug, Clone, PartialEq)]
pub enum ExternMember {
    Function(ExternFunctionDecl),
    Variable(ExternVariableDecl),
}

/// Foreign function block: `extern "C" { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternBlockDecl {
    pub attributes: Vec<Attribute>,
    pub abi: StringLiteral,
    pub members: Vec<ExternMember>,
    pub span: Span,
}

impl ExternBlockDecl {
    pub fn new(
        attributes: Vec<Attribute>,
        abi: StringLiteral,
        members: Vec<ExternMember>,
        span: Span,
    ) -> Self {
        Self {
            attributes,
            abi,
            members,
            span,
        }
    }
}

/// Standalone extern function declaration: `extern "C" function puts(s: rawptr): int;`.
#[derive(Debug, Clone, PartialEq)]
pub struct SingleExternDecl {
    pub attributes: Vec<Attribute>,
    pub abi: StringLiteral,
    pub name: Ident,
    pub type_parameters: Option<TypeParameters>,
    pub params: ExternParameterList,
    pub return_type: Option<Type>,
    pub body: Option<Block>,
    pub span: Span,
}

impl SingleExternDecl {
    pub fn new(
        attributes: Vec<Attribute>,
        abi: StringLiteral,
        name: Ident,
        type_parameters: Option<TypeParameters>,
        params: ExternParameterList,
        return_type: Option<Type>,
        body: Option<Block>,
        span: Span,
    ) -> Self {
        Self {
            attributes,
            abi,
            name,
            type_parameters,
            params,
            return_type,
            body,
            span,
        }
    }
}

/// Specific variant of a declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum DeclKind {
    /// Function declaration.
    Function(FunctionDecl),
    /// Nominal stack value type (struct).
    Struct(StructDecl),
    /// Nominal heap reference type (class).
    Class(ClassDecl),
    /// Structural duck-typed interface.
    Interface(InterfaceDecl),
    /// Tagged union / algebraic data type (enum).
    Enum(EnumDecl),
    /// Type alias declaration.
    TypeAlias(TypeAliasDecl),
    /// Variable declaration (`let` / `const`).
    Variable(VariableStmt),
    /// RAII resource disposable declaration (`using`).
    Using(UsingStmt),
    /// FFI extern block (`extern "C" { ... }`).
    ExternBlock(ExternBlockDecl),
    /// Standalone FFI extern declaration (`extern "C" function ...`).
    SingleExtern(SingleExternDecl),
    /// Placeholder produced by fault-tolerant error recovery in the parser.
    Error,
}

/// A complete declaration node carrying its node ID, kind, and span.
#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    pub id: NodeId,
    pub kind: DeclKind,
    pub span: Span,
}

impl Decl {
    pub fn new(kind: DeclKind, span: Span) -> Self {
        Self {
            id: NodeId::DUMMY,
            kind,
            span,
        }
    }

    /// Assigns an explicit `NodeId` to this declaration node.
    pub const fn with_id(mut self, id: NodeId) -> Self {
        self.id = id;
        self
    }

    /// Constructs a synthetic error recovery declaration node.
    pub fn error(span: Span) -> Self {
        Self::new(DeclKind::Error, span)
    }

    pub fn function(decl: FunctionDecl) -> Self {
        let span = decl.span;
        Self::new(DeclKind::Function(decl), span)
    }

    pub fn struct_decl(decl: StructDecl) -> Self {
        let span = decl.span;
        Self::new(DeclKind::Struct(decl), span)
    }

    pub fn class_decl(decl: ClassDecl) -> Self {
        let span = decl.span;
        Self::new(DeclKind::Class(decl), span)
    }

    pub fn interface_decl(decl: InterfaceDecl) -> Self {
        let span = decl.span;
        Self::new(DeclKind::Interface(decl), span)
    }

    pub fn enum_decl(decl: EnumDecl) -> Self {
        let span = decl.span;
        Self::new(DeclKind::Enum(decl), span)
    }

    pub fn type_alias(decl: TypeAliasDecl) -> Self {
        let span = decl.span;
        Self::new(DeclKind::TypeAlias(decl), span)
    }

    pub fn variable(stmt: VariableStmt) -> Self {
        let span = stmt.span;
        Self::new(DeclKind::Variable(stmt), span)
    }

    pub fn using_decl(stmt: UsingStmt) -> Self {
        let span = stmt.span;
        Self::new(DeclKind::Using(stmt), span)
    }

    pub fn extern_block(decl: ExternBlockDecl) -> Self {
        let span = decl.span;
        Self::new(DeclKind::ExternBlock(decl), span)
    }

    pub fn single_extern(decl: SingleExternDecl) -> Self {
        let span = decl.span;
        Self::new(DeclKind::SingleExtern(decl), span)
    }
}
