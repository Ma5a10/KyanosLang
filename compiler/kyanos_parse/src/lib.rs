// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Recursive-descent and Pratt parser implementation for the Kyanos programming language.
//!
//! Reference grammar: `design/grammer/kyanos_syntax_grammar.g4`.

pub mod decl;
pub mod error;
pub mod expr;
pub mod literal;
pub mod pattern;
pub mod stmt;
pub mod stream;
pub mod types;

use kyanos_ast::{Expr, NodeId, Program, Span};
use kyanos_lexer::{Token, TokenKind};

pub use error::{ParseError, ParseErrorKind, ParseResult};
pub use stream::{Checkpoint, TokenStream};

/// Parses a complete Kyanos compilation unit from source text.
pub fn parse(source: &str) -> ParseOutput {
    let mut parser = Parser::new(source);
    let program = parser.parse_program();
    ParseOutput {
        program,
        errors: parser.take_errors(),
    }
}

/// Parses a single expression from source text, returning any recovered node and diagnostics.
pub fn parse_expr(source: &str) -> (Expr, Vec<ParseError>) {
    let mut parser = Parser::new(source);
    let expr = parser.parse_expr_with_recovery();
    if !parser.is_eof() {
        let trailing = parser.current();
        parser.report_error(ParseError::unexpected(
            "end of expression",
            trailing.kind,
            trailing.span,
        ));
    }
    (expr, parser.take_errors())
}

/// Complete output of parsing a Kyanos compilation unit.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutput {
    pub program: Program,
    pub errors: Vec<ParseError>,
}

impl ParseOutput {
    /// Returns `true` if any lexical or syntax error was recorded.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Handcrafted fault-tolerant recursive-descent + Pratt parser for Kyanos.
pub struct Parser<'a> {
    stream: TokenStream<'a>,
    errors: Vec<ParseError>,
    next_node_id: u32,
    /// Nesting depth of `(...)` and `[...]` groupings allowing multiline expressions.
    pub(crate) grouping_depth: u32,
    /// Allows `else` to terminate an immediately enclosed braceless `if` then-branch.
    pub(crate) eos_allows_else: bool,
    /// Allows `while` to terminate an immediately enclosed braceless `do` body.
    pub(crate) eos_allows_while: bool,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for the given Kyanos source code.
    pub fn new(source: &'a str) -> Self {
        Self::with_offset(source, 0, 0)
    }

    /// Creates a parser with a starting byte offset and initial `NodeId` counter.
    ///
    /// Used when recursively parsing embedded expressions (e.g. inside template strings)
    /// so generated `Span`s and `NodeId`s remain globally consistent within the module.
    pub fn with_offset(source: &'a str, base_offset: usize, start_node_id: u32) -> Self {
        let (stream, errors) = TokenStream::with_offset(source, base_offset);
        Self {
            stream,
            errors,
            next_node_id: start_node_id,
            grouping_depth: 0,
            eos_allows_else: false,
            eos_allows_while: false,
        }
    }

    /// Creates a parser directly from an existing [`TokenStream`].
    pub fn from_stream(stream: TokenStream<'a>) -> Self {
        Self {
            stream,
            errors: Vec::new(),
            next_node_id: 0,
            grouping_depth: 0,
            eos_allows_else: false,
            eos_allows_while: false,
        }
    }

    /// Parses an entire Kyanos compilation unit into a [`Program`] AST.
    ///
    /// Any syntax diagnostics encountered during parsing are recorded in [`Self::errors`].
    pub fn parse_program(&mut self) -> Program {
        self.parse_program_inner()
    }

    /// Returns all accumulated parse and lexer diagnostics.
    #[inline]
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Consumes and returns all accumulated diagnostics, leaving the internal buffer empty.
    #[inline]
    pub fn take_errors(&mut self) -> Vec<ParseError> {
        std::mem::take(&mut self.errors)
    }

    /// Returns the next sequential [`NodeId`] value that would be assigned.
    #[inline]
    pub fn next_node_id_raw(&self) -> u32 {
        self.next_node_id
    }

    /// Allocates a fresh sequential [`NodeId`] for an AST node.
    #[inline]
    pub fn alloc_id(&mut self) -> NodeId {
        let id = NodeId::new(self.next_node_id);
        self.next_node_id = self.next_node_id.saturating_add(1);
        id
    }

    /// Records a diagnostic error into the parser's error list.
    #[inline]
    pub fn report_error(&mut self, err: ParseError) {
        self.errors.push(err);
    }

    // -------------------------------------------------------------------------
    // Token Cursor & Lookahead Helpers
    // -------------------------------------------------------------------------

    /// Returns the current unconsumed token.
    #[inline]
    pub fn current(&self) -> Token {
        self.stream.current()
    }

    /// Returns the [`TokenKind`] of the current token.
    #[inline]
    pub fn current_kind(&self) -> TokenKind {
        self.stream.current().kind
    }

    /// Returns the [`Span`] of the current token.
    #[inline]
    pub fn current_span(&self) -> Span {
        self.stream.current().span
    }

    /// Returns the source slice corresponding to `span`.
    #[inline]
    pub fn slice(&self, span: Span) -> &'a str {
        self.stream.slice(span)
    }

    /// Returns the source slice of the current token.
    #[inline]
    pub fn current_slice(&self) -> &'a str {
        self.stream.slice(self.current_span())
    }

    /// Peeks `offset` tokens ahead (`offset = 0` is the current token).
    #[inline]
    pub fn peek(&self, offset: usize) -> Token {
        self.stream.peek(offset)
    }

    /// Peeks the [`TokenKind`] `offset` tokens ahead.
    #[inline]
    pub fn peek_kind(&self, offset: usize) -> TokenKind {
        self.stream.peek(offset).kind
    }

    /// Returns `true` if the parser has reached end-of-file.
    #[inline]
    pub fn is_eof(&self) -> bool {
        self.stream.is_eof()
    }

    /// Returns `true` if the current token has the given [`TokenKind`].
    #[inline]
    pub fn at(&self, kind: TokenKind) -> bool {
        self.current_kind() == kind
    }

    /// Returns `true` if the current token matches any kind in `kinds`.
    #[inline]
    pub fn at_any(&self, kinds: &[TokenKind]) -> bool {
        let kind = self.current_kind();
        kinds.contains(&kind)
    }

    /// Returns `true` if the current token is an `Identifier` matching `text`
    /// (used for contextual keywords such as `get` and `set`).
    #[inline]
    pub fn at_ident_str(&self, text: &str) -> bool {
        self.at(TokenKind::Identifier) && self.current_slice() == text
    }

    /// Consumes and returns the current token.
    #[inline]
    pub fn bump(&mut self) -> Token {
        self.stream.bump()
    }

    /// Consumes the current token if its kind matches `kind`, otherwise returns `None`.
    #[inline]
    pub fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    /// Expects the current token to match `kind`, consuming it or returning an error.
    pub fn expect(&mut self, kind: TokenKind, expected: &str) -> ParseResult<Token> {
        if self.at(kind) {
            Ok(self.bump())
        } else {
            let tok = self.current();
            Err(ParseError::unexpected(expected, tok.kind, tok.span))
        }
    }

    /// Attempts to consume a generic closing `>` (splitting `>>`, `>>>`, `>=`, etc. if needed).
    #[inline]
    pub fn eat_generic_gt(&mut self) -> Option<Span> {
        self.stream.eat_generic_gt()
    }

    /// Expects a generic closing `>` (splitting `>>`, `>>>`, `>=`, etc. if needed).
    pub fn expect_generic_gt(&mut self, expected: &str) -> ParseResult<Span> {
        if let Some(span) = self.eat_generic_gt() {
            Ok(span)
        } else {
            let tok = self.current();
            Err(ParseError::unexpected(expected, tok.kind, tok.span))
        }
    }

    /// Merges `start` with the span of the most recently consumed token.
    #[inline]
    pub fn span_from(&self, start: Span) -> Span {
        start.merge(self.stream.last_span())
    }

    // -------------------------------------------------------------------------
    // Speculative Checkpointing
    // -------------------------------------------------------------------------

    /// Saves the current parser cursor and diagnostic count for backtracking.
    #[inline]
    pub fn checkpoint(&self) -> Checkpoint {
        self.stream
            .save_position(self.errors.len(), self.grouping_depth)
    }

    /// Rewinds the parser cursor and truncates diagnostics back to `cp`.
    #[inline]
    pub fn restore(&mut self, cp: Checkpoint) {
        self.stream.restore_position(&cp);
        self.errors.truncate(cp.error_len);
        self.grouping_depth = cp.grouping_depth;
    }

    /// Executes `f` inside a delimited grouping (`(...)` or `[...]`), permitting
    /// multi-line binary and postfix expression continuations.
    #[inline]
    pub(crate) fn with_grouping<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.grouping_depth += 1;
        let res = f(self);
        self.grouping_depth -= 1;
        res
    }

    // -------------------------------------------------------------------------
    // Swift-Style End-of-Statement (`eos`) & Error Recovery
    // -------------------------------------------------------------------------

    /// Validates and consumes a Swift-style statement terminator (`eos`).
    ///
    /// Succeeds when:
    /// 1. An explicit `;` is present (consumed),
    /// 2. The next token is `}` or `EOF` (not consumed),
    /// 3. The next token is `else` / `while` closing a braceless `if` / `do` branch, or
    /// 4. The next token was preceded by one or more line terminators (not consumed).
    pub fn parse_eos(&mut self) -> ParseResult<()> {
        if self.eat(TokenKind::Semicolon).is_some() {
            return Ok(());
        }

        let tok = self.current();
        if matches!(tok.kind, TokenKind::CloseBrace | TokenKind::Eof)
            || tok.has_leading_newline
            || (self.eos_allows_else && tok.kind == TokenKind::Else)
            || (self.eos_allows_while && tok.kind == TokenKind::While)
        {
            return Ok(());
        }

        Err(ParseError::expected_eos(tok.kind, tok.span))
    }

    /// Synchronizes the parser after a syntax error by advancing to the next likely
    /// statement or declaration boundary (`;`, `}`, `EOF`, or newline-leading keyword).
    pub fn synchronize(&mut self) {
        while !self.is_eof() {
            let tok = self.current();
            if tok.kind == TokenKind::Semicolon {
                self.bump();
                return;
            }
            if tok.kind == TokenKind::CloseBrace {
                return;
            }
            if tok.has_leading_newline && is_sync_start_keyword(tok.kind) {
                return;
            }
            self.bump();
        }
    }

    /// Consumes at least one token (unless at `}` or `EOF`) and then synchronizes
    /// to the next statement or declaration boundary, guaranteeing loop progress.
    pub fn advance_and_synchronize(&mut self) {
        if !self.is_eof() && !self.at(TokenKind::CloseBrace) {
            self.bump();
        }
        self.synchronize();
    }

    /// Synchronizes after an error, forcing at least one token advance only if the parser
    /// has not moved past `start_cursor` (preserving newline-aligned next declarations).
    pub(crate) fn synchronize_from(&mut self, start_cursor: usize) {
        if self.checkpoint().cursor > start_cursor {
            self.synchronize();
        } else {
            self.advance_and_synchronize();
        }
    }

    /// Synchronizes after a failed `parse_eos()` call, advancing past `;` or until the
    /// next line / enclosing `}` / `EOF`.
    pub fn synchronize_eos(&mut self) {
        while !self.is_eof() {
            let tok = self.current();
            if tok.kind == TokenKind::Semicolon {
                self.bump();
                return;
            }
            if tok.kind == TokenKind::CloseBrace || tok.has_leading_newline {
                return;
            }
            self.bump();
        }
    }
}

/// Returns `true` if `kind` unambiguously starts a top-level declaration or statement
/// when preceded by a line break, making it a safe synchronization point.
#[inline]
pub(crate) fn is_sync_start_keyword(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Import
            | TokenKind::Export
            | TokenKind::Let
            | TokenKind::Const
            | TokenKind::Using
            | TokenKind::Function
            | TokenKind::Async
            | TokenKind::Struct
            | TokenKind::Class
            | TokenKind::Interface
            | TokenKind::Enum
            | TokenKind::Type
            | TokenKind::Extern
            | TokenKind::HashBracket
            | TokenKind::If
            | TokenKind::For
            | TokenKind::While
            | TokenKind::Do
            | TokenKind::Switch
            | TokenKind::Return
            | TokenKind::Break
            | TokenKind::Continue
            | TokenKind::Defer
            | TokenKind::Unsafe
            | TokenKind::Debugger
    )
}
