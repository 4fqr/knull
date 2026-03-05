# Knull Usage Guide

Complete reference for the Knull CLI, REPL, and project workflow.

---

## Table of Contents

1. [Installation](#installation)
2. [CLI Commands](#cli-commands)
3. [REPL](#repl)
4. [Writing Programs](#writing-programs)
5. [Project Management](#project-management)
6. [Editor Support](#editor-support)
7. [Troubleshooting](#troubleshooting)

---

## Installation

### Build from Source

```bash
git clone https://github.com/4fqr/knull.git
cd knull/src
cargo build --release --no-default-features
export PATH="$PWD/target/release:$PATH"
```

### Quick Install Script

```bash
curl -sSL https://raw.githubusercontent.com/4fqr/knull/master/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"
```

### Verify

```bash
knull version
# Knull v2.0.0 — The God Programming Language
```

---

## CLI Commands

### `run` — Execute a file

```bash
knull run hello.knull
knull run -v src/main.knull        # verbose output
```

### `eval` — Inline evaluation

```bash
knull eval 'println(42 * 7)'
knull eval 'let x = [1,2,3]; println(x.map(fn(n){n*n}))'
```

### `repl` — Interactive session

```bash
knull repl
```

### `build` — Compile to binary

```bash
knull build main.knull                    # debug build
knull build main.knull -o myapp           # custom output name
knull build --release main.knull          # optimised
knull build --target wasm32 app.knull     # WebAssembly
```

### `check` — Syntax checking

```bash
knull check src/main.knull
```

### `fmt` — Format source

```bash
knull fmt src/main.knull
```

### `new` — Scaffold project

```bash
knull new my-project
cd my-project
knull run src/main.knull
```

### `add` / `test`

```bash
knull add json ^1.0         # add dependency
knull test                  # run test suite
```

### Global flag

```bash
knull -v run file.knull     # verbose everywhere
```

---

## REPL

Start with `knull repl`. State (variables, functions) persists between lines.

### Session example

```
knull❯ let x = 100
knull❯ let y = 200
knull❯ x + y
300 // number
knull❯ fn double(n) { n * 2 }
knull❯ double(x)
200 // number
knull❯ :vars
Variables:
  x = 100
  y = 200
knull❯ :fns
Functions:
  double
knull❯ :quit
Goodbye! 👋
```

### Commands

| Command | Description |
|---------|-------------|
| `:help` / `:h` | Show help |
| `:quit` / `:q` | Exit |
| `:vars` | List all variables |
| `:fns` | List all defined functions |
| `:reset` | Clear all state |
| `:load <file>` | Execute a file into this session |
| `:type <expr>` | Show type of expression |
| `:clear` | Clear screen |

### Multi-line blocks

```
knull❯ fn fib(n) {
  ...   if n <= 1 { n }
  ...   else { fib(n-1) + fib(n-2) }
  ... }
knull❯ fib(10)
55 // number
```

---

## Writing Programs

### Minimal program

```knull
println("Hello, Knull!")
```

No `main()` required — top-level code executes in order.

### With main function

```knull
fn main() {
    println("Hello!")
}

main()
```

### Functions must be defined before use (or at top level)

```knull
// Functions defined anywhere at top level are hoisted
fn greet(name) {
    "Hello, " + name + "!"
}

println(greet("World"))
```

### Implicit return

```knull
fn square(n) { n * n }          // last expression is returned
fn abs(n) { if n < 0 { -n } else { n } }
```

### Closures

```knull
let mul = fn(a, b) { a * b }
let double = fn(x) { x * 2 }
println(mul(3, 4))    // 12
println(double(7))    // 14
```

---

## Project Management

### Project structure

```
my-project/
├── knull.toml
├── src/
│   └── main.knull
├── tests/
│   └── tests.knull
└── packages/
```

### knull.toml

```toml
[package]
name    = "my-project"
version = "0.1.0"
author  = "Your Name"

[dependencies]
json    = "^1.0"
```

### Adding dependencies

```bash
knull add json ^1.0
knull add crypto ^2.0
```

### Running tests

```bash
knull test                # runs all *.knull in tests/
```

---

## Editor Support

### VS Code

Install the bundled extension from `editor/`:

```bash
cd editor && npm install && npm run compile
code --install-extension knull-lang-*.vsix
```

Or add `.knull` → language association manually.

Features provided:
- Syntax highlighting
- Bracket matching / auto-close
- Line & block comments (`//`, `/* */`)
- Snippets: `fn`, `let`, `if`, `while`, `match`, `struct`, `impl`

### Other editors

TextMate grammar at `editor/syntaxes/knull.tmLanguage.json` works with any editor that supports TextMate grammars (Sublime Text, Atom, Nova, etc.).

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `knull: command not found` | Add `target/release` to `$PATH` |
| `Runtime error: Undefined variable` | Check spelling / scope |
| `Parse error: Expected ...` | Check syntax — use `knull check` |
| Build fails (LLVM) | Build with `--no-default-features` |
| Stack overflow on recursion | Add a base case; reduce recursion depth |
| `spawn` thread errors | Ensure block syntax: `spawn { ... }` |

### Verbose run

```bash
knull -v run file.knull
```

### Check syntax only

```bash
knull check file.knull
```
