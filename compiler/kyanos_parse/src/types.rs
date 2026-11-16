// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Type annotation parser routines for Kyanos.
//!
//! Phase 2 provides predefined types, qualified type references, generic type
//! parameters/arguments, and return types needed by expressions. Phase 3 expands
//! this module with pointers, references, arrays, tuples, anonymous structs, and
//! function types.

use kyanos_ast::{
    Ident, PredefinedType, ReturnType, Type, TypeArguments, TypeKind, TypeName, TypeParameter,
    TypeParameters, TypePredicate,
};
use kyanos_lexer::TokenKind;

use crate::error::{ParseError, ParseResult};
use crate::Parser;

/// Maps a primitive type token kind to [`PredefinedType`], or `None` otherwise.
#[inline]
pub fn token_to_predefined_type(kind: TokenKind) -> Option<PredefinedType> {
    match kind {
        TokenKind::TypeByte => Some(PredefinedType::Byte),
        TokenKind::TypeShort => Some(PredefinedType::Short),
        TokenKind::TypeInt => Some(PredefinedType::Int),
        TokenKind::TypeLong => Some(PredefinedType::Long),
        TokenKind::TypeSizeT => Some(PredefinedType::SizeT),
        TokenKind::TypeUbyte => Some(PredefinedType::Ubyte),
        TokenKind::TypeUshort => Some(PredefinedType::Ushort),
        TokenKind::TypeUint => Some(PredefinedType::Uint),
        TokenKind::TypeUlong => Some(PredefinedType::Ulong),
        TokenKind::TypeFloat => Some(PredefinedType::Float),
        TokenKind::TypeDouble => Some(PredefinedType::Double),
        TokenKind::TypeBool => Some(PredefinedType::Bool),
        TokenKind::TypeChar => Some(PredefinedType::Char),
        TokenKind::TypeString => Some(PredefinedType::String),
        TokenKind::TypeVoid => Some(PredefinedType::Void),
        TokenKind::TypeNever => Some(PredefinedType::Never),
        TokenKind::TypeRawptr => Some(PredefinedType::Rawptr),
        _ => None,
    }
}

impl<'a> Parser<'a> {
    /// Parses a return type annotation (`type_` or `Identifier is type_`).
    pub fn parse_return_type(&mut self) -> ParseResult<ReturnType> {
        if self.at(TokenKind::Identifier) && self.peek_kind(1) == TokenKind::Is {
            let ident_tok = self.bump();
            let target = Ident::new(self.slice(ident_tok.span), ident_tok.span);
            self.bump(); // consume `is`
            let ty = self.parse_type()?;
            let span = self.span_from(ident_tok.span);
            return Ok(ReturnType::Predicate(TypePredicate::new(target, ty, span)));
        }
        Ok(ReturnType::Type(self.parse_type()?))
    }

    /// Parses a type annotation.
    ///
    /// TODO(Phase 3): Add `*T`/`*const T`, `&T`/`&const T`, `T?`, `T[]`, `T[N]`,
    /// tuple types, anonymous struct types, and function types.
    pub fn parse_type(&mut self) -> ParseResult<Type> {
        let tok = self.current();

        let mut ty = if let Some(pt) = token_to_predefined_type(tok.kind) {
            self.bump();
            let id = self.alloc_id();
            Type::predefined(pt, tok.span).with_id(id)
        } else if tok.kind == TokenKind::This {
            self.bump();
            let id = self.alloc_id();
            Type::this(tok.span).with_id(id)
        } else if tok.kind == TokenKind::OpenParen {
            let open = self.bump();
            let inner = self.parse_type()?;
            self.expect(TokenKind::CloseParen, "')'")?;
            let span = self.span_from(open.span);
            let id = self.alloc_id();
            Type::new(TypeKind::Parenthesized(Box::new(inner)), span).with_id(id)
        } else if tok.kind == TokenKind::Identifier {
            let name = self.parse_type_name()?;
            let type_args = if self.at(TokenKind::LessThan) {
                Some(self.parse_type_arguments()?)
            } else {
                None
            };
            let span = self.span_from(name.span);
            let id = self.alloc_id();
            Type::reference(name, type_args, span).with_id(id)
        } else {
            return Err(ParseError::unexpected("type", tok.kind, tok.span));
        };

        // Support basic postfix `?` and `[]` in Phase 2 expressions such as `x as int?`.
        loop {
            if self.eat(TokenKind::Question).is_some() {
                let span = self.span_from(ty.span);
                let id = self.alloc_id();
                ty = Type::optional(ty, span).with_id(id);
            } else if self.at(TokenKind::OpenBracket)
                && self.peek_kind(1) == TokenKind::CloseBracket
            {
                self.bump();
                self.bump();
                let span = self.span_from(ty.span);
                let id = self.alloc_id();
                ty = Type::dynamic_array(ty, span).with_id(id);
            } else {
                break;
            }
        }

        Ok(ty)
    }

    /// Parses a qualified type name: `Identifier ('.' Identifier)*`.
    pub fn parse_type_name(&mut self) -> ParseResult<TypeName> {
        let first_tok = self.expect(TokenKind::Identifier, "type name")?;
        let start = first_tok.span;
        let mut parts = vec![Ident::new(self.slice(first_tok.span), first_tok.span)];

        while self.at(TokenKind::Dot) && self.peek_kind(1) == TokenKind::Identifier {
            self.bump(); // consume '.'
            let seg = self.bump();
            parts.push(Ident::new(self.slice(seg.span), seg.span));
        }

        let span = self.span_from(start);
        Ok(TypeName::new(parts, span))
    }

    /// Parses a generic type argument list: `<T, U>`.
    pub fn parse_type_arguments(&mut self) -> ParseResult<TypeArguments> {
        let open = self.expect(TokenKind::LessThan, "'<'")?;
        let mut args = Vec::new();

        if !self.at(TokenKind::GreaterThan) {
            loop {
                args.push(self.parse_type()?);
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
                if self.at_generic_gt_start() {
                    break;
                }
            }
        }

        self.expect_generic_gt("'>'")?;
        let span = self.span_from(open.span);
        Ok(TypeArguments::new(args, span))
    }

    /// Parses a generic type parameter declaration list: `<T extends Bound = Default>`.
    pub fn parse_type_parameters(&mut self) -> ParseResult<TypeParameters> {
        let open = self.expect(TokenKind::LessThan, "'<'")?;
        let mut params = Vec::new();

        if !self.at_generic_gt_start() {
            loop {
                let name_tok = self.expect(TokenKind::Identifier, "type parameter name")?;
                let name = Ident::new(self.slice(name_tok.span), name_tok.span);

                let constraint = if self.eat(TokenKind::Extends).is_some() {
                    Some(self.parse_type()?)
                } else {
                    None
                };

                let default = if self.eat(TokenKind::Assign).is_some() {
                    Some(self.parse_type()?)
                } else {
                    None
                };

                let param_span = self.span_from(name_tok.span);
                params.push(TypeParameter::new(name, constraint, default, param_span));

                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
                if self.at_generic_gt_start() {
                    break;
                }
            }
        }

        self.expect_generic_gt("'>'")?;
        let span = self.span_from(open.span);
        Ok(TypeParameters::new(params, span))
    }

    /// Returns `true` if the current token begins with `>` (including `>>`, `>>>`, `>=`, etc.).
    #[inline]
    pub(crate) fn at_generic_gt_start(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::GreaterThan
                | TokenKind::ShiftRight
                | TokenKind::UnsignedShiftRight
                | TokenKind::GreaterThanEquals
                | TokenKind::ShiftRightAssign
                | TokenKind::UnsignedShiftRightAssign
        )
    }
}
