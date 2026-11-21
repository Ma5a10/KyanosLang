// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Statement and block parser routines for Kyanos.

use kyanos_ast::{
    Block, ForInit, ForIterationKind, Ident, Stmt, SwitchCase, UsingStmt, VarModifier,
    VariableDecl, VariableStmt,
};
use kyanos_lexer::TokenKind;

use crate::error::{ParseError, ParseErrorKind, ParseResult};
use crate::Parser;

impl<'a> Parser<'a> {
    /// Parses a `{ ... }` block containing declarations and statements.
    pub fn parse_block(&mut self) -> ParseResult<Block> {
        self.enter_recursion()?;
        let res = self.parse_block_inner();
        self.exit_recursion();
        res
    }

    fn parse_block_inner(&mut self) -> ParseResult<Block> {
        let open = self.expect(TokenKind::OpenBrace, "'{'")?;
        let prev_else = self.eos_allows_else;
        let prev_while = self.eos_allows_while;
        self.eos_allows_else = false;
        self.eos_allows_while = false;

        let mut statements = Vec::new();
        self.with_isolated_ternary_scope(|p| {
            while !p.at(TokenKind::CloseBrace) && !p.is_eof() {
                let stmt = p.parse_block_item();
                statements.push(stmt);
            }
        });

        self.eos_allows_else = prev_else;
        self.eos_allows_while = prev_while;
        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(open.span);
        Ok(Block::new(statements, span))
    }

    #[inline(never)]
    fn parse_block_item(&mut self) -> Stmt {
        let start_cursor = self.checkpoint().cursor;
        let parsed = if self.at(TokenKind::OpenBrace) {
            match self.parse_block() {
                Ok(block) => {
                    let id = self.alloc_id();
                    Ok(Stmt::block(block).with_id(id))
                }
                Err(err) => Err(err),
            }
        } else if self.at_declaration_start() {
            self.parse_block_decl_stmt()
        } else {
            self.parse_stmt()
        };

        match parsed {
            Ok(stmt) => stmt,
            Err(err) => {
                let err_span = err.span();
                self.report_error(err);
                self.synchronize_from(start_cursor);
                let id = self.alloc_id();
                Stmt::error(err_span).with_id(id)
            }
        }
    }

    #[inline(never)]
    fn parse_block_decl_stmt(&mut self) -> ParseResult<Stmt> {
        let decl = self.parse_decl()?;
        let id = self.alloc_id();
        Ok(Stmt::decl(decl).with_id(id))
    }

    /// Parses a single statement.
    pub fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        if self.at(TokenKind::OpenBrace) {
            let block = self.parse_block()?;
            let id = self.alloc_id();
            return Ok(Stmt::block(block).with_id(id));
        }
        self.enter_recursion()?;
        let res = self.parse_stmt_inner();
        self.exit_recursion();
        res
    }

    #[inline(never)]
    fn parse_stmt_inner(&mut self) -> ParseResult<Stmt> {
        let tok = self.current();
        match tok.kind {
            TokenKind::Let | TokenKind::Const => {
                let var_stmt = self.parse_variable_stmt_inner()?;
                let span = var_stmt.span;
                let id = self.alloc_id();
                Ok(Stmt::variable(var_stmt.modifier, var_stmt.declarations, span).with_id(id))
            }
            TokenKind::Using => {
                let using_stmt = self.parse_using_stmt_inner()?;
                let id = self.alloc_id();
                Ok(Stmt::using_stmt(
                    using_stmt.name,
                    using_stmt.ty,
                    using_stmt.init,
                    using_stmt.span,
                )
                .with_id(id))
            }
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::Do => self.parse_do_while_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Switch => self.parse_switch_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Break => self.parse_break_stmt(),
            TokenKind::Continue => self.parse_continue_stmt(),
            TokenKind::Defer => self.parse_defer_stmt(),
            TokenKind::Unsafe => self.parse_unsafe_stmt(),
            TokenKind::Debugger => self.parse_debugger_stmt(),
            TokenKind::Semicolon => {
                let semi = self.bump();
                let id = self.alloc_id();
                Ok(Stmt::empty(semi.span).with_id(id))
            }
            TokenKind::Identifier if self.peek_kind(1) == TokenKind::Colon => {
                self.parse_labeled_stmt()
            }
            _ => self.parse_expression_stmt(),
        }
    }

    // -------------------------------------------------------------------------
    // Variable & Resource Statements (Shared with Phase 5 Declarations)
    // -------------------------------------------------------------------------

    /// Parses `let ...` or `const ...` followed by `eos`.
    pub(crate) fn parse_variable_stmt_inner(&mut self) -> ParseResult<VariableStmt> {
        let mod_tok = self.current();
        let modifier = self.parse_var_modifier_token();
        let declarations = self.parse_variable_decl_list(modifier == VarModifier::Const)?;
        self.expect_eos_or_recover();
        let span = self.span_from(mod_tok.span);
        Ok(VariableStmt::new(modifier, declarations, span))
    }

    fn parse_variable_decl_list(&mut self, require_init: bool) -> ParseResult<Vec<VariableDecl>> {
        let mut decls = Vec::new();
        loop {
            decls.push(self.parse_single_variable_decl(require_init)?);
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        Ok(decls)
    }

    fn parse_single_variable_decl(&mut self, require_init: bool) -> ParseResult<VariableDecl> {
        let start = self.current_span();
        let target = self.parse_binding_target()?;
        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.eat(TokenKind::Assign).is_some() {
            Some(self.parse_expr()?)
        } else {
            if require_init {
                let err_span = self.span_from(start);
                self.report_error(
                    ParseError::new(
                        ParseErrorKind::MissingInitializer,
                        err_span,
                        "'const' declarations must be initialized",
                    )
                    .with_hint("Provide an initializer with '= <expr>' or use 'let'."),
                );
            }
            None
        };

        let span = self.span_from(start);
        Ok(VariableDecl::new(target, ty, init, span))
    }

    /// Parses `using ident (: type_)? = expr eos`.
    pub(crate) fn parse_using_stmt_inner(&mut self) -> ParseResult<UsingStmt> {
        let using_tok = self.expect(TokenKind::Using, "'using'")?;
        let name_tok = self.expect(TokenKind::Identifier, "resource variable identifier")?;
        let name = Ident::new(self.slice(name_tok.span), name_tok.span);

        let ty = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(TokenKind::Assign, "'='")?;
        let init = self.parse_expr()?;
        self.expect_eos_or_recover();
        let span = self.span_from(using_tok.span);
        Ok(UsingStmt::new(name, ty, init, span))
    }

    // -------------------------------------------------------------------------
    // Control Flow Statements
    // -------------------------------------------------------------------------

    fn parse_if_stmt(&mut self) -> ParseResult<Stmt> {
        let if_tok = self.expect(TokenKind::If, "'if'")?;
        self.expect(TokenKind::OpenParen, "'('")?;
        let condition = self.with_grouping(|p| p.parse_expr())?;
        self.expect(TokenKind::CloseParen, "')'")?;

        let prev_else = self.eos_allows_else;
        self.eos_allows_else = true;
        let then_res = self.parse_stmt();
        self.eos_allows_else = prev_else;
        let then_branch = then_res?;

        let else_branch = if self.eat(TokenKind::Else).is_some() {
            Some(self.parse_stmt()?)
        } else {
            None
        };

        let span = self.span_from(if_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::if_stmt(condition, then_branch, else_branch, span).with_id(id))
    }

    fn parse_while_stmt(&mut self) -> ParseResult<Stmt> {
        let while_tok = self.expect(TokenKind::While, "'while'")?;
        self.expect(TokenKind::OpenParen, "'('")?;
        let condition = self.with_grouping(|p| p.parse_expr())?;
        self.expect(TokenKind::CloseParen, "')'")?;

        let body = self.parse_stmt()?;
        let span = self.span_from(while_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::while_stmt(condition, body, span).with_id(id))
    }

    fn parse_do_while_stmt(&mut self) -> ParseResult<Stmt> {
        let do_tok = self.expect(TokenKind::Do, "'do'")?;
        let prev_while = self.eos_allows_while;
        self.eos_allows_while = true;
        let body_res = self.parse_stmt();
        self.eos_allows_while = prev_while;
        let body = body_res?;

        self.expect(TokenKind::While, "'while'")?;
        self.expect(TokenKind::OpenParen, "'('")?;
        let condition = self.with_grouping(|p| p.parse_expr())?;
        self.expect(TokenKind::CloseParen, "')'")?;
        self.expect_eos_or_recover();

        let span = self.span_from(do_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::do_while(body, condition, span).with_id(id))
    }

    fn parse_for_stmt(&mut self) -> ParseResult<Stmt> {
        let for_tok = self.expect(TokenKind::For, "'for'")?;
        self.expect(TokenKind::OpenParen, "'('")?;

        // Check for `for (let|const bindingTarget (of|in) expr)` before standard `for (init; test; update)`.
        if self.at(TokenKind::Let) || self.at(TokenKind::Const) {
            let cp = self.checkpoint();
            if let Some(iter_stmt) = self.try_parse_for_iteration(for_tok.span)? {
                return Ok(iter_stmt);
            }
            self.restore(cp);
        }

        let (init, test, update) = self.with_grouping(|p| {
            let init = if p.at(TokenKind::Semicolon) {
                None
            } else if p.eat(TokenKind::Let).is_some() {
                Some(ForInit::Let(p.parse_variable_decl_list(false)?))
            } else if p.eat(TokenKind::Const).is_some() {
                Some(ForInit::Const(p.parse_variable_decl_list(true)?))
            } else {
                Some(ForInit::Expr(p.parse_expression_sequence()?))
            };
            p.expect(TokenKind::Semicolon, "';'")?;

            let test = if p.at(TokenKind::Semicolon) {
                None
            } else {
                Some(p.parse_expr()?)
            };
            p.expect(TokenKind::Semicolon, "';'")?;

            let update = if p.at(TokenKind::CloseParen) {
                None
            } else {
                Some(p.parse_expression_sequence()?)
            };
            Ok::<_, ParseError>((init, test, update))
        })?;

        self.expect(TokenKind::CloseParen, "')'")?;
        let body = self.parse_stmt()?;
        let span = self.span_from(for_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::for_standard(init, test, update, body, span).with_id(id))
    }

    fn try_parse_for_iteration(
        &mut self,
        for_start: kyanos_ast::Span,
    ) -> ParseResult<Option<Stmt>> {
        let header = self.with_grouping(|p| {
            let modifier = p.parse_var_modifier_token();
            let Ok(target) = p.parse_binding_target() else {
                return Ok(None);
            };
            let kind = if p.eat(TokenKind::Of).is_some() {
                ForIterationKind::Of
            } else if p.eat(TokenKind::In).is_some() {
                ForIterationKind::In
            } else {
                return Ok(None);
            };
            let iterable = p.parse_expr()?;
            Ok::<_, ParseError>(Some((modifier, target, kind, iterable)))
        })?;

        let Some((modifier, target, kind, iterable)) = header else {
            return Ok(None);
        };

        self.expect(TokenKind::CloseParen, "')'")?;
        let body = self.parse_stmt()?;
        let span = self.span_from(for_start);
        let id = self.alloc_id();
        Ok(Some(
            Stmt::for_iteration(modifier, target, kind, iterable, body, span).with_id(id),
        ))
    }

    fn parse_switch_stmt(&mut self) -> ParseResult<Stmt> {
        let switch_tok = self.expect(TokenKind::Switch, "'switch'")?;
        self.expect(TokenKind::OpenParen, "'('")?;
        let target = self.with_grouping(|p| p.parse_expr())?;
        self.expect(TokenKind::CloseParen, "')'")?;
        self.expect(TokenKind::OpenBrace, "'{'")?;

        let mut cases = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            match self.parse_switch_case() {
                Ok(arm) => cases.push(arm),
                Err(err) => {
                    self.report_error(err);
                    self.synchronize_to_switch_arm();
                }
            }
        }

        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(switch_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::switch_stmt(target, cases, span).with_id(id))
    }

    fn parse_switch_case(&mut self) -> ParseResult<SwitchCase> {
        let start = self.current_span();
        if self.eat(TokenKind::Case).is_some() {
            let pattern = self.parse_pattern()?;
            let guard = if self.eat(TokenKind::When).is_some() {
                self.pending_ternary_then += 1;
                let guard_res = self.parse_expr();
                self.pending_ternary_then -= 1;
                Some(guard_res?)
            } else {
                None
            };
            self.expect(TokenKind::Colon, "':'")?;
            let body = self.parse_switch_arm_body();
            let span = self.span_from(start);
            Ok(SwitchCase::Case {
                pattern,
                guard,
                body,
                span,
            })
        } else if self.eat(TokenKind::Default).is_some() {
            self.expect(TokenKind::Colon, "':'")?;
            let body = self.parse_switch_arm_body();
            let span = self.span_from(start);
            Ok(SwitchCase::Default { body, span })
        } else {
            let tok = self.current();
            Err(ParseError::unexpected(
                "'case' or 'default'",
                tok.kind,
                tok.span,
            ))
        }
    }

    fn parse_switch_arm_body(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while !self.at(TokenKind::Case)
            && !self.at(TokenKind::Default)
            && !self.at(TokenKind::CloseBrace)
            && !self.is_eof()
        {
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => {
                    let err_span = err.span();
                    self.report_error(err);
                    self.synchronize_to_switch_arm();
                    let id = self.alloc_id();
                    stmts.push(Stmt::error(err_span).with_id(id));
                }
            }
        }
        stmts
    }

    fn synchronize_to_switch_arm(&mut self) {
        if !self.is_eof()
            && !self.at(TokenKind::CloseBrace)
            && !self.at(TokenKind::Case)
            && !self.at(TokenKind::Default)
        {
            self.bump();
        }
        while !self.is_eof() {
            let tok = self.current();
            if tok.kind == TokenKind::Semicolon {
                self.bump();
                return;
            }
            if matches!(
                tok.kind,
                TokenKind::CloseBrace | TokenKind::Case | TokenKind::Default
            ) {
                return;
            }
            if tok.has_leading_newline && crate::is_sync_start_keyword(tok.kind) {
                return;
            }
            self.bump();
        }
    }

    // -------------------------------------------------------------------------
    // Jump, Defer, Unsafe, Labeled, and Expression Statements
    // -------------------------------------------------------------------------

    fn parse_return_stmt(&mut self) -> ParseResult<Stmt> {
        let ret_tok = self.expect(TokenKind::Return, "'return'")?;
        let value = if self.at_eos_boundary() {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect_eos_or_recover();
        let span = self.span_from(ret_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::return_stmt(value, span).with_id(id))
    }

    fn parse_break_stmt(&mut self) -> ParseResult<Stmt> {
        let brk_tok = self.expect(TokenKind::Break, "'break'")?;
        let label = self.parse_optional_same_line_ident();
        self.expect_eos_or_recover();
        let span = self.span_from(brk_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::break_stmt(label, span).with_id(id))
    }

    fn parse_continue_stmt(&mut self) -> ParseResult<Stmt> {
        let cont_tok = self.expect(TokenKind::Continue, "'continue'")?;
        let label = self.parse_optional_same_line_ident();
        self.expect_eos_or_recover();
        let span = self.span_from(cont_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::continue_stmt(label, span).with_id(id))
    }

    fn parse_defer_stmt(&mut self) -> ParseResult<Stmt> {
        let defer_tok = self.expect(TokenKind::Defer, "'defer'")?;
        let inner = self.parse_stmt()?;
        let span = self.span_from(defer_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::defer_stmt(inner, span).with_id(id))
    }

    fn parse_unsafe_stmt(&mut self) -> ParseResult<Stmt> {
        let unsafe_tok = self.expect(TokenKind::Unsafe, "'unsafe'")?;
        let block = self.parse_block()?;
        let span = self.span_from(unsafe_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::unsafe_stmt(block, span).with_id(id))
    }

    fn parse_labeled_stmt(&mut self) -> ParseResult<Stmt> {
        let label_tok = self.expect(TokenKind::Identifier, "label identifier")?;
        let label = Ident::new(self.slice(label_tok.span), label_tok.span);
        self.expect(TokenKind::Colon, "':'")?;
        let stmt = self.parse_stmt()?;
        let span = self.span_from(label_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::labeled(label, stmt, span).with_id(id))
    }

    fn parse_debugger_stmt(&mut self) -> ParseResult<Stmt> {
        let dbg_tok = self.expect(TokenKind::Debugger, "'debugger'")?;
        self.expect_eos_or_recover();
        let span = self.span_from(dbg_tok.span);
        let id = self.alloc_id();
        Ok(Stmt::debugger(span).with_id(id))
    }

    fn parse_expression_stmt(&mut self) -> ParseResult<Stmt> {
        let expr = self.parse_expr()?;
        self.expect_eos_or_recover();
        let id = self.alloc_id();
        Ok(Stmt::expr(expr).with_id(id))
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    #[inline]
    fn at_eos_boundary(&self) -> bool {
        let tok = self.current();
        matches!(
            tok.kind,
            TokenKind::Semicolon | TokenKind::CloseBrace | TokenKind::Eof
        ) || tok.has_leading_newline
    }

    #[inline]
    fn parse_optional_same_line_ident(&mut self) -> Option<Ident> {
        let tok = self.current();
        if tok.kind == TokenKind::Identifier && !tok.has_leading_newline {
            self.bump();
            Some(Ident::new(self.slice(tok.span), tok.span))
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn expect_eos_or_recover(&mut self) {
        if let Err(err) = self.parse_eos() {
            self.report_error(err);
            self.synchronize_eos();
        }
    }
}
