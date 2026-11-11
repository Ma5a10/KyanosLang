// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Literal representations in Kyanos source code.

use std::fmt;

use crate::Span;

/// Base radix for integer literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Radix {
    Binary = 2,
    Octal = 8,
    Decimal = 10,
    Hex = 16,
}

impl Radix {
    pub const fn as_u32(&self) -> u32 {
        *self as u32
    }
}

/// An integer literal with parsed value, source text, and radix.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegerLiteral {
    /// Raw representation in source text (e.g. `0xFF_00`, `123`).
    pub raw: String,
    /// Parsed unsigned 64-bit value.
    pub value: u64,
    /// Number base.
    pub radix: Radix,
}

impl IntegerLiteral {
    pub fn new(raw: impl Into<String>, value: u64, radix: Radix) -> Self {
        Self {
            raw: raw.into(),
            value,
            radix,
        }
    }
}

/// A 64-bit floating point literal.
#[derive(Debug, Clone, PartialEq)]
pub struct FloatLiteral {
    /// Raw representation in source text (e.g. `3.14159`, `1e-4`).
    pub raw: String,
    /// Parsed 64-bit float value.
    pub value: f64,
}

impl FloatLiteral {
    pub fn new(raw: impl Into<String>, value: f64) -> Self {
        Self {
            raw: raw.into(),
            value,
        }
    }
}

/// A BigInt literal with 'n' or 'N' suffix (e.g. `9007199254740991n`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BigIntLiteral {
    /// Raw representation in source text including 'n' suffix.
    pub raw: String,
}

impl BigIntLiteral {
    pub fn new(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }
}

/// A standard UTF-8 string literal (`"..."`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StringLiteral {
    /// Evaluated string value after unescaping.
    pub value: String,
    /// Raw representation in source code (including quotes).
    pub raw: String,
}

impl StringLiteral {
    pub fn new(value: impl Into<String>, raw: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            raw: raw.into(),
        }
    }
}

/// A C-compatible null-terminated string literal (`c"..."`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CStringLiteral {
    /// Evaluated string value after unescaping.
    pub value: String,
    /// Raw representation in source code (e.g. `c"hello\0"`).
    pub raw: String,
}

impl CStringLiteral {
    pub fn new(value: impl Into<String>, raw: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            raw: raw.into(),
        }
    }
}

/// A single unicode character literal (`'c'`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharLiteral {
    /// Evaluated UTF-32 scalar character.
    pub value: char,
    /// Raw representation in source code.
    pub raw: String,
}

impl CharLiteral {
    pub fn new(value: char, raw: impl Into<String>) -> Self {
        Self {
            value,
            raw: raw.into(),
        }
    }
}

/// The specific kind of literal.
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralKind {
    Integer(IntegerLiteral),
    Float(FloatLiteral),
    BigInt(BigIntLiteral),
    String(StringLiteral),
    CString(CStringLiteral),
    Char(CharLiteral),
    Bool(bool),
    Null,
}

/// A literal AST node carrying its kind and span.
#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub kind: LiteralKind,
    pub span: Span,
}

impl Literal {
    pub fn new(kind: LiteralKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn integer(raw: impl Into<String>, value: u64, radix: Radix, span: Span) -> Self {
        Self::new(LiteralKind::Integer(IntegerLiteral::new(raw, value, radix)), span)
    }

    pub fn float(raw: impl Into<String>, value: f64, span: Span) -> Self {
        Self::new(LiteralKind::Float(FloatLiteral::new(raw, value)), span)
    }

    pub fn big_int(raw: impl Into<String>, span: Span) -> Self {
        Self::new(LiteralKind::BigInt(BigIntLiteral::new(raw)), span)
    }

    pub fn string(value: impl Into<String>, raw: impl Into<String>, span: Span) -> Self {
        Self::new(LiteralKind::String(StringLiteral::new(value, raw)), span)
    }

    pub fn c_string(value: impl Into<String>, raw: impl Into<String>, span: Span) -> Self {
        Self::new(LiteralKind::CString(CStringLiteral::new(value, raw)), span)
    }

    pub fn char(value: char, raw: impl Into<String>, span: Span) -> Self {
        Self::new(LiteralKind::Char(CharLiteral::new(value, raw)), span)
    }

    pub fn bool(value: bool, span: Span) -> Self {
        Self::new(LiteralKind::Bool(value), span)
    }

    pub fn null(span: Span) -> Self {
        Self::new(LiteralKind::Null, span)
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            LiteralKind::Integer(lit) => write!(f, "{}", lit.raw),
            LiteralKind::Float(lit) => write!(f, "{}", lit.raw),
            LiteralKind::BigInt(lit) => write!(f, "{}", lit.raw),
            LiteralKind::String(lit) => write!(f, "{}", lit.raw),
            LiteralKind::CString(lit) => write!(f, "{}", lit.raw),
            LiteralKind::Char(lit) => write!(f, "{}", lit.raw),
            LiteralKind::Bool(val) => write!(f, "{}", val),
            LiteralKind::Null => write!(f, "null"),
        }
    }
}
