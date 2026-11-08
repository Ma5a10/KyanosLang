// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Test helper utilities for compiler tests.

use kyanos_lexer::{Lexer, TokenKind};

/// Collect all token kinds from input until LineTerminator or Err.
pub fn tokenize(input: &str) -> Result<Vec<TokenKind>, String> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token()?;
        if tok == TokenKind::Eof {
            break;
        }
        tokens.push(tok);
    }
    Ok(tokens)
}

