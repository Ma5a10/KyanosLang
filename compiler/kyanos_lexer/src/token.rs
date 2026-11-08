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
    // Whitespace & Comments (Trivia)
    Whitespace,
    LineTerminator,
    SingleLineComment,
    MultiLineComment,

    // Keywords
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

    // Primitive Type Keywords
    TypeByte,   // 'byte' | 'i8'
    TypeShort,  // 'short' | 'i16'
    TypeInt,    // 'int' | 'i32'
    TypeLong,   // 'long' | 'i64'
    TypeSizeT,  // 'size_t' | 'isize'
    TypeUbyte,  // 'ubyte' | 'u8'
    TypeUshort, // 'ushort' | 'u16'
    TypeUint,   // 'uint' | 'u32'
    TypeUlong,  // 'ulong' | 'u64'
    TypeFloat,  // 'float' | 'f32'
    TypeDouble, // 'double' | 'f64'
    TypeBool,   // 'bool'
    TypeChar,   // 'char'
    TypeString, // 'string'
    TypeVoid,   // 'void'
    TypeNever,  // 'never'
    TypeRawptr, // 'rawptr'

    // Punctuators & Delimiters
    OpenBrace,      // '{'
    CloseBrace,     // '}'
    OpenParen,      // '('
    CloseParen,     // ')'
    OpenBracket,    // '['
    CloseBracket,   // ']'
    Dot,            // '.'
    Ellipsis,       // '...'
    Semicolon,      // ';'
    Comma,          // ','
    Colon,          // ':'
    Question,       // '?'
    OptionalChain,  // '?.'
    NullCoalescing, // '??'
    Arrow,          // '=>'
    HashBracket,    // '#['
    Underscore,     // '_'

    // Arithmetic & Bitwise Operators
    Plus,               // '+'
    Minus,              // '-'
    Star,               // '*'
    Slash,              // '/'
    Percent,            // '%'
    Power,              // '**'
    PlusPlus,           // '++'
    MinusMinus,         // '--'
    And,                // '&'
    Bar,                // '|'
    Caret,              // '^'
    Tilde,              // '~'
    ShiftLeft,          // '<<'
    ShiftRight,         // '>>'
    UnsignedShiftRight, // '>>>'

    // Logical & Relational Operators
    Bang,              // '!'
    AndAnd,            // '&&'
    BarBar,            // '||'
    Equals,            // '=='
    NotEquals,         // '!='
    LessThan,          // '<'
    LessThanEquals,    // '<='
    GreaterThan,       // '>'
    GreaterThanEquals, // '>='

    // Assignment Operators
    Assign,                   // '='
    PlusAssign,               // '+='
    MinusAssign,              // '-='
    StarAssign,               // '*='
    SlashAssign,              // '/='
    PercentAssign,            // '%='
    PowerAssign,              // '**='
    ShiftLeftAssign,          // '<<='
    ShiftRightAssign,         // '>>='
    UnsignedShiftRightAssign, // '>>>='
    AndAssign,                // '&='
    BarAssign,                // '|='
    CaretAssign,              // '^='
    AndAndAssign,             // '&&='
    BarBarAssign,             // '||='
    NullCoalescingAssign,     // '??='

    // Type Operators
    Is,
    Typeof,
    Sizeof,

    // Literals
    IntegerLiteral,
    FloatLiteral,
    BigIntLiteral,
    StringLiteral,
    CStringLiteral,
    CharLiteral,
    TemplateStringLiteral,

    // Identifiers
    Identifier,
    PrivateIdentifier, // '#identifier'

    // Special Tokens
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
                | Self::BarAssign
                | Self::CaretAssign
                | Self::AndAndAssign
                | Self::BarBarAssign
                | Self::NullCoalescingAssign
        )
    }

    /// keyword convert from string
    pub fn from_keyword(ident: &str) -> Option<TokenKind> {
        match ident {
            // Declarations & Definitions
            "let" => Some(TokenKind::Let),
            "const" => Some(TokenKind::Const),
            "function" => Some(TokenKind::Function),
            "class" => Some(TokenKind::Class),
            "struct" => Some(TokenKind::Struct),
            "interface" => Some(TokenKind::Interface),
            "enum" => Some(TokenKind::Enum),
            "constructor" => Some(TokenKind::Constructor),
            // Control flow
            "if" => Some(TokenKind::If),
            "else" => Some(TokenKind::Else),
            "do" => Some(TokenKind::Do),
            "while" => Some(TokenKind::While),
            "for" => Some(TokenKind::For),
            "in" => Some(TokenKind::In),
            "of" => Some(TokenKind::Of),
            "return" => Some(TokenKind::Return),
            "break" => Some(TokenKind::Break),
            "continue" => Some(TokenKind::Continue),
            "switch" => Some(TokenKind::Switch),
            "case" => Some(TokenKind::Case),
            "default" => Some(TokenKind::Default),
            "when" => Some(TokenKind::When),
            "debugger" => Some(TokenKind::Debugger),
            // Resource & Error handling
            "try" => Some(TokenKind::Try),
            "defer" => Some(TokenKind::Defer),
            "using" => Some(TokenKind::Using),
            // Modifiers
            "public" => Some(TokenKind::Public),
            "protected" => Some(TokenKind::Protected),
            "private" => Some(TokenKind::Private),
            "static" => Some(TokenKind::Static),
            "readonly" => Some(TokenKind::Readonly),
            // OOP & Types
            "new" => Some(TokenKind::New),
            "this" => Some(TokenKind::This),
            "super" => Some(TokenKind::Super),
            "extends" => Some(TokenKind::Extends),
            "implements" => Some(TokenKind::Implements),
            "as" => Some(TokenKind::As),
            // Modules
            "import" => Some(TokenKind::Import),
            "export" => Some(TokenKind::Export),
            "from" => Some(TokenKind::From),
            // Async & Concurrency
            "async" => Some(TokenKind::Async),
            "await" => Some(TokenKind::Await),
            "yield" => Some(TokenKind::Yield),
            "spawn" => Some(TokenKind::Spawn),
            // Low-level & Memory
            "unsafe" => Some(TokenKind::Unsafe),
            "extern" => Some(TokenKind::Extern),
            "delete" => Some(TokenKind::Delete),
            // Literals
            "true" => Some(TokenKind::True),
            "false" => Some(TokenKind::False),
            "null" => Some(TokenKind::Null),
            // Primitive Types (Aliases included)
            "byte" | "i8" => Some(TokenKind::TypeByte),
            "short" | "i16" => Some(TokenKind::TypeShort),
            "int" | "i32" => Some(TokenKind::TypeInt),
            "long" | "i64" => Some(TokenKind::TypeLong),
            "size_t" | "isize" => Some(TokenKind::TypeSizeT),
            "ubyte" | "u8" => Some(TokenKind::TypeUbyte),
            "ushort" | "u16" => Some(TokenKind::TypeUshort),
            "uint" | "u32" => Some(TokenKind::TypeUint),
            "ulong" | "u64" => Some(TokenKind::TypeUlong),
            "float" | "f32" => Some(TokenKind::TypeFloat),
            "double" | "f64" => Some(TokenKind::TypeDouble),
            "bool" => Some(TokenKind::TypeBool),
            "char" => Some(TokenKind::TypeChar),
            "string" => Some(TokenKind::TypeString),
            "void" => Some(TokenKind::TypeVoid),
            "never" => Some(TokenKind::TypeNever),
            "rawptr" => Some(TokenKind::TypeRawptr),
            "is" => Some(TokenKind::Is),
            "typeof" => Some(TokenKind::Typeof),
            "sizeof" => Some(TokenKind::Sizeof),
            _ => None,
        }
    }
}
