// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pratt expression parser for Kyanos.
//!
//! Precedence levels and associativity follow `kyanos_ast::op::precedence`
//! and `design/grammer/kyanos_syntax_grammar.g4`.

use kyanos_ast::{
    precedence, Argument, ArrayElement, ArrowBody, AssignOp, BinaryOp, Expr, ExprKind, Ident,
    Literal, MemberProperty, ObjectMember, ObjectMethod, Parameter, ParameterList, PostfixUnaryOp,
    PrefixUnaryOp, PrivateIdent, PropertyName, ReturnType, SizeofTarget, Span, StringLiteral,
    StructFieldInit, SwitchExprArm, TypeArguments, TypeName, TypeParameters,
};
use kyanos_lexer::TokenKind;

use crate::error::{ParseError, ParseErrorKind, ParseResult};
use crate::types::token_to_predefined_type;
use crate::Parser;

impl<'a> Parser<'a> {
    /// Parses a single expression using full Pratt operator precedence.
    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_expr_bp(0)
    }

    /// Parses a single expression, returning an `ExprKind::Error` node if parsing fails.
    pub fn parse_expr_with_recovery(&mut self) -> Expr {
        match self.parse_expr() {
            Ok(expr) => expr,
            Err(err) => {
                let span = err.span();
                self.report_error(err);
                let id = self.alloc_id();
                Expr::error(span).with_id(id)
            }
        }
    }

    /// Parses a comma-separated sequence of expressions (`singleExpression (',' singleExpression)*`).
    pub fn parse_expression_sequence(&mut self) -> ParseResult<Vec<Expr>> {
        let mut exprs = vec![self.parse_expr()?];
        while self.eat(TokenKind::Comma).is_some() {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }

    /// Core Pratt parser loop with minimum binding power `min_bp`.
    pub(crate) fn parse_expr_bp(&mut self, min_bp: u8) -> ParseResult<Expr> {
        self.enter_recursion()?;
        let res = self.parse_expr_bp_inner(min_bp);
        self.exit_recursion();
        res
    }

    fn parse_expr_bp_inner(&mut self, min_bp: u8) -> ParseResult<Expr> {
        let mut lhs = self.parse_prefix_expr()?;

        while self.step_led(&mut lhs, min_bp)? {}

        Ok(lhs)
    }

    #[inline(never)]
    fn step_led(&mut self, lhs: &mut Expr, min_bp: u8) -> ParseResult<bool> {
        let tok = self.current();

        // Prevent statement-starting tokens after a line break from unintentionally
        // continuing the previous expression when outside `(...)` / `[...]`.
        if self.grouping_depth == 0
            && tok.has_leading_newline
            && is_newline_statement_boundary(tok.kind)
        {
            return Ok(false);
        }

        // 1. Postfix operators (`++`, `--`, `!`)
        if let Some(post_op) = postfix_op_from_token(tok.kind) {
            let l_bp = post_op.left_binding_power();
            if l_bp < min_bp {
                return Ok(false);
            }
            if matches!(post_op, PostfixUnaryOp::PostInc | PostfixUnaryOp::PostDec) {
                self.report_if_invalid_assignment_target(lhs, "postfix update expression");
            }
            self.bump();
            let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
            let span = self.span_from(prev.span);
            let id = self.alloc_id();
            *lhs = Expr::unary_postfix(post_op, prev, span).with_id(id);
            return Ok(true);
        }

        // 2. Postfix member access, call, index, generic call, and struct instantiation
        match tok.kind {
            TokenKind::Dot => {
                if precedence::POSTFIX_LEFT_BP < min_bp {
                    return Ok(false);
                }
                self.bump();
                let prop = self.parse_member_property()?;
                let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
                let span = self.span_from(prev.span);
                let id = self.alloc_id();
                *lhs = Expr::member(prev, prop, false, span).with_id(id);
                return Ok(true);
            }
            TokenKind::OptionalChain => {
                if precedence::POSTFIX_LEFT_BP < min_bp {
                    return Ok(false);
                }
                let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
                *lhs = self.parse_optional_chain_postfix(prev)?;
                return Ok(true);
            }
            TokenKind::OpenBracket => {
                if precedence::POSTFIX_LEFT_BP < min_bp {
                    return Ok(false);
                }
                let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
                *lhs = self.parse_index_postfix(prev, false)?;
                return Ok(true);
            }
            TokenKind::OpenParen => {
                if precedence::POSTFIX_LEFT_BP < min_bp {
                    return Ok(false);
                }
                let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
                *lhs = self.parse_call_postfix(prev, None)?;
                return Ok(true);
            }
            TokenKind::OpenBrace => {
                if precedence::POSTFIX_LEFT_BP < min_bp {
                    return Ok(false);
                }
                if let Some(struct_name) = expr_to_type_name(lhs) {
                    *lhs = self.parse_struct_instantiation_body(struct_name, None)?;
                    return Ok(true);
                }
                return Ok(false);
            }
            _ => {}
        }

        // 3. Generic call `f<T>(args)` / generic struct instantiation `Pair<T> { ... }`
        //    versus relational `<` comparison.
        if tok.kind == TokenKind::LessThan {
            let (lt_l_bp, _) = BinaryOp::Lt.binding_power();
            if precedence::POSTFIX_LEFT_BP >= min_bp {
                if let Some(next_lhs) = self.try_parse_generic_postfix(lhs)? {
                    *lhs = next_lhs;
                    return Ok(true);
                }
            }
            if lt_l_bp < min_bp {
                return Ok(false);
            }
        }

        // 4. Type cast: `expr as Type`
        if tok.kind == TokenKind::As {
            if precedence::CAST_LEFT_BP < min_bp {
                return Ok(false);
            }
            let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
            *lhs = self.parse_cast_postfix(prev)?;
            return Ok(true);
        }

        // 5. Runtime type test: `expr is Type`
        if tok.kind == TokenKind::Is {
            let (l_bp, _) = precedence::TYPE_TEST_BP;
            if l_bp < min_bp {
                return Ok(false);
            }
            let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
            *lhs = self.parse_type_test_postfix(prev)?;
            return Ok(true);
        }

        // 6. Binary operators (arithmetic, shift, relational, equality, bitwise, logical, ??)
        if let Some(bin_op) = binary_op_from_token(tok.kind) {
            let (l_bp, r_bp) = bin_op.binding_power();
            if l_bp < min_bp {
                return Ok(false);
            }
            self.bump();
            let rhs = self.parse_expr_bp(r_bp)?;
            let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
            let id = self.alloc_id();
            *lhs = Expr::binary(bin_op, prev, rhs).with_id(id);
            return Ok(true);
        }

        // 7. Ternary conditional: `cond ? then_expr : else_expr`
        if tok.kind == TokenKind::Question {
            let (l_bp, r_bp) = precedence::TERNARY_BP;
            if l_bp < min_bp {
                return Ok(false);
            }
            let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
            *lhs = self.parse_ternary_postfix(prev, r_bp)?;
            return Ok(true);
        }

        // 8. Assignment operators: `lhs = rhs`, `lhs += rhs`, etc.
        if let Some(assign_op) = assign_op_from_token(tok.kind) {
            let (l_bp, r_bp) = assign_op.binding_power();
            if l_bp < min_bp {
                return Ok(false);
            }
            let prev = std::mem::replace(lhs, Expr::error(Span::DUMMY));
            *lhs = self.parse_assign_postfix(assign_op, prev, r_bp)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Parses a prefix / primary expression (`nud`).
    fn parse_prefix_expr(&mut self) -> ParseResult<Expr> {
        let tok = self.current();

        // Predefined primitive type member access: `string.fromCString(...)`, `int.MAX_VALUE`
        if let Some(pt) = token_to_predefined_type(tok.kind) {
            return self.parse_predefined_type_member_expr(pt, tok.span);
        }

        // Prefix unary operators
        if prefix_op_from_token(tok.kind).is_some() {
            return self.parse_prefix_unary_chain();
        }

        match tok.kind {
            // Literals
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
                Ok(Expr::literal(lit).with_id(id))
            }
            TokenKind::TemplateNoSubstitution | TokenKind::TemplateHead => {
                self.bump();
                self.parse_template_string_expr(tok)
            }
            TokenKind::This => {
                self.bump();
                let id = self.alloc_id();
                Ok(Expr::this(tok.span).with_id(id))
            }
            TokenKind::Super => {
                self.bump();
                let id = self.alloc_id();
                Ok(Expr::super_kw(tok.span).with_id(id))
            }
            TokenKind::Identifier => {
                if self.peek_kind(1) == TokenKind::Arrow {
                    return self.parse_single_ident_arrow_expr();
                }
                self.bump();
                let ident = Ident::new(self.slice(tok.span), tok.span);
                let id = self.alloc_id();
                Ok(Expr::ident(ident).with_id(id))
            }
            TokenKind::OpenParen => self.parse_paren_or_arrow_expr(),
            TokenKind::Async => self.parse_async_arrow_expr(),
            TokenKind::LessThan => self.parse_generic_arrow_expr(),
            TokenKind::OpenBracket => self.parse_array_literal(),
            TokenKind::OpenBrace => self.parse_object_literal(),
            TokenKind::Switch => self.parse_switch_expr(),
            TokenKind::Unsafe => self.parse_unsafe_expr(),
            TokenKind::Sizeof => self.parse_sizeof_expr(),
            TokenKind::New => self.parse_new_expr(),
            other => Err(ParseError::unexpected("expression", other, tok.span)),
        }
    }

    #[inline(never)]
    fn parse_predefined_type_member_expr(
        &mut self,
        ty: kyanos_ast::PredefinedType,
        start: Span,
    ) -> ParseResult<Expr> {
        self.bump();
        self.expect(TokenKind::Dot, "'.'")?;
        let member_tok = self.expect(TokenKind::Identifier, "member identifier")?;
        let member = Ident::new(self.slice(member_tok.span), member_tok.span);
        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Expr::new(ExprKind::PredefinedTypeMember { ty, member }, span).with_id(id))
    }

    #[inline(never)]
    fn parse_single_ident_arrow_expr(&mut self) -> ParseResult<Expr> {
        let ident_tok = self.bump();
        let ident = Ident::new(self.slice(ident_tok.span), ident_tok.span);
        let params = ParameterList::new(
            vec![Parameter::simple(ident, None, ident_tok.span)],
            None,
            ident_tok.span,
        );
        self.finish_arrow_function(ident_tok.span, false, None, params, None)
    }

    /// Parses a chain of prefix unary operators iteratively to keep stack usage O(1) while enforcing `MAX_RECURSION_DEPTH`.
    fn parse_prefix_unary_chain(&mut self) -> ParseResult<Expr> {
        let mut ops: Vec<(PrefixUnaryOp, Span)> = Vec::new();
        while let Some(op) = prefix_op_from_token(self.current_kind()) {
            if let Err(err) = self.enter_recursion() {
                for _ in 0..ops.len() {
                    self.exit_recursion();
                }
                return Err(err);
            }
            let tok = self.bump();
            ops.push((op, tok.span));
        }

        let last_rbp = ops
            .last()
            .expect("at least one prefix op")
            .0
            .right_binding_power();
        let res = self.parse_expr_bp(last_rbp);
        let mut target = match res {
            Ok(expr) => expr,
            Err(err) => {
                for _ in 0..ops.len() {
                    self.exit_recursion();
                }
                return Err(err);
            }
        };

        for (op, op_span) in ops.into_iter().rev() {
            self.exit_recursion();
            if matches!(op, PrefixUnaryOp::PreInc | PrefixUnaryOp::PreDec) {
                self.report_if_invalid_assignment_target(&target, "prefix update expression");
            }
            let span = op_span.merge(target.span);
            let id = self.alloc_id();
            target = Expr::unary_prefix(op, target, span).with_id(id);
        }
        Ok(target)
    }

    // -------------------------------------------------------------------------
    // Postfix Helpers
    // -------------------------------------------------------------------------

    #[inline(never)]
    fn try_parse_generic_postfix(&mut self, lhs: &Expr) -> ParseResult<Option<Expr>> {
        if !self.can_start_type_at(1) {
            return Ok(None);
        }
        let memo_key = self.memo_pos_key();
        if let Some(key) = memo_key {
            if self.memo.failed_generic_postfix.contains(&key)
                || self.memo.failed_type_arguments.contains(&key)
            {
                return Ok(None);
            }
        }

        let cp = self.checkpoint();
        if let Some(type_args) = self.try_parse_type_arguments() {
            let next = self.current();
            if next.kind == TokenKind::OpenParen
                && (self.grouping_depth > 0 || !next.has_leading_newline)
            {
                return self
                    .parse_call_postfix(lhs.clone(), Some(type_args))
                    .map(Some);
            }
            if next.kind == TokenKind::OpenBrace && !next.has_leading_newline {
                if let Some(struct_name) = expr_to_type_name(lhs) {
                    return self
                        .parse_struct_instantiation_body(struct_name, Some(type_args))
                        .map(Some);
                }
            }
        }
        self.restore(cp);
        if let Some(key) = memo_key {
            self.memo.failed_generic_postfix.insert(key);
        }
        Ok(None)
    }

    #[inline(never)]
    fn parse_cast_postfix(&mut self, lhs: Expr) -> ParseResult<Expr> {
        self.bump();
        let ty = self.with_stop_optional_before_ternary(true, |p| p.parse_type())?;
        let span = self.span_from(lhs.span);
        let id = self.alloc_id();
        Ok(Expr::cast(lhs, ty, span).with_id(id))
    }

    #[inline(never)]
    fn parse_type_test_postfix(&mut self, lhs: Expr) -> ParseResult<Expr> {
        self.bump();
        let ty = self.with_stop_optional_before_ternary(true, |p| p.parse_type())?;
        let span = self.span_from(lhs.span);
        let id = self.alloc_id();
        Ok(Expr::type_test(lhs, ty, span).with_id(id))
    }

    #[inline(never)]
    fn parse_ternary_postfix(&mut self, lhs: Expr, r_bp: u8) -> ParseResult<Expr> {
        self.bump(); // consume '?'
        self.pending_ternary_then += 1;
        let then_res = self.parse_expr();
        self.pending_ternary_then -= 1;
        let then_expr = then_res?;
        self.expect(TokenKind::Colon, "':'")?;
        let else_expr = self.parse_expr_bp(r_bp)?;
        let id = self.alloc_id();
        Ok(Expr::ternary(lhs, then_expr, else_expr).with_id(id))
    }

    #[inline(never)]
    fn parse_assign_postfix(
        &mut self,
        assign_op: AssignOp,
        lhs: Expr,
        r_bp: u8,
    ) -> ParseResult<Expr> {
        self.report_if_invalid_assignment_target(&lhs, "assignment expression");
        self.bump();
        let rhs = self.parse_expr_bp(r_bp)?;
        let id = self.alloc_id();
        Ok(Expr::assign(assign_op, lhs, rhs).with_id(id))
    }

    #[inline]
    fn report_if_invalid_assignment_target(&mut self, expr: &Expr, context: &str) {
        if !is_valid_assignment_target(expr) {
            self.report_error(ParseError::new(
                ParseErrorKind::InvalidAssignmentTarget,
                expr.span,
                format!("Invalid left-hand side in {context}"),
            ));
        }
    }

    fn parse_member_property(&mut self) -> ParseResult<MemberProperty> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Identifier => {
                self.bump();
                Ok(MemberProperty::Ident(Ident::new(
                    self.slice(tok.span),
                    tok.span,
                )))
            }
            TokenKind::PrivateIdentifier => {
                self.bump();
                Ok(MemberProperty::Private(PrivateIdent::new(
                    self.slice(tok.span),
                    tok.span,
                )))
            }
            other => Err(ParseError::unexpected(
                "property identifier",
                other,
                tok.span,
            )),
        }
    }

    fn parse_index_postfix(&mut self, target: Expr, is_optional: bool) -> ParseResult<Expr> {
        self.expect(TokenKind::OpenBracket, "'['")?;
        let mut exprs = self.with_grouping(|p| p.parse_expression_sequence())?;
        self.expect(TokenKind::CloseBracket, "']'")?;
        let index = if exprs.len() == 1 {
            exprs.pop().expect("single expression")
        } else {
            let seq_span = exprs
                .first()
                .map(|e| e.span)
                .unwrap_or(Span::DUMMY)
                .merge(exprs.last().map(|e| e.span).unwrap_or(Span::DUMMY));
            let seq_id = self.alloc_id();
            Expr::new(ExprKind::Sequence(exprs), seq_span).with_id(seq_id)
        };
        let span = self.span_from(target.span);
        let id = self.alloc_id();
        Ok(Expr::index(target, index, is_optional, span).with_id(id))
    }

    fn parse_call_postfix(
        &mut self,
        callee: Expr,
        type_arguments: Option<TypeArguments>,
    ) -> ParseResult<Expr> {
        let args = self.parse_argument_list()?;
        let span = self.span_from(callee.span);
        let id = self.alloc_id();
        Ok(Expr::call(callee, type_arguments, args, span).with_id(id))
    }

    fn parse_argument_list(&mut self) -> ParseResult<Vec<Argument>> {
        self.expect(TokenKind::OpenParen, "'('")?;
        let args = self.with_grouping(|p| {
            let mut list = Vec::new();
            if !p.at(TokenKind::CloseParen) {
                loop {
                    if let Some(spread_tok) = p.eat(TokenKind::Ellipsis) {
                        let expr = p.parse_expr()?;
                        let span = p.span_from(spread_tok.span);
                        list.push(Argument::spread(expr, span));
                    } else {
                        let expr = p.parse_expr()?;
                        list.push(Argument::normal(expr));
                    }
                    if p.eat(TokenKind::Comma).is_none() || p.at(TokenKind::CloseParen) {
                        break;
                    }
                }
            }
            Ok::<_, ParseError>(list)
        })?;
        self.expect(TokenKind::CloseParen, "')'")?;
        Ok(args)
    }

    // -------------------------------------------------------------------------
    // Parenthesized, Sequence, and Arrow Function Expressions
    // -------------------------------------------------------------------------

    fn parse_paren_or_arrow_expr(&mut self) -> ParseResult<Expr> {
        // Fast path: `()` is either a 0-param arrow function or the unit literal `()`.
        if self.peek_kind(1) == TokenKind::CloseParen {
            let open = self.bump(); // '('
            self.bump(); // ')'
            let paren_span = self.span_from(open.span);

            let cp = self.checkpoint();
            let return_type = if self.eat(TokenKind::Colon).is_some() {
                self.parse_return_type().ok()
            } else {
                None
            };

            if self.at(TokenKind::Arrow) {
                let params = ParameterList::empty(paren_span);
                return self.finish_arrow_function(open.span, false, None, params, return_type);
            }

            self.restore(cp);
            let id = self.alloc_id();
            return Ok(Expr::literal(Literal::unit(paren_span)).with_id(id));
        }

        // Pre-scan without recursion: if the matching `)` is followed by neither `=>` nor `:`,
        // this cannot be `(params) (: ret)? => body`, so jump straight to parenthesized expression.
        let memo_key = self.memo_pos_key();
        let can_try_arrow = self
            .stream
            .scan_paren_followed_by(&mut self.memo.non_arrow_expr_parens, true)
            && memo_key.is_none_or(|key| !self.memo.failed_arrow_expr_parens.contains(&key));

        if !can_try_arrow {
            return self.parse_parenthesized_sequence();
        }

        let start = self.current_span();
        let cp = self.checkpoint();
        let mut cache_arrow_failure = true;

        // Speculatively parse as an arrow parameter list `(params) (: ret)? => body`.
        match self.with_grouping(|p| p.parse_parameter_list()) {
            Ok(params) => {
                let definitely_params = params_require_arrow(&params);
                let return_type = if self.eat(TokenKind::Colon).is_some() {
                    match self.parse_return_type() {
                        Ok(rt) => Some(rt),
                        Err(err) => {
                            if *err.kind() == ParseErrorKind::RecursionLimitExceeded {
                                cache_arrow_failure = false;
                            }
                            if definitely_params {
                                return Err(err);
                            }
                            self.restore(cp);
                            if cache_arrow_failure {
                                if let Some(key) = memo_key {
                                    self.memo.failed_arrow_expr_parens.insert(key);
                                }
                            }
                            return self.parse_parenthesized_sequence();
                        }
                    }
                } else {
                    None
                };

                if self.at(TokenKind::Arrow) {
                    return self.finish_arrow_function(start, false, None, params, return_type);
                }

                if definitely_params {
                    let tok = self.current();
                    return Err(ParseError::unexpected("'=>'", tok.kind, tok.span));
                }
            }
            Err(err) => {
                if *err.kind() == ParseErrorKind::RecursionLimitExceeded {
                    cache_arrow_failure = false;
                }
            }
        }

        self.restore(cp);
        if cache_arrow_failure {
            if let Some(key) = memo_key {
                self.memo.failed_arrow_expr_parens.insert(key);
            }
        }
        self.parse_parenthesized_sequence()
    }

    fn parse_parenthesized_sequence(&mut self) -> ParseResult<Expr> {
        let open = self.expect(TokenKind::OpenParen, "'('")?;
        let mut exprs = self.with_grouping(|p| p.parse_expression_sequence())?;
        self.expect(TokenKind::CloseParen, "')'")?;
        let span = self.span_from(open.span);
        let id = self.alloc_id();
        if exprs.len() == 1 {
            let inner = exprs.pop().expect("single expression");
            Ok(Expr::new(ExprKind::Parenthesized(Box::new(inner)), span).with_id(id))
        } else {
            Ok(Expr::new(ExprKind::Sequence(exprs), span).with_id(id))
        }
    }

    fn parse_async_arrow_expr(&mut self) -> ParseResult<Expr> {
        let async_tok = self.expect(TokenKind::Async, "'async'")?;
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };
        let params = self.parse_arrow_params_after_prefix(type_parameters.is_some())?;
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_return_type()?)
        } else {
            None
        };
        self.finish_arrow_function(async_tok.span, true, type_parameters, params, return_type)
    }

    fn parse_generic_arrow_expr(&mut self) -> ParseResult<Expr> {
        let start = self.current_span();
        let type_parameters = Some(self.parse_type_parameters()?);
        let params = self.with_grouping(|p| p.parse_parameter_list())?;
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_return_type()?)
        } else {
            None
        };
        self.finish_arrow_function(start, false, type_parameters, params, return_type)
    }

    fn parse_arrow_params_after_prefix(
        &mut self,
        had_type_params: bool,
    ) -> ParseResult<ParameterList> {
        if !had_type_params && self.at(TokenKind::Identifier) {
            let ident_tok = self.bump();
            let ident = Ident::new(self.slice(ident_tok.span), ident_tok.span);
            Ok(ParameterList::new(
                vec![Parameter::simple(ident, None, ident_tok.span)],
                None,
                ident_tok.span,
            ))
        } else {
            self.with_grouping(|p| p.parse_parameter_list())
        }
    }

    fn finish_arrow_function(
        &mut self,
        start: Span,
        is_async: bool,
        type_parameters: Option<TypeParameters>,
        params: ParameterList,
        return_type: Option<ReturnType>,
    ) -> ParseResult<Expr> {
        self.expect(TokenKind::Arrow, "'=>'")?;
        let body = if self.at(TokenKind::OpenBrace) {
            ArrowBody::Block(self.parse_block()?)
        } else {
            ArrowBody::Expr(Box::new(self.parse_expr()?))
        };
        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(
            Expr::arrow_function(is_async, type_parameters, params, return_type, body, span)
                .with_id(id),
        )
    }

    // -------------------------------------------------------------------------
    // Composite Literals & Control Expressions
    // -------------------------------------------------------------------------

    fn parse_array_literal(&mut self) -> ParseResult<Expr> {
        let open = self.expect(TokenKind::OpenBracket, "'['")?;
        let elements = self.with_grouping(|p| {
            let mut elements = Vec::new();
            while !p.at(TokenKind::CloseBracket) && !p.is_eof() {
                let elem = if let Some(spread_tok) = p.eat(TokenKind::Ellipsis) {
                    let expr = p.parse_expr_bp(0)?;
                    let span = p.span_from(spread_tok.span);
                    ArrayElement::spread(expr, span)
                } else {
                    ArrayElement::normal(p.parse_expr_bp(0)?)
                };
                elements.push(elem);
                if p.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            Ok::<_, ParseError>(elements)
        })?;
        self.expect(TokenKind::CloseBracket, "']'")?;
        let span = self.span_from(open.span);
        let id = self.alloc_id();
        Ok(Expr::new(ExprKind::Array(elements), span).with_id(id))
    }

    fn parse_object_literal(&mut self) -> ParseResult<Expr> {
        let open = self.expect(TokenKind::OpenBrace, "'{'")?;
        let members = self.with_isolated_ternary_scope(|p| {
            let mut members = Vec::new();
            while !p.at(TokenKind::CloseBrace) && !p.is_eof() {
                members.push(p.parse_object_member()?);
                if p.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            Ok::<_, ParseError>(members)
        })?;

        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(open.span);
        let id = self.alloc_id();
        Ok(Expr::new(ExprKind::Object(members), span).with_id(id))
    }

    fn parse_object_member(&mut self) -> ParseResult<ObjectMember> {
        // Spread: `...expr`
        if let Some(spread_tok) = self.eat(TokenKind::Ellipsis) {
            let expr = self.parse_expr()?;
            let span = self.span_from(spread_tok.span);
            return Ok(ObjectMember::Spread { expr, span });
        }

        // Async object method: `async foo(...) { ... }`
        if let Some(async_tok) = self.eat(TokenKind::Async) {
            let name_tok = self.expect(TokenKind::Identifier, "method name")?;
            let name = Ident::new(self.slice(name_tok.span), name_tok.span);
            return self.finish_object_method(async_tok.span, true, name);
        }

        let tok = self.current();
        match tok.kind {
            TokenKind::Identifier => {
                let next = self.peek_kind(1);
                if next == TokenKind::OpenParen || next == TokenKind::LessThan {
                    self.bump();
                    let name = Ident::new(self.slice(tok.span), tok.span);
                    return self.finish_object_method(tok.span, false, name);
                }
                self.bump();
                let ident = Ident::new(self.slice(tok.span), tok.span);
                if self.eat(TokenKind::Colon).is_some() {
                    let value = self.parse_expr()?;
                    let span = self.span_from(tok.span);
                    Ok(ObjectMember::Property {
                        name: PropertyName::Ident(ident),
                        value,
                        span,
                    })
                } else {
                    Ok(ObjectMember::Shorthand {
                        name: ident,
                        span: tok.span,
                    })
                }
            }
            TokenKind::StringLiteral => {
                self.bump();
                let raw = self.slice(tok.span);
                let lit = self.parse_literal_token(tok)?;
                let value_str = match lit.kind {
                    kyanos_ast::LiteralKind::String(s) => s.value,
                    _ => String::new(),
                };
                let key = PropertyName::String(StringLiteral::new(value_str, raw, tok.span));
                self.expect(TokenKind::Colon, "':'")?;
                let value = self.parse_expr()?;
                let span = self.span_from(tok.span);
                Ok(ObjectMember::Property {
                    name: key,
                    value,
                    span,
                })
            }
            TokenKind::IntegerLiteral | TokenKind::FloatLiteral => {
                self.bump();
                let lit = self.parse_literal_token(tok)?;
                self.expect(TokenKind::Colon, "':'")?;
                let value = self.parse_expr()?;
                let span = self.span_from(tok.span);
                Ok(ObjectMember::Property {
                    name: PropertyName::Numeric(lit),
                    value,
                    span,
                })
            }
            TokenKind::OpenBracket => {
                let open = self.bump();
                let key_expr = self.with_grouping(|p| p.parse_expr())?;
                self.expect(TokenKind::CloseBracket, "']'")?;
                self.expect(TokenKind::Colon, "':'")?;
                let value = self.parse_expr()?;
                let span = self.span_from(open.span);
                Ok(ObjectMember::Property {
                    name: PropertyName::Computed(Box::new(key_expr)),
                    value,
                    span,
                })
            }
            other => Err(ParseError::unexpected("object member", other, tok.span)),
        }
    }

    fn finish_object_method(
        &mut self,
        start: Span,
        is_async: bool,
        name: Ident,
    ) -> ParseResult<ObjectMember> {
        let type_parameters = if self.at(TokenKind::LessThan) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };
        let params = self.with_grouping(|p| p.parse_parameter_list())?;
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_return_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = self.span_from(start);
        Ok(ObjectMember::Method(ObjectMethod {
            is_async,
            name,
            type_parameters,
            params,
            return_type,
            body,
            span,
        }))
    }

    fn parse_struct_instantiation_body(
        &mut self,
        struct_name: TypeName,
        type_arguments: Option<TypeArguments>,
    ) -> ParseResult<Expr> {
        let start = struct_name.span;
        self.expect(TokenKind::OpenBrace, "'{'")?;
        let fields = self.with_isolated_ternary_scope(|p| {
            let mut fields = Vec::new();

            while !p.at(TokenKind::CloseBrace) && !p.is_eof() {
                if let Some(spread_tok) = p.eat(TokenKind::Ellipsis) {
                    let expr = p.parse_expr()?;
                    let span = p.span_from(spread_tok.span);
                    fields.push(StructFieldInit::Spread { expr, span });
                } else {
                    let field_tok = p.expect(TokenKind::Identifier, "struct field name")?;
                    let name = Ident::new(p.slice(field_tok.span), field_tok.span);
                    let value = if p.eat(TokenKind::Colon).is_some() {
                        Some(p.parse_expr()?)
                    } else {
                        None
                    };
                    let span = p.span_from(field_tok.span);
                    fields.push(StructFieldInit::Field { name, value, span });
                }
                if p.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            Ok::<_, ParseError>(fields)
        })?;

        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(start);
        let id = self.alloc_id();
        Ok(Expr::struct_instantiation(struct_name, type_arguments, fields, span).with_id(id))
    }

    fn parse_switch_expr(&mut self) -> ParseResult<Expr> {
        let switch_tok = self.expect(TokenKind::Switch, "'switch'")?;
        self.expect(TokenKind::OpenParen, "'('")?;
        let target = self.with_grouping(|p| p.parse_expr())?;
        self.expect(TokenKind::CloseParen, "')'")?;
        self.expect(TokenKind::OpenBrace, "'{'")?;

        let arms = self.with_isolated_ternary_scope(|p| {
            let mut arms = Vec::new();
            while !p.at(TokenKind::CloseBrace) && !p.is_eof() {
                let arm_start = p.current_span();
                if p.eat(TokenKind::Case).is_some() {
                    let pattern = p.parse_pattern()?;
                    let guard = if p.eat(TokenKind::When).is_some() {
                        Some(Box::new(p.parse_expr()?))
                    } else {
                        None
                    };
                    p.expect(TokenKind::Arrow, "'=>'")?;
                    let body = Box::new(p.parse_expr()?);
                    let span = p.span_from(arm_start);
                    arms.push(SwitchExprArm::Case {
                        pattern,
                        guard,
                        body,
                        span,
                    });
                } else if p.eat(TokenKind::Default).is_some() {
                    p.expect(TokenKind::Arrow, "'=>'")?;
                    let body = Box::new(p.parse_expr()?);
                    let span = p.span_from(arm_start);
                    arms.push(SwitchExprArm::Default { body, span });
                } else {
                    let tok = p.current();
                    return Err(ParseError::unexpected(
                        "'case' or 'default'",
                        tok.kind,
                        tok.span,
                    ));
                }

                if p.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            Ok::<_, ParseError>(arms)
        })?;

        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(switch_tok.span);
        let id = self.alloc_id();
        Ok(Expr::new(
            ExprKind::Switch {
                target: Box::new(target),
                arms,
            },
            span,
        )
        .with_id(id))
    }

    fn parse_unsafe_expr(&mut self) -> ParseResult<Expr> {
        let unsafe_tok = self.expect(TokenKind::Unsafe, "'unsafe'")?;
        let block = self.parse_block()?;
        let span = self.span_from(unsafe_tok.span);
        let id = self.alloc_id();
        Ok(Expr::new(ExprKind::Unsafe(block), span).with_id(id))
    }

    fn parse_sizeof_expr(&mut self) -> ParseResult<Expr> {
        let sizeof_tok = self.expect(TokenKind::Sizeof, "'sizeof'")?;
        self.expect(TokenKind::OpenParen, "'('")?;

        let target = self.with_grouping(|p| {
            let memo_key = p.memo_pos_key();
            let should_try_type = p.can_start_type()
                && memo_key.is_none_or(|key| !p.memo.failed_sizeof_types.contains(&key));
            if should_try_type {
                let cp = p.checkpoint();
                let cache_failure = match p.parse_type() {
                    Ok(ty) if p.at(TokenKind::CloseParen) => {
                        return Ok(SizeofTarget::Type(Box::new(ty)));
                    }
                    Ok(_) => true,
                    Err(err) => *err.kind() != ParseErrorKind::RecursionLimitExceeded,
                };
                p.restore(cp);
                if cache_failure {
                    if let Some(key) = memo_key {
                        p.memo.failed_sizeof_types.insert(key);
                    }
                }
            }
            let expr = p.parse_expr()?;
            Ok(SizeofTarget::Expr(Box::new(expr)))
        })?;

        self.expect(TokenKind::CloseParen, "')'")?;
        let span = self.span_from(sizeof_tok.span);
        let id = self.alloc_id();
        Ok(Expr::new(ExprKind::Sizeof(target), span).with_id(id))
    }

    fn parse_new_expr(&mut self) -> ParseResult<Expr> {
        let new_tok = self.expect(TokenKind::New, "'new'")?;
        let mut callee = self.parse_new_callee_base()?;

        // Allow chained `.prop` / `?.prop` / `[idx]` on the constructor target without
        // greedily consuming `<T>(args)` as a call before `new` sees them.
        loop {
            let tok = self.current();
            if self.grouping_depth == 0 && tok.has_leading_newline {
                break;
            }
            match tok.kind {
                TokenKind::Dot | TokenKind::OptionalChain => {
                    let is_optional = tok.kind == TokenKind::OptionalChain;
                    self.bump();
                    if is_optional && self.at(TokenKind::OpenBracket) {
                        callee = self.parse_index_postfix(callee, true)?;
                    } else {
                        let prop = self.parse_member_property()?;
                        let span = self.span_from(callee.span);
                        let id = self.alloc_id();
                        callee = Expr::member(callee, prop, is_optional, span).with_id(id);
                    }
                }
                TokenKind::OpenBracket => {
                    callee = self.parse_index_postfix(callee, false)?;
                }
                _ => break,
            }
        }

        let type_arguments = if self.at(TokenKind::LessThan) {
            self.try_parse_type_arguments()
        } else {
            None
        };

        let args = if self.at(TokenKind::OpenParen)
            && (self.grouping_depth > 0 || !self.current().has_leading_newline)
        {
            Some(self.parse_argument_list()?)
        } else {
            None
        };

        let span = self.span_from(new_tok.span);
        let id = self.alloc_id();
        Ok(Expr::new_expr(callee, type_arguments, args, span).with_id(id))
    }

    fn parse_new_callee_base(&mut self) -> ParseResult<Expr> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Identifier => {
                self.bump();
                let id = self.alloc_id();
                Ok(Expr::ident(Ident::new(self.slice(tok.span), tok.span)).with_id(id))
            }
            TokenKind::This => {
                self.bump();
                let id = self.alloc_id();
                Ok(Expr::this(tok.span).with_id(id))
            }
            TokenKind::Super => {
                self.bump();
                let id = self.alloc_id();
                Ok(Expr::super_kw(tok.span).with_id(id))
            }
            TokenKind::OpenParen => self.parse_parenthesized_sequence(),
            other => Err(ParseError::unexpected(
                "constructor expression",
                other,
                tok.span,
            )),
        }
    }
}

// =============================================================================
// Operator Mapping & Validation Helpers
// =============================================================================

impl<'a> Parser<'a> {
    /// Postfix member or optional-index helper so `OptionalChain` handles both `?.prop` and `?.[i]`.
    pub(crate) fn parse_optional_chain_postfix(&mut self, lhs: Expr) -> ParseResult<Expr> {
        self.expect(TokenKind::OptionalChain, "'?.'")?;
        if self.at(TokenKind::OpenBracket) {
            self.parse_index_postfix(lhs, true)
        } else {
            let prop = self.parse_member_property()?;
            let span = self.span_from(lhs.span);
            let id = self.alloc_id();
            Ok(Expr::member(lhs, prop, true, span).with_id(id))
        }
    }
}

#[inline]
fn binary_op_from_token(kind: TokenKind) -> Option<BinaryOp> {
    match kind {
        TokenKind::Plus => Some(BinaryOp::Add),
        TokenKind::Minus => Some(BinaryOp::Sub),
        TokenKind::Star => Some(BinaryOp::Mul),
        TokenKind::Slash => Some(BinaryOp::Div),
        TokenKind::Percent => Some(BinaryOp::Rem),
        TokenKind::Power => Some(BinaryOp::Pow),
        TokenKind::ShiftLeft => Some(BinaryOp::Shl),
        TokenKind::ShiftRight => Some(BinaryOp::Shr),
        TokenKind::UnsignedShiftRight => Some(BinaryOp::Ushr),
        TokenKind::And => Some(BinaryOp::BitAnd),
        TokenKind::Caret => Some(BinaryOp::BitXor),
        TokenKind::Bar => Some(BinaryOp::BitOr),
        TokenKind::LessThan => Some(BinaryOp::Lt),
        TokenKind::LessThanEquals => Some(BinaryOp::Le),
        TokenKind::GreaterThan => Some(BinaryOp::Gt),
        TokenKind::GreaterThanEquals => Some(BinaryOp::Ge),
        TokenKind::Equals => Some(BinaryOp::Eq),
        TokenKind::NotEquals => Some(BinaryOp::Ne),
        TokenKind::AndAnd => Some(BinaryOp::LogicalAnd),
        TokenKind::BarBar => Some(BinaryOp::LogicalOr),
        TokenKind::NullCoalescing => Some(BinaryOp::NullCoalescing),
        _ => None,
    }
}

#[inline]
fn assign_op_from_token(kind: TokenKind) -> Option<AssignOp> {
    match kind {
        TokenKind::Assign => Some(AssignOp::Assign),
        TokenKind::PlusAssign => Some(AssignOp::AddAssign),
        TokenKind::MinusAssign => Some(AssignOp::SubAssign),
        TokenKind::StarAssign => Some(AssignOp::MulAssign),
        TokenKind::SlashAssign => Some(AssignOp::DivAssign),
        TokenKind::PercentAssign => Some(AssignOp::RemAssign),
        TokenKind::PowerAssign => Some(AssignOp::PowAssign),
        TokenKind::ShiftLeftAssign => Some(AssignOp::ShlAssign),
        TokenKind::ShiftRightAssign => Some(AssignOp::ShrAssign),
        TokenKind::UnsignedShiftRightAssign => Some(AssignOp::UshrAssign),
        TokenKind::AndAssign => Some(AssignOp::BitAndAssign),
        TokenKind::BarAssign => Some(AssignOp::BitOrAssign),
        TokenKind::CaretAssign => Some(AssignOp::BitXorAssign),
        TokenKind::AndAndAssign => Some(AssignOp::LogicalAndAssign),
        TokenKind::BarBarAssign => Some(AssignOp::LogicalOrAssign),
        TokenKind::NullCoalescingAssign => Some(AssignOp::NullCoalescingAssign),
        _ => None,
    }
}

#[inline]
fn prefix_op_from_token(kind: TokenKind) -> Option<PrefixUnaryOp> {
    match kind {
        TokenKind::Plus => Some(PrefixUnaryOp::Plus),
        TokenKind::Minus => Some(PrefixUnaryOp::Minus),
        TokenKind::Bang => Some(PrefixUnaryOp::Not),
        TokenKind::Tilde => Some(PrefixUnaryOp::BitNot),
        TokenKind::PlusPlus => Some(PrefixUnaryOp::PreInc),
        TokenKind::MinusMinus => Some(PrefixUnaryOp::PreDec),
        TokenKind::And => Some(PrefixUnaryOp::AddressOf),
        TokenKind::Star => Some(PrefixUnaryOp::Deref),
        TokenKind::Try => Some(PrefixUnaryOp::Try),
        TokenKind::Await => Some(PrefixUnaryOp::Await),
        TokenKind::Spawn => Some(PrefixUnaryOp::Spawn),
        TokenKind::Delete => Some(PrefixUnaryOp::Delete),
        TokenKind::Typeof => Some(PrefixUnaryOp::Typeof),
        _ => None,
    }
}

/// Returns `true` if `kind` can begin a prefix/primary expression in [`Parser::parse_prefix_expr`].
#[inline]
pub(crate) fn can_start_expr_kind(kind: TokenKind) -> bool {
    token_to_predefined_type(kind).is_some()
        || prefix_op_from_token(kind).is_some()
        || matches!(
            kind,
            TokenKind::IntegerLiteral
                | TokenKind::FloatLiteral
                | TokenKind::BigIntLiteral
                | TokenKind::StringLiteral
                | TokenKind::CStringLiteral
                | TokenKind::CharLiteral
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::TemplateNoSubstitution
                | TokenKind::TemplateHead
                | TokenKind::This
                | TokenKind::Super
                | TokenKind::Identifier
                | TokenKind::OpenParen
                | TokenKind::Async
                | TokenKind::LessThan
                | TokenKind::OpenBracket
                | TokenKind::OpenBrace
                | TokenKind::Switch
                | TokenKind::Unsafe
                | TokenKind::Sizeof
                | TokenKind::New
        )
}

#[inline]
fn postfix_op_from_token(kind: TokenKind) -> Option<PostfixUnaryOp> {
    match kind {
        TokenKind::PlusPlus => Some(PostfixUnaryOp::PostInc),
        TokenKind::MinusMinus => Some(PostfixUnaryOp::PostDec),
        TokenKind::Bang => Some(PostfixUnaryOp::NonNull),
        _ => None,
    }
}

/// Tokens that can begin a standalone statement or prefix expression when placed at the
/// start of a new line outside `(...)` / `[...]`.
#[inline]
fn is_newline_statement_boundary(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::OpenParen
            | TokenKind::OpenBracket
            | TokenKind::OpenBrace
            | TokenKind::PlusPlus
            | TokenKind::MinusMinus
            | TokenKind::Bang
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::And
            | TokenKind::LessThan
    )
}

/// Returns `true` if `expr` evaluates through an optional-chaining step (`?.` / `?.[...]`)
/// without an intervening non-null `!` unwrap.
fn contains_optional_chain(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Member {
            target,
            is_optional,
            ..
        }
        | ExprKind::Index {
            target,
            is_optional,
            ..
        } => *is_optional || contains_optional_chain(target),
        ExprKind::Call { callee, .. } => contains_optional_chain(callee),
        ExprKind::Parenthesized(inner) => contains_optional_chain(inner),
        _ => false,
    }
}

/// Checks whether `expr` is syntactically valid as an assignment or update (`++`/`--`) target.
fn is_valid_assignment_target(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Ident(_) | ExprKind::Error => true,
        ExprKind::Member {
            target,
            is_optional: false,
            ..
        }
        | ExprKind::Index {
            target,
            is_optional: false,
            ..
        } => !contains_optional_chain(target),
        ExprKind::UnaryPrefix {
            op: PrefixUnaryOp::Deref,
            ..
        } => true,
        ExprKind::Parenthesized(inner) => is_valid_assignment_target(inner),
        _ => false,
    }
}

/// Converts a pure identifier / non-optional member chain into a `TypeName` for struct literals.
fn expr_to_type_name(expr: &Expr) -> Option<TypeName> {
    let mut parts = Vec::new();
    collect_type_name_parts(expr, &mut parts)?;
    Some(TypeName::new(parts, expr.span))
}

fn collect_type_name_parts(expr: &Expr, parts: &mut Vec<Ident>) -> Option<()> {
    match &expr.kind {
        ExprKind::Ident(id) => {
            parts.push(id.clone());
            Some(())
        }
        ExprKind::Member {
            target,
            property: MemberProperty::Ident(id),
            is_optional: false,
        } => {
            collect_type_name_parts(target, parts)?;
            parts.push(id.clone());
            Some(())
        }
        _ => None,
    }
}

/// Returns `true` if `params` uses syntax that cannot appear in a parenthesized expression
/// (modifiers, type annotations, `?` optionality, wildcard `_`, or `...rest`).
///
/// Default values (`a = 1`) and tuple/object destructuring shapes (`[a, b]`, `{ x, y }`) are
/// intentionally excluded because they also match valid parenthesized assignments/literals.
fn params_require_arrow(params: &ParameterList) -> bool {
    if params.rest.is_some() {
        return true;
    }
    params.params.iter().any(|p| {
        p.accessibility.is_some()
            || p.is_readonly
            || p.is_optional
            || p.ty.is_some()
            || matches!(p.target, kyanos_ast::BindingTarget::Wildcard(_))
    })
}
