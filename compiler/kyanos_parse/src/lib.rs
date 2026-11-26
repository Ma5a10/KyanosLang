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

use std::collections::HashSet;

use kyanos_ast::{Expr, NodeId, Program, Span};
use kyanos_lexer::{Token, TokenKind};

pub use error::{ParseError, ParseErrorKind, ParseResult};
pub use stream::{Checkpoint, TokenStream};

/// Default maximum syntax nesting depth permitted by the parser (blocks, expressions, types, patterns).
pub const MAX_RECURSION_DEPTH: usize = 128;

/// Configuration options controlling parser behavior and safety limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    /// Maximum syntax nesting depth permitted before emitting [`ParseErrorKind::RecursionLimitExceeded`].
    pub max_recursion_depth: usize,
}

impl ParseOptions {
    /// Creates options initialized with default safety limits ([`MAX_RECURSION_DEPTH`]).
    pub const fn new() -> Self {
        Self {
            max_recursion_depth: MAX_RECURSION_DEPTH,
        }
    }

    /// Overrides the maximum syntax recursion depth limit.
    pub const fn with_max_recursion_depth(mut self, max_recursion_depth: usize) -> Self {
        self.max_recursion_depth = max_recursion_depth;
        self
    }
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Parses a complete Kyanos compilation unit from source text using default options.
pub fn parse(source: &str) -> ParseOutput {
    parse_with_options(source, ParseOptions::default())
}

/// Parses a complete Kyanos compilation unit from source text with custom [`ParseOptions`].
pub fn parse_with_options(source: &str, options: ParseOptions) -> ParseOutput {
    let mut parser = Parser::with_options(source, options);
    let program = parser.parse_program();
    ParseOutput {
        program,
        errors: parser.take_errors(),
    }
}

/// Parses a single expression from source text using default options, returning any recovered node and diagnostics.
pub fn parse_expr(source: &str) -> (Expr, Vec<ParseError>) {
    parse_expr_with_options(source, ParseOptions::default())
}

/// Parses a single expression from source text with custom [`ParseOptions`].
pub fn parse_expr_with_options(source: &str, options: ParseOptions) -> (Expr, Vec<ParseError>) {
    let mut parser = Parser::with_options(source, options);
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

/// Memoization caches preventing exponential/quadratic backtracking across speculative parser paths (CWE-407).
#[derive(Debug, Default)]
pub(crate) struct BacktrackMemo {
    /// Token indices of `(` in `parse_type` whose matching `)` is not followed by `=>`.
    pub(crate) non_arrow_type_parens: HashSet<usize>,
    /// Token indices of `(` in `parse_paren_or_arrow_expr` whose matching `)` is followed by neither `=>` nor `:`.
    pub(crate) non_arrow_expr_parens: HashSet<usize>,
    /// `(cursor, in_grouping)` keys where speculative arrow parameter parsing failed and must parse as parenthesized expr.
    pub(crate) failed_arrow_expr_parens: HashSet<(usize, bool)>,
    /// `(cursor, in_grouping)` keys where `sizeof(<type>)` failed and must fall back to an expression.
    pub(crate) failed_sizeof_types: HashSet<(usize, bool)>,
    /// `(cursor, in_grouping)` keys where `parse_type_arguments` failed with a syntax error.
    pub(crate) failed_type_arguments: HashSet<(usize, bool)>,
    /// `(cursor, in_grouping)` keys where `try_parse_generic_postfix` did not form a generic call/init.
    pub(crate) failed_generic_postfix: HashSet<(usize, bool)>,
    /// Token indices of `?` at depth 0 proven to have no matching `:` before the enclosing boundary.
    pub(crate) no_colon_after_questions: HashSet<usize>,
}

/// Handcrafted fault-tolerant recursive-descent + Pratt parser for Kyanos.
pub struct Parser<'a> {
    stream: TokenStream<'a>,
    errors: Vec<ParseError>,
    next_node_id: u32,
    /// Prevents emitting duplicate diagnostics once `next_node_id` reaches `NodeId::DUMMY`.
    node_id_exhausted: bool,
    /// Parser configuration options including recursion limits.
    options: ParseOptions,
    /// Memoization caches preserved across speculative `restore` calls.
    pub(crate) memo: BacktrackMemo,
    /// Nesting depth of `(...)` and `[...]` groupings allowing multiline expressions.
    pub(crate) grouping_depth: u32,
    /// Current recursive-descent nesting depth guarded against `options.max_recursion_depth`.
    pub(crate) recursion_depth: usize,
    /// Allows `else` to terminate an immediately enclosed braceless `if` then-branch.
    pub(crate) eos_allows_else: bool,
    /// Allows `while` to terminate an immediately enclosed braceless `do` body.
    pub(crate) eos_allows_while: bool,
    /// When `true` (on the RHS of `is`/`as`), trailing `?` yields to a ternary `? ... : ...`.
    pub(crate) stop_optional_before_ternary: bool,
    /// Number of enclosing unclosed ternary `then` branches / `switch` `when` guards expecting `:`.
    pub(crate) pending_ternary_then: u32,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for the given Kyanos source code using default options.
    pub fn new(source: &'a str) -> Self {
        Self::with_options(source, ParseOptions::default())
    }

    /// Creates a new parser for the given Kyanos source code with custom [`ParseOptions`].
    pub fn with_options(source: &'a str, options: ParseOptions) -> Self {
        let (stream, errors) = TokenStream::with_offset(source, 0);
        Self::from_parts(stream, errors, 0, options)
    }

    /// Creates a parser with a starting byte offset and initial `NodeId` counter.
    ///
    /// Used when parsing embedded source fragments while preserving global `Span` offsets
    /// and sequential `NodeId` allocation.
    pub fn with_offset(source: &'a str, base_offset: usize, start_node_id: u32) -> Self {
        let (stream, errors) = TokenStream::with_offset(source, base_offset);
        Self::from_parts(stream, errors, start_node_id, ParseOptions::default())
    }

    /// Creates a parser directly from an existing [`TokenStream`] using default options.
    pub fn from_stream(stream: TokenStream<'a>) -> Self {
        Self::from_stream_with_options(stream, ParseOptions::default())
    }

    /// Creates a parser directly from an existing [`TokenStream`] with custom [`ParseOptions`].
    pub fn from_stream_with_options(stream: TokenStream<'a>, options: ParseOptions) -> Self {
        Self::from_parts(stream, Vec::new(), 0, options)
    }

    fn from_parts(
        stream: TokenStream<'a>,
        errors: Vec<ParseError>,
        next_node_id: u32,
        options: ParseOptions,
    ) -> Self {
        Self {
            stream,
            errors,
            next_node_id,
            node_id_exhausted: false,
            options,
            memo: BacktrackMemo::default(),
            grouping_depth: 0,
            recursion_depth: 0,
            eos_allows_else: false,
            eos_allows_while: false,
            stop_optional_before_ternary: false,
            pending_ternary_then: 0,
        }
    }

    /// Returns the [`ParseOptions`] active for this parser instance.
    #[inline]
    pub const fn options(&self) -> ParseOptions {
        self.options
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
        if self.next_node_id == u32::MAX {
            if !self.node_id_exhausted {
                self.node_id_exhausted = true;
                self.report_error(ParseError::syntax(
                    "AST node ID space exhausted",
                    self.current_span(),
                ));
            }
            return NodeId::DUMMY;
        }
        let id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;
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
            .save_position(self.errors.len(), self.grouping_depth, self.next_node_id)
    }

    /// Rewinds the parser cursor, AST node ID counter, and diagnostics back to `cp`.
    #[inline]
    pub fn restore(&mut self, cp: Checkpoint) {
        self.stream.restore_position(&cp);
        self.errors.truncate(cp.error_len);
        self.grouping_depth = cp.grouping_depth;
        self.next_node_id = cp.next_node_id;
    }

    /// Returns a memoization key `(cursor, in_grouping)` when no split virtual token is active.
    #[inline]
    pub(crate) fn memo_pos_key(&self) -> Option<(usize, bool)> {
        self.stream
            .memo_cursor()
            .map(|cursor| (cursor, self.grouping_depth > 0))
    }

    /// Executes `f` inside a delimited grouping (`(...)` or `[...]`), permitting
    /// multi-line binary and postfix expression continuations while isolating ternary/EOS state.
    #[inline]
    pub(crate) fn with_grouping<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.grouping_depth += 1;
        let res = self.with_isolated_ternary_scope(f);
        self.grouping_depth -= 1;
        res
    }

    /// Temporarily resets ternary and single-statement EOS boundary flags inside delimited scopes.
    #[inline]
    pub(crate) fn with_isolated_ternary_scope<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let prev_stop = std::mem::replace(&mut self.stop_optional_before_ternary, false);
        let prev_pending = std::mem::replace(&mut self.pending_ternary_then, 0);
        let prev_else = std::mem::replace(&mut self.eos_allows_else, false);
        let prev_while = std::mem::replace(&mut self.eos_allows_while, false);
        let res = f(self);
        self.stop_optional_before_ternary = prev_stop;
        self.pending_ternary_then = prev_pending;
        self.eos_allows_else = prev_else;
        self.eos_allows_while = prev_while;
        res
    }

    /// Executes `f` inside a statement block where outer expression grouping does not
    /// suppress Swift-style newline statement boundaries.
    #[inline]
    pub(crate) fn with_block_statement_scope<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let prev_grouping = std::mem::replace(&mut self.grouping_depth, 0);
        let res = self.with_isolated_ternary_scope(f);
        self.grouping_depth = prev_grouping;
        res
    }

    /// Runs `f` with `stop_optional_before_ternary` set to `stop` (used for `is` / `as` RHS types).
    #[inline]
    pub(crate) fn with_stop_optional_before_ternary<F, T>(&mut self, stop: bool, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        let prev = std::mem::replace(&mut self.stop_optional_before_ternary, stop);
        let res = f(self);
        self.stop_optional_before_ternary = prev;
        res
    }

    /// Increments recursion depth, returning `Err` if `options.max_recursion_depth` is reached (CWE-674).
    #[inline]
    pub(crate) fn enter_recursion(&mut self) -> ParseResult<()> {
        if self.recursion_depth >= self.options.max_recursion_depth {
            return Err(ParseError::recursion_limit_with_max(
                self.current_span(),
                self.options.max_recursion_depth,
            ));
        }
        self.recursion_depth += 1;
        Ok(())
    }

    /// Decrements recursion depth after leaving a guarded parse routine.
    #[inline]
    pub(crate) fn exit_recursion(&mut self) {
        self.recursion_depth -= 1;
    }

    // -------------------------------------------------------------------------
    // Swift-Style End-of-Statement (`eos`) & Error Recovery
    // -------------------------------------------------------------------------

    /// Returns `true` if the current token marks a statement boundary (`;`, `}`, `EOF`,
    /// a leading newline, or `else`/`while` closing an immediately enclosed braceless `if`/`do`).
    #[inline]
    pub(crate) fn at_eos_boundary(&self) -> bool {
        let tok = self.current();
        matches!(
            tok.kind,
            TokenKind::Semicolon | TokenKind::CloseBrace | TokenKind::Eof
        ) || tok.has_leading_newline
            || (self.eos_allows_else && tok.kind == TokenKind::Else)
            || (self.eos_allows_while && tok.kind == TokenKind::While)
    }

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

        if self.at_eos_boundary() {
            return Ok(());
        }

        let tok = self.current();
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

    /// Synchronizes inside a `{ ... }` member/arm list from `start_cursor`, guaranteeing at
    /// least one token of forward progress before skipping to the next delimiter/boundary.
    pub(crate) fn synchronize_braced_element_from(
        &mut self,
        start_cursor: usize,
        stop_at_switch_arm: bool,
        stop_at_semicolon_or_newline: bool,
    ) {
        if self.checkpoint().cursor == start_cursor
            && !self.at(TokenKind::CloseBrace)
            && !self.is_eof()
        {
            self.bump();
        }
        self.synchronize_braced_element(stop_at_switch_arm, stop_at_semicolon_or_newline);
    }

    /// Synchronizes inside a `{ ... }` member/arm list after an element parse error.
    ///
    /// Tracks nested `(...)`, `[...]`, and `{...}` depths so inner delimiters do not trigger
    /// premature synchronization. Consumes `,` (and `;` when `stop_at_semicolon_or_newline` is
    /// `true`) at depth 0, or stops before `}` / `EOF` (and depth-0 `case`/`default` or newline
    /// boundaries when requested).
    pub(crate) fn synchronize_braced_element(
        &mut self,
        stop_at_switch_arm: bool,
        stop_at_semicolon_or_newline: bool,
    ) {
        let mut depth = 0usize;
        while !self.is_eof() {
            let tok = self.current();
            match tok.kind {
                TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::OpenBrace => {
                    depth += 1;
                    self.bump();
                }
                TokenKind::CloseParen | TokenKind::CloseBracket => {
                    depth = depth.saturating_sub(1);
                    self.bump();
                }
                TokenKind::CloseBrace => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    self.bump();
                }
                TokenKind::Comma if depth == 0 => {
                    self.bump();
                    return;
                }
                TokenKind::Semicolon if depth == 0 && stop_at_semicolon_or_newline => {
                    self.bump();
                    return;
                }
                TokenKind::Case | TokenKind::Default if depth == 0 && stop_at_switch_arm => {
                    return;
                }
                _ if depth == 0 && stop_at_semicolon_or_newline && tok.has_leading_newline => {
                    return;
                }
                _ => {
                    self.bump();
                }
            }
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
