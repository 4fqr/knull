# Knull Implementation Notes

Architecture overview for contributors and language implementors.

---

## Architecture

```
Source (.knull)
     │
     ▼
┌─────────┐
│  Lexer  │  src/lexer.rs      — tokenises source into Token stream
└────┬────┘
     ▼
┌─────────┐
│ Parser  │  src/parser.rs     — recursive-descent → ASTNode tree
└────┬────┘
     ▼
┌─────────────┐
│ Interpreter │  src/interpreter.rs  — tree-walking execution
└────┬────────┘
     ├── C Codegen  src/c_codegen.rs   → native binary via GCC/Clang
     ├── WASM       src/wasm_codegen.rs → .wasm
     └── LLVM       src/llvm_codegen.rs → native (opt.)
```

---

## Key Components

### Lexer (`src/lexer.rs`)

Single-pass tokeniser. Produces a flat `Vec<Token>`. Handles:
- Keywords, identifiers, literals (int, float, string, bool)
- Operators, delimiters
- Comments (`//`, `/* */`)
- String interpolation markers

### Parser (`src/parser.rs`)

Recursive-descent parser producing `ASTNode` variants:

- `Program(Vec<ASTNode>)` — top-level items
- `Function { name, params, body }` — function definition
- `Let { name, value }` — variable binding
- `If { cond, then_body, else_body }` — conditional
- `While { cond, body }` — loop
- `Match { expr, arms }` — pattern matching
- `StructDef / Impl` — type definitions
- `Call { func, args }` — function/method call
- `TryCatch / Throw` — error handling
- `Block(Vec<ASTNode>)` — statement block

### Interpreter (`src/interpreter.rs`)

Tree-walking interpreter (~18,000 lines). Key design:

- **Scoped environments**: `Vec<Scope>` (scope stack). `push_scope()` / `pop_scope()` per function call and block.
- **Function table**: `HashMap<String, FunctionDef>` — collected in a first pass over top-level items.
- **`execute(node)`**: Statement execution — drives side effects.
- **`evaluate(node)`**: Expression evaluation — returns `Value`.
- **`call_body(body)`**: Calls a function body — last expression is return value.
- **Built-in dispatch**: `call_builtin(name, args)` handles hundreds of built-in functions.

### Value System

```rust
enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Null,
    Closure { params, body, env },
    StructDef(Box<StructDef>),
    StructInstance { name, fields },
    // + resource handles (TCP, file, db, etc.)
}
```

### Pattern Matching

`Pattern` variants: `Wildcard`, `Literal`, `Identifier`, `Or(Vec<Pattern>)`, `Struct`, `Enum`.

OR-patterns (`1 | 2 | 3`) parsed in `parse_match()`, evaluated via `Pattern::Or` in `pattern_matches()`.

Guards: `arm.guard` is an optional `ASTNode` evaluated after pattern match.

---

## Control Flow Flags

The interpreter uses three flags on `Interpreter`:

| Flag | Purpose |
|------|---------|
| `return_value: Option<Value>` | Early return from function |
| `break_flag: bool` | Break out of loop |
| `continue_flag: bool` | Skip to next iteration |

These are checked in loop bodies and `call_body` after each statement.

---

## Adding a Built-in Function

1. In `src/interpreter.rs`, locate `call_builtin(name, args)` (around line 2900+)
2. Add a new arm:
   ```rust
   "my_func" => {
       let arg = args.into_iter().next().ok_or("my_func requires 1 arg")?;
       Some(Ok(/* compute result */))
   }
   ```
3. Add a test in `tests/` or `examples/`

---

## Adding a Keyword / Syntax

1. **Lexer** (`lexer.rs`): Add `TokenKind::MyKeyword` and match the string in `tokenize()`
2. **Parser** (`parser.rs`): Add parsing in `parse()` dispatch; create a new `ASTNode` variant in `ast.rs`
3. **Interpreter** (`interpreter.rs`): Handle the new node in `execute_node()` and/or `evaluate()`

---

## Test Suite

```bash
bash test_all.sh          # runs tests/ directory
```

33 integration tests covering: variables, arithmetic, strings, booleans, comparisons, while loops, conditionals, functions, recursion, closures, patterns, structs.

---

## Known Limitations

- No module system yet (single-file programs; stdlib is built-in)
- LLVM backend is optional and requires separate installation
- Parser error messages don't always include line/column numbers
- No type inference in expert mode yet (annotations optional but not enforced)
