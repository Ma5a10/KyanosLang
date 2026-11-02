// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use kyanos_ast::Span;

/// A single lexical token in Kyanos source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Token kinds corresponding to `kyanos_lex_grammar.g4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // =========================================================================
    // Whitespace & Comments (Trivia)
    // =========================================================================
    Whitespace,
    LineTerminator,
    SingleLineComment,
    MultiLineComment,

    // =========================================================================
    // Keywords
    // =========================================================================
    Let,
    Const,
    Function,
    Class,
    Struct,
    Interface,
    Enum,
    Constructor,

    If,
    Else,
    Do,
    While,
    For,
    In,
    Of,
    Return,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    When,
    Debugger,

    Try,
    Defer,
    Using,

    Public,
    Protected,
    Private,
    Static,
    Readonly,

    New,
    This,
    Super,
    Extends,
    Implements,
    As,

    Import,
    Export,
    From,

    Async,
    Await,
    Yield,
    YieldStar,
    Spawn,

    Unsafe,
    Extern,
    Delete,

    True,
    False,
    Null,

    // =========================================================================
    // Primitive Type Keywords
    // =========================================================================
    TypeByte,       // 'byte' | 'i8'
    TypeShort,      // 'short' | 'i16'
    TypeInt,        // 'int' | 'i32'
    TypeLong,       // 'long' | 'i64'
    TypeSizeT,      // 'size_t' | 'isize'
    TypeUbyte,      // 'ubyte' | 'u8'
    TypeUshort,     // 'ushort' | 'u16'
    TypeUint,       // 'uint' | 'u32'
    TypeUlong,      // 'ulong' | 'u64'
    TypeFloat,      // 'float' | 'f32'
    TypeDouble,     // 'double' | 'f64'
    TypeBool,       // 'bool'
    TypeChar,       // 'char'
    TypeString,     // 'string'
    TypeVoid,       // 'void'
    TypeNever,      // 'never'
    TypeRawptr,     // 'rawptr'

    // =========================================================================
    // Punctuators & Delimiters
    // =========================================================================
    OpenBrace,          // '{'
    CloseBrace,         // '}'
    OpenParen,          // '('
    CloseParen,         // ')'
    OpenBracket,        // '['
    CloseBracket,       // ']'
    Dot,                // '.'
    Ellipsis,           // '...'
    Semicolon,          // ';'
    Comma,              // ','
    Colon,              // ':'
    Question,           // '?'
    OptionalChain,      // '?.'
    NullCoalescing,     // '??'
    Arrow,              // '=>'
    HashBracket,        // '#['
    Underscore,         // '_'

    // =========================================================================
    // Arithmetic & Bitwise Operators
    // =========================================================================
    Plus,               // '+'
    Minus,              // '-'
    Star,               // '*'
    Slash,              // '/'
    Percent,            // '%'
    Power,              // '**'
    PlusPlus,           // '++'
    MinusMinus,         // '--'
    Amp,                // '&'
    Bar,                // '|'
    Caret,              // '^'
    Tilde,              // '~'
    ShiftLeft,          // '<<'
    ShiftRight,         // '>>'
    UnsignedShiftRight, // '>>>'

    // =========================================================================
    // Logical & Relational Operators
    // =========================================================================
    Bang,               // '!'
    AmpAmp,             // '&&'
    BarBar,             // '||'
    Equals,             // '=='
    NotEquals,          // '!='
    LessThan,           // '<'
    LessThanEquals,     // '<='
    GreaterThan,        // '>'
    GreaterThanEquals,  // '>='

    // =========================================================================
    // Assignment Operators
    // =========================================================================
    Assign,                 // '='
    PlusAssign,             // '+='
    MinusAssign,            // '-='
    StarAssign,             // '*='
    SlashAssign,            // '/='
    PercentAssign,          // '%='
    PowerAssign,            // '**='
    ShiftLeftAssign,        // '<<='
    ShiftRightAssign,       // '>>='
    UnsignedShiftRightAssign,// '>>>='
    AndAssign,              // '&='
    OrAssign,               // '|='
    XorAssign,              // '^='
    AmpAmpAssign,           // '&&='
    BarBarAssign,           // '||='
    NullCoalescingAssign,   // '??='

    // =========================================================================
    // Type Operators
    // =========================================================================
    Is,     // 'is'
    Typeof, // 'typeof'
    Sizeof, // 'sizeof'

    // =========================================================================
    // Literals
    // =========================================================================
    IntegerLiteral,
    FloatLiteral,
    BigIntLiteral,
    StringLiteral,
    CStringLiteral,
    CharLiteral,
    TemplateStringLiteral,

    // =========================================================================
    // Identifiers
    // =========================================================================
    Identifier,
    PrivateIdentifier, // '#identifier'

    // =========================================================================
    // Special Tokens
    // =========================================================================
    Eof,
    Error,
}

impl TokenKind {
    /// Returns true if this token is a trivia token (whitespace or comment).
    pub fn is_trivia(&self) -> bool {
        matches!(
            self,
            Self::Whitespace
                | Self::LineTerminator
                | Self::SingleLineComment
                | Self::MultiLineComment
        )
    }

    /// Returns true if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Self::Let
                | Self::Const
                | Self::Function
                | Self::Class
                | Self::Struct
                | Self::Interface
                | Self::Enum
                | Self::Constructor
                | Self::If
                | Self::Else
                | Self::Do
                | Self::While
                | Self::For
                | Self::In
                | Self::Of
                | Self::Return
                | Self::Break
                | Self::Continue
                | Self::Switch
                | Self::Case
                | Self::Default
                | Self::When
                | Self::Debugger
                | Self::Try
                | Self::Defer
                | Self::Using
                | Self::Public
                | Self::Protected
                | Self::Private
                | Self::Static
                | Self::Readonly
                | Self::New
                | Self::This
                | Self::Super
                | Self::Extends
                | Self::Implements
                | Self::As
                | Self::Import
                | Self::Export
                | Self::From
                | Self::Async
                | Self::Await
                | Self::Yield
                | Self::YieldStar
                | Self::Spawn
                | Self::Unsafe
                | Self::Extern
                | Self::Delete
                | Self::True
                | Self::False
                | Self::Null
                | Self::Is
                | Self::Typeof
                | Self::Sizeof
        )
    }

    /// Returns true if this token is a primitive type keyword.
    pub fn is_primitive_type(&self) -> bool {
        matches!(
            self,
            Self::TypeByte
                | Self::TypeShort
                | Self::TypeInt
                | Self::TypeLong
                | Self::TypeSizeT
                | Self::TypeUbyte
                | Self::TypeUshort
                | Self::TypeUint
                | Self::TypeUlong
                | Self::TypeFloat
                | Self::TypeDouble
                | Self::TypeBool
                | Self::TypeChar
                | Self::TypeString
                | Self::TypeVoid
                | Self::TypeNever
                | Self::TypeRawptr
        )
    }

    /// Returns true if this token is a literal.
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            Self::IntegerLiteral
                | Self::FloatLiteral
                | Self::BigIntLiteral
                | Self::StringLiteral
                | Self::CStringLiteral
                | Self::CharLiteral
                | Self::TemplateStringLiteral
                | Self::True
                | Self::False
                | Self::Null
        )
    }

    /// Returns true if this token is an assignment operator.
    pub fn is_assignment_operator(&self) -> bool {
        matches!(
            self,
            Self::Assign
                | Self::PlusAssign
                | Self::MinusAssign
                | Self::StarAssign
                | Self::SlashAssign
                | Self::PercentAssign
                | Self::PowerAssign
                | Self::ShiftLeftAssign
                | Self::ShiftRightAssign
                | Self::UnsignedShiftRightAssign
                | Self::AndAssign
                | Self::OrAssign
                | Self::XorAssign
                | Self::AmpAmpAssign
                | Self::BarBarAssign
                | Self::NullCoalescingAssign
        )
    }
}
