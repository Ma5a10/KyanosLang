// Copyright (c) 2026 ma5a10
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use compiler_tests::tokenize;
use kyanos_lexer::{Lexer, TokenKind};

#[test]
fn test_operators_continuous() {
    let mut lexer = Lexer::new("{}()[].;,:?_+-*/%&|~!< > =");
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::OpenBrace));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::CloseBrace));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::OpenParen));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::CloseParen));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::OpenBracket));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::CloseBracket));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Dot));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Semicolon));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Comma));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Colon));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Question));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Underscore));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Plus));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Minus));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Star));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Slash));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Percent));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::And));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Bar));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Tilde));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Bang));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::LessThan));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::GreaterThan));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Assign));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Eof));
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

#[test]
fn test_identifier_start_and_part() {
    assert!(Lexer::is_identifier_start('a'));
    assert!(!Lexer::is_identifier_start('_')); // Single '_' is not IdentifierStart
    assert!(Lexer::is_identifier_start('$'));
    assert!(Lexer::is_identifier_start('あ'));
    assert!(!Lexer::is_identifier_start('0'));

    assert!(Lexer::is_identifier_part('a'));
    assert!(Lexer::is_identifier_part('0'));
    assert!(Lexer::is_identifier_part('_'));
    assert!(Lexer::is_identifier_part('$'));
    assert!(Lexer::is_identifier_part('あ'));
    assert!(!Lexer::is_identifier_part('+'));
}

#[test]
fn test_scan_identifier() {
    let mut lexer = Lexer::new("foo_bar123 $val _temp __");
    assert_eq!(
        lexer.scan_identifier_or_keyword(),
        Some("foo_bar123".to_string())
    );
    lexer.skip_whitespace();
    assert_eq!(lexer.scan_identifier_or_keyword(), Some("$val".to_string()));
    lexer.skip_whitespace();
    assert_eq!(
        lexer.scan_identifier_or_keyword(),
        Some("_temp".to_string())
    );
    lexer.skip_whitespace();
    assert_eq!(lexer.scan_identifier_or_keyword(), Some("__".to_string()));

    // Single '_' should not be scanned as an identifier
    let mut wildcard_lexer = Lexer::new("_");
    assert_eq!(wildcard_lexer.scan_identifier_or_keyword(), None);
    assert_eq!(wildcard_lexer.next_token().map(|t| t.kind), Ok(TokenKind::Underscore));
}

#[test]
fn test_next_token_identifier() {
    let mut lexer = Lexer::new("myVar + other");
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Identifier));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Plus));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Identifier));
}

#[test]
fn test_keywords() {
    // Declarations & Definitions
    let decls = tokenize("let const function class struct interface enum constructor").unwrap();
    assert_eq!(
        decls,
        vec![
            TokenKind::Let,
            TokenKind::Const,
            TokenKind::Function,
            TokenKind::Class,
            TokenKind::Struct,
            TokenKind::Interface,
            TokenKind::Enum,
            TokenKind::Constructor,
        ]
    );

    // Control flow
    let control = tokenize(
        "if else do while for in of return break continue switch case default when debugger",
    )
    .unwrap();
    assert_eq!(
        control,
        vec![
            TokenKind::If,
            TokenKind::Else,
            TokenKind::Do,
            TokenKind::While,
            TokenKind::For,
            TokenKind::In,
            TokenKind::Of,
            TokenKind::Return,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Switch,
            TokenKind::Case,
            TokenKind::Default,
            TokenKind::When,
            TokenKind::Debugger,
        ]
    );

    // Resource & Modifiers
    let res_mods = tokenize("try defer using public protected private static readonly").unwrap();
    assert_eq!(
        res_mods,
        vec![
            TokenKind::Try,
            TokenKind::Defer,
            TokenKind::Using,
            TokenKind::Public,
            TokenKind::Protected,
            TokenKind::Private,
            TokenKind::Static,
            TokenKind::Readonly,
        ]
    );

    // OOP, Modules & Concurrency
    let oop_mod = tokenize("new this super extends implements as import export from async await yield spawn unsafe extern delete true false null").unwrap();
    assert_eq!(
        oop_mod,
        vec![
            TokenKind::New,
            TokenKind::This,
            TokenKind::Super,
            TokenKind::Extends,
            TokenKind::Implements,
            TokenKind::As,
            TokenKind::Import,
            TokenKind::Export,
            TokenKind::From,
            TokenKind::Async,
            TokenKind::Await,
            TokenKind::Yield,
            TokenKind::Spawn,
            TokenKind::Unsafe,
            TokenKind::Extern,
            TokenKind::Delete,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Null,
        ]
    );

    // Type Operators
    let type_ops = tokenize("is typeof sizeof").unwrap();
    assert_eq!(
        type_ops,
        vec![TokenKind::Is, TokenKind::Typeof, TokenKind::Sizeof,]
    );
}

#[test]
fn test_yield_star() {
    let tokens = tokenize("yield* yield * yield*foo").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::YieldStar,
            TokenKind::Yield,
            TokenKind::Star,
            TokenKind::YieldStar,
            TokenKind::Identifier,
        ]
    );
}

#[test]
fn test_primitive_types() {
    let standard = tokenize("byte short int long size_t ubyte ushort uint ulong float double bool char string void never rawptr").unwrap();
    assert_eq!(
        standard,
        vec![
            TokenKind::TypeByte,
            TokenKind::TypeShort,
            TokenKind::TypeInt,
            TokenKind::TypeLong,
            TokenKind::TypeSizeT,
            TokenKind::TypeUbyte,
            TokenKind::TypeUshort,
            TokenKind::TypeUint,
            TokenKind::TypeUlong,
            TokenKind::TypeFloat,
            TokenKind::TypeDouble,
            TokenKind::TypeBool,
            TokenKind::TypeChar,
            TokenKind::TypeString,
            TokenKind::TypeVoid,
            TokenKind::TypeNever,
            TokenKind::TypeRawptr,
        ]
    );

    // Rust-like aliases
    let aliases = tokenize("i8 i16 i32 i64 isize u8 u16 u32 u64 f32 f64").unwrap();
    assert_eq!(
        aliases,
        vec![
            TokenKind::TypeByte,
            TokenKind::TypeShort,
            TokenKind::TypeInt,
            TokenKind::TypeLong,
            TokenKind::TypeSizeT,
            TokenKind::TypeUbyte,
            TokenKind::TypeUshort,
            TokenKind::TypeUint,
            TokenKind::TypeUlong,
            TokenKind::TypeFloat,
            TokenKind::TypeDouble,
        ]
    );
}

#[test]
fn test_keyword_prefix_identifiers() {
    let tokens =
        tokenize("letgo const_val if_then format while1 class_name island typeof_val sizeof_t")
            .unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
        ]
    );
}

#[test]
fn test_type_operator_expressions() {
    let is_expr = tokenize("x is string;").unwrap();
    assert_eq!(
        is_expr,
        vec![
            TokenKind::Identifier,
            TokenKind::Is,
            TokenKind::TypeString,
            TokenKind::Semicolon,
        ]
    );

    let typeof_expr = tokenize("typeof x;").unwrap();
    assert_eq!(
        typeof_expr,
        vec![
            TokenKind::Typeof,
            TokenKind::Identifier,
            TokenKind::Semicolon,
        ]
    );

    let sizeof_expr = tokenize("sizeof(int);").unwrap();
    assert_eq!(
        sizeof_expr,
        vec![
            TokenKind::Sizeof,
            TokenKind::OpenParen,
            TokenKind::TypeInt,
            TokenKind::CloseParen,
            TokenKind::Semicolon,
        ]
    );
}

#[test]
fn test_simple_statements() {
    let tokens = tokenize("let x: int = y;").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier,
            TokenKind::Colon,
            TokenKind::TypeInt,
            TokenKind::Assign,
            TokenKind::Identifier,
            TokenKind::Semicolon,
        ]
    );

    let if_tokens = tokenize("if (x) { return y; }").unwrap();
    assert_eq!(
        if_tokens,
        vec![
            TokenKind::If,
            TokenKind::OpenParen,
            TokenKind::Identifier,
            TokenKind::CloseParen,
            TokenKind::OpenBrace,
            TokenKind::Return,
            TokenKind::Identifier,
            TokenKind::Semicolon,
            TokenKind::CloseBrace,
        ]
    );
}

#[test]
fn test_line_terminators() {
    assert!(Lexer::is_line_terminator('\r'));
    assert!(Lexer::is_line_terminator('\n'));
    assert!(Lexer::is_line_terminator('\u{2028}'));
    assert!(Lexer::is_line_terminator('\u{2029}'));
    assert!(!Lexer::is_line_terminator(' '));
    assert!(!Lexer::is_line_terminator('a'));

    let mut lexer = Lexer::new("a\r\nb\nc\rd\u{2028}e\u{2029}f");
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Identifier));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Identifier));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Identifier));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Identifier));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Identifier));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Identifier));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Eof));

    let tokens = tokenize("let x: int = y;\n\r\n  let z: int = w;").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier,
            TokenKind::Colon,
            TokenKind::TypeInt,
            TokenKind::Assign,
            TokenKind::Identifier,
            TokenKind::Semicolon,
            TokenKind::Let,
            TokenKind::Identifier,
            TokenKind::Colon,
            TokenKind::TypeInt,
            TokenKind::Assign,
            TokenKind::Identifier,
            TokenKind::Semicolon,
        ]
    );
}

#[test]
fn test_single_line_comment() {
    let mut lexer = Lexer::new("// this is a comment\nlet");
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::SingleLineComment));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Let));
}

#[test]
fn test_multi_line_comment() {
    let mut lexer = Lexer::new("/* comment\nline2 */ let");
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::MultiLineComment));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Let));
}

#[test]
fn test_unterminated_multi_line_comment() {
    let mut lexer = Lexer::new("/* unclosed comment");
    assert!(lexer.next_token().is_err());
}

#[test]
fn test_slash_and_slash_assign() {
    let mut lexer = Lexer::new("/ /=");
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::Slash));
    assert_eq!(lexer.next_token().map(|t| t.kind), Ok(TokenKind::SlashAssign));
}

#[test]
fn test_arithmetic_operators() {
    let tokens = tokenize("+ - * / % ** ++ --").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::Power,
            TokenKind::PlusPlus,
            TokenKind::MinusMinus,
        ]
    );

    // Expressions with arithmetic and increment/decrement operators
    let expr = tokenize("x++ + y-- - z ** w").unwrap();
    assert_eq!(
        expr,
        vec![
            TokenKind::Identifier,
            TokenKind::PlusPlus,
            TokenKind::Plus,
            TokenKind::Identifier,
            TokenKind::MinusMinus,
            TokenKind::Minus,
            TokenKind::Identifier,
            TokenKind::Power,
            TokenKind::Identifier,
        ]
    );
}

#[test]
fn test_bitwise_and_shift_operators() {
    let tokens = tokenize("& | ^ ~ << >> >>>").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::And,
            TokenKind::Bar,
            TokenKind::Caret,
            TokenKind::Tilde,
            TokenKind::ShiftLeft,
            TokenKind::ShiftRight,
            TokenKind::UnsignedShiftRight,
        ]
    );

    // Differentiate between comparison and shift operators
    let diff = tokenize("< <= << <<= > >= >> >>= >>> >>>=").unwrap();
    assert_eq!(
        diff,
        vec![
            TokenKind::LessThan,
            TokenKind::LessThanEquals,
            TokenKind::ShiftLeft,
            TokenKind::ShiftLeftAssign,
            TokenKind::GreaterThan,
            TokenKind::GreaterThanEquals,
            TokenKind::ShiftRight,
            TokenKind::ShiftRightAssign,
            TokenKind::UnsignedShiftRight,
            TokenKind::UnsignedShiftRightAssign,
        ]
    );
}

#[test]
fn test_logical_and_relational_operators() {
    let tokens = tokenize("! && || == != < <= > >=").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::Bang,
            TokenKind::AndAnd,
            TokenKind::BarBar,
            TokenKind::Equals,
            TokenKind::NotEquals,
            TokenKind::LessThan,
            TokenKind::LessThanEquals,
            TokenKind::GreaterThan,
            TokenKind::GreaterThanEquals,
        ]
    );

    let expr = tokenize("!a && b || c == d != e").unwrap();
    assert_eq!(
        expr,
        vec![
            TokenKind::Bang,
            TokenKind::Identifier,
            TokenKind::AndAnd,
            TokenKind::Identifier,
            TokenKind::BarBar,
            TokenKind::Identifier,
            TokenKind::Equals,
            TokenKind::Identifier,
            TokenKind::NotEquals,
            TokenKind::Identifier,
        ]
    );
}

#[test]
fn test_all_assignment_operators() {
    // All 16 assignment operators defined in Kyanos grammar
    let tokens = tokenize("= += -= *= /= %= **= <<= >>= >>>= &= |= ^= &&= ||= ??=").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::Assign,
            TokenKind::PlusAssign,
            TokenKind::MinusAssign,
            TokenKind::StarAssign,
            TokenKind::SlashAssign,
            TokenKind::PercentAssign,
            TokenKind::PowerAssign,
            TokenKind::ShiftLeftAssign,
            TokenKind::ShiftRightAssign,
            TokenKind::UnsignedShiftRightAssign,
            TokenKind::AndAssign,
            TokenKind::BarAssign,
            TokenKind::CaretAssign,
            TokenKind::AndAndAssign,
            TokenKind::BarBarAssign,
            TokenKind::NullCoalescingAssign,
        ]
    );
}

#[test]
fn test_ternary_optional_null_coalescing_and_punctuators() {
    // Ternary operator (? :), optional chaining (?.), null coalescing (??)
    let tokens = tokenize("? ?. ?? ??= => ... .").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::Question,
            TokenKind::OptionalChain,
            TokenKind::NullCoalescing,
            TokenKind::NullCoalescingAssign,
            TokenKind::Arrow,
            TokenKind::Ellipsis,
            TokenKind::Dot,
        ]
    );

    // Ternary expression: cond ? expr1 : expr2
    let ternary_expr = tokenize("cond ? a : b").unwrap();
    assert_eq!(
        ternary_expr,
        vec![
            TokenKind::Identifier,
            TokenKind::Question,
            TokenKind::Identifier,
            TokenKind::Colon,
            TokenKind::Identifier,
        ]
    );

    // Optional chaining and null coalescing: a?.b ?? c
    let opt_expr = tokenize("a?.b ?? c").unwrap();
    assert_eq!(
        opt_expr,
        vec![
            TokenKind::Identifier,
            TokenKind::OptionalChain,
            TokenKind::Identifier,
            TokenKind::NullCoalescing,
            TokenKind::Identifier,
        ]
    );

    // Arrow function syntax: (x) => x
    let arrow_expr = tokenize("(x) => x").unwrap();
    assert_eq!(
        arrow_expr,
        vec![
            TokenKind::OpenParen,
            TokenKind::Identifier,
            TokenKind::CloseParen,
            TokenKind::Arrow,
            TokenKind::Identifier,
        ]
    );
}

#[test]
fn test_string_literal() {
    let tokens = tokenize(r#""hello world" "" "escaped \" \\ \n \t" "\u0041 \u{1F600}""#).unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::StringLiteral,
            TokenKind::StringLiteral,
            TokenKind::StringLiteral,
            TokenKind::StringLiteral,
        ]
    );

    // Negative tests
    let mut unclosed = Lexer::new(r#""unclosed string"#);
    assert!(unclosed.next_token().is_err());

    let mut newline_in_string = Lexer::new("\"multi\nline\"");
    assert!(newline_in_string.next_token().is_err());

    let mut invalid_escape = Lexer::new(r#""\q invalid""#);
    assert!(invalid_escape.next_token().is_err());
}

#[test]
fn test_c_string_literal() {
    let tokens = tokenize(r#"c"hello C-string\0" c"" c"#).unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::CStringLiteral,
            TokenKind::CStringLiteral,
            TokenKind::Identifier, // 'c' followed by whitespace is an identifier
        ]
    );

    // C-string in expression: c + c"foo"
    let expr = tokenize(r#"c + c"foo""#).unwrap();
    assert_eq!(
        expr,
        vec![
            TokenKind::Identifier,
            TokenKind::Plus,
            TokenKind::CStringLiteral,
        ]
    );
}

#[test]
fn test_char_literal() {
    let tokens = tokenize(r#"'a' '\n' '\'' '\\' '\u0041' '\u{1F600}'"#).unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::CharLiteral,
            TokenKind::CharLiteral,
            TokenKind::CharLiteral,
            TokenKind::CharLiteral,
            TokenKind::CharLiteral,
            TokenKind::CharLiteral,
        ]
    );

    // Negative tests
    let mut empty_char = Lexer::new("''");
    assert!(empty_char.next_token().is_err());

    let mut multi_char = Lexer::new("'abc'");
    assert!(multi_char.next_token().is_err());

    let mut unclosed_char = Lexer::new("'a");
    assert!(unclosed_char.next_token().is_err());
}

#[test]
fn test_integer_literals() {
    let tokens = tokenize("0 42 1_000_000 0x0 0xFF 0x1a_2b 0XDEAD 0b0 0b1010 0b11_00 0o0 0o755 0o12_34").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::IntegerLiteral, // 0
            TokenKind::IntegerLiteral, // 42
            TokenKind::IntegerLiteral, // 1_000_000
            TokenKind::IntegerLiteral, // 0x0
            TokenKind::IntegerLiteral, // 0xFF
            TokenKind::IntegerLiteral, // 0x1a_2b
            TokenKind::IntegerLiteral, // 0XDEAD
            TokenKind::IntegerLiteral, // 0b0
            TokenKind::IntegerLiteral, // 0b1010
            TokenKind::IntegerLiteral, // 0b11_00
            TokenKind::IntegerLiteral, // 0o0
            TokenKind::IntegerLiteral, // 0o755
            TokenKind::IntegerLiteral, // 0o12_34
        ]
    );

    // Negative tests
    let mut leading_zero = Lexer::new("0123");
    assert!(leading_zero.next_token().is_err());

    let mut trailing_underscore = Lexer::new("123_");
    assert!(trailing_underscore.next_token().is_err());

    let mut consecutive_underscores = Lexer::new("1__23");
    assert!(consecutive_underscores.next_token().is_err());

    let mut empty_hex = Lexer::new("0x");
    assert!(empty_hex.next_token().is_err());

    let mut ident_suffix = Lexer::new("123foo");
    assert!(ident_suffix.next_token().is_err());
}

#[test]
fn test_float_literals() {
    let tokens = tokenize("0.0 3.14 1_000.50_00 1e10 1E+5 1e-3 0.5e2 3.14E-10 0e5").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::FloatLiteral, // 0.0
            TokenKind::FloatLiteral, // 3.14
            TokenKind::FloatLiteral, // 1_000.50_00
            TokenKind::FloatLiteral, // 1e10
            TokenKind::FloatLiteral, // 1E+5
            TokenKind::FloatLiteral, // 1e-3
            TokenKind::FloatLiteral, // 0.5e2
            TokenKind::FloatLiteral, // 3.14E-10
            TokenKind::FloatLiteral, // 0e5
        ]
    );

    // Differentiate float literal from property access: 123.toString()
    let prop_access = tokenize("123.toString").unwrap();
    assert_eq!(
        prop_access,
        vec![
            TokenKind::IntegerLiteral,
            TokenKind::Dot,
            TokenKind::Identifier,
        ]
    );

    // Negative tests
    let mut empty_exponent = Lexer::new("1e");
    assert!(empty_exponent.next_token().is_err());

    // "1." is tokenized as IntegerLiteral(1) followed by Dot(.)
    let mut trailing_dot = Lexer::new("1.");
    assert_eq!(trailing_dot.next_token().map(|t| t.kind), Ok(TokenKind::IntegerLiteral));
    assert_eq!(trailing_dot.next_token().map(|t| t.kind), Ok(TokenKind::Dot));
}

#[test]
fn test_bigint_literals() {
    let tokens = tokenize("0n 123n 1_000n 0xFFn 0X1Fn 0b1010n 0o777n 123N").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::BigIntLiteral, // 0n
            TokenKind::BigIntLiteral, // 123n
            TokenKind::BigIntLiteral, // 1_000n
            TokenKind::BigIntLiteral, // 0xFFn
            TokenKind::BigIntLiteral, // 0X1Fn
            TokenKind::BigIntLiteral, // 0b1010n
            TokenKind::BigIntLiteral, // 0o777n
            TokenKind::BigIntLiteral, // 123N
        ]
    );

    // Expression with BigInt and arithmetic
    let expr = tokenize("x + 100n * y").unwrap();
    assert_eq!(
        expr,
        vec![
            TokenKind::Identifier,
            TokenKind::Plus,
            TokenKind::BigIntLiteral,
            TokenKind::Star,
            TokenKind::Identifier,
        ]
    );

    // Float with 'n' is not allowed (FloatLiteral followed by invalid suffix)
    let mut float_bigint = Lexer::new("1.5n");
    assert!(float_bigint.next_token().is_err());
}

#[test]
fn test_template_string_literals() {
    let tokens = tokenize(r#"`hello world` `` `escaped \` and \${x}`"#).unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::TemplateStringLiteral,
            TokenKind::TemplateStringLiteral,
            TokenKind::TemplateStringLiteral,
        ]
    );

    // Template with interpolation
    let interpolated = tokenize(r#"`count: ${count}, user: ${user.name}`"#).unwrap();
    assert_eq!(
        interpolated,
        vec![TokenKind::TemplateStringLiteral]
    );

    // Template with nested braces and strings inside expression
    let nested = tokenize(r#"`result: ${format("{item}", { id: 1 })}`"#).unwrap();
    assert_eq!(
        nested,
        vec![TokenKind::TemplateStringLiteral]
    );

    // Negative tests
    let mut unclosed = Lexer::new("`unclosed template");
    assert!(unclosed.next_token().is_err());

    let mut unclosed_interp = Lexer::new("`hello ${world`");
    assert!(unclosed_interp.next_token().is_err());

    let mut newline_in_template = Lexer::new("`line1\nline2`");
    assert!(newline_in_template.next_token().is_err());
}

#[test]
fn test_hash_bracket_and_private_identifier() {
    // HashBracket: #[
    let attr_tokens = tokenize("#[inline]").unwrap();
    assert_eq!(
        attr_tokens,
        vec![
            TokenKind::HashBracket,
            TokenKind::Identifier,
            TokenKind::CloseBracket,
        ]
    );

    // PrivateIdentifier: #ident, #_private, #_
    let private_tokens = tokenize("#field #_private #__ #_ #$secret #var123").unwrap();
    assert_eq!(
        private_tokens,
        vec![
            TokenKind::PrivateIdentifier, // #field
            TokenKind::PrivateIdentifier, // #_private
            TokenKind::PrivateIdentifier, // #__
            TokenKind::PrivateIdentifier, // #_
            TokenKind::PrivateIdentifier, // #$secret
            TokenKind::PrivateIdentifier, // #var123
        ]
    );

    // Private identifier in member access: this.#field = 42;
    let member_access = tokenize("this.#field = 42;").unwrap();
    assert_eq!(
        member_access,
        vec![
            TokenKind::This,
            TokenKind::Dot,
            TokenKind::PrivateIdentifier,
            TokenKind::Assign,
            TokenKind::IntegerLiteral,
            TokenKind::Semicolon,
        ]
    );

    // Negative tests
    let mut lonely_hash = Lexer::new("#");
    assert!(lonely_hash.next_token().is_err());

    let mut hash_with_space = Lexer::new("# foo");
    assert!(hash_with_space.next_token().is_err());

    let mut hash_with_digit = Lexer::new("#123");
    assert!(hash_with_digit.next_token().is_err());

    let mut hash_with_invalid_char = Lexer::new("#+");
    assert!(hash_with_invalid_char.next_token().is_err());
}

#[test]
fn test_unicode_whitespace() {
    // Basic whitespaces & BOM
    assert!(Lexer::is_whitespace(' '));
    assert!(Lexer::is_whitespace('\t'));
    assert!(Lexer::is_whitespace('\u{000B}'));
    assert!(Lexer::is_whitespace('\u{000C}'));
    assert!(Lexer::is_whitespace('\u{FEFF}'));

    // Unicode space separators (\p{Zs})
    assert!(Lexer::is_whitespace('\u{00A0}')); // NO-BREAK SPACE
    assert!(Lexer::is_whitespace('\u{1680}')); // OGHAM SPACE MARK
    assert!(Lexer::is_whitespace('\u{2000}')); // EN QUAD
    assert!(Lexer::is_whitespace('\u{2005}')); // FOUR-PER-EM SPACE
    assert!(Lexer::is_whitespace('\u{200A}')); // HAIR SPACE
    assert!(Lexer::is_whitespace('\u{202F}')); // NARROW NO-BREAK SPACE
    assert!(Lexer::is_whitespace('\u{205F}')); // MEDIUM MATHEMATICAL SPACE
    assert!(Lexer::is_whitespace('\u{3000}')); // IDEOGRAPHIC SPACE (全角スペース)

    // Line terminators must NOT be classified as whitespace (they serve as statement terminators)
    assert!(!Lexer::is_whitespace('\n'));
    assert!(!Lexer::is_whitespace('\r'));
    assert!(!Lexer::is_whitespace('\u{2028}'));
    assert!(!Lexer::is_whitespace('\u{2029}'));

    // Tokenization with full-width space and other Unicode whitespaces
    let tokens = tokenize("let\u{3000}x\u{00A0}=\u{2003}42;").unwrap();
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier,
            TokenKind::Assign,
            TokenKind::IntegerLiteral,
            TokenKind::Semicolon,
        ]
    );
}

#[test]
fn test_token_text_via_span() {
    let mut lexer = Lexer::new("let myVariable = 1_000_000;");
    // 1. 'let'
    let tok1 = lexer.next_token().unwrap();
    assert_eq!(tok1.kind, TokenKind::Let);
    assert_eq!(lexer.slice(tok1.span), "let");
    // 2. 'myVariable' (識別子名を取得)
    let tok2 = lexer.next_token().unwrap();
    assert_eq!(tok2.kind, TokenKind::Identifier);
    assert_eq!(lexer.slice(tok2.span), "myVariable");
    // 3. '='
    let tok3 = lexer.next_token().unwrap();
    assert_eq!(tok3.kind, TokenKind::Assign);
    assert_eq!(lexer.slice(tok3.span), "=");
    // 4. '1_000_000' (リテラルテキストを取得)
    let tok4 = lexer.next_token().unwrap();
    assert_eq!(tok4.kind, TokenKind::IntegerLiteral);
    assert_eq!(lexer.slice(tok4.span), "1_000_000");
}