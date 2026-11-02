// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Lexer and token definitions for the Kyanos programming language.
/// reference design/kyanos_lex_grammar.g4
use std::iter::Peekable;
use std::str::Chars;

pub mod token;
pub use token::{Token, TokenKind};

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    /// Skip whitespace characters (U+000B, U+000C, U+00A0, U+FEFF).
    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek() {
            if Lexer::is_whitespace(ch) {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn is_whitespace(ch: char) -> bool {
        ch == '\u{000B}' || ch == '\u{000C}' || ch == '\u{00A0}' || ch == '\u{FEFF}'
    }

    

    pub fn next_token(&mut self) -> Result<TokenKind, String> {
        self.skip_whitespace();

        let ch = match self.chars.next() {
            Some(c) => c,
            None => return Ok(TokenKind::LineTerminator),
        };
        
        match ch {
            '+' => Ok(TokenKind::Plus),
            '-' => Ok(TokenKind::Minus),
            '*' => Ok(TokenKind::Star),
            '/' => Ok(TokenKind::Slash),
            c => Err(format!("Invalid token: {}", c)),
        }
    }
}
