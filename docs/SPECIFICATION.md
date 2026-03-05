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

```
let  fn  if  else  while  for  in  return  break  continue
match  struct  impl  self  spawn  channel  try  catch  throw
true  false  null
```

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

Last expression in a block/function is the implicit return value.

Truthiness: `false` and `null` are falsy; everything else (including 0, "", []) is truthy.

---

## 6. Error Handling

- `throw value` — raise exception
- `try { } catch e { }` — catch any thrown value, bind to `e`
- Uncaught exceptions terminate the program

---

## 7. Concurrency

- `spawn { block }` → JoinHandle (new OS thread)
- `handle.join()` → blocks, returns last value from thread
- `channel()` → synchronous channel (rendezvous)
- `ch.send(v)` / `ch.recv()` — blocking send/receive

Values sent over channels are deep-copied. No shared mutable state.

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
