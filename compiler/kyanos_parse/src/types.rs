// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Type annotation and generic parameter/argument parser routines for Kyanos.

use kyanos_ast::{
    AnonymousStructField, AnonymousStructMember, AnonymousStructMethod, ArraySize, FunctionType,
    FunctionTypeParam, Ident, LiteralKind, PredefinedType, ReturnType, Type, TypeArguments,
    TypeKind, TypeName, TypeParameter, TypeParameters, TypePredicate,
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

    /// Parses a type annotation: `functionType | prefixType | typeOrPostfix`.
    pub fn parse_type(&mut self) -> ParseResult<Type> {
        self.enter_recursion()?;
        let res = self.parse_type_inner();
        self.exit_recursion();
        res
    }

    fn parse_type_inner(&mut self) -> ParseResult<Type> {
        // 1. Generic function type: `<T>(params) => returnType`
        if self.at(TokenKind::LessThan) {
            return self.parse_function_type(true);
        }

        // 2. Non-generic function type `(params) => returnType` vs parenthesized `(T)`
        if self.at(TokenKind::OpenParen) {
            if let Some(fn_ty) = self.try_parse_non_generic_function_type()? {
                return Ok(fn_ty);
            }
        }

        // 3. Prefix pointer / reference types: `*T`, `*const T`, `&T`, `&const T`
        if self.at(TokenKind::Star) || self.at(TokenKind::And) {
            return self.parse_prefix_type();
        }

        // 4. Primary type with optional postfix suffixes (`?`, `[]`, `[N]`)
        self.parse_type_or_postfix()
    }

    fn parse_prefix_type(&mut self) -> ParseResult<Type> {
        let op_tok = self.bump();
        let is_const = self.eat(TokenKind::Const).is_some();
        let elem = self.parse_type()?;
        let span = self.span_from(op_tok.span);
        let id = self.alloc_id();

        if op_tok.kind == TokenKind::Star {
            Ok(Type::pointer(elem, is_const, span).with_id(id))
        } else {
            Ok(Type::reference_type(elem, is_const, span).with_id(id))
        }
    }

    fn parse_type_or_postfix(&mut self) -> ParseResult<Type> {
        let mut ty = self.parse_primary_type()?;

        loop {
            if self.grouping_depth == 0 && self.current().has_leading_newline {
                break;
            }
            if self.eat(TokenKind::Question).is_some() {
                let span = self.span_from(ty.span);
                let id = self.alloc_id();
                ty = Type::optional(ty, span).with_id(id);
            } else if self.eat(TokenKind::OpenBracket).is_some() {
                if self.eat(TokenKind::CloseBracket).is_some() {
                    let span = self.span_from(ty.span);
                    let id = self.alloc_id();
                    ty = Type::dynamic_array(ty, span).with_id(id);
                } else {
                    let size = self.parse_array_size()?;
                    self.expect(TokenKind::CloseBracket, "']'")?;
                    let span = self.span_from(ty.span);
                    let id = self.alloc_id();
                    ty = Type::fixed_array(ty, size, span).with_id(id);
                }
            } else {
                break;
            }
        }

        Ok(ty)
    }

    fn parse_array_size(&mut self) -> ParseResult<ArraySize> {
        if self.at(TokenKind::IntegerLiteral) && self.peek_kind(1) == TokenKind::CloseBracket {
            let tok = self.bump();
            let lit = self.parse_literal_token(tok)?;
            if let LiteralKind::Integer(int_lit) = lit.kind {
                return Ok(ArraySize::Literal(int_lit.value));
            }
        }
        let expr = self.with_grouping(|p| p.parse_expr())?;
        Ok(ArraySize::Expr(Box::new(expr)))
    }

    fn parse_primary_type(&mut self) -> ParseResult<Type> {
        let tok = self.current();

        if let Some(pt) = token_to_predefined_type(tok.kind) {
            self.bump();
            let id = self.alloc_id();
            return Ok(Type::predefined(pt, tok.span).with_id(id));
        }

        match tok.kind {
            TokenKind::Identifier => {
                let name = self.parse_type_name()?;
                let type_args = if self.at(TokenKind::LessThan) {
                    Some(self.parse_type_arguments()?)
                } else {
                    None
                };
                let span = self.span_from(name.span);
                let id = self.alloc_id();
                Ok(Type::reference(name, type_args, span).with_id(id))
            }
            TokenKind::OpenBracket => self.parse_tuple_type(),
            TokenKind::OpenBrace => self.parse_anonymous_struct_type(),
            TokenKind::OpenParen => {
                let open = self.bump();
                let inner = self.with_grouping(|p| p.parse_type())?;
                self.expect(TokenKind::CloseParen, "')'")?;
                let span = self.span_from(open.span);
                let id = self.alloc_id();
                Ok(Type::new(TypeKind::Parenthesized(Box::new(inner)), span).with_id(id))
            }
            TokenKind::This => {
                self.bump();
                let id = self.alloc_id();
                Ok(Type::this(tok.span).with_id(id))
            }
            other => Err(ParseError::unexpected("type", other, tok.span)),
        }
    }

    fn parse_tuple_type(&mut self) -> ParseResult<Type> {
        let open = self.expect(TokenKind::OpenBracket, "'['")?;
        let elems = self.with_grouping(|p| {
            let mut list = Vec::new();
            while !p.at(TokenKind::CloseBracket) && !p.is_eof() {
                list.push(p.parse_type()?);
                if p.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            Ok::<_, ParseError>(list)
        })?;
        self.expect(TokenKind::CloseBracket, "']'")?;
        let span = self.span_from(open.span);
        let id = self.alloc_id();
        Ok(Type::tuple(elems, span).with_id(id))
    }

    fn parse_anonymous_struct_type(&mut self) -> ParseResult<Type> {
        let open = self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut members = Vec::new();

        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            if self.eat(TokenKind::Semicolon).is_some() {
                continue;
            }

            let member_start = self.current_span();
            let is_readonly = self.eat(TokenKind::Readonly).is_some();
            let name_tok = self.expect(TokenKind::Identifier, "member identifier")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            let is_optional = self.eat(TokenKind::Question).is_some();

            let member = if !is_readonly
                && (self.at(TokenKind::LessThan) || self.at(TokenKind::OpenParen))
            {
                let type_parameters = if self.at(TokenKind::LessThan) {
                    Some(self.parse_type_parameters()?)
                } else {
                    None
                };
                let params = self.with_grouping(|p| p.parse_parameter_list())?;
                self.expect(TokenKind::Colon, "':'")?;
                let return_type = self.parse_return_type()?;
                let member_span = self.span_from(member_start);
                AnonymousStructMember::Method(AnonymousStructMethod::new(
                    name,
                    is_optional,
                    type_parameters,
                    params,
                    return_type,
                    member_span,
                ))
            } else {
                self.expect(TokenKind::Colon, "':'")?;
                let ty = self.parse_type()?;
                let member_span = self.span_from(member_start);
                AnonymousStructMember::Field(AnonymousStructField::new(
                    is_readonly,
                    name,
                    is_optional,
                    ty,
                    member_span,
                ))
            };

            members.push(member);

            if self.eat(TokenKind::Comma).is_none() {
                self.parse_eos()?;
            }
        }

        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(open.span);
        let id = self.alloc_id();
        Ok(Type::new(TypeKind::AnonymousStruct(members), span).with_id(id))
    }

    fn try_parse_non_generic_function_type(&mut self) -> ParseResult<Option<Type>> {
        // Fast path: `()` must be a function type `() => ReturnType`.
        if self.peek_kind(1) == TokenKind::CloseParen {
            return self.parse_function_type(false).map(Some);
        }

        let cp = self.checkpoint();
        let start = self.current_span();
        if let Ok(params) = self.parse_function_type_param_list() {
            let definitely_fn = params.len() > 1
                || params
                    .iter()
                    .any(|p| p.name.is_some() || p.is_readonly || p.is_optional || p.is_rest);
            if self.at(TokenKind::Arrow) {
                self.bump(); // consume '=>'
                let ret = self.parse_return_type()?;
                let span = self.span_from(start);
                let id = self.alloc_id();
                return Ok(Some(
                    Type::function(FunctionType::new(None, params, ret, span), span).with_id(id),
                ));
            }
            if definitely_fn {
                let tok = self.current();
                return Err(ParseError::unexpected("'=>'", tok.kind, tok.span));
            }
        }

        self.restore(cp);
        Ok(None)
    }

    fn parse_function_type(&mut self, with_type_params: bool) -> ParseResult<Type> {
        let start = self.current_span();
        let type_parameters = if with_type_params && self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };
        let params = self.parse_function_type_param_list()?;
        self.expect(TokenKind::Arrow, "'=>'")?;
        let return_type = self.parse_return_type()?;
        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Type::function(
            FunctionType::new(type_parameters, params, return_type, span),
            span,
        )
        .with_id(id))
    }

    fn parse_function_type_param_list(&mut self) -> ParseResult<Vec<FunctionTypeParam>> {
        self.expect(TokenKind::OpenParen, "'('")?;
        let params = self.with_grouping(|p| {
            let mut list = Vec::new();
            if !p.at(TokenKind::CloseParen) {
                loop {
                    let param = p.parse_function_type_param()?;
                    let was_rest = param.is_rest;
                    list.push(param);
                    if p.eat(TokenKind::Comma).is_none() || p.at(TokenKind::CloseParen) {
                        break;
                    }
                    if was_rest {
                        let tok = p.current();
                        return Err(ParseError::syntax(
                            "Rest parameter must be last in function type parameter list",
                            tok.span,
                        ));
                    }
                }
            }
            Ok::<_, ParseError>(list)
        })?;
        self.expect(TokenKind::CloseParen, "')'")?;
        Ok(params)
    }

    fn parse_function_type_param(&mut self) -> ParseResult<FunctionTypeParam> {
        let start = self.current_span();
        let is_rest = self.eat(TokenKind::Ellipsis).is_some();
        let is_readonly = self.eat(TokenKind::Readonly).is_some();

        let (name, is_optional, ty) = if self.at(TokenKind::Identifier)
            && (self.peek_kind(1) == TokenKind::Colon
                || (self.peek_kind(1) == TokenKind::Question
                    && self.peek_kind(2) == TokenKind::Colon))
        {
            let name_tok = self.bump();
            let ident = Ident::new(self.slice(name_tok.span), name_tok.span);
            let opt = self.eat(TokenKind::Question).is_some();
            self.bump(); // consume ':'
            let ty = self.parse_type()?;
            (Some(ident), opt, ty)
        } else {
            let ty = self.parse_type()?;
            (None, false, ty)
        };

        let span = self.span_from(start);
        Ok(FunctionTypeParam::new(
            name,
            is_readonly,
            is_optional,
            is_rest,
            ty,
            span,
        ))
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

        loop {
            args.push(self.parse_type()?);
            if self.eat(TokenKind::Comma).is_none() || self.at_generic_gt_start() {
                break;
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

            if self.eat(TokenKind::Comma).is_none() || self.at_generic_gt_start() {
                break;
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
