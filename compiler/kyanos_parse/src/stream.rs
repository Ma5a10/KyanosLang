// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Buffered token stream with trivia filtering, checkpointing, and generic `>` splitting.

use std::collections::HashSet;

use kyanos_ast::Span;
use kyanos_lexer::{Lexer, Token, TokenKind};

use crate::error::ParseError;

/// Safety cap on consecutive lexer errors to guarantee termination on malformed input.
const MAX_CONSECUTIVE_LEXER_ERRORS: usize = 1024;

/// Saved cursor state enabling speculative lookahead and backtracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checkpoint {
    pub(crate) cursor: usize,
    pub(crate) virtual_token: Option<Token>,
    pub(crate) last_span: Span,
    pub(crate) error_len: usize,
    pub(crate) grouping_depth: u32,
}

/// Token stream supplying non-trivia tokens to [`crate::Parser`].
///
/// All whitespace and comment tokens (`TokenKind::is_trivia()`) are filtered out
/// while preserving `Token.has_leading_newline` for Swift-style statement termination.
#[derive(Debug, Clone)]
pub struct TokenStream<'a> {
    source: &'a str,
    base_offset: usize,
    tokens: Vec<Token>,
    cursor: usize,
    /// Remainder of a split compound token (`>>`, `>>>`, `>=`, etc.) during generic parsing.
    virtual_token: Option<Token>,
    last_span: Span,
}

impl<'a> TokenStream<'a> {
    /// Lexes `source` and builds a trivia-free token stream, returning any lexer diagnostics.
    pub fn new(source: &'a str) -> (Self, Vec<ParseError>) {
        Self::with_offset(source, 0)
    }

    /// Lexes `source` while shifting all token spans by `base_offset` (for nested parses).
    pub fn with_offset(source: &'a str, base_offset: usize) -> (Self, Vec<ParseError>) {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        let mut lex_errors = Vec::new();
        let mut last_end = 0usize;
        let mut consecutive_errors = 0usize;

        loop {
            match lexer.next_token() {
                Ok(mut tok) => {
                    consecutive_errors = 0;
                    last_end = tok.span.end;
                    let is_eof = tok.kind == TokenKind::Eof;
                    if !tok.kind.is_trivia() {
                        tok.span =
                            Span::new(tok.span.start + base_offset, tok.span.end + base_offset);
                        tokens.push(tok);
                    }
                    if is_eof {
                        break;
                    }
                }
                Err(msg) => {
                    let err_pos = last_end + base_offset;
                    lex_errors.push(ParseError::lexer(msg, Span::new(err_pos, err_pos)));
                    consecutive_errors += 1;
                    if consecutive_errors >= MAX_CONSECUTIVE_LEXER_ERRORS {
                        break;
                    }
                }
            }
        }

        // Guarantee a terminating EOF token even if lexing aborted early.
        let eof_pos = source.len() + base_offset;
        if tokens.last().map(|t| t.kind) != Some(TokenKind::Eof) {
            tokens.push(Token::new(
                TokenKind::Eof,
                Span::new(eof_pos, eof_pos),
                false,
            ));
        }

        let stream = Self {
            source,
            base_offset,
            tokens,
            cursor: 0,
            virtual_token: None,
            last_span: Span::DUMMY,
        };
        (stream, lex_errors)
    }

    /// Builds a token stream directly from pre-collected tokens (filtering trivia).
    pub fn from_tokens(source: &'a str, tokens: Vec<Token>) -> Self {
        Self::from_tokens_with_offset(source, tokens, 0)
    }

    /// Builds a token stream from pre-collected tokens with a base byte offset.
    pub fn from_tokens_with_offset(
        source: &'a str,
        tokens: Vec<Token>,
        base_offset: usize,
    ) -> Self {
        let mut filtered: Vec<Token> = tokens
            .into_iter()
            .filter(|t| !t.kind.is_trivia())
            .map(|mut t| {
                t.span = Span::new(t.span.start + base_offset, t.span.end + base_offset);
                t
            })
            .collect();

        if filtered.last().map(|t| t.kind) != Some(TokenKind::Eof) {
            let eof_pos = source.len() + base_offset;
            filtered.push(Token::new(
                TokenKind::Eof,
                Span::new(eof_pos, eof_pos),
                false,
            ));
        }

        Self {
            source,
            base_offset,
            tokens: filtered,
            cursor: 0,
            virtual_token: None,
            last_span: Span::DUMMY,
        }
    }

    /// Returns the original source text backing this token stream.
    #[inline]
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Returns the base byte offset applied to token spans in this stream.
    #[inline]
    pub fn base_offset(&self) -> usize {
        self.base_offset
    }

    /// Returns the slice of source code covered by `span` (adjusted for `base_offset`).
    #[inline]
    pub fn slice(&self, span: Span) -> &'a str {
        let start = span.start.saturating_sub(self.base_offset);
        let end = span
            .end
            .saturating_sub(self.base_offset)
            .min(self.source.len());
        if start <= end {
            &self.source[start..end]
        } else {
            ""
        }
    }

    /// Returns the current token without consuming it.
    #[inline]
    pub fn current(&self) -> Token {
        if let Some(tok) = self.virtual_token {
            tok
        } else {
            self.tokens[self.cursor]
        }
    }

    /// Looks ahead `offset` tokens (`offset = 0` is [`Self::current`]).
    #[inline]
    pub fn peek(&self, offset: usize) -> Token {
        if offset == 0 {
            return self.current();
        }
        let idx = (self.cursor + offset).min(self.tokens.len() - 1);
        self.tokens[idx]
    }

    /// Returns the span of the most recently consumed token, or `Span::DUMMY` at start.
    #[inline]
    pub fn last_span(&self) -> Span {
        self.last_span
    }

    /// Returns `true` if the stream has reached `TokenKind::Eof`.
    #[inline]
    pub fn is_eof(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }

    /// Consumes and returns the current token, advancing the cursor.
    pub fn bump(&mut self) -> Token {
        if let Some(tok) = self.virtual_token.take() {
            self.last_span = tok.span;
            if self.cursor + 1 < self.tokens.len() {
                self.cursor += 1;
            }
            return tok;
        }

        let tok = self.tokens[self.cursor];
        if tok.kind != TokenKind::Eof {
            self.last_span = tok.span;
            self.cursor += 1;
        }
        tok
    }

    /// Splits a compound token beginning with `>` (`>`, `>>`, `>>>`, `>=`, `>>=`, `>>>=`)
    /// so nested generics like `Map<string, Array<int>>` parse without lexer hacks.
    pub fn eat_generic_gt(&mut self) -> Option<Span> {
        let tok = self.current();
        let remainder_kind = match tok.kind {
            TokenKind::GreaterThan => {
                self.bump();
                return Some(tok.span);
            }
            TokenKind::ShiftRight => TokenKind::GreaterThan,
            TokenKind::UnsignedShiftRight => TokenKind::ShiftRight,
            TokenKind::GreaterThanEquals => TokenKind::Assign,
            TokenKind::ShiftRightAssign => TokenKind::GreaterThanEquals,
            TokenKind::UnsignedShiftRightAssign => TokenKind::ShiftRightAssign,
            _ => return None,
        };

        let first_span = Span::new(tok.span.start, tok.span.start + 1);
        let rest_span = Span::new(tok.span.start + 1, tok.span.end);
        self.virtual_token = Some(Token::new(remainder_kind, rest_span, false));
        self.last_span = first_span;
        Some(first_span)
    }

    /// Captures the stream's cursor state for speculative parsing.
    #[inline]
    pub(crate) fn save_position(&self, error_len: usize, grouping_depth: u32) -> Checkpoint {
        Checkpoint {
            cursor: self.cursor,
            virtual_token: self.virtual_token,
            last_span: self.last_span,
            error_len,
            grouping_depth,
        }
    }

    /// Restores the stream's cursor state from a previously saved checkpoint.
    #[inline]
    pub(crate) fn restore_position(&mut self, cp: &Checkpoint) {
        self.cursor = cp.cursor;
        self.virtual_token = cp.virtual_token;
        self.last_span = cp.last_span;
    }

    /// Returns the raw token cursor when no split `virtual_token` is active, for memoization keys.
    #[inline]
    pub(crate) fn memo_cursor(&self) -> Option<usize> {
        if self.virtual_token.is_none() {
            Some(self.cursor)
        } else {
            None
        }
    }

    /// Scans from the current `(` to its matching `)` without recursion and checks whether `=>`
    /// immediately follows. Every `(` encountered whose matching `)` is not followed by `=>` is
    /// memoized in `non_arrow_cache` so nested parenthesized types resolve in O(N) total time.
    pub(crate) fn scan_paren_has_arrow(&self, non_arrow_cache: &mut HashSet<usize>) -> bool {
        if self.virtual_token.is_some() {
            return false;
        }
        let start = self.cursor;
        if non_arrow_cache.contains(&start) {
            return false;
        }
        if self.tokens.get(start).map(|t| t.kind) != Some(TokenKind::OpenParen) {
            return false;
        }

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Delim {
            Paren,
            Bracket,
            Brace,
        }

        let mut stack: Vec<(Delim, usize)> = Vec::new();
        let mut idx = start;
        while idx < self.tokens.len() {
            match self.tokens[idx].kind {
                TokenKind::OpenParen => stack.push((Delim::Paren, idx)),
                TokenKind::OpenBracket => stack.push((Delim::Bracket, idx)),
                TokenKind::OpenBrace => stack.push((Delim::Brace, idx)),
                TokenKind::CloseParen => {
                    if let Some((Delim::Paren, open_idx)) = stack.last().copied() {
                        stack.pop();
                        let has_arrow = self
                            .tokens
                            .get(idx + 1)
                            .is_some_and(|tok| tok.kind == TokenKind::Arrow);
                        if !has_arrow {
                            non_arrow_cache.insert(open_idx);
                        }
                        if stack.is_empty() {
                            return has_arrow;
                        }
                    } else {
                        break;
                    }
                }
                TokenKind::CloseBracket => {
                    if let Some((Delim::Bracket, _)) = stack.last() {
                        stack.pop();
                    } else {
                        break;
                    }
                }
                TokenKind::CloseBrace => {
                    if let Some((Delim::Brace, _)) = stack.last() {
                        stack.pop();
                    } else {
                        break;
                    }
                }
                TokenKind::Eof => break,
                _ => {}
            }
            idx += 1;
        }

        for (delim, open_idx) in stack {
            if delim == Delim::Paren {
                non_arrow_cache.insert(open_idx);
            }
        }
        false
    }
}
