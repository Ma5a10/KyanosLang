# Kyanos 型推論仕様書 (Type Inference Specification)

## 1. 基本方針 (Overview & Philosophy)

Kyanos の型システムは、**「TypeScript/JavaScript の親しみやすく軽快な書き味」** と **「AOTコンパイルによる C/Rust 級の実行性能・メモリ安全性」** の両立を基本思想とする。

この目的を達成するため、型推論には **双方向 局所型推論 (Bidirectional Local Type Inference)** を採用する。

* **局所推論 (Local Scope)**:
  * 関数の境界（シグネチャ）を型チェックの防壁とし、推論の適用範囲を関数およびブロック内部に限定する。
  * これにより、コンパイル速度を高速に保ち、型エラーの発生位置を正確かつ分かりやすく開発者へ伝える。
* **双方向型推論 (Bidirectional Inference)**:
  * 式そのものから型を導出する **「上向き合成 (Synthesizing)」** と、左辺や呼び出し先から期待される型を伝える **「下向き伝播 / 文脈的型付け (Contextual Typing)」**、および関数内の後続利用箇所からの **「局所制約伝播（逆推論）」** を組み合わせて柔軟に型を解決する。
* **関数・メソッドの境界ルール**:
  * トップレベル関数・名前付き関数・クラス/構造体メソッド: 引数型は原則として明示必須（戻り値型は非再帰であれば本体から推論可能）。
  * コールバック・匿名関数（アロー関数）: 文脈から推論可能なため、引数型・戻り値型の明示は**不要（省略可能）**。

---

## 2. リテラルの推論規則とデフォルト型 (Literal Defaulting)

### 2.1 数値リテラル (Numeric Literals: 逆推論＋`double` フォールバック方式)

TypeScript/JavaScript の直感（すべての数値が `number`）を再現しつつ、システムプログラミングにおける整数操作（配列添字・ループカウンタ・ビット演算など）を型注釈なしで自然に記述できるようにするため、**「整数文脈からの逆推論 ＋ `double` フォールバック」** 方式を採用する。

1. **小数点・指数表記を含むリテラル (`3.14`, `42.0`, `1e-5`)**:
   * 常に浮動小数点型として扱われ、文脈指定（`float` 等）がなければ即座に **`double`** として確定する。
2. **整数表記のリテラル (`42`, `0`, `1_000`)**:
   * 出現時点では一時的に **未確定の整数リテラル型（`{integer}`）** として保持される。
   * **ステップ1（文脈および後続利用からの逆推論）**:
     * 直接の文脈（型注釈、関数引数、配列添字、ビット演算等）または、そのリテラルで初期化された変数の**関数内での後続利用箇所**から整数型（`int`, `size_t`, `ulong` 等）が要求された場合、最初からその整数型であったものとして静的に確定する（実行時の型変換オーバーヘッドはゼロ）。
   * **ステップ2（`double` へのフォールバック）**:
     * スコープ終了時点で整数型でなければならない制約が一度も発生しなかった場合、デフォルト型である **`double` (IEEE 754 64bit浮動小数点数)** に確定（フォールバック）する。

```typescript
// 1. Default fallback to double (no integer constraints in scope)
let x = 42;       // Inferred as double (via fallback)
let y = 3.14;     // Inferred as double
let z = x * 2;    // Inferred as double

// 2. Direct contextual typing
let count: int = 10; // OK: 10 is resolved as int directly without 'as'

function sleep(ms: int) { ... }
sleep(100);          // OK: 100 is inferred as int from parameter context

let flag = 1 << 4;   // OK: bitwise shift forces int context -> flag is int

// 3. Reverse inference from subsequent usage in the function
function sum(arr: int[]): int {
    let total = 0; // Initially {integer}, later resolved to int via 'total += arr[i]' and 'return total'
    for (let i = 0; i < arr.length; i += 1) {
        // 'i' is used in 'arr[i]' (requires integer index) and 'i < arr.length'
        // -> 'i' is statically resolved to int from the start!
        total += arr[i];
    }
    return total;
}
```

### 2.2 その他のプリミティブリテラル
| リテラル表記 | 推論される型 | 備考 |
| :--- | :--- | :--- |
| `true`, `false` | `bool` | 真偽値 |
| `'a'`, `'漢'` | `char` | Unicode スカラー値 (UTF-32 / 4バイト) |
| `"Hello, World"` | `string` | フレキシブル内部表現 (ORC管理ヒープ文字列) |

---

## 3. コレクション・複合リテラルの推論 (Collection & Composite Inference)

### 3.1 要素を持つ配列リテラルと共通型規則
* 型注釈のない配列リテラルは、デフォルトで **可変長動的配列 (`T[]`)** として推論される。
* 左辺や引数に固定長配列の型注釈がある場合は、文脈的型付けにより **固定長配列 (`T[N]`)** として推論される。
* 要素がすべて整数表記リテラル（例: `let arr = [1, 2, 3];`）の場合も、2.1 の規則に従い、後続で `int[]` として使われなければ **`double[]`** にフォールバックし、`int[]` を受け取る関数に渡されるなど整数配列の制約があれば **`int[]`** に逆推論される。

#### 異種要素の混在と `Object` 頂点型への暗黙アップキャスト禁止
Kyanos はすべての型が `Object` から派生する統一型階層を持つが、タイプミスによる意図しないボックス化（Heap Boxing）を防ぐため、**型注釈のない配列リテラルにおいて無関係な型を `Object[]` へ暗黙にまとめることは禁止（コンパイルエラー）** とする。

* **推論が許可される共通型**:
  1. すべての要素が同一の型 `T` である場合（$\to$ `T[]`）。
  2. 整数リテラルと浮動小数点リテラルが混在する場合（例: `[1, 2.5, 3]` $\to$ `double[]`）。
  3. 要素が共通のユーザー定義基底クラス（`Object` 以外）を継承する派生クラス同士である場合（例: `[new Dog(), new Cat()]` $\to$ `Animal[]`）。
* **`Object[]` やインターフェース配列を作りたい場合**:
  * 左辺または引数で型を明示する（例: `let items: Object[] = [1, "hello", true];`）。

```typescript
// 1. Dynamic array (Heap / ORC): falls back to double[] if unconstrained
let numbers = [1, 2, 3]; 

// 2. Fixed-length array (Stack / ValueType): inferred as int[3] via expected type
let fixed: int[3] = [1, 2, 3]; 

// 3. Mixed numeric literals: unified to double[]
let mixedNums = [1, 2.5, 3]; // OK: double[]

// 4. Heterogeneous types without annotation -> Compile Error (no implicit Object[] boxing)
// let bad = [1, "hello"]; // Compile Error: Incompatible element types (double and string)
let boxed: Object[] = [1, "hello"]; // OK: explicit Object[] allows auto-boxing
```

### 3.2 空配列リテラル `[]` の推論仕様

要素が存在しない空配列 `[]` は、単体では要素型 `T` を決定できない。Kyanos では段階的に推論能力を拡張する。

#### 第1段階 (現行仕様: パターンB「文脈必須 / Contextual Typing」)
前後の文脈から「期待される型（Expected Type）」が存在する場所でのみ、空配列 `[]` の直接記述を許可する。

```typescript
// 1. Variable declaration with explicit type annotation
let list: int[] = []; // OK: inferred as int[]

// 2. Function return value
function getEmptyList(): string[] {
    return []; // OK: inferred as string[] from return type
}

// 3. Function call argument
function processItems(items: double[]) { ... }
processItems([]); // OK: inferred as double[] from parameter type

// 4. Isolated declaration without context
// let list = []; // Compile Error: Cannot infer element type for empty array. Please provide a type annotation.
```

#### 将来的な拡張 (ロードマップ: パターンC「後続利用による遅延確定 / Flow-based Typing」)
未初期化 `let` 変数や整数リテラルの逆推論と同様に、空配列で初期化した後、**初回の `.push()` や要素代入の型から要素型を遅延推論する拡張** を提供する。

```typescript
// Future Plan (Pattern C):
let list = [];    // Element type initially undetermined (Array<?>)
list.push(10);    // Inferred as double[] upon first push
// list.push("a"); // Compile Error: cannot push string into double[]
```

### 3.3 オブジェクトリテラル
* **識別子キー `{ x: 1, y: 2 }`**: 匿名構造体（値型・スタック確保）として推論。
* **文字列キー `{ "x": 1, "y": 2 }`**: ディクショナリ型（`Map<string, V>`、参照型・ヒープ確保）として推論。

### 3.4 `Option<T>` (`null` / `None`) および `Result<T, E>` (`Ok` / `Err`) の文脈推論
空配列 `[]` と同様、単体では型パラメータの一部が決定できない以下の値・コンストラクタは、**期待される型（変数型注釈、関数の戻り値型、他方の分岐など）の文脈から不足している型パラメータを推論**する。文脈が存在しない単独宣言はコンパイルエラーとなる。

* **`null` / `Option.None`**: 期待される型 `T?`（`Option<T>`）の文脈から `T` を決定する。
* **`Ok(val)`**: 値 `val` から `T` を合成し、エラー型 `E` は文脈（関数の戻り値型 `Result<T, E>` や同一関数内の `return Err(e)`）から決定する。
* **`Err(err)`**: 値 `err` から `E` を合成し、成功型 `T` は文脈から決定する。

```typescript
// 1. Contextual inference for null / None
let name: string? = null; // OK: inferred as Option<string>.None
// let unknown = null;    // Compile Error: Cannot infer type 'T' for Option<T> from bare 'null'

// 2. Contextual inference for Ok / Err from function return type
function readFile(path: string): Result<string, int> {
    if (path == "") {
        return Err(404); // OK: T is inferred as string from return type
    }
    return Ok("data");   // OK: E is inferred as int from return type
}

// 3. Isolated Ok / Err without full type context
// let r = Ok(10);                // Compile Error: Cannot infer error type 'E' for Result<double, E>
let r: Result<double, string> = Ok(10); // OK
```

---

## 4. 変数宣言と遅延初期化推論 (`let` / `const`)

### 4.1 不変変数 (`const`)
`const` は宣言と同時に初期化が必須である。初期化式の型から静的に推論される。宣言のみの記述は構文エラーとなる。

```typescript
const PI = 3.1415926535; // Inferred as double
// const MAX;            // Compile Error: const declarations must be initialized
```

### 4.2 可変変数 (`let`)
`let` は初期化式を伴う宣言のほか、**初期化式なしの宣言（未初期化宣言）** をサポートする。

#### 初回代入による型確定
未初期化の `let` 変数は、**「最初にその変数に代入された値の型」** に従って変数の型が静的かつ恒久的に固定される。以降に異なる型の値を代入することはできない。

```typescript
let x;     // Type is currently unbound
// Some processing...
x = 10;    // The type of x is now fixed (double by fallback, or int if used in integer context)
// x = 'a'; // Compile Error: Type mismatch
```

#### 確定代入解析 (Definite Assignment Analysis)
安全性を保証するため、コンパイラはデータフロー解析を行い、**「値が確実に代入される前に変数を参照・評価すること」** をコンパイルエラーとして排除する。

```typescript
let value;
// println(value); // Compile Error: Variable 'value' is used before being assigned

if (flag) {
    value = 100;
} else {
    value = 200;
}
println(value); // OK: Guaranteed to be initialized in all code paths
```

#### 分岐での型整合性検査
複数の分岐で初回代入が行われる場合、すべての実行パスで代入される型が一致していなければならない。

```typescript
let result;
if (condition) {
    result = 42;
} else {
    // result = "error"; // Compile Error: Conflicting types in branch (double vs string)
    result = 0;          // OK
}
```

---

## 5. 関数・メソッド境界とコールバック/匿名関数の推論

### 5.1 名前付き関数・トップレベル関数・クラス/構造体メソッド
* **引数型**: **明示が必須**。関数やメソッドのシグネチャがそのままインターフェース仕様書となり、単相化（Monomorphization）やモジュール間最適化を効率的に行うためである。
* **戻り値型**: 以下の規則に従い、推論可能な場合は**省略可能**（パブリックAPIでは明示を推奨）。
  1. **`void` の自動推論**: 関数本体に値を返す `return expr` が存在しない場合、自動的に `void` として推論される。
  2. **`return` 式からの合成**: 本体内のすべての `return` 文の型が一致する場合（または `return Ok(v)` と `return Err(e)` の両方が出現し `Result<T, E>` が一意に定まる場合）、その型を戻り値型として推論する。
  3. **再帰関数の制限**: **自己再帰関数および相互再帰関数では、推論の循環を防ぐため戻り値型の明示を必須**とする（省略時はコンパイルエラー）。

```typescript
// 1. Arguments require explicit types, return type is inferred as double
function calculateArea(width: double, height: double) {
    return width * height; 
}

// 2. No return value -> inferred as void
function logMessage(msg: string) {
    println(`[LOG] ${msg}`);
}

// 3. Recursive function -> explicit return type is required
function factorial(n: int): int {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}
```

### 5.2 コールバック・匿名関数（アロー関数）
アロー関数やクロージャは、**文脈（呼び出し側の仮引数型や代入先変数の型）から推論されるため、引数や戻り値の型注釈を完全に省略可能** とする。

```typescript
// Expected callback type: (val: double) => double
let numbers = [1, 2, 3];

// 1. Completely omitted: x is inferred as double, return value is double
let doubled = numbers.map((x) => x * 2);

// 2. Arrow function stored in a typed variable
let predicate: (v: double) => bool = (v) => v > 0; // v is inferred as double
```

---

## 6. ジェネリクスにおける型引数推論 (Generic Type Inference)

ジェネリック関数、ジェネリック構造体（`struct`）のリテラル、およびジェネリッククラス（`class`）のコンストラクタ呼び出しでは、実引数や文脈から型パラメータ `T` が自動的に推論されるため、明示的な型引数 `<T>` の指定を省略できる。

```typescript
// 1. Generic function
function identity<T>(value: T): T {
    return value;
}
let a = identity(10);       // Inferred as identity<double>(10)
let b = identity("hello");  // Inferred as identity<string>("hello")

// 2. Generic struct instantiation
struct Pair<T, U> {
    first: T;
    second: U;
}
// Inferred as Pair<double, string> without writing Pair<double, string> { ... }
let pair = Pair { first: 10, second: "point" }; 

// 3. Generic class constructor instantiation
class Box<T> {
    value: T;
    constructor(value: T) { this.value = value; }
}
let boxed = new Box("Kyanos"); // Inferred as Box<string>
```

---

## 7. 制御フローと型の絞り込み (Flow-Sensitive Type Narrowing)

直和型 (`enum`)、`Option<T>` (`T?`)、`Result<T, E>` に対してパターンマッチングや条件判定を行う際、コンパイラはブロック内での型を安全に絞り込む。

### 7.1 `Result<T, E>` と `Option<T>` のパターンマッチング
```typescript
function parseNumber(s: string): Result<double, string> { ... }

let res = parseNumber("123");

// Pattern matching: Ok unwraps to double, Err unwraps to string
switch (res) {
    case Ok(let val):
        // val is statically inferred as double
        println(`Value: ${val * 2}`);
    case Err(let err):
        // err is statically inferred as string
        println(`Failed: ${err}`);
}
```

### 7.2 Null / None チェックによる絞り込みと可変変数 (`let`) の規則
`if (x != null)` などの条件判定により、ブロック内で `T?`（`Option<T>`）から `T` へと型が絞り込まれる（コンパイラが内部的に安全な自動アンラップを行う）。

可変変数（`let`）に対する型の絞り込みは、メモリ安全性を保証するため **TypeScript 準拠のデータフロー規則** に従う：
1. **再代入による絞り込みリセット**: ブロック内で対象変数に `null` または `T?` 型の値が再代入された時点で、型の絞り込みは解除され元の `T?` に戻る（`T` 型の値を再代入した場合は `T` のまま維持）。
2. **クロージャ境界での無効化**: `let` 変数はいつ書き換えられるか実行タイミングが静的に定まらないため、ブロック内で定義されたクロージャ（匿名関数）の内部には `let` 変数の型絞り込みを引き継がない（クロージャ内でも絞り込みたい場合は `const` 変数に束縛するか、`switch` の `case Some(const v):` を使用する）。

```typescript
let optionalName: string? = getName();

if (optionalName != null) {
    // Inside this block, optionalName is narrowed from string? to string
    println(optionalName.length);

    // If reassigned to null, narrowing is immediately reset back to string?
    optionalName = null;
    // println(optionalName.length); // Compile Error: optionalName is now string? again
}
```

### 7.3 `is` 演算子とユーザー定義型ガード (`expr is Type` / `param is Type`)

統一型階層（`Object`、派生 `class`、`interface`、`Option<T>`）における動的な型判定と静的な型の絞り込み（Type Narrowing）を統合するため、`is` キーワードによる **型検査式** および **型ガード述語（Type Predicate）** を提供する。

1. **型検査式 (`expr is Type`)**:
   * 式 `expr` が指定された型 `Type` のインスタンス（またはインターフェース実装・ボックス化された値型）であるかを判定し、`bool` を返す（JavaScript の `instanceof` と `typeof` を単一構文に統合）。
   * `if` や三項演算子、`while` の条件式内で `x is Type` が真となるブランチでは、変数 `x` の静的な型が自動的に `Type` へ絞り込まれる（可変変数 `let` の再代入リセット・クロージャ境界ルールは 7.2 と同一）。
2. **ユーザー定義型ガード (`function f(x: Base): x is Derived`)**:
   * 関数・メソッドの戻り値型として `引数名 is 型`（Type Predicate）を記述可能（実行時の戻り値 ABI は `bool` と同一でオーバーヘッドゼロ）。
   * その関数が `true` を返した分岐内において、対象引数の型を `Derived` に絞り込む。

```typescript
// 1. Type test expression (`expr is Type`) with automatic flow narrowing
function processEntity(entity: Entity) {
    if (entity is Player) {
        // `entity` is narrowed from `Entity` to `Player`
        println(`Player name: ${entity.playerName}`);
    }
}

// 2. Unwrapping a boxed value from `Object`
let boxed: Object = 42;
if (boxed is int) {
    // `boxed` is narrowed to unboxed `int`
    println(`Doubled: ${boxed * 2}`);
}

// 3. User-defined type guard (`param is Type`)
function isAdmin(user: User): user is AdminUser {
    return user is AdminUser && user.canAccessAdmin();
}

function handleRequest(currentUser: User) {
    if (isAdmin(currentUser)) {
        // `currentUser` is narrowed to `AdminUser` via type predicate
        currentUser.printRole();
    }
}
```
