// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Literal token decoding and template string interpolation parsing.

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

    /// Parses a `TemplateStringLiteral` token into `ExprKind::TemplateString`.
    ///
    /// Embedded `${expr}` interpolations are parsed using a child [`Parser`] with
    /// shifted byte offsets and shared sequential `NodeId` allocation.
    pub(crate) fn parse_template_string_expr(&mut self, tok: Token) -> ParseResult<Expr> {
        let raw = self.slice(tok.span);
        let inner = strip_delimiters(raw, 1, 1, tok.span)?;
        let inner_base_offset = tok.span.start + 1;

        let mut parts = Vec::new();
        let mut current_text = String::new();
        let mut idx = 0usize;

        while idx < inner.len() {
            if inner[idx..].starts_with("${") {
                if !current_text.is_empty() {
                    parts.push(TemplateStringPart::String(std::mem::take(
                        &mut current_text,
                    )));
                }

                let expr_start = idx + 2;
                let max_depth = self.options.max_recursion_depth;
                let expr_end =
                    find_interpolation_end(inner, expr_start, 0, max_depth).ok_or_else(|| {
                        ParseError::new(
                            ParseErrorKind::InvalidLiteral(
                                "Unterminated template interpolation".to_string(),
                            ),
                            tok.span,
                            "Unterminated '${...}' in template literal",
                        )
                    })?;

                let expr_slice = &inner[expr_start..expr_end];
                let expr_abs_offset = inner_base_offset + expr_start;
                let expr_span = Span::new(expr_abs_offset, inner_base_offset + expr_end);

                if expr_slice.trim().is_empty() {
                    self.report_error(ParseError::syntax(
                        "Template interpolation '${}' cannot be empty",
                        expr_span,
                    ));
                    let err_id = self.alloc_id();
                    parts.push(TemplateStringPart::Expr(Box::new(
                        Expr::error(expr_span).with_id(err_id),
                    )));
                } else {
                    let mut sub_parser = Parser::with_offset_and_state(
                        expr_slice,
                        expr_abs_offset,
                        self.next_node_id,
                        self.recursion_depth,
                        self.options,
                    );
                    let expr = sub_parser.parse_expr_with_recovery();
                    if !sub_parser.is_eof() {
                        let trailing = sub_parser.current();
                        sub_parser.report_error(ParseError::unexpected(
                            "'}'",
                            trailing.kind,
                            trailing.span,
                        ));
                    }
                    self.next_node_id = sub_parser.next_node_id_raw();
                    self.errors.extend(sub_parser.take_errors());
                    parts.push(TemplateStringPart::Expr(Box::new(expr)));
                }

                idx = expr_end + 1;
                continue;
            }

            let ch = inner[idx..].chars().next().expect("idx < inner.len()");
            if ch == '\\' {
                idx += 1;
                let escaped = decode_escape_at(inner, &mut idx, tok.span)?;
                current_text.push(escaped);
            } else {
                current_text.push(ch);
                idx += ch.len_utf8();
            }
        }

        if !current_text.is_empty() || parts.is_empty() {
            parts.push(TemplateStringPart::String(current_text));
        }

        let id = self.alloc_id();
        Ok(Expr::new(ExprKind::TemplateString(parts), tok.span).with_id(id))
    }
}

/// Decodes a decimal, hexadecimal, octal, or binary integer literal into `u64`.
fn decode_integer_literal(raw: &str, span: Span) -> ParseResult<Literal> {
    let clean: String = raw.chars().filter(|&c| c != '_').collect();
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
        (Radix::Decimal, clean.as_str())
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

/// Decodes a floating-point literal into `f64`.
fn decode_float_literal(raw: &str, span: Span) -> ParseResult<Literal> {
    let clean: String = raw.chars().filter(|&c| c != '_').collect();
    let value = clean.parse::<f64>().map_err(|err| {
        ParseError::new(
            ParseErrorKind::InvalidLiteral(err.to_string()),
            span,
            format!("Invalid float literal '{}': {}", raw, err),
        )
    })?;

    Ok(Literal::float(raw, value, span))
}

/// Strips prefix/suffix delimiter bytes from a raw literal slice.
fn strip_delimiters(raw: &str, prefix: usize, suffix: usize, span: Span) -> ParseResult<&str> {
    if raw.len() < prefix + suffix {
        return Err(ParseError::new(
            ParseErrorKind::InvalidLiteral("Malformed literal delimiters".to_string()),
            span,
            format!("Malformed literal '{}'", raw),
        ));
    }
    Ok(&raw[prefix..raw.len() - suffix])
}

/// Unescapes the body of a string or C-string literal.
fn unescape_string(inner: &str, span: Span) -> ParseResult<String> {
    let mut out = String::with_capacity(inner.len());
    let mut idx = 0usize;
    while idx < inner.len() {
        let ch = inner[idx..].chars().next().expect("idx < inner.len()");
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
    let esc = inner[*idx..].chars().next().ok_or_else(|| {
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
    let value = if inner[*idx..].starts_with('{') {
        *idx += 1;
        let close_rel = inner[*idx..].find('}').ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral("Unterminated unicode escape".to_string()),
                span,
                "Unterminated '\\u{...}' escape sequence",
            )
        })?;
        let hex = &inner[*idx..*idx + close_rel];
        *idx += close_rel + 1;
        u32::from_str_radix(hex, 16).map_err(|err| {
            ParseError::new(
                ParseErrorKind::InvalidLiteral(err.to_string()),
                span,
                format!("Invalid unicode escape '\\u{{{}}}': {}", hex, err),
            )
        })?
    } else {
        if inner.len() - *idx < 4 {
            return Err(ParseError::new(
                ParseErrorKind::InvalidLiteral("Truncated unicode escape".to_string()),
                span,
                "Expected 4 hex digits after '\\u'",
            ));
        }
        let hex = &inner[*idx..*idx + 4];
        *idx += 4;
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

/// Finds the byte index of the closing `}` matching `${` at `start`, mirroring lexer nesting rules.
fn find_interpolation_end(
    inner: &str,
    start: usize,
    depth: usize,
    max_depth: usize,
) -> Option<usize> {
    if depth >= max_depth {
        return None;
    }
    let mut brace_depth = 1usize;
    let mut i = start;

    while i < inner.len() {
        let rest = &inner[i..];
        let ch = rest.chars().next()?;
        match ch {
            '{' => {
                brace_depth += 1;
                i += 1;
            }
            '}' => {
                brace_depth -= 1;
                if brace_depth == 0 {
                    return Some(i);
                }
                i += 1;
            }
            'c' if rest.starts_with("c\"") => {
                i = skip_quoted(inner, i + 2, '"')?;
            }
            '"' => {
                i = skip_quoted(inner, i + 1, '"')?;
            }
            '\'' => {
                i = skip_quoted(inner, i + 1, '\'')?;
            }
            '`' => {
                i = skip_nested_template(inner, i + 1, depth + 1, max_depth)?;
            }
            '/' if rest.starts_with("//") => {
                i += 2;
                while i < inner.len() {
                    let c = inner[i..].chars().next()?;
                    i += c.len_utf8();
                    if c == '\n' || c == '\r' || c == '\u{2028}' || c == '\u{2029}' {
                        break;
                    }
                }
            }
            '/' if rest.starts_with("/*") => {
                let end_rel = inner[i + 2..].find("*/")?;
                i = i + 2 + end_rel + 2;
            }
            '\\' => {
                i += 1;
                let escaped = inner[i..].chars().next()?;
                i += escaped.len_utf8();
            }
            _ => {
                i += ch.len_utf8();
            }
        }
    }
    None
}

/// Skips past a quoted string or char literal inside an interpolation expression.
fn skip_quoted(inner: &str, mut i: usize, quote: char) -> Option<usize> {
    while i < inner.len() {
        let ch = inner[i..].chars().next()?;
        i += ch.len_utf8();
        if ch == quote {
            return Some(i);
        }
        if ch == '\\' {
            let escaped = inner[i..].chars().next()?;
            i += escaped.len_utf8();
        }
    }
    None
}

/// Skips past a nested template string literal inside an interpolation expression.
fn skip_nested_template(
    inner: &str,
    mut i: usize,
    depth: usize,
    max_depth: usize,
) -> Option<usize> {
    if depth >= max_depth {
        return None;
    }
    while i < inner.len() {
        if inner[i..].starts_with("${") {
            let close = find_interpolation_end(inner, i + 2, depth + 1, max_depth)?;
            i = close + 1;
            continue;
        }
        let ch = inner[i..].chars().next()?;
        i += ch.len_utf8();
        if ch == '`' {
            return Some(i);
        }
        if ch == '\\' {
            let escaped = inner[i..].chars().next()?;
            i += escaped.len_utf8();
        }
    }
    None
}
