# Knull Language Specification v2.0.0

Formal description of Knull syntax, semantics, and type system.

---

## 1. Lexical Grammar

### Comments

```
line_comment   ::= "//" .* EOL
block_comment  ::= "/*" .* "*/"
```

### Literals

```
integer  ::= [0-9]+
float    ::= [0-9]+ "." [0-9]+
string   ::= '"' char* '"'
bool     ::= "true" | "false"
null     ::= "null"
```

### Identifiers

```
ident ::= [a-zA-Z_][a-zA-Z0-9_]*
```

### Keywords

Reserved words that cannot be used as identifiers:

```
let   fn    if     else    while   for     in      return
break continue match struct impl   self    pub     use
spawn try    catch  throw   true   false   null

// The following are accepted as aliases and are also reserved:
val   var          -- alias for `let`
func  def  fun     -- alias for `fn`
import  from       -- alias for `use`
mod   module       -- alias for... (note: module directive is NOT supported; do not use)
nil   none         -- alias for `null`
raise except       -- alias for `throw` / `catch`
not   and   or     -- alias for `!` / `&&` / `||`
```

**Note:** `val`, `var`, `func`, `def`, `fun`, `import`, `from`, `nil`, `none`, `raise`, `except` etc. cannot be used as variable names.

### Operators

| Precedence | Operators |
|-----------|-----------|
| 1 (low) | or / || |
| 2 | and / && |
| 3 | == != < <= > >= |
| 4 | + - |
| 5 | * / % |
| 6 | ** (right-assoc) |
| 7 | unary - ! |
| 8 (high) | ( ) . [ ] |

---

## 2. Syntax Grammar

### Program

```
program     ::= statement*
statement   ::= let_stmt | fn_decl | struct_decl | impl_block
              | if_stmt | while_stmt | for_stmt | match_stmt
              | return_stmt | break_stmt | continue_stmt
              | try_stmt | throw_stmt | expr_stmt | block
```

### Declarations

```
let_stmt    ::= "let" ident "=" expr
fn_decl     ::= "fn" ident "(" params? ")" block
params      ::= param ("," param)* ("," ".." ident)?
param       ::= ident ("=" expr)?
struct_decl ::= "struct" ident "{" (ident ","?)* "}"
impl_block  ::= "impl" ident "{" fn_decl* "}"
```

### Control Flow

```
if_stmt    ::= "if" expr block ("else" (if_stmt | block))?
while_stmt ::= "while" expr block
for_stmt   ::= "for" ident "in" expr block
match_stmt ::= "match" expr "{" match_arm* "}"
match_arm  ::= pattern ("if" expr)? "=>" (expr | block) ","?
pattern    ::= "_" | literal | ident | pattern "|" pattern
             | ident "{" (ident (":" pattern)? ","?)* "}"
```

### Expressions

```
expr ::= literal | ident | "(" expr ")"
       | expr op expr | unary_op expr
       | expr "(" args? ")"       -- call
       | expr "." ident           -- field access
       | expr "[" expr "]"        -- index
       | "fn" "(" params? ")" block     -- closure
       | "if" expr block "else" block   -- if expression
       | "spawn" block              -- spawn thread
       | "[" (expr ("," expr)*)? "]"   -- array literal
       | "{" (str ":" expr ("," str ":" expr)*)? "}"  -- map literal
       | ident "{" (ident ":" expr ("," ident ":" expr)*)? "}"  -- struct literal
```

---

## 3. Type System

Knull is dynamically typed:

| Type | Description |
|------|-------------|
| number | i64 or f64 (unified) |
| string | UTF-8 |
| bool | true / false |
| array | ordered sequence |
| map | string-keyed dictionary |
| null | absence |
| fn | first-class function or closure |
| struct | named struct instance |

No implicit coercion. Use `to_string()`, `to_int()`, `to_float()` explicitly.

---

## 4. Scoping

- Lexical scoping; closures close over definition environment
- `let` in block → block-scoped
- Top-level `fn` are hoisted (collected before execution)
- Top-level `let` are not hoisted

---

## 5. Execution Model

1. Lex → tokens
2. Parse → AST
3. Collect top-level fn declarations
4. Execute top-level statements in order

**Implicit return:** In named functions (`fn foo() { ... }`), the last expression in the body is the implicit return value if no `return` statement is reached.

**Lambda return:** Anonymous functions (`fn(x) { x * 2 }`) do NOT use implicit return — the last expression is silently discarded. Always use explicit `return` in lambdas:
```knull
let double = fn(x) { return x * 2 }   // correct
let double = fn(x) { x * 2 }          // returns null!
```

Truthiness: `false` and `null` are falsy; everything else (including `0`, `""`, `[]`) is truthy. Use explicit comparison when needed: `x == 0`, `len(arr) == 0`.

---

## 6. Error Handling

- `throw value` — raise exception
- `try { } catch e { }` — catch any thrown value, bind to `e`
- Uncaught exceptions terminate the program

---

## 7. Concurrency

- `spawn { block }` → JoinHandle (new OS thread)
- `handle.join()` → blocks until thread completes, returns last value from thread

**Channels** (synchronous rendezvous):
```knull
let ch = chan_create()      // returns a map: { "id": N }
let id = ch["id"]           // extract the channel id
chan_send(id, value)         // blocking send
let v = chan_recv(id)        // blocking receive
```

**Important limitations:**
- Closures passed to `spawn` capture values by copy at creation time, not by reference. Mutations inside the spawned thread are not visible to the parent thread.
- Cross-thread channel communication works correctly only when the channel id is passed as a direct value argument (not via closure capture).
- For simple parallel workloads without shared state, `spawn`/`join` works reliably.

---

## 8. Pattern Matching

Arms tested top-to-bottom; first match wins.

| Pattern | Matches when |
|---------|-------------|
| `_` | always (wildcard) |
| literal | value equals literal |
| `ident` | always; binds to `ident` |
| `p1 \| p2` | p1 or p2 matches |
| `S { f }` | struct S with field; binds field |

Guard: `pat if cond =>` — cond evaluated after pattern match.
