// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Operators and punctuators in Kyanos expressions.

use std::fmt;

/// Associativity of a binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Associativity {
    Left,
    Right,
}

/// Binary operator variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Arithmetic
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Rem, // %
    Pow, // **

    // Bitwise
    Shl,  // <<
    Shr,  // >>
    Ushr, // >>>
    BitAnd, // &
    BitXor, // ^
    BitOr,  // |

    // Relational & Equality
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
    Eq, // ==
    Ne, // !=
    Is, // is

    // Logical
    LogicalAnd,     // &&
    LogicalOr,      // ||
    NullCoalescing, // ??
}

impl BinaryOp {
    /// Returns the symbol representation of the operator.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Pow => "**",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::Ushr => ">>>",
            Self::BitAnd => "&",
            Self::BitXor => "^",
            Self::BitOr => "|",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Is => "is",
            Self::LogicalAnd => "&&",
            Self::LogicalOr => "||",
            Self::NullCoalescing => "??",
        }
    }

    /// Returns the associativity of this binary operator.
    pub const fn associativity(&self) -> Associativity {
        match self {
            // Exponentiation (**) is right-associative in Kyanos
            Self::Pow => Associativity::Right,
            _ => Associativity::Left,
        }
    }

    /// Returns the operator precedence level (higher value binds tighter).
    pub const fn precedence(&self) -> u8 {
        match self {
            Self::NullCoalescing => 1,
            Self::LogicalOr => 2,
            Self::LogicalAnd => 3,
            Self::BitOr => 4,
            Self::BitXor => 5,
            Self::BitAnd => 6,
            Self::Eq | Self::Ne => 7,
            Self::Lt | Self::Le | Self::Gt | Self::Ge | Self::Is => 8,
            Self::Shl | Self::Shr | Self::Ushr => 9,
            Self::Add | Self::Sub => 10,
            Self::Mul | Self::Div | Self::Rem => 11,
            Self::Pow => 12,
        }
    }

    /// Returns the Pratt parser binding powers (left_bp, right_bp).
    pub const fn binding_power(&self) -> (u8, u8) {
        let prec = self.precedence();
        match self.associativity() {
            Associativity::Left => (prec * 2, prec * 2 + 1),
            Associativity::Right => (prec * 2 + 1, prec * 2),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Prefix unary operator variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrefixUnaryOp {
    // Arithmetic / Sign
    Plus,  // +
    Minus, // -

    // Logical & Bitwise
    Not,    // !
    BitNot, // ~

    // Pre-increment / Pre-decrement
    PreInc, // ++
    PreDec, // --

    // Pointer & Reference
    AddressOf, // &
    Deref,     // *

    // Control & Keywords
    Try,    // try
    Await,  // await
    Spawn,  // spawn
    Delete, // delete
    Typeof, // typeof
}

impl PrefixUnaryOp {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Not => "!",
            Self::BitNot => "~",
            Self::PreInc => "++",
            Self::PreDec => "--",
            Self::AddressOf => "&",
            Self::Deref => "*",
            Self::Try => "try",
            Self::Await => "await",
            Self::Spawn => "spawn",
            Self::Delete => "delete",
            Self::Typeof => "typeof",
        }
    }
}

impl fmt::Display for PrefixUnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Postfix unary operator variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostfixUnaryOp {
    PostInc,  // ++
    PostDec,  // --
    NonNull,  // ! (non-null assertion)
}

impl PostfixUnaryOp {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PostInc => "++",
            Self::PostDec => "--",
            Self::NonNull => "!",
        }
    }
}

impl fmt::Display for PostfixUnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Assignment operator variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignOp {
    Assign,               // =
    AddAssign,            // +=
    SubAssign,            // -=
    MulAssign,            // *=
    DivAssign,            // /=
    RemAssign,            // %=
    PowAssign,            // **=
    ShlAssign,            // <<=
    ShrAssign,            // >>=
    UshrAssign,           // >>>=
    BitAndAssign,         // &=
    BitOrAssign,          // |=
    BitXorAssign,         // ^=
    LogicalAndAssign,     // &&=
    LogicalOrAssign,      // ||=
    NullCoalescingAssign, // ??=
}

impl AssignOp {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
            Self::RemAssign => "%=",
            Self::PowAssign => "**=",
            Self::ShlAssign => "<<=",
            Self::ShrAssign => ">>=",
            Self::UshrAssign => ">>>=",
            Self::BitAndAssign => "&=",
            Self::BitOrAssign => "|=",
            Self::BitXorAssign => "^=",
            Self::LogicalAndAssign => "&&=",
            Self::LogicalOrAssign => "||=",
            Self::NullCoalescingAssign => "??=",
        }
    }

    /// Converts a compound assignment operator to its corresponding binary operator,
    /// or `None` if this is a simple assignment `=`.
    pub const fn to_binary_op(&self) -> Option<BinaryOp> {
        match self {
            Self::Assign => None,
            Self::AddAssign => Some(BinaryOp::Add),
            Self::SubAssign => Some(BinaryOp::Sub),
            Self::MulAssign => Some(BinaryOp::Mul),
            Self::DivAssign => Some(BinaryOp::Div),
            Self::RemAssign => Some(BinaryOp::Rem),
            Self::PowAssign => Some(BinaryOp::Pow),
            Self::ShlAssign => Some(BinaryOp::Shl),
            Self::ShrAssign => Some(BinaryOp::Shr),
            Self::UshrAssign => Some(BinaryOp::Ushr),
            Self::BitAndAssign => Some(BinaryOp::BitAnd),
            Self::BitOrAssign => Some(BinaryOp::BitOr),
            Self::BitXorAssign => Some(BinaryOp::BitXor),
            Self::LogicalAndAssign => Some(BinaryOp::LogicalAnd),
            Self::LogicalOrAssign => Some(BinaryOp::LogicalOr),
            Self::NullCoalescingAssign => Some(BinaryOp::NullCoalescing),
        }
    }
}

impl fmt::Display for AssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
