parser grammar kyanos_syntax_grammar;

/**
 * =============================================================================
 * Kyanos Programming Language (KyanosLang) - Syntax Grammar
 * =============================================================================
 * Normative Specifications & References:
 *   - design/design.md (Core Language Specification, Memory Model, OOP & Hybrid Typing)
 *   - design/design_ffi.md (C Foreign Function Interface, ABI, and Struct Memory Layout)
 *   - design/design_type_inference.md (Bidirectional Local Type Inference & Literal Rules)
 *
 * Architecture & Implementation Notes:
 *   - This ANTLR v4 grammar serves as the formal specification and reference model
 *     for the Kyanos parser.
 *   - Production Parser Strategy: The Kyanos compiler utilizes a handwritten
 *     recursive-descent parser with a Pratt parser for expression precedence parsing
 *     (rather than ANTLR generated code), matching the rules and precedence here.
 *   - Memory Model: Nim-style ORC (thread-local ARC + cycle collector without STW GC).
 *   - Type System: Hybrid nominal types (class / struct) + structural interfaces.
 * =============================================================================
 */

options {
	tokenVocab = kyanos_lex_grammar;
}

// =============================================================================
// Program Structure & Modules (ES Modules Style)
// =============================================================================
// Design Principles (design.md Section 2 & 5.5):
// - 1 File = 1 Module: Each .ky source file represents an isolated module boundary.
// - Static Module Resolution: Top-level 'import' and 'export' declarations establish
//   static dependency graphs, enabling deterministic compilation and dead-code elimination.

program: sourceElement* EOF;

sourceElement:
	importStatement
	| exportStatement
	| declaration
	| statement;

// Import Statements
// Supports:
//   import { Foo, Bar as Baz } from "./module";
//   import * as Math from "./math";
//   import DefaultItem from "./item";
//   import "./side_effect";
importStatement:
	Import importClause From StringLiteral eos
	| Import StringLiteral eos;

importClause:
	importNamedClause
	| Star As Identifier
	| Identifier (Comma importNamedClause)?;

importNamedClause:
	OpenBrace (importSpecifier (Comma importSpecifier)* Comma?)? CloseBrace;

importSpecifier: Identifier (As Identifier)?;

// Export Statements
// Supports:
//   export function add(...) { ... }
//   export struct Point { ... }
//   export { Foo, Bar as Baz };
//   export default expr;
exportStatement:
	attribute* Export declaration
	| Export exportClause (From StringLiteral)? eos
	| Export Default singleExpression eos;

exportClause:
	OpenBrace (exportSpecifier (Comma exportSpecifier)* Comma?)? CloseBrace;

exportSpecifier: Identifier (As Identifier)?;

// =============================================================================
// Declarations
// =============================================================================

declaration:
	variableStatement
	| usingStatement
	| functionDeclaration
	| structDeclaration
	| classDeclaration
	| interfaceDeclaration
	| enumDeclaration
	| externBlock
	| singleExternDeclaration;

// Variable & Resource Declarations (design.md Section 2, 4.1, 5.1, 5.4):
// - 'let': Mutable variable declaration.
// - 'const': Immutable variable (reassignment prohibited).
// - Const Mutability Semantics:
//   * Value types (struct, T[N], anonymous struct): 'const' enforces deep immutability
//     (mutating fields or elements is a compile-time error).
//   * Reference types (class, T[], dictionaries): 'const' enforces shallow immutability
//     (variable cannot be reassigned; internal fields are mutable unless marked 'readonly').
// - Binding targets support identifier names as well as tuple and object destructuring patterns.
variableStatement:
	Let variableDeclarationList eos
	| Const constDeclarationList eos;

varModifier: Let | Const;

variableDeclarationList:
	variableDeclaration (Comma variableDeclaration)*;

variableDeclaration:
	bindingTarget (Colon type_)? (Assign singleExpression)?;

constDeclarationList:
	constDeclaration (Comma constDeclaration)*;

constDeclaration:
	bindingTarget (Colon type_)? Assign singleExpression;

bindingTarget: Identifier | tuplePattern | objectPattern;

// Deterministic Resource Management (design.md Section 4.1 & 5.4):
// - 'using' declaration (TypeScript 5.2 / C# style).
// - Applicable to instances implementing the 'Disposable' interface (method 'dispose(): void').
// - Automatically calls .dispose() upon exiting the enclosing lexical block scope.
usingStatement:
	Using Identifier (Colon type_)? Assign singleExpression eos;

// Function Declaration (design.md Section 5.2):
// - Standalone top-level, local, or exported function definitions.
// - 'async' modifier denotes a stackless coroutine returning 'Promise<T>'.
// - Memory Model & Closures:
//   * Capturing closures allocate an environment record managed on the heap by Nim-style ORC.
//   * Non-capturing functions are represented as zero-cost 16-byte fat pointers [code_ptr, null],
//     which can be passed directly to C function pointer parameters in FFI.
// - Lexical Scope: No hoisting; functions are strictly bound to their block scope.
functionDeclaration:
	attribute* Async? Function Identifier typeParameters? OpenParen parameterList? CloseParen (
		Colon returnType
	)? (block | eos);

parameterList:
	parameter (Comma parameter)* (Comma restParameter)? Comma?
	| restParameter;

parameter:
	accessibilityModifier? Readonly? bindingTarget Question? (
		Colon type_
	)? (Assign singleExpression)?;

restParameter: Ellipsis Identifier (Colon type_)?;

accessibilityModifier: Public | Protected | Private;

// =============================================================================
// Struct Declaration (Nominal Value Type, Stack / C-ABI)
// =============================================================================
// Design Principles (design.md Section 3.1 & 5.3):
// - Nominal Value Type: Directly allocated on the stack; zero heap allocation overhead.
// - 1:1 C-ABI Compatibility: Field memory layout aligns with C structs (support for #[repr(C)]).
// - Zero-cost Direct Method Calls: Struct methods (including constructors) are resolved
//   statically at compile time and inlined without virtual table (vtable) dispatch.
// - Compile-Time Mutability Check: The compiler statically inspects methods modifying fields.
//   Calling mutating methods on a 'const' struct instance produces a compile-time error.
// - Inheritance Forbidden: 'extends' is prohibited for structs (C# model) to eliminate object
//   slicing and memory fragmentation; composition and interface conformance are used instead.

structDeclaration:
	attribute* Struct Identifier typeParameters? OpenBrace structMember* CloseBrace;

structMember:
	structField
	| constructorDeclaration
	| methodDeclaration
	| Semicolon;

structField:
	accessibilityModifier? Static? Readonly? Identifier (
		Colon type_
	)? (Assign singleExpression)? eos;

constructorDeclaration:
	accessibilityModifier? Constructor OpenParen parameterList? CloseParen block;

methodDeclaration:
	accessibilityModifier? Static? Async? Identifier typeParameters? OpenParen parameterList?
		CloseParen (Colon returnType)? block;

// =============================================================================
// Class Declaration (Nominal Reference Type, Heap / ORC)
// =============================================================================
// Design Principles (design.md Section 2, 3.1 & 5.3):
// - Memory Management: Heap-allocated and tracked via Nim-style ORC (thread-local fast ARC +
//   cycle collector without global STW GC pauses).
// - Single Inheritance: Single base class via 'extends' (implicitly inherits from root 'Object').
// - Multiple Interfaces: Conformance to multiple structural interfaces via 'implements'.
// - Member Accessibility:
//   * Default (omitted) / 'public': Accessible from any scope.
//   * 'protected': Accessible within defining class and derived subclasses.
//   * 'private' / '#field': Lexically restricted to the defining class body.
// - 'readonly': Field value cannot be reassigned after constructor execution.

classDeclaration:
	attribute* Class classElementName typeParameters? classHeritage? OpenBrace classMember*
		CloseBrace;

classHeritage:
	classExtendsClause implementsClause?
	| implementsClause;

classExtendsClause: Extends typeReference;

implementsClause: Implements interfaceTypeList;

interfaceTypeList: typeReference (Comma typeReference)*;

classMember:
	classField
	| constructorDeclaration
	| classMethod
	| getterSetter
	| Semicolon;

classField:
	accessibilityModifier? Static? Readonly? classElementName (
		Colon type_
	)? (Assign singleExpression)? eos;

classMethod:
	accessibilityModifier? Static? Async? classElementName typeParameters? OpenParen parameterList?
		CloseParen (Colon returnType)? block;

getterSetter:
	accessibilityModifier? Static? Identifier classElementName OpenParen parameterList? CloseParen (
		Colon type_
	)? block;

classElementName: Identifier | PrivateIdentifier;

// =============================================================================
// Interface Declaration (Structural Typing)
// =============================================================================
// Design Principles (design.md Section 3.1 & 5.10):
// - Structural Duck Typing: Any struct or class providing the required properties and methods
//   satisfies the interface without requiring an explicit 'implements' clause.
// - Dual Execution & Dispatch Models:
//   1. Static Monomorphization (Generics '<T extends Interface>'): Concrete types are inlined
//      at compile time with zero boxing and zero indirect call overhead.
//   2. Dynamic Dispatch (Fat Pointer): Interface variables hold [data_ptr, vtable_ptr].
//   3. Auto-Boxing: Passing a stack value type ('struct' or anonymous struct) to a dynamic
//      interface variable automatically boxes it on the heap under ORC management, preventing
//      dangling stack pointers when escaping scope.

interfaceDeclaration:
	Interface Identifier typeParameters? interfaceExtendsClause? OpenBrace interfaceMember*
		CloseBrace;

interfaceExtendsClause: Extends interfaceTypeList;

interfaceMember:
	interfaceProperty
	| interfaceMethod
	| indexSignature
	| Semicolon;

interfaceProperty:
	Readonly? Identifier Question? Colon type_ eos;

interfaceMethod:
	Identifier Question? typeParameters? OpenParen parameterList? CloseParen Colon returnType eos;

indexSignature:
	OpenBracket Identifier Colon (TypeInt | TypeString) CloseBracket Colon type_ eos;

// =============================================================================
// Enum & Algebraic Data Types (Tagged Union / Sum Types)
// =============================================================================
// Design Principles (design.md Section 3.4 & 5.6):
// - Algebraic Data Types (ADT / Tagged Union): Enum variants can carry typed payloads
//   (e.g., KeyPress(char), Click(x: int, y: int)) or unit values (e.g., PageLoad, Ok = 200).
// - Value Type Layout: Stack-allocated value types represented as [tag: int + payload union].
// - Non-Recursive Stack Rule: Direct recursive enums (e.g., Expr(Expr, Expr)) are invalid because
//   stack size cannot be determined at compile time; recursive payloads must be boxed via
//   'class', dynamic arrays 'T[]', or 'Box<T>'.
// - Core Standard Library Primitives: Standard types such as 'Option<T>' (Some(T) | None) and
//   'Result<T, E>' (Ok(T) | Err(E)) are standard ADTs rather than hardcoded syntax primitives.

enumDeclaration:
	attribute* Enum Identifier typeParameters? (
		Colon predefinedType
	)? OpenBrace enumVariantList? CloseBrace;

enumVariantList: enumVariant (Comma enumVariant)* Comma?;

enumVariant:
	Identifier OpenParen parameterList? CloseParen // Variant with payload, e.g. KeyPress(char)
	| Identifier (Assign singleExpression)?; // Simple numeric/unit variant, e.g. PageLoad, Ok = 200

// =============================================================================
// C-FFI (Foreign Function Interface) & Attributes
// =============================================================================
// Design Principles (design.md Section 5.4 & design_ffi.md):
// - Zero-cost C ABI interoperability: Direct external function calling and C symbol exporting.
// - Attributes: Rust/C# style syntax #[attr] or #[attr(args)] for linking (#[link = "..."]),
//   calling conventions (#[cdecl]), and symbol names (#[export_name = "..."]).
// - 'extern "C"' block syntax and single extern declarations for declaring C functions and globals.
// - Supports C variadic parameters ('...').

attribute:
	HashBracket Identifier (
		OpenParen attributeArgList? CloseParen
		| Assign singleExpression
	)? CloseBracket;

attributeArgList: attributeArg (Comma attributeArg)* Comma?;

attributeArg: (Identifier Assign)? singleExpression;

externBlock:
	attribute* Extern StringLiteral OpenBrace externMember* CloseBrace;

externMember:
	attribute* Function Identifier typeParameters? OpenParen externParameterList? CloseParen (
		Colon type_
	)? eos
	| attribute* (Let | Const) Identifier Colon type_ eos
	| Semicolon;

singleExternDeclaration:
	attribute* Extern StringLiteral Function Identifier typeParameters? OpenParen
		externParameterList? CloseParen (Colon type_)? (
		block
		| eos
	);

externParameterList:
	parameter (Comma parameter)* (
		Comma (Ellipsis | restParameter)
	)? Comma?
	| Ellipsis
	| restParameter;

// =============================================================================
// Type System
// =============================================================================
// Design Principles (design.md Section 3.2, 3.3, 3.6 & design_type_inference.md):
// - Unified Type Hierarchy: Single root 'Object' ('object'). Classes implicitly inherit Object.
//   Primitives and structs conceptually derive from ValueType -> Object, unboxed by default.
// - Type Predicates: 'Identifier is type_' return type annotation for TypeScript-style user-defined
//   type guards, narrowing types in conditional branches.
// - Pointers vs References:
//   * Raw Pointers: '*T' (mutable raw pointer), '*const T' (immutable raw pointer), 'rawptr' (void*).
//   * References: '&T' (mutable reference), '&const T' (immutable reference).
//   * No 'mut' keyword; mutability is uniformly governed by presence/absence of 'const'.
//   * Second-Class References: Restricted to local variables and function parameters to prevent
//     dangling pointers (cannot be returned from functions or stored in class/struct fields).
// - Arrays:
//   * 'T[]': Dynamic array (heap-allocated, ORC managed, resizable).
//   * 'T[N]': Fixed-size array (stack value type, inlined in structs, pass-by-value copy unless '&T[N]').
// - Syntactic Sugar: 'T?' is syntactic sugar for 'Option<T>'.

returnType:
	type_
	| typePredicate;

typePredicate:
	Identifier Is type_;

type_:
	functionType
	| prefixType
	| typeOrPostfix;

prefixType:
	pointerType
	| referenceType;

typeOrPostfix:
	primaryType									# PrimaryTypeWrap
	| typeOrPostfix Question					# OptionalTypeSugar // T? (Option<T>)
	| typeOrPostfix OpenBracket CloseBracket	# DynamicArrayType // T[]
	| typeOrPostfix OpenBracket (
		IntegerLiteral
		| singleExpression
	) CloseBracket # FixedArrayType ; // T[N]

primaryType:
	predefinedType					# PredefinedTypeRef
	| typeReference					# ReferenceTypeRef
	| tupleType						# TupleTypeRef
	| anonymousStructType			# AnonymousStructTypeRef
	| OpenParen type_ CloseParen	# ParenthesizedTypeRef
	| This							# ThisTypeRef;

// Predefined Built-in Primitives (design.md Section 3.3):
// - Integers: byte(i8), short(i16), int(i32), long(i64), size_t(isize) and unsigned ubyte, ushort, uint, ulong.
// - Floats: float(f32), double(f64).
// - String & Char: char (UTF-32 scalar, 4 bytes), string (PEP 393 flexible representation with O(1) indexing).
// - Low-level & Control: rawptr (void*), void, never, bool.
predefinedType:
	TypeByte
	| TypeShort
	| TypeInt
	| TypeLong
	| TypeSizeT
	| TypeUbyte
	| TypeUshort
	| TypeUint
	| TypeUlong
	| TypeFloat
	| TypeDouble
	| TypeBool
	| TypeChar
	| TypeString
	| TypeVoid
	| TypeNever
	| TypeRawptr;

typeReference: typeName typeArguments?;

typeName: Identifier (Dot Identifier)*;

// Pointer Types: '*T' (mutable raw pointer), '*const T' (immutable raw pointer)
pointerType:
	Star Const? type_;

// Reference Types: '&T' (mutable reference), '&const T' (immutable reference)
referenceType:
	Amp Const? type_;

// Tuple Type: [T, U, V] (stack value type)
tupleType:
	OpenBracket (type_ (Comma type_)* Comma?)? CloseBracket;

// Anonymous Struct Type: { x: int, y: double } (value type, fixed shape)
anonymousStructType: OpenBrace (interfaceMember)* CloseBrace;

// Function Type: (x: int, y: int) => int
functionType:
	typeParameters? OpenParen parameterList? CloseParen Arrow returnType;

typeParameters: LessThan typeParameterList GreaterThan;

typeParameterList: typeParameter (Comma typeParameter)* Comma?;

typeParameter: Identifier (Extends type_)? (Assign type_)?;

typeArguments: LessThan typeArgumentList GreaterThan;

typeArgumentList: type_ (Comma type_)* Comma?;

// =============================================================================
// Statements
// =============================================================================
// Design Principles (design.md Section 2, 4.1, 5.1, 5.4, 5.7):
// - Swift-style Semicolon Optionality: Newlines terminate statements; semicolons are optional
//   delimiters used when placing multiple statements on the same line.
// - Strict Block Scoping: All '{}' blocks form distinct lexical scopes with deterministic
//   RAII destructors on stack structs and ORC ref-count decrements executed at scope exit.
// - Modernized 'switch': Pattern matching with no fallthrough (implicit break); 'when' guard clauses.
// - 'defer': Defers execution of cleanup code until enclosing scope exits (LIFO order).
// - 'unsafe': Required when dereferencing raw pointers ('*ptr') or performing low-level operations.

statement:
	block
	| variableStatement
	| usingStatement
	| ifStatement
	| doStatement
	| whileStatement
	| forStatement
	| switchStatement
	| returnStatement
	| breakStatement
	| continueStatement
	| deferStatement
	| unsafeStatement
	| debuggerStatement
	| expressionStatement
	| emptyStatement;

block: OpenBrace statement* CloseBrace;

ifStatement:
	If OpenParen singleExpression CloseParen statement (
		Else statement
	)?;

doStatement:
	Do statement While OpenParen singleExpression CloseParen eos;

whileStatement:
	While OpenParen singleExpression CloseParen statement;

// For Statements:
// 1. Standard C-style loop: for (let i = 0; i < 10; i += 1) { ... }
// 2. Collection loop: for (let item of array) { ... }
// 3. Dictionary key loop: for (let key in dict) { ... }
forStatement:
	For OpenParen forInit? Semicolon singleExpression? Semicolon expressionSequence? CloseParen
		statement																			# StandardForStatement
	| For OpenParen varModifier bindingTarget (Of | In) singleExpression CloseParen statement #
		ForOfStatement;

forInit:
	Let variableDeclarationList
	| Const constDeclarationList
	| expressionSequence;

// Modernized Switch Statement:
// - Automatic break; no implicit fallthrough.
// - Pattern matching across literals, enums, structs, tuples with optional 'when' guard.
switchStatement:
	Switch OpenParen singleExpression CloseParen OpenBrace switchCase* CloseBrace;

switchCase:
	Case pattern (When singleExpression)? Colon statement*
	| Default Colon statement*;

returnStatement: Return singleExpression? eos;

breakStatement: Break Identifier? eos;

continueStatement: Continue Identifier? eos;

// 'defer' Statement (Go/Swift/Zig style):
// Executed in LIFO order upon exiting the enclosing block (normal return, early return, or error).
deferStatement: Defer statement;

// 'unsafe' Statement: Encloses low-level raw pointer dereferencing and unchecked FFI calls.
unsafeStatement: Unsafe block;

debuggerStatement: Debugger eos;

expressionStatement: singleExpression eos;

emptyStatement: Semicolon;

// =============================================================================
// Patterns (Pattern Matching for Switch & Destructuring)
// =============================================================================
// Design Principles (design.md Section 5.7):
// - Exhaustive pattern matching across algebraic data types, structs, tuples, and literals.
// - Struct field destructuring supports field binding and wildcards: Point { x: 0, y: let y }.
// - Variant patterns match tagged unions: Ok(let val), KeyPress(ch).
// - Tuple and array destructuring support rest patterns: [head, ...tail].
// - Wildcard pattern '_' matches any value without binding.

pattern:
	literalPattern
	| structPattern
	| variantPattern
	| tuplePattern
	| wildcardPattern
	| varPattern;

literalPattern:
	(Plus | Minus)? IntegerLiteral
	| (Plus | Minus)? FloatLiteral
	| (Plus | Minus)? BigIntLiteral
	| StringLiteral
	| CStringLiteral
	| CharLiteral
	| BooleanLiteral
	| Null;

structPattern:
	typeName OpenBrace (
		structFieldPattern (Comma structFieldPattern)* Comma?
	)? CloseBrace;

structFieldPattern:
	Identifier Colon pattern
	| (Let | Const)? Identifier
	| Ellipsis;

variantPattern:
	typeName (
		OpenParen (pattern (Comma pattern)* Comma?)? CloseParen
	)?;

tuplePattern:
	OpenBracket (
		tuplePatternElement (Comma tuplePatternElement)* Comma?
	)? CloseBracket;

tuplePatternElement:
	Ellipsis? pattern;

objectPattern:
	OpenBrace (
		objectPatternProperty (Comma objectPatternProperty)* Comma?
	)? CloseBrace;

objectPatternProperty:
	Identifier (Colon bindingTarget)? (Assign singleExpression)?
	| Ellipsis Identifier;

wildcardPattern: Underscore;

varPattern: (Let | Const)? Identifier;

// =============================================================================
// Expressions
// =============================================================================
// Design Principles (design.md Section 2, 3.7, 5.1, 5.4, 5.6, 5.11 & design_type_inference.md):
// - Precedence is formally ordered using ANTLR4 left-recursive rules, matching Pratt parser binding powers.
// - Numeric Literal Type Inference:
//   * Floating-point numbers default directly to 'double'.
//   * Integers start as uncommitted '{integer}', resolved via contextual reverse inference from
//     subsequent usage (e.g. array indexing, bitwise ops), falling back to 'double' if unconstrained.
// - Object Literals vs Dictionaries:
//   * Identifier keys '{ key: val }': Stack-allocated anonymous struct (value type, fixed shape).
//   * String keys '{ "key": val }': Heap-allocated dictionary (Map<string, V>, index access returns V?).
// - Specialized Prefix Unary Operators:
//   * '&val': Address-of / reference operator ('&T' or '&const T').
//   * '*ptr': Raw pointer dereference (requires 'unsafe').
//   * 'try expr': Early-return error propagation unwrapping Ok(v) or returning Err(e) (Rust '?' equivalent).
//   * 'await expr': Asynchronous suspension point (stackless coroutine returning Promise<T>).
//   * 'spawn expr': Offloads closure/task to thread pool, returning Promise<T>.
// - Explicit Casting: 'expr as Type' (strict explicit casts; no implicit widening).
// - Equality (==, !=): Value equality for primitives and structs; string content equality; class reference equality.

expressionSequence: singleExpression (Comma singleExpression)*;

singleExpression:
	// --- Primary Expressions ---
	literal														# LiteralExpression
	| Identifier												# IdentifierExpression
	| This														# ThisExpression
	| Super														# SuperExpression
	| OpenParen expressionSequence CloseParen					# ParenthesizedExpression
	| arrayLiteral												# ArrayLiteralExpression
	| objectLiteral												# ObjectLiteralExpression
	| structInstantiation										# StructInstantiationExpression
	| arrowFunction												# ArrowFunctionExpression
	| switchExpression											# SwitchExpressionArm
	| unsafeExpression											# UnsafeExpressionArm
	| Sizeof OpenParen (type_ | singleExpression) CloseParen	# SizeofExpression
	| predefinedType Dot Identifier								# PredefinedTypeMemberExpression

	// --- Postfix Expressions ---
	// Member Indexing: arr[i], dict["key"], optArr?.[i]
	| singleExpression OptionalChain? OpenBracket expressionSequence CloseBracket #
		MemberIndexExpression
	// Optional Chaining: obj?.prop
	| singleExpression OptionalChain Identifier													# OptionalChainExpression
	// Member Dot Access: obj.prop, obj.#privateProp
	| singleExpression Dot classElementName														# MemberDotExpression
	// Function Call: f(x, y)
	| singleExpression OpenParen argumentList? CloseParen										# CallExpression
	// Generic Function Call: f<int, string>(x, y)
	| singleExpression LessThan typeArgumentList GreaterThan OpenParen argumentList? CloseParen	#
		GenericCallExpression
	// Post-Increment & Post-Decrement: x++, x--
	| singleExpression PlusPlus		# PostIncrementExpression
	| singleExpression MinusMinus	# PostDecrementExpression
	// Non-Null Assertion: expr! (forces unwrapping of Option<T> / null check)
	| singleExpression Bang			# NonNullAssertionExpression

	// --- Prefix Unary Expressions ---
	// Pre-Increment & Pre-Decrement: ++x, --x
	| PlusPlus singleExpression		# PreIncrementExpression
	| MinusMinus singleExpression	# PreDecrementExpression
	// Unary Plus & Negation: +x, -x
	| Plus singleExpression			# UnaryPlusExpression
	| Minus singleExpression		# UnaryMinusExpression
	// Bitwise NOT: ~x
	| Tilde singleExpression		# BitNotExpression
	// Logical NOT: !x
	| Bang singleExpression			# NotExpression
	// Address-Of / Reference: &val (yields &T if 'let', &const T if 'const')
	| Amp singleExpression			# AddressOfExpression
	// Raw Pointer Dereference: *ptr (requires unsafe block or unsafe expression)
	| Star singleExpression			# DereferenceExpression
	// Early Return Error Propagation: try expr (unwraps Ok(v) or early-returns Err(e))
	| Try singleExpression			# TryExpression
	// Asynchronous Suspension: await promiseExpr (stackless coroutine)
	| Await singleExpression		# AwaitExpression
	// Thread-Pool Offloading: spawn taskExpr (returns Promise<T>)
	| Spawn singleExpression		# SpawnExpression
	// Dictionary Key Deletion: delete dict.key / delete dict["key"]
	| Delete singleExpression		# DeleteExpression
	// Type Query: typeof expr
	| Typeof singleExpression		# TypeofExpression
	// Object Allocation: new ClassName(...) / new StructName(...)
	| New singleExpression typeArguments? (
		OpenParen argumentList? CloseParen
	)? # NewExpression

	// --- Type Cast Expression ---
	// Explicit Cast: expr as Type (mandatory explicit conversion; no implicit widening)
	| singleExpression As type_ # CastExpression

	// --- Binary Arithmetic, Bitwise, Shift ---
	// Exponentiation (Right-Associative): a ** b
	| <assoc = right> singleExpression Power singleExpression		# PowerExpression
	// Multiplicative: a * b, a / b, a % b
	| singleExpression (Star | Slash | Percent) singleExpression	# MultiplicativeExpression
	// Additive: a + b, a - b
	| singleExpression (Plus | Minus) singleExpression				# AdditiveExpression
	// Bit Shifts: a << b, a >> b, a >>> b
	| singleExpression (
		ShiftLeft
		| ShiftRight
		| UnsignedShiftRight
	) singleExpression # BitShiftExpression

	// --- Relational & Equality ---
	// Relational Comparison: <, <=, >, >=
	| singleExpression (
		LessThan
		| LessThanEquals
		| GreaterThan
		| GreaterThanEquals
	) singleExpression											# RelationalExpression
	// Runtime Type Test: expr is Type (tests class hierarchy, interface match, or unboxed primitive)
	| singleExpression Is type_									# TypeTestExpression
	// Equality Comparison: ==, != (value equality for primitives/structs, content for strings, reference for classes)
	| singleExpression (Equals | NotEquals) singleExpression	# EqualityExpression

	// --- Bitwise Logical ---
	| singleExpression Amp singleExpression		# BitAndExpression
	| singleExpression Caret singleExpression	# BitXorExpression
	| singleExpression Bar singleExpression		# BitOrExpression

	// --- Short-Circuit Logical & Null-Coalescing ---
	| singleExpression AmpAmp singleExpression			# LogicalAndExpression
	| singleExpression BarBar singleExpression			# LogicalOrExpression
	// Null-Coalescing Operator: a ?? b (if 'a' is None/null, falls back to 'b')
	| singleExpression NullCoalescing singleExpression	# NullCoalescingExpression

	// --- Ternary Conditional Expression ---
	| <assoc = right> singleExpression Question singleExpression Colon singleExpression #
		TernaryExpression

	// --- Assignment Expressions ---
	| <assoc = right> singleExpression assignmentOperator singleExpression # AssignmentExpression;

// Struct Instantiation Literal: Point { x: 10, y: 20 }
structInstantiation:
	typeName typeArguments? OpenBrace (
		structFieldInit (Comma structFieldInit)* Comma?
	)? CloseBrace;

structFieldInit:
	Identifier (Colon singleExpression)?
	| Ellipsis singleExpression;

// Switch Expression: let msg = switch (status) { case Ok(v) => "Success", default => "Error" };
switchExpression:
	Switch OpenParen singleExpression CloseParen OpenBrace (
		switchExprArm (Comma switchExprArm)* Comma?
	)? CloseBrace;

switchExprArm:
	Case pattern (When singleExpression)? Arrow singleExpression
	| Default Arrow singleExpression;

unsafeExpression: Unsafe block;

// Arrow Function: (x: int, y: int): int => x + y
arrowFunction:
	Async? typeParameters? (
		Identifier
		| OpenParen parameterList? CloseParen
	) (Colon returnType)? Arrow (singleExpression | block);

// Array Literal: [1, 2, 3] (infers T[] dynamic array unless annotated as T[N] fixed-size array)
arrayLiteral:
	OpenBracket (arrayElement (Comma arrayElement)* Comma?)? CloseBracket;

arrayElement: Ellipsis? singleExpression;

// Object Literal:
// - Identifier keys '{ x: 10, y: 20 }' -> Anonymous struct (value type, stack allocated)
// - String keys '{ "host": "localhost" }' -> Dictionary (Map<string, V>, heap allocated)
objectLiteral:
	OpenBrace (objectMember (Comma objectMember)* Comma?)? CloseBrace;

objectMember:
	propertyName Colon singleExpression
	| Ellipsis singleExpression
	| methodDeclarationInObject
	| Identifier;

methodDeclarationInObject:
	Async? Identifier typeParameters? OpenParen parameterList? CloseParen (
		Colon returnType
	)? block;

propertyName:
	Identifier
	| StringLiteral
	| numericLiteral
	| OpenBracket singleExpression CloseBracket;

argumentList: argument (Comma argument)* Comma?;

argument: Ellipsis? singleExpression;

assignmentOperator:
	Assign
	| PlusAssign
	| MinusAssign
	| StarAssign
	| SlashAssign
	| PercentAssign
	| PowerAssign
	| ShiftLeftAssign
	| ShiftRightAssign
	| UnsignedShiftRightAssign
	| AndAssign
	| XorAssign
	| OrAssign
	| AmpAmpAssign
	| BarBarAssign
	| NullCoalescingAssign;

literal:
	OpenParen CloseParen
	| Null
	| BooleanLiteral
	| numericLiteral
	| bigintLiteral
	| StringLiteral
	| CStringLiteral
	| CharLiteral
	| TemplateStringLiteral;

numericLiteral: FloatLiteral | IntegerLiteral;

bigintLiteral: BigIntLiteral;

// Statement Terminator (Swift style):
// In the handwritten parser, newlines terminate statements unless the line ends with an uncompleted
// operator or continuation. Semicolons are optional and used for multi-statement lines.
eos: Semicolon?;