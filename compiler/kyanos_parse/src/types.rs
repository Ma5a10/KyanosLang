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

use crate::error::{ParseError, ParseErrorKind, ParseResult};
use crate::Parser;

/// Returns `true` if `kind` can begin a Kyanos type annotation.
#[inline]
pub(crate) fn can_start_type_kind(kind: TokenKind) -> bool {
    token_to_predefined_type(kind).is_some()
        || matches!(
            kind,
            TokenKind::Identifier
                | TokenKind::LessThan
                | TokenKind::OpenParen
                | TokenKind::Star
                | TokenKind::And
                | TokenKind::OpenBracket
                | TokenKind::OpenBrace
                | TokenKind::This
        )
}

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
    /// Returns `true` if the current token can begin a type annotation.
    #[inline]
    pub(crate) fn can_start_type(&self) -> bool {
        can_start_type_kind(self.current_kind())
    }

    /// Returns `true` if the token `offset` positions ahead can begin a type annotation.
    #[inline]
    pub(crate) fn can_start_type_at(&self, offset: usize) -> bool {
        can_start_type_kind(self.peek_kind(offset))
    }

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
            if self.at(TokenKind::Question) {
                if self.should_defer_question_to_ternary() {
                    break;
                }
                self.bump();
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

    #[inline]
    fn should_defer_question_to_ternary(&mut self) -> bool {
        if !self.stop_optional_before_ternary {
            return false;
        }
        if !crate::expr::can_start_expr_kind(self.peek_kind(1)) {
            return false;
        }
        self.stream.has_ternary_colon_from_question(
            self.pending_ternary_then,
            &mut self.memo.no_colon_after_questions,
            crate::expr::can_start_expr_kind,
        )
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
                let type_args = if self.at(TokenKind::LessThan)
                    && (self.grouping_depth > 0 || !self.current().has_leading_newline)
                {
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
        let members = self.with_isolated_ternary_scope(|p| {
            let mut members = Vec::new();

            while !p.at(TokenKind::CloseBrace) && !p.is_eof() {
                if p.eat(TokenKind::Semicolon).is_some() {
                    continue;
                }

                let start_cursor = p.checkpoint().cursor;
                match p.parse_anonymous_struct_member() {
                    Ok((member, synced_separator)) => {
                        members.push(member);
                        if !synced_separator && p.eat(TokenKind::Comma).is_none() {
                            if let Err(err) = p.parse_eos() {
                                p.report_error(err);
                                p.synchronize_braced_element(false, true);
                            }
                        }
                    }
                    Err(err) => {
                        p.report_error(err);
                        p.synchronize_braced_element_from(start_cursor, false, true);
                    }
                }
            }
            Ok::<_, ParseError>(members)
        })?;

        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(open.span);
        let id = self.alloc_id();
        Ok(Type::new(TypeKind::AnonymousStruct(members), span).with_id(id))
    }

    fn parse_anonymous_struct_member(&mut self) -> ParseResult<(AnonymousStructMember, bool)> {
        let member_start = self.current_span();
        let is_readonly = self.eat(TokenKind::Readonly).is_some();
        let name_tok = self.expect(TokenKind::Identifier, "member identifier")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);
        let is_optional = self.eat(TokenKind::Question).is_some();

        if !is_readonly && (self.at(TokenKind::LessThan) || self.at(TokenKind::OpenParen)) {
            let type_parameters = if self.at(TokenKind::LessThan) {
                Some(self.parse_type_parameters()?)
            } else {
                None
            };
            let params = self.parse_parameter_list()?;
            self.expect(TokenKind::Colon, "':'")?;
            let mut synced_separator = false;
            let return_type = match self.parse_return_type() {
                Ok(rt) => rt,
                Err(err) => {
                    let err_span = err.span();
                    self.report_error(err);
                    self.synchronize_braced_element(false, true);
                    synced_separator = true;
                    let err_id = self.alloc_id();
                    ReturnType::Type(Type::error(err_span).with_id(err_id))
                }
            };
            let member_span = self.span_from(member_start);
            Ok((
                AnonymousStructMember::Method(AnonymousStructMethod::new(
                    name,
                    is_optional,
                    type_parameters,
                    params,
                    return_type,
                    member_span,
                )),
                synced_separator,
            ))
        } else {
            self.expect(TokenKind::Colon, "':'")?;
            let mut synced_separator = false;
            let ty = match self.parse_type() {
                Ok(ty) => ty,
                Err(err) => {
                    let err_span = err.span();
                    self.report_error(err);
                    self.synchronize_braced_element(false, true);
                    synced_separator = true;
                    let err_id = self.alloc_id();
                    Type::error(err_span).with_id(err_id)
                }
            };
            let member_span = self.span_from(member_start);
            Ok((
                AnonymousStructMember::Field(AnonymousStructField::new(
                    is_readonly,
                    name,
                    is_optional,
                    ty,
                    member_span,
                )),
                synced_separator,
            ))
        }
    }

    fn try_parse_non_generic_function_type(&mut self) -> ParseResult<Option<Type>> {
        // Fast path: `()` must be a function type `() => ReturnType`.
        if self.peek_kind(1) == TokenKind::CloseParen {
            return self.parse_function_type(false).map(Some);
        }

        // Pre-scan without recursion so deeply nested `((((T))))` skips speculative function
        // parsing in O(N) total time when no `=>` follows the matching `)`.
        if !self
            .stream
            .scan_paren_followed_by(&mut self.memo.non_arrow_type_parens, false)
        {
            return Ok(None);
        }

        self.parse_function_type(false).map(Some)
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
        let memo_key = self.memo_pos_key();
        let res = self.parse_type_arguments_inner();
        if let Err(ref err) = res {
            if *err.kind() != ParseErrorKind::RecursionLimitExceeded {
                if let Some(key) = memo_key {
                    self.memo.failed_type_arguments.insert(key);
                }
            }
        }
        res
    }

    fn parse_type_arguments_inner(&mut self) -> ParseResult<TypeArguments> {
        let open = self.expect(TokenKind::LessThan, "'<'")?;
        let args = self.with_grouping(|p| {
            let mut args = Vec::new();
            loop {
                args.push(p.parse_type()?);
                if p.eat(TokenKind::Comma).is_none() || p.at_generic_gt_start() {
                    break;
                }
            }
            Ok::<_, ParseError>(args)
        })?;

        self.expect_generic_gt("'>'")?;
        let span = self.span_from(open.span);
        Ok(TypeArguments::new(args, span))
    }

    /// Speculatively parses `<T, U>`, using start-token guards and failed-position memoization
    /// so chained relational `<` expressions avoid quadratic rescanning.
    pub(crate) fn try_parse_type_arguments(&mut self) -> Option<TypeArguments> {
        if !self.can_start_type_at(1) {
            return None;
        }
        if let Some(key) = self.memo_pos_key() {
            if self.memo.failed_type_arguments.contains(&key) {
                return None;
            }
        }
        let cp = self.checkpoint();
        match self.parse_type_arguments() {
            Ok(args) => Some(args),
            Err(_) => {
                self.restore(cp);
                None
            }
        }
    }

    /// Parses a generic type parameter declaration list: `<T extends Bound = Default>`.
    pub fn parse_type_parameters(&mut self) -> ParseResult<TypeParameters> {
        let open = self.expect(TokenKind::LessThan, "'<'")?;
        let params = self.with_grouping(|p| {
            let mut params = Vec::new();
            loop {
                let name_tok = p.expect(TokenKind::Identifier, "type parameter name")?;
                let name = Ident::new(p.slice(name_tok.span), name_tok.span);

                let constraint = if p.eat(TokenKind::Extends).is_some() {
                    Some(p.parse_type()?)
                } else {
                    None
                };

                let default = if p.eat(TokenKind::Assign).is_some() {
                    Some(p.parse_type()?)
                } else {
                    None
                };

                let param_span = p.span_from(name_tok.span);
                params.push(TypeParameter::new(name, constraint, default, param_span));

                if p.eat(TokenKind::Comma).is_none() || p.at_generic_gt_start() {
                    break;
                }
            }
            Ok::<_, ParseError>(params)
        })?;

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
