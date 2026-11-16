// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pattern and destructuring binding target parser routines for Kyanos.
//!
//! Phase 2 provides identifier/wildcard binding targets and basic literal, wildcard,
//! variable, and variant patterns for `switch` expressions and arrow parameters.
//! Phase 3 expands this module with struct, tuple, and object patterns.

use kyanos_ast::{BindingTarget, Ident, LiteralSign, Pattern, TypeName, VarModifier};
use kyanos_lexer::TokenKind;

use crate::error::{ParseError, ParseResult};
use crate::Parser;

impl<'a> Parser<'a> {
    /// Parses a binding target (`Identifier | Underscore | tuplePattern | objectPattern`).
    ///
    /// TODO(Phase 3): Add `tuplePattern` and `objectPattern`.
    pub fn parse_binding_target(&mut self) -> ParseResult<BindingTarget> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Identifier => {
                self.bump();
                Ok(BindingTarget::ident(Ident::new(
                    self.slice(tok.span),
                    tok.span,
                )))
            }
            TokenKind::Underscore => {
                self.bump();
                Ok(BindingTarget::wildcard(tok.span))
            }
            other => Err(ParseError::unexpected("binding target", other, tok.span)),
        }
    }

    /// Parses a match pattern used in `switch` cases and destructuring.
    ///
    /// TODO(Phase 3): Add `structPattern`, `tuplePattern`, and `objectPattern`.
    pub fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Underscore => {
                self.bump();
                let id = self.alloc_id();
                Ok(Pattern::wildcard(tok.span).with_id(id))
            }
            TokenKind::Plus | TokenKind::Minus => {
                let sign_tok = self.bump();
                let sign = if sign_tok.kind == TokenKind::Plus {
                    LiteralSign::Plus
                } else {
                    LiteralSign::Minus
                };
                let lit_tok = self.current();
                if !matches!(
                    lit_tok.kind,
                    TokenKind::IntegerLiteral | TokenKind::FloatLiteral | TokenKind::BigIntLiteral
                ) {
                    return Err(ParseError::unexpected(
                        "numeric literal",
                        lit_tok.kind,
                        lit_tok.span,
                    ));
                }
                self.bump();
                let lit = self.parse_literal_token(lit_tok)?;
                let span = self.span_from(sign_tok.span);
                let id = self.alloc_id();
                Ok(Pattern::signed_literal(sign, lit, span).with_id(id))
            }
            TokenKind::IntegerLiteral
            | TokenKind::FloatLiteral
            | TokenKind::BigIntLiteral
            | TokenKind::StringLiteral
            | TokenKind::CStringLiteral
            | TokenKind::CharLiteral
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null => {
                self.bump();
                let lit = self.parse_literal_token(tok)?;
                let id = self.alloc_id();
                Ok(Pattern::literal(lit, tok.span).with_id(id))
            }
            TokenKind::Let | TokenKind::Const => {
                let mod_tok = self.bump();
                let modifier = if mod_tok.kind == TokenKind::Let {
                    VarModifier::Let
                } else {
                    VarModifier::Const
                };
                let name_tok = self.expect(TokenKind::Identifier, "identifier")?;
                let name = Ident::new(self.slice(name_tok.span), name_tok.span);
                let span = self.span_from(mod_tok.span);
                let id = self.alloc_id();
                Ok(Pattern::var(name, Some(modifier), span).with_id(id))
            }
            TokenKind::Identifier => {
                let name = self.parse_type_name()?;
                if self.eat(TokenKind::OpenParen).is_some() {
                    let mut payload = Vec::new();
                    if !self.at(TokenKind::CloseParen) {
                        loop {
                            payload.push(self.parse_pattern()?);
                            if self.eat(TokenKind::Comma).is_none()
                                || self.at(TokenKind::CloseParen)
                            {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::CloseParen, "')'")?;
                    let span = self.span_from(name.span);
                    let id = self.alloc_id();
                    Ok(Pattern::variant(name, payload, span).with_id(id))
                } else if name.parts.len() > 1 {
                    let span = name.span;
                    let id = self.alloc_id();
                    Ok(Pattern::variant(name, Vec::new(), span).with_id(id))
                } else {
                    let TypeName { mut parts, span } = name;
                    let ident = parts.pop().expect("single identifier part");
                    let id = self.alloc_id();
                    Ok(Pattern::var(ident, None, span).with_id(id))
                }
            }
            other => Err(ParseError::unexpected("pattern", other, tok.span)),
        }
    }
}
