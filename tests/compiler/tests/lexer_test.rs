// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use compiler_tests::tokenize;
use kyanos_lexer::{Lexer, TokenKind};

#[test]
fn test_operators_continuous() {
    let mut lexer = Lexer::new("+-*/");
    assert_eq!(lexer.next_token(), Ok(TokenKind::Plus));
    assert_eq!(lexer.next_token(), Ok(TokenKind::Minus));
    assert_eq!(lexer.next_token(), Ok(TokenKind::Star));
    assert_eq!(lexer.next_token(), Ok(TokenKind::Slash));
    assert_eq!(lexer.next_token(), Ok(TokenKind::LineTerminator));
}

#[test]
fn test_tokenize_helper() {
    let tokens = tokenize("+-*/").expect("tokenization should succeed");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
        ]
    );
}