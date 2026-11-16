// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Statement and block parser routines for Kyanos.
//!
//! Phase 2 provides `parse_block` for arrow function bodies and `unsafe { ... }`
//! expressions. Phase 4 expands this module with full control-flow, variable,
//! switch, and declaration statements.

use kyanos_ast::{Block, Stmt, StmtKind};
use kyanos_lexer::TokenKind;

use crate::error::ParseResult;
use crate::Parser;

impl<'a> Parser<'a> {
    /// Parses a `{ ... }` block.
    ///
    /// TODO(Phase 4): Support full statement and local declaration rules inside blocks.
    pub fn parse_block(&mut self) -> ParseResult<Block> {
        let open = self.expect(TokenKind::OpenBrace, "'{'")?;
        let mut statements = Vec::new();

        while !self.at(TokenKind::CloseBrace) && !self.is_eof() {
            match self.parse_phase2_block_stmt() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    let err_span = err.span;
                    self.report_error(err);
                    self.advance_and_synchronize();
                    let id = self.alloc_id();
                    statements.push(Stmt::error(err_span).with_id(id));
                }
            }
        }

        self.expect(TokenKind::CloseBrace, "'}'")?;
        let span = self.span_from(open.span);
        Ok(Block::new(statements, span))
    }

    fn parse_phase2_block_stmt(&mut self) -> ParseResult<Stmt> {
        if let Some(semi) = self.eat(TokenKind::Semicolon) {
            let id = self.alloc_id();
            return Ok(Stmt::new(StmtKind::Empty, semi.span).with_id(id));
        }

        if let Some(ret_tok) = self.eat(TokenKind::Return) {
            let value = if self.at(TokenKind::Semicolon)
                || self.at(TokenKind::CloseBrace)
                || self.is_eof()
                || self.current().has_leading_newline
            {
                None
            } else {
                Some(self.parse_expr()?)
            };
            if let Err(err) = self.parse_eos() {
                self.report_error(err);
                self.synchronize_eos();
            }
            let span = self.span_from(ret_tok.span);
            let id = self.alloc_id();
            return Ok(Stmt::return_stmt(value, span).with_id(id));
        }

        let expr = self.parse_expr()?;
        if let Err(err) = self.parse_eos() {
            self.report_error(err);
            self.synchronize_eos();
        }
        let span = self.span_from(expr.span);
        let id = self.alloc_id();
        Ok(Stmt::new(StmtKind::Expr(expr), span).with_id(id))
    }
}
