# Knull Programming Language

```
  _  __              _ _
 | |/ /_ __  _   _| | |
 | \'| \'_ \\| | | | | |
 | . \\| | | | |_| | | |
 |_|\_\\_| |_|\\__,_|_|_|
```

**The God Programming Language** — v2.1.0

Simple as Python. Fast as C. Powerful as Assembly.

[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![GitHub](https://img.shields.io/badge/GitHub-4fqr%2Fknull-green)](https://github.com/4fqr/knull)

---

## Overview

Knull is a unified systems programming language designed to span the full spectrum of software development — from beginner scripting to bare-metal systems programming. Three integrated language modes adapt to your expertise level instead of requiring you to learn different languages.

### Key Features

- **Three language modes**: Novice · Expert · God
- **First-class functions, closures, recursion**
- **Pattern matching** with guards and OR-patterns (`1 | 2 | 3 =>`)
- **Structs and impl blocks** with self-methods
- **Try/Catch/Throw** error handling
- **Higher-order functions**: `map`, `filter`, `reduce`, `sorted_by`, `enumerate`, `zip`
- **Rich string API**: `len`, `upper`, `lower`, `split`, `contains`, `replace`, `trim`, …
- **File I/O**, **Networking (TCP/UDP/HTTP/WebSocket)**, **Threading**
- **Real-time GUI** with drawing primitives (`gui_rect`, `gui_line`, `gui_circle`, `gui_fill`, …)
- **SQLite database** built-in via rusqlite (`db_open`, `db_exec`, `db_query`, …)
- **Crypto builtins**: SHA-256, MD5, base64, random bytes, token generation
- **Working packages**: `json`, `http`, `crypto`, `sqlite` — importable with `import "packages/…"`
- **Snake & Pong** game examples (real-time minifb rendering)
- **C FFI**, **inline assembly**, **syscalls**
- **WebAssembly** output target
- **Interactive REPL** with persistent state, multi-line input, introspection commands
- **Interpreter + C codegen backend**

---

## Quick Start

### Hello World

```knull
fn main() {
    println("Hello, Knull!")
}

main()
```

```bash
knull run hello.knull
```

### Variables

```knull
let x      = 42
let pi     = 3.14159
let name   = "Knull"
let flag   = true
let items  = [1, 2, 3, 4, 5]
let coords = {"x": 10, "y": 20}

println("x = " + x + ", name = " + name)
```

### Functions & Recursion

```knull
fn factorial(n) {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

fn fib(n) {
    if n <= 1 { n } else { fib(n-1) + fib(n-2) }
}

println(factorial(10))  // 3628800
println(fib(15))        // 610
```

### Higher-Order Functions

```knull
let nums    = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
let evens   = nums.filter(fn(n) { return n % 2 == 0 })
let doubled = evens.map(fn(n) { return n * 2 })
let total   = doubled.reduce(fn(acc, n) { return acc + n }, 0)

println(total)  // 60
```

### Structs & Methods

```knull
struct Point { x, y }

impl Point {
    fn distance(self, other) {
        let dx = self.x - other.x
        let dy = self.y - other.y
        sqrt(dx*dx + dy*dy)
    }
}

let a = Point { x: 0, y: 0 }
let b = Point { x: 3, y: 4 }
println(a.distance(b))  // 5
```

### Pattern Matching

```knull
fn describe(n) {
    match n {
        0         => "zero"
        1 | 2     => "one or two"
        n if n < 0 => "negative"
        _         => "other"
    }
}

println(describe(0))   // zero
println(describe(2))   // one or two
```

### Try / Catch / Throw

```knull
fn safe_div(a, b) {
    if b == 0 { throw "division by zero" }
    return a / b
}

try {
    println(safe_div(10, 0))
} catch err {
    println("Caught: " + err)
}
```

### Closures & Currying

```knull
fn make_adder(n) { return fn(x) { return x + n } }

let add5  = make_adder(5)
let add10 = make_adder(10)
println(add5(3))    // 8
println(add10(3))   // 13
```

---

## The Three Modes

| Mode | Memory | Use Case |
|------|--------|----------|
| **Novice** (default) | Garbage collected | Scripting, learning, prototyping |
| **Expert** | RAII + ownership | High-performance applications |
| **God** | Manual + syscalls | OS kernels, embedded systems |

```knull
// Novice (default — no declaration needed)
let x = 42
println(x)

// Expert — static types
mode expert
fn compute(x: i32) -> i32 { x * 2 }

// God — unsafe hardware access + asm
mode god
unsafe { let ptr = 0x1000 as *mut u8; *ptr = 0xFF }
asm volatile("nop")
syscall(60, 0)
```

---

## Installation

```bash
# Clone and build
git clone https://github.com/4fqr/knull.git
cd knull/src
cargo build --release --no-default-features

# Add to PATH
export PATH="$PWD/target/release:$PATH"

# Quick install script
curl -sSL https://raw.githubusercontent.com/4fqr/knull/master/install.sh | bash
```

---

## CLI Reference

| Command | Description |
|---------|-------------|
| `knull run <file>` | Execute a .knull file |
| `knull eval <expr>` | Evaluate an inline snippet |
| `knull repl` | Interactive REPL session |
| `knull build <file>` | Compile to native binary |
| `knull check <file>` | Syntax/type check |
| `knull fmt <file>` | Format source file |
| `knull new <name>` | Scaffold new project |
| `knull add <pkg>` | Add a package dependency |
| `knull test` | Run test suite |
| `knull version` | Show version info |

```bash
knull run hello.knull
knull eval 'println(2 + 2)'
knull repl
knull build --release main.knull -o myapp
knull build --target wasm32 app.knull
```

---

## REPL

```
$ knull repl

  Knull v2.1.0  —  The God Programming Language

knull❯ let x = 42
knull❯ x * 2
84 // number
knull❯ fn square(n) { n * n }
knull❯ square(7)
49 // number
knull❯ :vars
Variables:
  x = 42
```

**REPL Commands**: `:help` · `:vars` · `:fns` · `:reset` · `:load <file>` · `:type <expr>` · `:clear` · `:quit`

Multi-line: enter a `{` and keep typing — the `...` prompt waits for you to close the block.

---

## Standard Library (Selected)

```knull
// Math
abs(n)  sqrt(n)  pow(a,b)  floor(n)  ceil(n)
min(a,b)  max(a,b)  sin(x)  cos(x)  tan(x)  log(x)

// Arrays
len(arr)  push(arr, x)  pop(arr)  reverse(arr)
sort(arr)  sum(arr)  map(arr, fn)  filter(arr, fn)
reduce(arr, fn, init)  sorted_by(arr, fn)
zip(a, b)  flatten(arr)  join(arr, sep)  enumerate(arr)

// Strings
len(s)  upper(s)  lower(s)  trim(s)
split(s, sep)  contains(s, sub)  replace(s, old, new)
starts_with(s, p)  ends_with(s, suf)

// Conversion
str(x)  to_int(s)  to_float(s)  typeof(x)

// File I/O
read_file(p)  write_file(p, s)  append_file(p, s)
file_exists(p)  file_remove(p)  dir_list(p)  mkdir(p)

// Network / HTTP
http_get(url)  http_post(url, body, content_type)
http_put(url, body, content_type)  http_delete(url)
tcp_connect(host, port)

// Database (SQLite — built-in)
db_open(path)  db_open_memory()
db_exec(h, sql)  db_query(h, sql)  db_query_one(h, sql)
db_last_insert_id(h)

// GUI / Graphics (minifb framebuffer)
gui_window(title, w, h)  gui_fill(h, rgb)  gui_present(h)
gui_set_pixel(h, x, y, rgb)  gui_rect(h, x, y, w, h, rgb)
gui_rect_outline(h, x, y, w, h, rgb)
gui_line(h, x0, y0, x1, y1, rgb)
gui_circle(h, cx, cy, r, rgb)  gui_circle_outline(h, cx, cy, r, rgb)
gui_rgb(r, g, b)  gui_size(h)  gui_set_title(h, t)
gui_is_open(h)  gui_get_keys(h)  gui_get_mouse(h)
gui_close(h)

// Crypto
sha256(s)  md5(s)  base64_encode(s)  base64_decode(s)  random_bytes(n)

// JSON
json_parse(s)  json_stringify(v)

// System
time()  time_millis()  sleep(ms)  exec(cmd)
env_get(k)  env_set(k, value)
rand()  exit(code)
```

### Packages

Import packages with `import "packages/<name>/src/lib.knull"`:

| Package | Import | Key Functions |
|---------|--------|---------------|
| JSON helpers | `import "packages/json/src/lib.knull"` | `parse`, `stringify`, `merge`, `has`, `json_keys` |
| HTTP helpers | `import "packages/http/src/lib.knull"` | `get`, `post`, `get_json`, `post_json`, `build_query` |
| Crypto helpers | `import "packages/crypto/src/lib.knull"` | `hash_password`, `verify_password`, `token_generate`, `hmac_simple` |
| SQLite ORM | `import "packages/sqlite/src/lib.knull"` | `connect`, `create_table`, `db_insert`, `select_all`, `select_where`, `db_count` |

---

## Project Structure

```
my-project/
├── knull.toml          # Manifest
├── src/
│   └── main.knull      # Entry point
├── tests/
│   └── *.knull         # Tests
└── packages/           # Dependencies
```

---

## License

Dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE).

---

## Links

- **Repository**: https://github.com/4fqr/knull
- **Issues**: https://github.com/4fqr/knull/issues
- **Docs**: [docs/](docs/)
- **Examples**: [examples/](examples/)
