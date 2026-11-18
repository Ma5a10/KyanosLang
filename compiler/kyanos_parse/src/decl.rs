// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Declaration, parameter list, attribute, and module parser routines for Kyanos.
//!
//! Phase 2 provides `parse_parameter_list` for arrow functions and object methods.
//! Phase 5 expands this module with full declarations, attributes, FFI, and modules.

use kyanos_ast::{Accessibility, Decl, Ident, Parameter, ParameterList, RestParameter};
use kyanos_lexer::TokenKind;

use crate::error::{ParseError, ParseResult};
use crate::Parser;

impl<'a> Parser<'a> {
    /// Returns `true` if the current token begins a non-variable declaration
    /// (`#[...]`, `function`, `async function`, `struct`, `class`, `interface`,
    /// `enum`, `type`, or `extern`).
    #[inline]
    pub(crate) fn at_declaration_start(&self) -> bool {
        match self.current_kind() {
            TokenKind::HashBracket
            | TokenKind::Function
            | TokenKind::Struct
            | TokenKind::Class
            | TokenKind::Interface
            | TokenKind::Enum
            | TokenKind::Type
            | TokenKind::Extern => true,
            TokenKind::Async => self.peek_kind(1) == TokenKind::Function,
            _ => false,
        }
    }

    /// Parses a declaration.
    ///
    /// TODO(Phase 5): Implement full declaration, attribute, and FFI parsing.
    pub fn parse_decl(&mut self) -> ParseResult<Decl> {
        let tok = self.current();
        Err(ParseError::syntax(
            format!(
                "Declaration starting with {:?} is not yet implemented (Phase 5)",
                tok.kind
            ),
            tok.span,
        ))
    }
    /// Parses a parenthesized parameter list: `(param1, param2, ...rest)`.
    pub fn parse_parameter_list(&mut self) -> ParseResult<ParameterList> {
        let open = self.expect(TokenKind::OpenParen, "'('")?;
        let mut params = Vec::new();
        let mut rest = None;

        if !self.at(TokenKind::CloseParen) {
            loop {
                if self.at(TokenKind::Ellipsis) {
                    rest = Some(self.parse_rest_parameter()?);
                    self.eat(TokenKind::Comma);
                    break;
                }

                params.push(self.parse_parameter()?);
                if self.eat(TokenKind::Comma).is_none() || self.at(TokenKind::CloseParen) {
                    break;
                }
            }
        }

        self.expect(TokenKind::CloseParen, "')'")?;
        let span = self.span_from(open.span);
        Ok(ParameterList::new(params, rest, span))
    }

    fn parse_parameter(&mut self) -> ParseResult<Parameter> {
        let start = self.current_span();
        let accessibility = self.parse_optional_accessibility();
        let is_readonly = self.eat(TokenKind::Readonly).is_some();
        let target = self.parse_binding_target()?;
        let is_optional = self.eat(TokenKind::Question).is_some();

        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };

        let default = if self.eat(TokenKind::Assign).is_some() {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = self.span_from(start);
        Ok(Parameter::new(
            accessibility,
            is_readonly,
            target,
            is_optional,
            ty,
            default,
            span,
        ))
    }

    fn parse_rest_parameter(&mut self) -> ParseResult<RestParameter> {
        let ellipsis = self.expect(TokenKind::Ellipsis, "'...'")?;
        let name_tok = self.expect(TokenKind::Identifier, "rest parameter name")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let span = self.span_from(ellipsis.span);
        Ok(RestParameter::new(name, ty, span))
    }

    /// Parses an optional `public` / `protected` / `private` accessibility modifier.
    pub(crate) fn parse_optional_accessibility(&mut self) -> Option<Accessibility> {
        let kind = match self.current_kind() {
            TokenKind::Public => Accessibility::Public,
            TokenKind::Protected => Accessibility::Protected,
            TokenKind::Private => Accessibility::Private,
            _ => return None,
        };
        self.bump();
        Some(kind)
    }
}
