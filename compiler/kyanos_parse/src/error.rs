// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Diagnostic error types for the Kyanos parser.

use std::fmt;

use kyanos_ast::Span;
use kyanos_lexer::TokenKind;

/// Convenience result alias used by parser routines.
pub type ParseResult<T> = Result<T, ParseError>;

/// Categorized syntax error variants produced by [`crate::Parser`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Lexical scanning failed before or during token extraction.
    LexerError(String),
    /// Encountered a token that does not match the expected grammar rule.
    UnexpectedToken { expected: String, found: TokenKind },
    /// Reached end-of-file while more syntax was expected.
    UnexpectedEof { expected: String },
    /// Missing statement terminator (`;` or line break before the next token).
    ExpectedEos { found: TokenKind },
    /// Literal token could not be decoded into a valid AST value.
    InvalidLiteral(String),
    /// Left-hand side of an assignment expression is not assignable.
    InvalidAssignmentTarget,
    /// `const` declaration is missing its mandatory initializer (`= expr`).
    MissingInitializer,
    /// Syntax nesting exceeded the parser's maximum recursion depth.
    RecursionLimitExceeded,
    /// General contextual syntax error.
    SyntaxError(String),
}

/// Heap-allocated payload for [`ParseError`] to keep stack frames compact.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParseErrorInner {
    kind: ParseErrorKind,
    span: Span,
    message: String,
    hint: Option<String>,
}

/// A syntax diagnostic with source location and optional recovery hint.
///
/// The payload is boxed so `Result<T, ParseError>` remains pointer-sized on
/// recursive parser paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(Box<ParseErrorInner>);

impl ParseError {
    /// Creates a new parse error with an explicit kind, span, and message.
    pub fn new(kind: ParseErrorKind, span: Span, message: impl Into<String>) -> Self {
        Self(Box::new(ParseErrorInner {
            kind,
            span,
            message: message.into(),
            hint: None,
        }))
    }

    /// Returns the error category.
    pub fn kind(&self) -> &ParseErrorKind {
        &self.0.kind
    }

    /// Returns the source span where the error occurred.
    pub fn span(&self) -> Span {
        self.0.span
    }

    /// Returns the human-readable error message.
    pub fn message(&self) -> &str {
        &self.0.message
    }

    /// Returns the optional developer-facing recovery hint.
    pub fn hint(&self) -> Option<&str> {
        self.0.hint.as_deref()
    }

    /// Attaches an optional developer-facing recovery hint to this error.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.0.hint = Some(hint.into());
        self
    }

    /// Constructs a lexer error wrapped as a parse diagnostic.
    pub fn lexer(message: impl Into<String>, span: Span) -> Self {
        let msg = message.into();
        Self::new(
            ParseErrorKind::LexerError(msg.clone()),
            span,
            format!("Lexical error: {}", msg),
        )
    }

    /// Constructs an unexpected-token or unexpected-EOF diagnostic depending on `found`.
    pub fn unexpected(expected: impl Into<String>, found: TokenKind, span: Span) -> Self {
        let expected_str = expected.into();
        if found == TokenKind::Eof {
            Self::new(
                ParseErrorKind::UnexpectedEof {
                    expected: expected_str.clone(),
                },
                span,
                format!("Unexpected end of file: expected {}", expected_str),
            )
        } else {
            Self::new(
                ParseErrorKind::UnexpectedToken {
                    expected: expected_str.clone(),
                    found,
                },
                span,
                format!(
                    "Expected {}, found {}",
                    expected_str,
                    token_kind_display(found)
                ),
            )
        }
    }

    /// Constructs a diagnostic for a missing Swift-style statement terminator.
    pub fn expected_eos(found: TokenKind, span: Span) -> Self {
        Self::new(
            ParseErrorKind::ExpectedEos { found },
            span,
            format!(
                "Expected ';' or newline after statement, found {}",
                token_kind_display(found)
            ),
        )
        .with_hint(
            "Separate statements on the same line with ';', or place them on separate lines.",
        )
    }

    /// Constructs a contextual syntax error with a custom message.
    pub fn syntax(message: impl Into<String>, span: Span) -> Self {
        let msg = message.into();
        Self::new(ParseErrorKind::SyntaxError(msg.clone()), span, msg)
    }

    /// Constructs a diagnostic when syntax nesting exceeds [`crate::MAX_RECURSION_DEPTH`].
    pub fn recursion_limit(span: Span) -> Self {
        Self::recursion_limit_with_max(span, crate::MAX_RECURSION_DEPTH)
    }

    /// Constructs a diagnostic when syntax nesting exceeds an explicit `max_depth` limit.
    pub fn recursion_limit_with_max(span: Span, max_depth: usize) -> Self {
        Self::new(
            ParseErrorKind::RecursionLimitExceeded,
            span,
            format!("Maximum syntax recursion depth ({}) exceeded", max_depth),
        )
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (at {}..{})",
            self.message(),
            self.span().start,
            self.span().end
        )?;
        if let Some(hint) = self.hint() {
            write!(f, " [hint: {}]", hint)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// Returns a user-facing label for `kind` in parser error messages.
pub fn token_kind_display(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Whitespace => "whitespace",
        TokenKind::LineTerminator => "newline",
        TokenKind::SingleLineComment | TokenKind::MultiLineComment => "comment",

        TokenKind::Let => "'let'",
        TokenKind::Const => "'const'",
        TokenKind::Function => "'function'",
        TokenKind::Class => "'class'",
        TokenKind::Struct => "'struct'",
        TokenKind::Interface => "'interface'",
        TokenKind::Enum => "'enum'",
        TokenKind::Type => "'type'",
        TokenKind::Constructor => "'constructor'",

        TokenKind::If => "'if'",
        TokenKind::Else => "'else'",
        TokenKind::Do => "'do'",
        TokenKind::While => "'while'",
        TokenKind::For => "'for'",
        TokenKind::In => "'in'",
        TokenKind::Of => "'of'",
        TokenKind::Return => "'return'",
        TokenKind::Break => "'break'",
        TokenKind::Continue => "'continue'",
        TokenKind::Switch => "'switch'",
        TokenKind::Case => "'case'",
        TokenKind::Default => "'default'",
        TokenKind::When => "'when'",
        TokenKind::Debugger => "'debugger'",

        TokenKind::Try => "'try'",
        TokenKind::Defer => "'defer'",
        TokenKind::Using => "'using'",

        TokenKind::Public => "'public'",
        TokenKind::Protected => "'protected'",
        TokenKind::Private => "'private'",
        TokenKind::Static => "'static'",
        TokenKind::Readonly => "'readonly'",

        TokenKind::New => "'new'",
        TokenKind::This => "'this'",
        TokenKind::Super => "'super'",
        TokenKind::Extends => "'extends'",
        TokenKind::Implements => "'implements'",
        TokenKind::As => "'as'",

        TokenKind::Import => "'import'",
        TokenKind::Export => "'export'",
        TokenKind::From => "'from'",

        TokenKind::Async => "'async'",
        TokenKind::Await => "'await'",
        TokenKind::Yield => "'yield'",
        TokenKind::YieldStar => "'yield*'",
        TokenKind::Spawn => "'spawn'",

        TokenKind::Unsafe => "'unsafe'",
        TokenKind::Extern => "'extern'",
        TokenKind::Delete => "'delete'",

        TokenKind::True => "'true'",
        TokenKind::False => "'false'",
        TokenKind::Null => "'null'",

        TokenKind::TypeByte => "'byte'",
        TokenKind::TypeShort => "'short'",
        TokenKind::TypeInt => "'int'",
        TokenKind::TypeLong => "'long'",
        TokenKind::TypeIsize => "'isize'",
        TokenKind::TypeSizeT => "'size_t'",
        TokenKind::TypeUbyte => "'ubyte'",
        TokenKind::TypeUshort => "'ushort'",
        TokenKind::TypeUint => "'uint'",
        TokenKind::TypeUlong => "'ulong'",
        TokenKind::TypeFloat => "'float'",
        TokenKind::TypeDouble => "'double'",
        TokenKind::TypeBool => "'bool'",
        TokenKind::TypeChar => "'char'",
        TokenKind::TypeString => "'string'",
        TokenKind::TypeVoid => "'void'",
        TokenKind::TypeNever => "'never'",
        TokenKind::TypeRawptr => "'rawptr'",

        TokenKind::OpenBrace => "'{'",
        TokenKind::CloseBrace => "'}'",
        TokenKind::OpenParen => "'('",
        TokenKind::CloseParen => "')'",
        TokenKind::OpenBracket => "'['",
        TokenKind::CloseBracket => "']'",
        TokenKind::Dot => "'.'",
        TokenKind::Ellipsis => "'...'",
        TokenKind::Semicolon => "';'",
        TokenKind::Comma => "','",
        TokenKind::Colon => "':'",
        TokenKind::Question => "'?'",
        TokenKind::OptionalChain => "'?.'",
        TokenKind::NullCoalescing => "'??'",
        TokenKind::Arrow => "'=>'",
        TokenKind::HashBracket => "'#['",
        TokenKind::Underscore => "'_'",

        TokenKind::Plus => "'+'",
        TokenKind::Minus => "'-'",
        TokenKind::Star => "'*'",
        TokenKind::Slash => "'/'",
        TokenKind::Percent => "'%'",
        TokenKind::Power => "'**'",
        TokenKind::PlusPlus => "'++'",
        TokenKind::MinusMinus => "'--'",
        TokenKind::And => "'&'",
        TokenKind::Bar => "'|'",
        TokenKind::Caret => "'^'",
        TokenKind::Tilde => "'~'",
        TokenKind::ShiftLeft => "'<<'",
        TokenKind::ShiftRight => "'>>'",
        TokenKind::UnsignedShiftRight => "'>>>'",

        TokenKind::Bang => "'!'",
        TokenKind::AndAnd => "'&&'",
        TokenKind::BarBar => "'||'",
        TokenKind::Equals => "'=='",
        TokenKind::NotEquals => "'!='",
        TokenKind::LessThan => "'<'",
        TokenKind::LessThanEquals => "'<='",
        TokenKind::GreaterThan => "'>'",
        TokenKind::GreaterThanEquals => "'>='",

        TokenKind::Assign => "'='",
        TokenKind::PlusAssign => "'+='",
        TokenKind::MinusAssign => "'-='",
        TokenKind::StarAssign => "'*='",
        TokenKind::SlashAssign => "'/='",
        TokenKind::PercentAssign => "'%='",
        TokenKind::PowerAssign => "'**='",
        TokenKind::ShiftLeftAssign => "'<<='",
        TokenKind::ShiftRightAssign => "'>>='",
        TokenKind::UnsignedShiftRightAssign => "'>>>='",
        TokenKind::AndAssign => "'&='",
        TokenKind::BarAssign => "'|='",
        TokenKind::CaretAssign => "'^='",
        TokenKind::AndAndAssign => "'&&='",
        TokenKind::BarBarAssign => "'||='",
        TokenKind::NullCoalescingAssign => "'??='",

        TokenKind::Is => "'is'",
        TokenKind::Typeof => "'typeof'",
        TokenKind::Sizeof => "'sizeof'",

        TokenKind::IntegerLiteral => "integer literal",
        TokenKind::FloatLiteral => "float literal",
        TokenKind::BigIntLiteral => "bigint literal",
        TokenKind::StringLiteral => "string literal",
        TokenKind::CStringLiteral => "C-string literal",
        TokenKind::CharLiteral => "character literal",
        TokenKind::TemplateNoSubstitution
        | TokenKind::TemplateHead
        | TokenKind::TemplateMiddle
        | TokenKind::TemplateTail => "template literal",

        TokenKind::Identifier => "identifier",
        TokenKind::PrivateIdentifier => "private identifier",

        TokenKind::Eof => "end of file",
        TokenKind::Error => "invalid token",
    }
}
