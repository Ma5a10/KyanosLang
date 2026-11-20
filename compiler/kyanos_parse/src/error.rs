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
                format!("Expected {}, found {:?}", expected_str, found),
            )
        }
    }

    /// Constructs a diagnostic for a missing Swift-style statement terminator.
    pub fn expected_eos(found: TokenKind, span: Span) -> Self {
        Self::new(
            ParseErrorKind::ExpectedEos { found },
            span,
            format!("Expected ';' or newline after statement, found {:?}", found),
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
