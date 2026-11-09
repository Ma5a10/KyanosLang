// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Lexer and token definitions for the Kyanos programming language.
/// reference design/kyanos_lex_grammar.g4
use kyanos_ast::Span;

pub mod token;
pub use token::{Token, TokenKind};

pub struct Lexer<'a> {
    /// The entire source code strings.
    input: &'a str,
    /// current byte offset position.
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, cursor: 0 }
    }

    /// Look ahead at the current character (without consuming it)
    #[inline]
    fn peek(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    /// Look ahead to the second character (for identifying sequences like 'c"' or '&&=')
    #[inline]
    fn peek_second(&self) -> Option<char> {
        let mut chars = self.input[self.cursor..].chars();
        chars.next();
        chars.next()
    }

    /// Consume one character, return that character, and advance the cursor.
    #[inline]
    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }
    /// Consume only if the condition matches.
    #[inline]
    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.cursor += expected.len_utf8();
            true
        } else {
            false
        }
    }
    /// Get the string slice of the source code corresponding to the specified Span.
    #[inline]
    pub fn slice(&self, span: Span) -> &'a str {
        &self.input[span.start..span.end]
    }

    // Whitespace:
    // [\t\u000B\u000C\u00A0\uFEFF\p{Zs}]+ -> channel(HIDDEN);
    pub fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if Lexer::is_whitespace(ch) {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// Returns true if the character matches Kyanos `Whitespace`:
    /// `\t`, `\u000B`, `\u000C`, `\uFEFF`, and all Unicode space separators (`\p{Zs}`).
    #[inline]
    pub fn is_whitespace(ch: char) -> bool {
        match ch {
            ' '
            | '\t'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{00A0}'
            | '\u{FEFF}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}' => true,
            _ => false,
        }
    }

    // LineTerminator:
    // ('\r\n' | [\r\n\u2028\u2029]) -> channel(HIDDEN);
    pub fn skip_line_terminator(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\r' {
                self.bump();
                if self.peek() == Some('\n') {
                    self.bump();
                }
            } else if Self::is_line_terminator(ch) {
                self.bump();
            } else {
                break;
            }
        }
    }

    #[inline]
    pub fn is_line_terminator(ch: char) -> bool {
        ch == '\r' || ch == '\n' || ch == '\u{2028}' || ch == '\u{2029}'
    }

    // Identifier: IdentifierStart IdentifierPart* | '_' IdentifierPart+;
    pub fn scan_identifier_or_keyword(&mut self) -> Option<String> {
        let first = self.peek()?;
        if first == '_' {
            // Identifiers starting with '_' must contain at least two characters (e.g. `_foo`, `__`).
            // A single '_' is not an identifier, but tokenized as `Underscore`.
            match self.peek_second() {
                Some(next) if Self::is_identifier_part(next) => {
                    // Valid identifier starting with '_'
                }
                _ => return None,
            }
        } else if !Self::is_identifier_start(first) {
            return None;
        }

        self.bump();
        let mut ident = String::new();
        ident.push(first);

        while let Some(ch) = self.peek() {
            if Self::is_identifier_part(ch) {
                ident.push(ch);
                self.bump();
            } else {
                break;
            }
        }

        Some(ident)
    }

    // fragment IdentifierStart: [\p{ID_Start}] | '$';
    #[inline]
    pub fn is_identifier_start(c: char) -> bool {
        unicode_ident::is_xid_start(c) || c == '$'
    }

    // fragment IdentifierPart: [\p{ID_Continue}] | '$';
    #[inline]
    pub fn is_identifier_part(c: char) -> bool {
        unicode_ident::is_xid_continue(c) || c == '$'
    }

    /// Skip whitespace and line terminators.
    /// Returns true if any line terminators were skipped.
    fn skip_whitespace_and_newlines(&mut self) -> bool {
        let mut had_newline = false;
        loop {
            self.skip_whitespace();
            if let Some(ch) = self.peek() {
                if Self::is_line_terminator(ch) {
                    had_newline = true;
                    self.skip_line_terminator();
                    continue;
                }
            }
            break;
        }
        had_newline
    }

    // next_token returns the next token kind.
    pub fn next_token(&mut self) -> Result<Token, String> {
        // Skip whitespace and line terminators. and get if there were newlines.
        let had_newline = self.skip_whitespace_and_newlines();

        // Record the starting position of the token.
        let start = self.cursor;

        // chech EOF
        let first = match self.peek() {
            Some(c) => c,
            None => {
                return Ok(Token::new(
                    TokenKind::Eof,
                    Span::new(start, start),
                    had_newline,
                ))
            }
        };

        // scan token kind (internal self.cursor moves)
        let kind = self.scan_token_kind(first)?;

        // Record the end position of the token.
        let end = self.cursor;

        Ok(Token::new(kind, Span::new(start, end), had_newline))
    }

    /// scan token kind
    fn scan_token_kind(&mut self, first: char) -> Result<TokenKind, String> {
        // Compile-time C-String Literal: c"..."
        if first == 'c' && self.peek_second() == Some('"') {
            self.bump(); // consume 'c'
            self.bump(); // consume '"'
            return self.scan_string_contents(TokenKind::CStringLiteral);
        }

        // Identifier: IdentifierStart IdentifierPart*;
        if Self::is_identifier_start(first) || first == '_' {
            // Note: scan_identifier_or_keyword returns None if single '_'
            if let Some(ident) = self.scan_identifier_or_keyword() {
                // yield*
                if ident == "yield" && self.peek() == Some('*') {
                    self.bump(); // consume '*'
                    return Ok(TokenKind::YieldStar);
                }

                // Keyword lookup or fallback to Identifier
                let kind = TokenKind::from_keyword(&ident).unwrap_or(TokenKind::Identifier);
                return Ok(kind);
            }
        }

        // Number literal: IntegerLiteral, FloatLiteral, BigIntLiteral
        if first.is_ascii_digit() {
            return self.scan_number_literal();
        }

        // 3. Operators & Punctuators (single char tokens consume here)
        let character = self.bump().unwrap();
        match character {
            // multi symbols
            '+' => self.plus_match(),
            '-' => self.minus_match(),
            '*' => self.star_match(),
            '/' => self.slash_match(),
            '%' => self.percent_match(),
            '&' => self.and_match(),
            '|' => self.bar_match(),
            '^' => self.caret_match(),
            '!' => self.bang_match(),
            '<' => self.less_than_match(),
            '>' => self.greater_than_match(),
            '=' => self.assign_match(),
            '.' => self.dot_match(),
            '?' => self.question_match(),

            // Character Literals
            '"' => self.scan_string_contents(TokenKind::StringLiteral),
            '\'' => self.scan_char_literal(),

            // Template Strings Literal
            '`' => self.scan_template_string_literal(),
            // hash rules
            '#' => self.hash_match(),

            // one symbols
            '{' => Ok(TokenKind::OpenBrace),
            '}' => Ok(TokenKind::CloseBrace),
            '(' => Ok(TokenKind::OpenParen),
            ')' => Ok(TokenKind::CloseParen),
            '[' => Ok(TokenKind::OpenBracket),
            ']' => Ok(TokenKind::CloseBracket),

            ';' => Ok(TokenKind::Semicolon),
            ',' => Ok(TokenKind::Comma),
            ':' => Ok(TokenKind::Colon),
            '~' => Ok(TokenKind::Tilde),
            '_' => Ok(TokenKind::Underscore),
            c => Err(format!("Invalid token: {}", c)),
        }
    }

    // + match rules
    fn plus_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('+') {
            return Ok(TokenKind::PlusPlus); // '++'
        } else if self.match_char('=') {
            Ok(TokenKind::PlusAssign) // '+='
        } else {
            Ok(TokenKind::Plus) // '+'
        }
    }

    // - match rules
    fn minus_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('-') {
            Ok(TokenKind::MinusMinus) // '--'
        } else if self.match_char('=') {
            Ok(TokenKind::MinusAssign) // '-='
        } else {
            Ok(TokenKind::Minus) // '-'
        }
    }

    // * match rules
    fn star_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('*') {
            if self.match_char('=') {
                Ok(TokenKind::PowerAssign) // '**='
            } else {
                Ok(TokenKind::Power) // '**'
            }
        } else if self.match_char('=') {
            Ok(TokenKind::StarAssign) // '*='
        } else {
            Ok(TokenKind::Star) // '*'
        }
    }

    // / match rules
    fn slash_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('/') {
            Ok(self.scan_single_line_comment())
        } else if self.match_char('*') {
            self.scan_multi_line_comment()
        } else if self.match_char('=') {
            Ok(TokenKind::SlashAssign) // '/='
        } else {
            Ok(TokenKind::Slash) // '/'
        }
    }

    /// Scans a single-line comment: `// ...`
    /// Comments are keeped as tokens during the lexical analysis phase so that they can be utilized by tools such as an LSP;
    /// plan to implement a separate mechanism in the compiler's parser to skip those comments.
    fn scan_single_line_comment(&mut self) -> TokenKind {
        while let Some(ch) = self.peek() {
            if Self::is_line_terminator(ch) {
                break;
            }
            self.bump();
        }
        TokenKind::SingleLineComment
    }

    /// Scans a multi-line comment: `/* ... */`
    /// Keep as a token, just like single-line comments.
    fn scan_multi_line_comment(&mut self) -> Result<TokenKind, String> {
        while let Some(ch) = self.bump() {
            if ch == '*' && self.peek() == Some('/') {
                self.bump(); // Consume '/'
                return Ok(TokenKind::MultiLineComment);
            }
        }
        Err("Unterminated multi-line comment".to_string())
    }

    // % match rules
    fn percent_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('=') {
            Ok(TokenKind::PercentAssign) // '%='
        } else {
            Ok(TokenKind::Percent) // '%'
        }
    }

    // & match rules
    fn and_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('&') {
            if self.match_char('=') {
                Ok(TokenKind::AndAndAssign) // '&&='
            } else {
                Ok(TokenKind::AndAnd) // '&&'
            }
        } else if self.match_char('=') {
            Ok(TokenKind::AndAssign) // '&='
        } else {
            Ok(TokenKind::And) // '&'
        }
    }

    // | match rules
    fn bar_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('|') {
            if self.match_char('=') {
                Ok(TokenKind::BarBarAssign) // '||='
            } else {
                Ok(TokenKind::BarBar) // '||'
            }
        } else if self.match_char('=') {
            Ok(TokenKind::BarAssign) // '|='
        } else {
            Ok(TokenKind::Bar) // '|'
        }
    }

    // ^ match rules
    fn caret_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('=') {
            Ok(TokenKind::CaretAssign) // '^='
        } else {
            Ok(TokenKind::Caret) // '^'
        }
    }

    // ! match rules
    fn bang_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('=') {
            Ok(TokenKind::NotEquals) // '!='
        } else {
            Ok(TokenKind::Bang) // '!'
        }
    }

    // < match rules
    fn less_than_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('<') {
            if self.match_char('=') {
                Ok(TokenKind::ShiftLeftAssign) // '<<='
            } else {
                Ok(TokenKind::ShiftLeft) // '<<'
            }
        } else if self.match_char('=') {
            Ok(TokenKind::LessThanEquals) // '<='
        } else {
            Ok(TokenKind::LessThan) // '<'
        }
    }

    // > match rules
    fn greater_than_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('>') {
            if self.match_char('>') {
                if self.match_char('=') {
                    Ok(TokenKind::UnsignedShiftRightAssign) // '>>>='
                } else {
                    Ok(TokenKind::UnsignedShiftRight) // '>>>'
                }
            } else if self.match_char('=') {
                Ok(TokenKind::ShiftRightAssign) // '>>='
            } else {
                Ok(TokenKind::ShiftRight) // '>>'
            }
        } else if self.match_char('=') {
            Ok(TokenKind::GreaterThanEquals) // '>='
        } else {
            Ok(TokenKind::GreaterThan) // '>'
        }
    }

    // = match rules
    fn assign_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('=') {
            Ok(TokenKind::Equals) // '=='
        } else if self.match_char('>') {
            Ok(TokenKind::Arrow) // '=>'
        } else {
            Ok(TokenKind::Assign) // '='
        }
    }

    // . match rules
    fn dot_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('.') {
            if self.match_char('.') {
                Ok(TokenKind::Ellipsis) // '...'
            } else {
                Err(format!("Invalid token: .."))
            }
        } else {
            Ok(TokenKind::Dot) // '.'
        }
    }

    // ? match rules
    fn question_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('?') {
            if self.match_char('=') {
                Ok(TokenKind::NullCoalescingAssign) // '??='
            } else {
                Ok(TokenKind::NullCoalescing) // '??'
            }
        } else if self.match_char('.') {
            Ok(TokenKind::OptionalChain) // '?.'
        } else {
            Ok(TokenKind::Question) // '?'
        }
    }

    // StringLiteral, CharLiteral, CStringLiteral
    /// Scans an escape sequence right after consuming `\`.
    fn scan_escape_sequence(&mut self) -> Result<(), String> {
        match self.bump() {
            Some('\'' | '"' | '`' | '$' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'v' | '0') => {
                Ok(())
            }
            // unicode escape
            Some('u') => {
                if self.match_char('{') {
                    // \u{HexDigit+}
                    let mut count = 0;
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_hexdigit() {
                            self.bump();
                            count += 1;
                        } else if ch == '}' {
                            self.bump();
                            if count == 0 {
                                return Err("Empty Unicode escape sequence".to_string());
                            }
                            return Ok(());
                        } else {
                            return Err(format!("Invalid character in Unicode escape: '{}'", ch));
                        }
                    }
                    Err("Unterminated Unicode escape sequence".to_string())
                } else {
                    // \uXXXX (4 hex digits)
                    for _ in 0..4 {
                        match self.bump() {
                            Some(ch) if ch.is_ascii_hexdigit() => {}
                            Some(ch) => {
                                return Err(format!(
                                    "Invalid hex digit in Unicode escape: '{}'",
                                    ch
                                ))
                            }
                            None => {
                                return Err("Unexpected EOF in Unicode escape sequence".to_string())
                            }
                        }
                    }
                    Ok(())
                }
            }
            Some(c) => Err(format!("Invalid escape sequence: '\\{}'", c)),
            None => Err("Unexpected EOF in escape sequence".to_string()),
        }
    }

    /// Scans contents of a double-quoted string literal until closing `"` (used for both StringLiteral and CStringLiteral).
    fn scan_string_contents(&mut self, kind: TokenKind) -> Result<TokenKind, String> {
        while let Some(ch) = self.bump() {
            if ch == '"' {
                return Ok(kind);
            } else if ch == '\\' {
                self.scan_escape_sequence()?;
            } else if Self::is_line_terminator(ch) {
                return Err("Unterminated string literal: newline in string".to_string());
            }
        }
        Err("Unterminated string literal: unexpected EOF".to_string())
    }

    /// Scans a character literal: `'...'`
    fn scan_char_literal(&mut self) -> Result<TokenKind, String> {
        match self.bump() {
            Some('\'') => return Err("Empty character literal".to_string()),
            Some('\\') => {
                self.scan_escape_sequence()?;
            }
            Some(ch) if Self::is_line_terminator(ch) => {
                return Err(
                    "Unterminated character literal: newline in character literal".to_string(),
                );
            }
            Some(_) => {}
            None => return Err("Unterminated character literal: unexpected EOF".to_string()),
        }

        if self.match_char('\'') {
            Ok(TokenKind::CharLiteral)
        } else {
            Err("Multi-character or unclosed character literal".to_string())
        }
    }

    /// Scans digits matching the predicate, supporting single underscores between digits (e.g. 1_000_000).
    fn scan_digits<F>(&mut self, is_digit: F) -> Result<(), String>
    where
        F: Fn(char) -> bool,
    {
        while let Some(ch) = self.peek() {
            if is_digit(ch) {
                self.bump();
            } else if ch == '_' {
                match self.peek_second() {
                    Some(next) if is_digit(next) => {
                        self.bump(); // consume '_'
                        self.bump(); // consume next digit
                    }
                    _ => {
                        return Err(
                            "Trailing or consecutive underscores in numeric literal".to_string()
                        );
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Scans the exponent part of a float literal (`[eE][+-]?[0-9]+`).
    fn scan_exponent(&mut self) -> Result<(), String> {
        self.bump(); // consume 'e' or 'E'
        if let Some(ch) = self.peek() {
            if ch == '+' || ch == '-' {
                self.bump();
            }
        }
        match self.peek() {
            Some(ch) if ch.is_ascii_digit() => self.scan_digits(|c| c.is_ascii_digit()),
            _ => Err("Exponent part must have at least one digit".to_string()),
        }
    }

    /// Scans radix-prefixed integers: hex (`0x`), binary (`0b`), octal (`0o`).
    fn scan_radix_integer<F>(&mut self, prefix: char, is_digit: F) -> Result<TokenKind, String>
    where
        F: Fn(char) -> bool,
    {
        self.bump(); // consume prefix character ('x', 'b', or 'o')
        match self.peek() {
            Some(ch) if is_digit(ch) => {
                self.scan_digits(is_digit)?;
            }
            _ => {
                return Err(format!(
                    "Radix prefix '0{}' must be followed by at least one valid digit",
                    prefix
                ));
            }
        }

        // Check for BigInt suffix 'n' or 'N'
        if self.match_char('n') || self.match_char('N') {
            self.check_no_ident_suffix()?;
            Ok(TokenKind::BigIntLiteral)
        } else {
            self.check_no_ident_suffix()?;
            Ok(TokenKind::IntegerLiteral)
        }
    }

    /// Verifies that no identifier characters or digits immediately follow a numeric literal (e.g. `123foo`).
    fn check_no_ident_suffix(&mut self) -> Result<(), String> {
        if let Some(ch) = self.peek() {
            if Self::is_identifier_start(ch) || ch.is_ascii_digit() {
                return Err(format!(
                    "Invalid character '{}' following numeric literal",
                    ch
                ));
            }
        }
        Ok(())
    }

    /// Scans a numeric literal: `IntegerLiteral`, `FloatLiteral`, or `BigIntLiteral`.
    fn scan_number_literal(&mut self) -> Result<TokenKind, String> {
        let first = self.bump().unwrap();

        if first == '0' {
            if let Some(ch) = self.peek() {
                match ch {
                    'x' | 'X' => return self.scan_radix_integer(ch, |c| c.is_ascii_hexdigit()),
                    'b' | 'B' => return self.scan_radix_integer(ch, |c| c == '0' || c == '1'),
                    'o' | 'O' => return self.scan_radix_integer(ch, |c| matches!(c, '0'..='7')),
                    '0'..='9' => {
                        return Err(
                            "Invalid leading zero in decimal integer literal. Octal literals use '0o' prefix"
                                .to_string(),
                        );
                    }
                    '.' => {
                        match self.peek_second() {
                            Some(next) if next.is_ascii_digit() => {
                                self.bump(); // consume '.'
                                self.scan_digits(|c| c.is_ascii_digit())?;
                                if let Some(exp) = self.peek() {
                                    if exp == 'e' || exp == 'E' {
                                        self.scan_exponent()?;
                                    }
                                }
                                self.check_no_ident_suffix()?;
                                return Ok(TokenKind::FloatLiteral);
                            }
                            _ => {}
                        }
                    }
                    'e' | 'E' => {
                        self.scan_exponent()?;
                        self.check_no_ident_suffix()?;
                        return Ok(TokenKind::FloatLiteral);
                    }
                    'n' | 'N' => {
                        self.bump(); // consume 'n' or 'N'
                        self.check_no_ident_suffix()?;
                        return Ok(TokenKind::BigIntLiteral);
                    }
                    _ => {}
                }
            }
            self.check_no_ident_suffix()?;
            return Ok(TokenKind::IntegerLiteral);
        }

        // first is '1'..='9'
        self.scan_digits(|c| c.is_ascii_digit())?;

        // Check for float (fraction or exponent)
        if let Some(ch) = self.peek() {
            if ch == '.' {
                match self.peek_second() {
                    Some(next) if next.is_ascii_digit() => {
                        self.bump(); // consume '.'
                        self.scan_digits(|c| c.is_ascii_digit())?;
                        if let Some(exp) = self.peek() {
                            if exp == 'e' || exp == 'E' {
                                self.scan_exponent()?;
                            }
                        }
                        self.check_no_ident_suffix()?;
                        return Ok(TokenKind::FloatLiteral);
                    }
                    _ => {}
                }
            } else if ch == 'e' || ch == 'E' {
                self.scan_exponent()?;
                self.check_no_ident_suffix()?;
                return Ok(TokenKind::FloatLiteral);
            }
        }

        // Check for BigInt suffix 'n' or 'N'
        if self.match_char('n') || self.match_char('N') {
            self.check_no_ident_suffix()?;
            return Ok(TokenKind::BigIntLiteral);
        }

        self.check_no_ident_suffix()?;
        Ok(TokenKind::IntegerLiteral)
    }

    /// Scans a template string literal: `` `...${expr}...` ``
    fn scan_template_string_literal(&mut self) -> Result<TokenKind, String> {
        while let Some(ch) = self.bump() {
            if ch == '`' {
                return Ok(TokenKind::TemplateStringLiteral);
            } else if ch == '\\' {
                self.scan_escape_sequence()?;
            } else if ch == '$' && self.match_char('{') {
                // Template interpolation: ${...}
                let mut brace_depth = 1;
                while let Some(next_ch) = self.bump() {
                    match next_ch {
                        '{' => brace_depth += 1,
                        '}' => {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                break;
                            }
                        }
                        '"' => {
                            self.scan_string_contents(TokenKind::StringLiteral)?;
                        }
                        '\'' => {
                            self.scan_char_literal()?;
                        }
                        '\\' => {
                            self.scan_escape_sequence()?;
                        }
                        c if Self::is_line_terminator(c) => {
                            return Err(
                                "Unterminated template interpolation: newline in template expression"
                                    .to_string(),
                            );
                        }
                        _ => {}
                    }
                }
                if brace_depth > 0 {
                    return Err("Unterminated template interpolation: unclosed '${'".to_string());
                }
            } else if Self::is_line_terminator(ch) {
                return Err(
                    "Unterminated template string literal: newline in template literal".to_string(),
                );
            }
        }
        Err("Unterminated template string literal: unexpected EOF".to_string())
    }

    // # rules: HashBracket ('#[') or PrivateIdentifier ('#' (IdentifierStart | '_') IdentifierPart*)
    fn hash_match(&mut self) -> Result<TokenKind, String> {
        if self.match_char('[') {
            return Ok(TokenKind::HashBracket);
        }

        match self.peek() {
            Some(first) if Self::is_identifier_start(first) || first == '_' => {
                self.bump(); // Consume first character
                while let Some(ch) = self.peek() {
                    if Self::is_identifier_part(ch) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                Ok(TokenKind::PrivateIdentifier)
            }
            Some(c) => Err(format!(
                "Invalid token following '#': expected '[' or identifier, found '{}'",
                c
            )),
            None => Err("Unexpected EOF following '#'".to_string()),
        }
    }
}
