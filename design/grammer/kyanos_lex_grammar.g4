lexer grammar kyanos_lex_grammar;

// ============================================================================= Whitespace &
// Comments =============================================================================

Whitespace:
	[ \t\u000B\u000C\u00A0\uFEFF\p{Zs}]+ -> channel(HIDDEN);

LineTerminator:
	('\r\n' | [\r\n\u2028\u2029]) -> channel(HIDDEN);

SingleLineComment:
	'//' ~[\r\n\u2028\u2029]* -> channel(HIDDEN);

MultiLineComment: '/*' .*? '*/' -> channel(HIDDEN);

// ============================================================================= Keywords (Strictly
// KyanosLang Spec) =============================================================================

Let: 'let';
Const: 'const';
Function: 'function';
Class: 'class';
Struct: 'struct';
Interface: 'interface';
Enum: 'enum';
Constructor: 'constructor';

If: 'if';
Else: 'else';
Do: 'do';
While: 'while';
For: 'for';
In: 'in';
Of: 'of';
Return: 'return';
Break: 'break';
Continue: 'continue';
Switch: 'switch';
Case: 'case';
Default: 'default';
When: 'when';
Debugger: 'debugger';

Try: 'try';
Defer: 'defer';
Using: 'using';

Public: 'public';
Protected: 'protected';
Private: 'private';
Static: 'static';
Readonly: 'readonly';

New: 'new';
This: 'this';
Super: 'super';
Extends: 'extends';
Implements: 'implements';
As: 'as';

Import: 'import';
Export: 'export';
From: 'from';

Async: 'async';
Await: 'await';
// Reserved for future generator/iterator implementation
Yield: 'yield';
YieldStar: 'yield*';
Spawn: 'spawn';

Unsafe: 'unsafe';
Extern: 'extern';
Delete: 'delete';

True: 'true';
False: 'false';
BooleanLiteral: True | False;
Null: 'null';

// Primitive Type Keywords
TypeByte: 'byte' | 'i8';
TypeShort: 'short' | 'i16';
TypeInt: 'int' | 'i32';
TypeLong: 'long' | 'i64';
TypeSizeT: 'size_t' | 'isize';
TypeUbyte: 'ubyte' | 'u8';
TypeUshort: 'ushort' | 'u16';
TypeUint: 'uint' | 'u32';
TypeUlong: 'ulong' | 'u64';
TypeFloat: 'float' | 'f32';
TypeDouble: 'double' | 'f64';
TypeBool: 'bool';
TypeChar: 'char';
TypeString: 'string';
TypeVoid: 'void';
TypeNever: 'never';
TypeRawptr: 'rawptr';

// ============================================================================= Operators &
// Punctuators =============================================================================

OpenBrace: '{';
CloseBrace: '}';
OpenParen: '(';
CloseParen: ')';
OpenBracket: '[';
CloseBracket: ']';
Dot: '.';
Ellipsis: '...';
Semicolon: ';';
Comma: ',';
Colon: ':';
Question: '?';
OptionalChain: '?.';
NullCoalescing: '??';
Arrow: '=>';
HashBracket: '#[';
Underscore: '_';

// Arithmetic & Bitwise
Plus: '+';
Minus: '-';
Star: '*';
Slash: '/';
Percent: '%';
Power: '**';
PlusPlus: '++';
MinusMinus: '--';
Amp: '&';
Bar: '|';
Caret: '^';
Tilde: '~';
ShiftLeft: '<<';
ShiftRight: '>>';
UnsignedShiftRight: '>>>';

// Logical & Relational
Bang: '!';
AmpAmp: '&&';
BarBar: '||';
Equals: '==';
NotEquals: '!=';
LessThan: '<';
LessThanEquals: '<=';
GreaterThan: '>';
GreaterThanEquals: '>=';

// Assignments
Assign: '=';
PlusAssign: '+=';
MinusAssign: '-=';
StarAssign: '*=';
SlashAssign: '/=';
PercentAssign: '%=';
PowerAssign: '**=';
ShiftLeftAssign: '<<=';
ShiftRightAssign: '>>=';
UnsignedShiftRightAssign: '>>>=';
AndAssign: '&=';
OrAssign: '|=';
XorAssign: '^=';
AmpAmpAssign: '&&=';
BarBarAssign: '||=';
NullCoalescingAssign: '??=';

Is: 'is';
Typeof: 'typeof';
Sizeof: 'sizeof';

// ============================================================================= Literals (Numeric,
// String, Char, C-String)
// =============================================================================

// Float literal
FloatLiteral:
	Digit+ '.' Digit+ ExponentPart?
	| Digit+ ExponentPart;

// Integer literals (Dec, Hex, Bin, Oct) with optional underscores
IntegerLiteral:
	DecimalIntegerLiteral
	| HexIntegerLiteral
	| OctalIntegerLiteral
	| BinaryIntegerLiteral;

// BigInt literal with 'n' or 'N' suffix
BigIntLiteral:
	(
		DecimalIntegerLiteral
		| HexIntegerLiteral
		| OctalIntegerLiteral
		| BinaryIntegerLiteral
	) [nN];

fragment DecimalIntegerLiteral:
	'0'
	| [1-9] ('_'? Digit)*;

fragment HexIntegerLiteral:
	'0' [xX] HexDigit ('_'? HexDigit)*;

fragment OctalIntegerLiteral:
	'0' [oO] OctalDigit ('_'? OctalDigit)*;

fragment BinaryIntegerLiteral:
	'0' [bB] BinaryDigit ('_'? BinaryDigit)*;

fragment ExponentPart: [eE] [+-]? Digit+;

// Standard String Literal ("...")
StringLiteral: '"' ( EscapeSequence | ~["\\\r\n])* '"';

// Compile-time C-String Literal (c"...")
CStringLiteral: 'c"' ( EscapeSequence | ~["\\\r\n])* '"';

// Char Literal ('a')
CharLiteral: '\'' ( EscapeSequence | ~['\\\r\n]) '\'';

// Template String Literal (`...` with ${expr} interpolation, desugared to expr.toString() concatenation)
TemplateStringLiteral:
	'`' (EscapeSequence | TemplateInterpolation | ~[`\\\r\n])* '`';

fragment TemplateInterpolation:
	'${' (StringLiteral | CharLiteral | ~[}\r\n])* '}';

fragment EscapeSequence:
	'\\' (['"\\/bfnrtv] | '0' | UnicodeEscape);

fragment Digit: [0-9];

fragment HexDigit: [0-9a-fA-F];

fragment OctalDigit: [0-7];

fragment BinaryDigit: [01];

fragment UnicodeEscape:
	'u' HexDigit HexDigit HexDigit HexDigit
	| 'u{' HexDigit+ '}';

// ============================================================================= Identifiers
// (Unicode ID_Start / ID_Continue supported)
// =============================================================================

Identifier: IdentifierStart IdentifierPart*;

PrivateIdentifier: '#' IdentifierStart IdentifierPart*;

fragment IdentifierStart: [\p{ID_Start}] | '$' | '_';

fragment IdentifierPart: [\p{ID_Continue}] | '$';