// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use kyanos_ast::{NodeId, Span};
use kyanos_lexer::TokenKind;
use kyanos_parse::{ParseError, ParseErrorKind, Parser};

#[test]
fn test_trivia_filtering_and_leading_newline_across_comments() {
    let source = "let a = 1 // trailing comment\n/* block */\nlet b = 2 /* inline */ let c = 3";
    let mut parser = Parser::new(source);

    // First statement tokens: `let`, `a`, `=`, `1`
    assert_eq!(parser.bump().kind, TokenKind::Let);
    assert_eq!(parser.bump().kind, TokenKind::Identifier);
    assert_eq!(parser.bump().kind, TokenKind::Assign);
    assert_eq!(parser.bump().kind, TokenKind::IntegerLiteral);

    // Comments are skipped; `let` on the new line keeps `has_leading_newline = true`.
    let tok_let_b = parser.current();
    assert_eq!(tok_let_b.kind, TokenKind::Let);
    assert!(tok_let_b.has_leading_newline);
    parser.bump(); // let
    assert_eq!(parser.current_slice(), "b");
    parser.bump(); // b
    parser.bump(); // =
    parser.bump(); // 2

    // Inline block comment without a newline keeps `has_leading_newline = false`.
    let tok_let_c = parser.current();
    assert_eq!(tok_let_c.kind, TokenKind::Let);
    assert!(!tok_let_c.has_leading_newline);
}

#[test]
fn test_cursor_lookahead_and_span_merge() {
    let mut parser = Parser::new("get value");

    assert!(parser.at_ident_str("get"));
    assert!(!parser.at_ident_str("set"));
    assert_eq!(parser.peek_kind(1), TokenKind::Identifier);
    assert_eq!(parser.peek_kind(2), TokenKind::Eof);

    let first = parser.expect(TokenKind::Identifier, "accessor").unwrap();
    let second = parser
        .expect(TokenKind::Identifier, "property name")
        .unwrap();
    let full = parser.span_from(first.span);

    assert_eq!(full, Span::new(first.span.start, second.span.end));
    assert_eq!(parser.slice(full), "get value");
    assert!(parser.is_eof());
}

#[test]
fn test_checkpoint_and_restore_rolls_back_cursor_and_errors() {
    let mut parser = Parser::new("a + b");
    let cp = parser.checkpoint();

    assert_eq!(parser.bump().kind, TokenKind::Identifier);
    assert_eq!(parser.bump().kind, TokenKind::Plus);
    parser.report_error(ParseError::syntax(
        "speculative error",
        parser.current_span(),
    ));
    assert_eq!(parser.errors().len(), 1);
    assert_eq!(parser.current_slice(), "b");

    parser.restore(cp);
    assert_eq!(parser.current_slice(), "a");
    assert!(parser.errors().is_empty());
}

#[test]
fn test_generic_gt_splitting_for_nested_generics() {
    // 1. `>>` splits into two `>` tokens.
    let mut parser = Parser::new("Array<Array<int>>");
    assert_eq!(parser.bump().kind, TokenKind::Identifier); // Array
    assert_eq!(parser.bump().kind, TokenKind::LessThan); // <
    assert_eq!(parser.bump().kind, TokenKind::Identifier); // Array
    assert_eq!(parser.bump().kind, TokenKind::LessThan); // <
    assert_eq!(parser.bump().kind, TokenKind::TypeInt); // int

    let inner_gt = parser.expect_generic_gt("'>'").unwrap();
    assert_eq!(parser.slice(inner_gt), ">");
    assert_eq!(parser.current_kind(), TokenKind::GreaterThan);

    let outer_gt = parser.expect_generic_gt("'>'").unwrap();
    assert_eq!(parser.slice(outer_gt), ">");
    assert!(parser.is_eof());

    // 2. `>>>` splits into three `>` tokens, and `>=` splits into `>` + `=`.
    let mut parser2 = Parser::new("Box<Box<Box<int>>>=1");
    for _ in 0..7 {
        parser2.bump(); // Box < Box < Box < int
    }
    let gt1 = parser2.expect_generic_gt("'>'").unwrap();
    let gt2 = parser2.expect_generic_gt("'>'").unwrap();
    let gt3 = parser2.expect_generic_gt("'>'").unwrap();
    assert_eq!(parser2.slice(gt1), ">");
    assert_eq!(parser2.slice(gt2), ">");
    assert_eq!(parser2.slice(gt3), ">");
    assert_eq!(parser2.bump().kind, TokenKind::Assign);
    assert_eq!(parser2.bump().kind, TokenKind::IntegerLiteral);
    assert!(parser2.is_eof());

    // 3. `Checkpoint` restores unsplit `>>` after speculative generic split.
    let mut parser3 = Parser::new("a >> b");
    parser3.bump(); // a
    let cp = parser3.checkpoint();
    assert!(parser3.eat_generic_gt().is_some());
    assert_eq!(parser3.current_kind(), TokenKind::GreaterThan);
    parser3.restore(cp);
    assert_eq!(parser3.current_kind(), TokenKind::ShiftRight);
}

#[test]
fn test_parse_eos_swift_style_rules() {
    // 1. Explicit semicolon is consumed.
    let mut p1 = Parser::new("1; 2");
    p1.bump(); // 1
    assert!(p1.parse_eos().is_ok());
    assert_eq!(p1.current_slice(), "2");

    // 2. Newline terminates statement without consuming the next token.
    let mut p2 = Parser::new("1\n2");
    p2.bump(); // 1
    assert!(p2.parse_eos().is_ok());
    assert_eq!(p2.current_slice(), "2");

    // 3. Close brace and EOF terminate statement without consuming.
    let mut p3 = Parser::new("{ 1 }");
    p3.bump(); // {
    p3.bump(); // 1
    assert!(p3.parse_eos().is_ok());
    assert_eq!(p3.current_kind(), TokenKind::CloseBrace);

    let mut p4 = Parser::new("1");
    p4.bump(); // 1
    assert!(p4.parse_eos().is_ok());
    assert!(p4.is_eof());

    // 4. Two statements on the same line without ';' trigger ExpectedEos.
    let mut p5 = Parser::new("1 2");
    p5.bump(); // 1
    let err = p5.parse_eos().expect_err("expected EOS error on same line");
    assert_eq!(
        err.kind,
        ParseErrorKind::ExpectedEos {
            found: TokenKind::IntegerLiteral
        }
    );
}

#[test]
fn test_error_recovery_synchronize_and_node_ids() {
    let mut parser = Parser::new("let = ???;\nlet ok = 1\n}\nreturn");

    assert_eq!(parser.alloc_id(), NodeId::new(0));
    assert_eq!(parser.alloc_id(), NodeId::new(1));
    assert_eq!(parser.next_node_id_raw(), 2);

    // Note: `???` produces '?' tokens, and `synchronize` skips until `;`.
    parser.bump(); // let
    parser.synchronize();
    assert_eq!(parser.current_kind(), TokenKind::Let);
    assert_eq!(parser.peek(1).kind, TokenKind::Identifier);

    // Test advance_and_synchronize guarantees progress even on a sync keyword.
    parser.advance_and_synchronize();
    assert_eq!(parser.current_kind(), TokenKind::CloseBrace);
}

#[test]
fn test_with_offset_and_lexer_error_propagation() {
    // Embedded expression slice starting at byte offset 10 with NodeId starting at 5.
    let mut parser = Parser::with_offset("a + @ b", 10, 5);
    assert_eq!(parser.alloc_id(), NodeId::new(5));

    // Lexer error on '@' was captured at collection time without halting token stream.
    assert_eq!(parser.errors().len(), 1);
    assert!(matches!(
        parser.errors()[0].kind,
        ParseErrorKind::LexerError(_)
    ));

    let a = parser.bump();
    assert_eq!(a.span, Span::new(10, 11));
    assert_eq!(parser.slice(a.span), "a");
    assert_eq!(parser.bump().kind, TokenKind::Plus);
    let b = parser.bump();
    assert_eq!(parser.slice(b.span), "b");
    assert_eq!(b.span, Span::new(16, 17));
}
