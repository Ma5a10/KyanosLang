// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Type annotation AST representations for Kyanos.

use std::fmt;

use crate::{Ident, Span};

/// Built-in predefined primitive types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PredefinedType {
    // Signed integers
    Byte,  // byte / i8
    Short, // short / i16
    Int,   // int / i32
    Long,  // long / i64
    SizeT, // size_t / isize

    // Unsigned integers
    Ubyte,  // ubyte / u8
    Ushort, // ushort / u16
    Uint,   // uint / u32
    Ulong,  // ulong / u64

    // Floating points
    Float,  // float / f32
    Double, // double / f64

    // Primitives
    Bool,   // bool
    Char,   // char (UTF-32)
    String, // string
    Void,   // void
    Never,  // never
    Rawptr, // rawptr (void*)
}

impl PredefinedType {
    /// Returns the canonical keyword string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Byte => "byte",
            Self::Short => "short",
            Self::Int => "int",
            Self::Long => "long",
            Self::SizeT => "size_t",
            Self::Ubyte => "ubyte",
            Self::Ushort => "ushort",
            Self::Uint => "uint",
            Self::Ulong => "ulong",
            Self::Float => "float",
            Self::Double => "double",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::String => "string",
            Self::Void => "void",
            Self::Never => "never",
            Self::Rawptr => "rawptr",
        }
    }
}

impl fmt::Display for PredefinedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Qualified type name such as `Foo` or `std.collections.List`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeName {
    pub parts: Vec<Ident>,
    pub span: Span,
}

impl TypeName {
    pub fn new(parts: Vec<Ident>, span: Span) -> Self {
        Self { parts, span }
    }

    pub fn single(ident: Ident) -> Self {
        let span = ident.span;
        Self {
            parts: vec![ident],
            span,
        }
    }
}

impl fmt::Display for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joined = self
            .parts
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(".");
        write!(f, "{}", joined)
    }
}

/// Generic type arguments list: `<T, U>`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeArguments {
    pub args: Vec<Type>,
    pub span: Span,
}

impl TypeArguments {
    pub fn new(args: Vec<Type>, span: Span) -> Self {
        Self { args, span }
    }
}

/// A reference to a nominal type: `TypeName<TypeArguments>?`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeReference {
    pub name: TypeName,
    pub type_arguments: Option<TypeArguments>,
    pub span: Span,
}

impl TypeReference {
    pub fn new(name: TypeName, type_arguments: Option<TypeArguments>, span: Span) -> Self {
        Self {
            name,
            type_arguments,
            span,
        }
    }
}

/// A generic type parameter declaration: `T extends Bound = Default`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParameter {
    pub name: Ident,
    pub constraint: Option<Type>,
    pub default: Option<Type>,
    pub span: Span,
}

impl TypeParameter {
    pub fn new(name: Ident, constraint: Option<Type>, default: Option<Type>, span: Span) -> Self {
        Self {
            name,
            constraint,
            default,
            span,
        }
    }
}

/// Generic type parameters list: `<T, U extends Bound>`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParameters {
    pub params: Vec<TypeParameter>,
    pub span: Span,
}

impl TypeParameters {
    pub fn new(params: Vec<TypeParameter>, span: Span) -> Self {
        Self { params, span }
    }
}

/// Fixed array size descriptor (e.g. `10` or a constant expression string).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArraySize {
    Literal(u64),
    Expr(String),
}

/// A member field in an anonymous struct type `{ x: int, readonly y?: double }`.
#[derive(Debug, Clone, PartialEq)]
pub struct AnonymousStructField {
    pub is_readonly: bool,
    pub name: Ident,
    pub is_optional: bool,
    pub ty: Type,
    pub span: Span,
}

impl AnonymousStructField {
    pub fn new(is_readonly: bool, name: Ident, is_optional: bool, ty: Type, span: Span) -> Self {
        Self {
            is_readonly,
            name,
            is_optional,
            ty,
            span,
        }
    }
}

/// Parameter in a function type `(x: int, y: double) => void`.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionTypeParam {
    pub name: Option<Ident>,
    pub is_readonly: bool,
    pub is_optional: bool,
    pub is_rest: bool,
    pub ty: Type,
    pub span: Span,
}

impl FunctionTypeParam {
    pub fn new(
        name: Option<Ident>,
        is_readonly: bool,
        is_optional: bool,
        is_rest: bool,
        ty: Type,
        span: Span,
    ) -> Self {
        Self {
            name,
            is_readonly,
            is_optional,
            is_rest,
            ty,
            span,
        }
    }
}

/// Function type: `<T>(x: int, y: int) => returnType`.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub type_parameters: Option<TypeParameters>,
    pub params: Vec<FunctionTypeParam>,
    pub return_type: Box<ReturnType>,
    pub span: Span,
}

impl FunctionType {
    pub fn new(
        type_parameters: Option<TypeParameters>,
        params: Vec<FunctionTypeParam>,
        return_type: ReturnType,
        span: Span,
    ) -> Self {
        Self {
            type_parameters,
            params,
            return_type: Box::new(return_type),
            span,
        }
    }
}

/// User-defined type guard predicate: `x is String`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypePredicate {
    pub target: Ident,
    pub ty: Type,
    pub span: Span,
}

impl TypePredicate {
    pub fn new(target: Ident, ty: Type, span: Span) -> Self {
        Self { target, ty, span }
    }
}

/// Function return type annotation: either a normal `Type` or a `TypePredicate`.
#[derive(Debug, Clone, PartialEq)]
pub enum ReturnType {
    Type(Type),
    Predicate(TypePredicate),
}

impl ReturnType {
    pub fn span(&self) -> Span {
        match self {
            Self::Type(ty) => ty.span,
            Self::Predicate(pred) => pred.span,
        }
    }
}

/// Specific variant of a type annotation.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    /// Predefined primitive types (e.g. `int`, `double`, `string`).
    Predefined(PredefinedType),
    /// Type reference (e.g. `User`, `List<int>`).
    Reference(TypeReference),
    /// Raw pointer: `*T` (mutable) or `*const T` (immutable).
    Pointer { is_const: bool, elem: Box<Type> },
    /// Reference: `&T` (mutable) or `&const T` (immutable).
    ReferenceType { is_const: bool, elem: Box<Type> },
    /// Dynamic array: `T[]`.
    DynamicArray(Box<Type>),
    /// Fixed-size array: `T[N]`.
    FixedArray { elem: Box<Type>, size: ArraySize },
    /// Syntactic sugar for `Option<T>`: `T?`.
    Optional(Box<Type>),
    /// Tuple type: `[T, U, V]`.
    Tuple(Vec<Type>),
    /// Anonymous struct type: `{ x: int, y: double }`.
    AnonymousStruct(Vec<AnonymousStructField>),
    /// Function type: `(x: int) => void`.
    Function(FunctionType),
    /// Parenthesized type: `(T)`.
    Parenthesized(Box<Type>),
    /// The `this` type.
    This,
    /// Union type: `T | U`.
    Union(Vec<Type>),
    /// Intersection type: `T & U`.
    Intersection(Vec<Type>),
}

/// A complete type annotation node with kind and span.
#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

impl Type {
    pub fn new(kind: TypeKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn predefined(pt: PredefinedType, span: Span) -> Self {
        Self::new(TypeKind::Predefined(pt), span)
    }

    pub fn reference(name: TypeName, args: Option<TypeArguments>, span: Span) -> Self {
        Self::new(
            TypeKind::Reference(TypeReference::new(name, args, span)),
            span,
        )
    }

    pub fn pointer(elem: Type, is_const: bool, span: Span) -> Self {
        Self::new(
            TypeKind::Pointer {
                is_const,
                elem: Box::new(elem),
            },
            span,
        )
    }

    pub fn reference_type(elem: Type, is_const: bool, span: Span) -> Self {
        Self::new(
            TypeKind::ReferenceType {
                is_const,
                elem: Box::new(elem),
            },
            span,
        )
    }

    pub fn dynamic_array(elem: Type, span: Span) -> Self {
        Self::new(TypeKind::DynamicArray(Box::new(elem)), span)
    }

    pub fn fixed_array(elem: Type, size: ArraySize, span: Span) -> Self {
        Self::new(
            TypeKind::FixedArray {
                elem: Box::new(elem),
                size,
            },
            span,
        )
    }

    pub fn optional(elem: Type, span: Span) -> Self {
        Self::new(TypeKind::Optional(Box::new(elem)), span)
    }

    pub fn tuple(elems: Vec<Type>, span: Span) -> Self {
        Self::new(TypeKind::Tuple(elems), span)
    }

    pub fn this(span: Span) -> Self {
        Self::new(TypeKind::This, span)
    }
}
