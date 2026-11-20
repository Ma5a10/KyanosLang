// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pattern and destructuring binding target parser routines for Kyanos.

use kyanos_ast::{
    BindingTarget, Ident, LiteralSign, ObjectPatternProp, Pattern, StructFieldPattern,
    TuplePatternElement, TypeName, VarModifier,
};
use kyanos_lexer::TokenKind;

use crate::error::{ParseError, ParseResult};
use crate::Parser;

impl<'a> Parser<'a> {
    /// Parses a binding target: `Identifier | Underscore | tuplePattern | objectPattern`.
    pub fn parse_binding_target(&mut self) -> ParseResult<BindingTarget> {
        self.enter_recursion()?;
        let res = self.parse_binding_target_inner();
        self.exit_recursion();
        res
    }

    fn parse_binding_target_inner(&mut self) -> ParseResult<BindingTarget> {
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
            TokenKind::OpenBracket => {
                let open = tok.span;
                let elements = self.parse_tuple_pattern_elements()?;
                let span = self.span_from(open);
                Ok(BindingTarget::tuple(elements, span))
            }
            TokenKind::OpenBrace => {
                let open = tok.span;
                let props = self.parse_object_pattern_props()?;
                let span = self.span_from(open);
                Ok(BindingTarget::object(props, span))
            }
            other => Err(ParseError::unexpected("binding target", other, tok.span)),
        }
    }

    /// Parses a match pattern used in `switch` arms and tuple destructuring.
    pub fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        self.enter_recursion()?;
        let res = self.parse_pattern_inner();
        self.exit_recursion();
        res
    }

    fn parse_pattern_inner(&mut self) -> ParseResult<Pattern> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Underscore => {
                self.bump();
                let id = self.alloc_id();
                Ok(Pattern::wildcard(tok.span).with_id(id))
            }
            TokenKind::Plus | TokenKind::Minus => self.parse_signed_literal_pattern(),
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
                let modifier = self.parse_var_modifier_token();
                let start = tok.span;
                let name_tok = self.expect(TokenKind::Identifier, "identifier")?;
                let name = Ident::new(self.slice(name_tok.span), name_tok.span);
                let span = self.span_from(start);
                let id = self.alloc_id();
                Ok(Pattern::var(name, Some(modifier), span).with_id(id))
            }
            TokenKind::OpenBracket => {
                let open = tok.span;
                let elements = self.parse_tuple_pattern_elements()?;
                let span = self.span_from(open);
                let id = self.alloc_id();
                Ok(Pattern::tuple(elements, span).with_id(id))
            }
            TokenKind::OpenBrace => {
                let open = tok.span;
                let props = self.parse_object_pattern_props()?;
                let span = self.span_from(open);
                let id = self.alloc_id();
                Ok(Pattern::object(props, span).with_id(id))
            }
            TokenKind::Identifier => self.parse_named_pattern(),
            other => Err(ParseError::unexpected("pattern", other, tok.span)),
        }
    }

    fn parse_signed_literal_pattern(&mut self) -> ParseResult<Pattern> {
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

    fn parse_named_pattern(&mut self) -> ParseResult<Pattern> {
        let name = self.parse_type_name()?;

        if self.at(TokenKind::OpenBrace) {
            return self.parse_struct_pattern(name);
        }

        if self.eat(TokenKind::OpenParen).is_some() {
            let payload = self.with_grouping(|p| {
                let mut items = Vec::new();
                while !p.at(TokenKind::CloseParen) && !p.is_eof() {
                    items.push(p.parse_pattern()?);
                    if p.eat(TokenKind::Comma).is_none() {
                        break;
                    }
                }
                Ok::<_, ParseError>(items)
            })?;
            self.expect(TokenKind::CloseParen, "')'")?;
            let span = self.span_from(name.span);
            let id = self.alloc_id();
            return Ok(Pattern::variant(name, payload, span).with_id(id));
        }

        if name.parts.len() > 1 {
            let span = name.span;
            let id = self.alloc_id();
            return Ok(Pattern::variant(name, Vec::new(), span).with_id(id));
        }

        // Single bare identifier (e.g. `ch` in `KeyPress(ch)` or unit variant `None`)
        // is parsed as `PatternKind::Var { modifier: None, .. }` and disambiguated by `sema`.
        let TypeName { mut parts, span } = name;
        let ident = parts.pop().expect("single identifier part");
        let id = self.alloc_id();
        Ok(Pattern::var(ident, None, span).with_id(id))
    }

    fn parse_struct_pattern(&mut self, struct_name: TypeName) -> ParseResult<Pattern> {
        let start = struct_name.span;
        self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut fields = Vec::new();

        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            fields.push(self.parse_struct_field_pattern()?);
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }

        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Pattern::struct_pattern(struct_name, fields, span).with_id(id))
    }

    fn parse_struct_field_pattern(&mut self) -> ParseResult<StructFieldPattern> {
        if let Some(ellipsis) = self.eat(TokenKind::Ellipsis) {
            return Ok(StructFieldPattern::Rest(ellipsis.span));
        }

        if self.at(TokenKind::Let) || self.at(TokenKind::Const) {
            let mod_tok = self.current();
            let modifier = self.parse_var_modifier_token();
            let name_tok = self.expect(TokenKind::Identifier, "field identifier")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            let span = self.span_from(mod_tok.span);
            return Ok(StructFieldPattern::Binding {
                modifier: Some(modifier),
                name,
                span,
            });
        }

        let name_tok = self.expect(TokenKind::Identifier, "struct field pattern")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        if self.eat(TokenKind::Colon).is_some() {
            let pattern = self.parse_pattern()?;
            let span = self.span_from(name_tok.span);
            Ok(StructFieldPattern::Field {
                name,
                pattern,
                span,
            })
        } else {
            Ok(StructFieldPattern::Binding {
                modifier: None,
                name,
                span: name_tok.span,
            })
        }
    }

    fn parse_tuple_pattern_elements(&mut self) -> ParseResult<Vec<TuplePatternElement>> {
        self.expect(TokenKind::OpenBracket, "'['")?;
        self.grouping_depth += 1;
        let mut list = Vec::new();
        let res = (|| {
            while !self.at(TokenKind::CloseBracket) && !self.is_eof() {
                if let Some(ellipsis) = self.eat(TokenKind::Ellipsis) {
                    let pat = self.parse_pattern()?;
                    let span = self.span_from(ellipsis.span);
                    list.push(TuplePatternElement::rest(pat, span));
                } else {
                    let pat = self.parse_pattern()?;
                    list.push(TuplePatternElement::normal(pat));
                }
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            Ok::<_, ParseError>(())
        })();
        self.grouping_depth -= 1;
        res?;
        self.expect(TokenKind::CloseBracket, "']'")?;
        Ok(list)
    }

    fn parse_object_pattern_props(&mut self) -> ParseResult<Vec<ObjectPatternProp>> {
        self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut props = Vec::new();

        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            if let Some(ellipsis) = self.eat(TokenKind::Ellipsis) {
                let target_tok = self.expect(TokenKind::Identifier, "rest binding identifier")?;
                let target = Ident::new(self.slice(target_tok.span), target_tok.span);
                let span = self.span_from(ellipsis.span);
                props.push(ObjectPatternProp::Rest { target, span });
            } else {
                let name_tok = self.expect(TokenKind::Identifier, "property identifier")?;
                let name = Ident::new(self.slice(name_tok.span), name_tok.span);
                let target = if self.eat(TokenKind::Colon).is_some() {
                    Some(self.parse_binding_target()?)
                } else {
                    None
                };
                let default = if self.eat(TokenKind::Assign).is_some() {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                let span = self.span_from(name_tok.span);
                props.push(ObjectPatternProp::Property {
                    name,
                    target,
                    default,
                    span,
                });
            }

            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }

        self.expect(TokenKind::CloseBrace, "'}'")?;
        Ok(props)
    }

    /// Consumes `let` or `const` and returns the corresponding [`VarModifier`].
    #[inline]
    pub(crate) fn parse_var_modifier_token(&mut self) -> VarModifier {
        let tok = self.bump();
        if tok.kind == TokenKind::Const {
            VarModifier::Const
        } else {
            VarModifier::Let
        }
    }
}
