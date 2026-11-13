// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Operators and punctuators in Kyanos expressions.

use std::fmt;

/// Unified Pratt parser operator precedence levels (`1..=17`, higher binds tighter)
/// and binding powers matching `design/grammer/kyanos_syntax_grammar.g4`.
pub mod precedence {
    pub const ASSIGN: u8 = 1;
    pub const TERNARY: u8 = 2;
    pub const NULL_COALESCING: u8 = 3;
    pub const LOGICAL_OR: u8 = 4;
    pub const LOGICAL_AND: u8 = 5;
    pub const BIT_OR: u8 = 6;
    pub const BIT_XOR: u8 = 7;
    pub const BIT_AND: u8 = 8;
    pub const EQUALITY: u8 = 9;
    pub const RELATIONAL: u8 = 10;
    pub const TYPE_TEST: u8 = RELATIONAL;
    pub const SHIFT: u8 = 11;
    pub const ADDITIVE: u8 = 12;
    pub const MULTIPLICATIVE: u8 = 13;
    pub const POWER: u8 = 14;
    pub const CAST: u8 = 15;
    pub const PREFIX: u8 = 16;
    pub const POSTFIX: u8 = 17;

    /// Assignment binding power `(left_bp, right_bp)`, right-associative.
    pub const ASSIGN_BP: (u8, u8) = (ASSIGN * 2 + 1, ASSIGN * 2);
    /// Ternary (`? :`) binding power `(left_bp, right_bp)`, right-associative.
    pub const TERNARY_BP: (u8, u8) = (TERNARY * 2 + 1, TERNARY * 2);
    /// Type test (`expr is Type`) infix binding power `(left_bp, right_bp)`.
    pub const TYPE_TEST_BP: (u8, u8) = (TYPE_TEST * 2, TYPE_TEST * 2 + 1);
    /// Type cast (`expr as Type`) left binding power.
    pub const CAST_LEFT_BP: u8 = CAST * 2;
    /// Prefix unary / `new` right binding power.
    pub const PREFIX_RIGHT_BP: u8 = PREFIX * 2 + 1;
    /// Postfix (`()`, `[]`, `.`, `?.`, `++`, `--`, `!`) left binding power.
    pub const POSTFIX_LEFT_BP: u8 = POSTFIX * 2 + 1;
}

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
    Shl,    // <<
    Shr,    // >>
    Ushr,   // >>>
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
            Self::NullCoalescing => precedence::NULL_COALESCING,
            Self::LogicalOr => precedence::LOGICAL_OR,
            Self::LogicalAnd => precedence::LOGICAL_AND,
            Self::BitOr => precedence::BIT_OR,
            Self::BitXor => precedence::BIT_XOR,
            Self::BitAnd => precedence::BIT_AND,
            Self::Eq | Self::Ne => precedence::EQUALITY,
            Self::Lt | Self::Le | Self::Gt | Self::Ge => precedence::RELATIONAL,
            Self::Shl | Self::Shr | Self::Ushr => precedence::SHIFT,
            Self::Add | Self::Sub => precedence::ADDITIVE,
            Self::Mul | Self::Div | Self::Rem => precedence::MULTIPLICATIVE,
            Self::Pow => precedence::POWER,
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

    /// Returns the unified precedence level for prefix unary operators.
    pub const fn precedence(&self) -> u8 {
        precedence::PREFIX
    }

    /// Returns the Pratt parser right binding power for prefix unary operators.
    pub const fn right_binding_power(&self) -> u8 {
        precedence::PREFIX_RIGHT_BP
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
    PostInc, // ++
    PostDec, // --
    NonNull, // ! (non-null assertion)
}

impl PostfixUnaryOp {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PostInc => "++",
            Self::PostDec => "--",
            Self::NonNull => "!",
        }
    }

    /// Returns the unified precedence level for postfix unary operators.
    pub const fn precedence(&self) -> u8 {
        precedence::POSTFIX
    }

    /// Returns the Pratt parser left binding power for postfix unary operators.
    pub const fn left_binding_power(&self) -> u8 {
        precedence::POSTFIX_LEFT_BP
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

    /// Returns the unified precedence level for assignment operators.
    pub const fn precedence(&self) -> u8 {
        precedence::ASSIGN
    }

    /// Assignment operators are always right-associative.
    pub const fn associativity(&self) -> Associativity {
        Associativity::Right
    }

    /// Returns the Pratt parser binding powers `(left_bp, right_bp)` for assignment.
    pub const fn binding_power(&self) -> (u8, u8) {
        precedence::ASSIGN_BP
    }
}

impl fmt::Display for AssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
