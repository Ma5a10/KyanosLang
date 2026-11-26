// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Literal token decoding and template string interpolation parsing.

use std::borrow::Cow;

use kyanos_ast::{Expr, ExprKind, Literal, Radix, Span, TemplateStringPart};
use kyanos_lexer::{Token, TokenKind};

use crate::error::{ParseError, ParseErrorKind, ParseResult};
use crate::Parser;

impl<'a> Parser<'a> {
    /// Parses a literal token (other than `()` unit or template strings) into a [`Literal`].
    pub fn parse_literal_token(&mut self, tok: Token) -> ParseResult<Literal> {
        let raw = self.slice(tok.span);
        match tok.kind {
            TokenKind::IntegerLiteral => decode_integer_literal(raw, tok.span),
            TokenKind::FloatLiteral => decode_float_literal(raw, tok.span),
            TokenKind::BigIntLiteral => Ok(Literal::big_int(raw, tok.span)),
            TokenKind::StringLiteral => {
                let inner = strip_delimiters(raw, 1, 1, tok.span)?;
                let value = unescape_string(inner, tok.span)?;
                Ok(Literal::string(value, raw, tok.span))
            }
            TokenKind::CStringLiteral => {
                let inner = strip_delimiters(raw, 2, 1, tok.span)?;
                let value = unescape_string(inner, tok.span)?;
                if value.contains('\0') {
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidLiteral(
                            "Null byte is not allowed inside C-string literal".to_string(),
                        ),
                        tok.span,
                        "Null byte is not allowed inside C-string literal",
                    ));
                }
                Ok(Literal::c_string(value, raw, tok.span))
            }
            TokenKind::CharLiteral => {
                let inner = strip_delimiters(raw, 1, 1, tok.span)?;
                let ch = decode_single_char(inner, tok.span)?;
                Ok(Literal::char(ch, raw, tok.span))
            }
            TokenKind::True => Ok(Literal::bool(true, tok.span)),
            TokenKind::False => Ok(Literal::bool(false, tok.span)),
            TokenKind::Null => Ok(Literal::null(tok.span)),
            other => Err(ParseError::unexpected("literal", other, tok.span)),
        }
    }

    /// Parses a template literal (`TemplateNoSubstitution` or `TemplateHead ... TemplateTail`)
    /// into `ExprKind::TemplateString`.
    pub(crate) fn parse_template_string_expr(&mut self, tok: Token) -> ParseResult<Expr> {
        if tok.kind == TokenKind::TemplateNoSubstitution {
            let value = self.decode_template_segment(tok, 1, 1)?;
            let id = self.alloc_id();
            return Ok(Expr::new(
                ExprKind::TemplateString(vec![TemplateStringPart::String(value)]),
                tok.span,
            )
            .with_id(id));
        }

        if tok.kind != TokenKind::TemplateHead {
            return Err(ParseError::unexpected("template literal", tok.kind, tok.span));
        }

        self.enter_recursion()?;
        let res = self.parse_interpolated_template_expr(tok);
        self.exit_recursion();
        res
    }

    fn parse_interpolated_template_expr(&mut self, head_tok: Token) -> ParseResult<Expr> {
        let start_pos = head_tok.span.start;
        let mut parts = Vec::new();
        let head_text = self.decode_template_segment(head_tok, 1, 2)?;
        if !head_text.is_empty() {
            parts.push(TemplateStringPart::String(head_text));
        }

        let mut interp_open_end = head_tok.span.end;
        let mut end_pos = head_tok.span.end;

        loop {
            if self.at(TokenKind::TemplateMiddle) || self.at(TokenKind::TemplateTail) {
                let expr_span = Span::new(interp_open_end, self.current().span.start);
                self.report_error(ParseError::syntax(
                    "Template interpolation '${}' cannot be empty",
                    expr_span,
                ));
                let err_id = self.alloc_id();
                parts.push(TemplateStringPart::Expr(Box::new(
                    Expr::error(expr_span).with_id(err_id),
                )));
            } else {
                let expr = self.with_grouping(|p| p.parse_expr_with_recovery());
                parts.push(TemplateStringPart::Expr(Box::new(expr)));

                if !self.at(TokenKind::TemplateMiddle)
                    && !self.at(TokenKind::TemplateTail)
                    && !self.is_eof()
                {
                    let trailing = self.current();
                    self.report_error(ParseError::unexpected(
                        "'}'",
                        trailing.kind,
                        trailing.span,
                    ));
                    self.recover_to_template_boundary();
                }
            }

            match self.current().kind {
                TokenKind::TemplateMiddle => {
                    let mid_tok = self.bump();
                    let mid_text = self.decode_template_segment(mid_tok, 1, 2)?;
                    if !mid_text.is_empty() {
                        parts.push(TemplateStringPart::String(mid_text));
                    }
                    interp_open_end = mid_tok.span.end;
                    end_pos = mid_tok.span.end;
                }
                TokenKind::TemplateTail => {
                    let tail_tok = self.bump();
                    let tail_text = self.decode_template_segment(tail_tok, 1, 1)?;
                    if !tail_text.is_empty() {
                        parts.push(TemplateStringPart::String(tail_text));
                    }
                    end_pos = tail_tok.span.end;
                    break;
                }
                _ => {
                    let unexpected = self.current();
                    let err_end = unexpected.span.end.max(end_pos);
                    self.report_error(ParseError::new(
                        ParseErrorKind::InvalidLiteral(
                            "Unterminated template interpolation".to_string(),
                        ),
                        Span::new(start_pos, err_end),
                        "Unterminated '${...}' in template literal",
                    ));
                    end_pos = err_end;
                    break;
                }
            }
        }

        let id = self.alloc_id();
        Ok(Expr::new(ExprKind::TemplateString(parts), Span::new(start_pos, end_pos)).with_id(id))
    }

    fn decode_template_segment(
        &self,
        tok: Token,
        prefix_len: usize,
        suffix_len: usize,
    ) -> ParseResult<String> {
        let raw = self.slice(tok.span);
        let inner = strip_delimiters(raw, prefix_len, suffix_len, tok.span)?;
        unescape_string(inner, tok.span)
    }

    /// Advances past trailing tokens inside a malformed `${...}` until reaching the enclosing
    /// template's `TemplateMiddle` or `TemplateTail` (skipping over any inner nested templates).
    fn recover_to_template_boundary(&mut self) {
        let mut nested_depth = 0usize;
        while !self.is_eof() {
            match self.current().kind {
                TokenKind::TemplateHead => {
                    nested_depth += 1;
                    self.bump();
                }
                TokenKind::TemplateMiddle if nested_depth == 0 => break,
                TokenKind::TemplateTail => {
                    if nested_depth == 0 {
                        break;
                    }
                    nested_depth -= 1;
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
    }
}

/// Strips numeric `_` separators without allocating when none are present.
#[inline]
fn strip_numeric_underscores(raw: &str) -> Cow<'_, str> {
    if raw.as_bytes().contains(&b'_') {
        Cow::Owned(raw.chars().filter(|&c| c != '_').collect())
    } else {
        Cow::Borrowed(raw)
    }
}

/// Decodes a decimal, hexadecimal, octal, or binary integer literal into `u64`.
fn decode_integer_literal(raw: &str, span: Span) -> ParseResult<Literal> {
    let clean = strip_numeric_underscores(raw);
    let (radix, digits) = if let Some(rest) = clean
        .strip_prefix("0x")
        .or_else(|| clean.strip_prefix("0X"))
    {
        (Radix::Hex, rest)
    } else if let Some(rest) = clean
        .strip_prefix("0o")
        .or_else(|| clean.strip_prefix("0O"))
    {
        (Radix::Octal, rest)
    } else if let Some(rest) = clean
        .strip_prefix("0b")
        .or_else(|| clean.strip_prefix("0B"))
    {
        (Radix::Binary, rest)
    } else {
        (Radix::Decimal, clean.as_ref())
    };

    let value = u64::from_str_radix(digits, radix.as_u32()).map_err(|err| {
        ParseError::new(
            ParseErrorKind::InvalidLiteral(err.to_string()),
            span,
            format!("Invalid integer literal '{}': {}", raw, err),
        )
        .with_hint("Use the 'n' suffix (e.g. 99999999999999999999n) for BigInt literals.")
    })?;

    Ok(Literal::integer(raw, value, radix, span))
}

/// Decodes a floating-point literal into `f64`, rejecting values that overflow to non-finite `f64`.
fn decode_float_literal(raw: &str, span: Span) -> ParseResult<Literal> {
    let clean = strip_numeric_underscores(raw);
    let value = clean.parse::<f64>().map_err(|err| {
        ParseError::new(
            ParseErrorKind::InvalidLiteral(err.to_string()),
            span,
            format!("Invalid float literal '{}': {}", raw, err),
        )
    })?;

    if !value.is_finite() {
        return Err(ParseError::new(
            ParseErrorKind::InvalidLiteral("Floating-point literal overflow".to_string()),
            span,
            format!(
                "Invalid float literal '{}': value overflows 64-bit floating-point range",
                raw
            ),
        ));
    }

    Ok(Literal::float(raw, value, span))
}

/// Strips prefix/suffix delimiter bytes from a raw literal slice.
fn strip_delimiters(raw: &str, prefix: usize, suffix: usize, span: Span) -> ParseResult<&str> {
    let Some(end) = raw.len().checked_sub(suffix) else {
        return Err(ParseError::new(
            ParseErrorKind::InvalidLiteral("Malformed literal delimiters".to_string()),
            span,
            format!("Malformed literal '{}'", raw),
        ));
    };
    raw.get(prefix..end).ok_or_else(|| {
        ParseError::new(
            ParseErrorKind::InvalidLiteral("Malformed literal delimiters".to_string()),
            span,
            format!("Malformed literal '{}'", raw),
        )
    })
}

/// Unescapes the body of a string or C-string literal.
fn unescape_string(inner: &str, span: Span) -> ParseResult<String> {
    if !inner.as_bytes().contains(&b'\\') {
        return Ok(inner.to_owned());
    }
    let mut out = String::with_capacity(inner.len());
    let mut idx = 0usize;
    while let Some(ch) = inner.get(idx..).and_then(|s| s.chars().next()) {
        if ch == '\\' {
            idx += 1;
            out.push(decode_escape_at(inner, &mut idx, span)?);
        } else {
            out.push(ch);
            idx += ch.len_utf8();
        }
    }
    Ok(out)
}

/// Decodes the body of a single character literal `'...'`.
fn decode_single_char(inner: &str, span: Span) -> ParseResult<char> {
    let mut idx = 0usize;
    let first = inner.chars().next().ok_or_else(|| {
        ParseError::new(
            ParseErrorKind::InvalidLiteral("Empty character literal".to_string()),
            span,
            "Empty character literal",
        )
    })?;

    let ch = if first == '\\' {
        idx += 1;
        decode_escape_at(inner, &mut idx, span)?
    } else {
        idx += first.len_utf8();
        first
    };

    if idx != inner.len() {
        return Err(ParseError::new(
            ParseErrorKind::InvalidLiteral("Multi-character literal".to_string()),
            span,
            "Character literal must contain exactly one character",
        ));
    }
    Ok(ch)
}

/// Decodes an escape sequence starting immediately after `\` at `inner[*idx..]`.
fn decode_escape_at(inner: &str, idx: &mut usize, span: Span) -> ParseResult<char> {
    let esc = inner
        .get(*idx..)
        .and_then(|s| s.chars().next())
        .ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral("Incomplete escape sequence".to_string()),
                span,
                "Unexpected end of literal in escape sequence",
            )
        })?;
    *idx += esc.len_utf8();

    match esc {
        '\'' => Ok('\''),
        '"' => Ok('"'),
        '`' => Ok('`'),
        '$' => Ok('$'),
        '\\' => Ok('\\'),
        '/' => Ok('/'),
        'b' => Ok('\u{0008}'),
        'f' => Ok('\u{000C}'),
        'n' => Ok('\n'),
        'r' => Ok('\r'),
        't' => Ok('\t'),
        'v' => Ok('\u{000B}'),
        '0' => Ok('\0'),
        'u' => decode_unicode_escape(inner, idx, span),
        other => Err(ParseError::new(
            ParseErrorKind::InvalidLiteral(format!("Invalid escape '\\{}'", other)),
            span,
            format!("Invalid escape sequence '\\{}'", other),
        )),
    }
}

/// Decodes `\uXXXX` or `\u{HexDigit+}` starting after `\u`.
fn decode_unicode_escape(inner: &str, idx: &mut usize, span: Span) -> ParseResult<char> {
    let rest = inner.get(*idx..).ok_or_else(|| {
        ParseError::new(
            ParseErrorKind::InvalidLiteral("Truncated unicode escape".to_string()),
            span,
            "Unexpected end of literal in unicode escape",
        )
    })?;

    let value = if let Some(after_brace) = rest.strip_prefix('{') {
        *idx += 1;
        let close_rel = after_brace.find('}').ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral("Unterminated unicode escape".to_string()),
                span,
                "Unterminated '\\u{...}' escape sequence",
            )
        })?;
        let hex = after_brace.get(..close_rel).ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral("Invalid unicode escape".to_string()),
                span,
                "Invalid '\\u{...}' escape sequence",
            )
        })?;
        *idx += close_rel + 1;
        if !(1..=6).contains(&hex.len()) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ParseError::new(
                ParseErrorKind::InvalidLiteral(format!("Invalid unicode escape '\\u{{{}}}'", hex)),
                span,
                format!(
                    "Unicode escape '\\u{{{}}}' must contain 1 to 6 hex digits",
                    hex
                ),
            ));
        }
        u32::from_str_radix(hex, 16).map_err(|err| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral(err.to_string()),
                span,
                format!("Invalid unicode escape '\\u{{{}}}': {}", hex, err),
            )
        })?
    } else {
        let end = idx.checked_add(4).ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral("Truncated unicode escape".to_string()),
                span,
                "Expected 4 hex digits after '\\u'",
            )
        })?;
        let hex = inner.get(*idx..end).ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral("Truncated unicode escape".to_string()),
                span,
                "Expected 4 hex digits after '\\u'",
            )
        })?;
        *idx = end;
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ParseError::new(
                ParseErrorKind::InvalidLiteral(format!("Invalid unicode escape '\\u{}'", hex)),
                span,
                format!("Invalid 4-digit unicode escape '\\u{}'", hex),
            ));
        }
        u32::from_str_radix(hex, 16).map_err(|err| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral(err.to_string()),
                span,
                format!("Invalid unicode escape '\\u{}': {}", hex, err),
            )
        })?
    };

    char::from_u32(value).ok_or_else(|| {
        ParseError::new(
            ParseErrorKind::InvalidLiteral(format!("Invalid scalar value U+{:X}", value)),
            span,
            format!("Invalid Unicode scalar value U+{:X}", value),
        )
    })
}
