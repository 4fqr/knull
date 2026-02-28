# Knull Language Guide

The comprehensive guide to programming in Knull.

---

## Introduction

Knull is a versatile programming language designed to scale with you from beginner scripting to bare-metal systems programming.

---

## Quick Start

```knull
fn main() {
    println "Hello, World!"
}
```

Run it:
```bash
knull run hello.knull
```

---

## Basic Syntax

### Variables

```knull
let x = 42           // Integer
let name = "Knull"   // String
let pi = 3.14        // Float
```

### Print

```knull
println "Hello, World!"           // With newline
print "No newline"                // Without newline
println "Value: " + x             // Concatenation
```

### Math

```knull
let a = 10
let b = 3

println a + b    // 13
println a - b    // 7
println a * b    // 30
println a / b    // 3
println a % b    // 1
```

### Comparison

```knull
println a == b    // false
println a != b    // true
println a > b     // true
println a < b     // false
println a >= b    // true
println a <= b    // true
```

### Control Flow

#### If/Else

```knull
let x = 5

if x > 3 {
    println "x is greater than 3"
} else {
    println "x is not greater than 3"
}
```

#### While Loop

```knull
let i = 1
while i <= 5 {
    println i
    i = i + 1
}
```

### Functions

```knull
fn add(a, b) {
    a + b
}

fn main() {
    let result = add(5, 3)
    println result    // 8
}
```

---

## Working Features

| Feature | Status | Example |
|---------|--------|---------|
| Variables | ✅ | `let x = 5` |
| Print | ✅ | `println "Hello"` |
| String concat | ✅ | `"a" + "b"` |
| Int + String | ✅ | `"x=" + 5` |
| Math ops | ✅ | `+ - * / %` |
| Comparison | ✅ | `== != < > <= >=` |
| While loops | ✅ | `while x < 5 { ... }` |
| If/else | ✅ | `if x > 0 { ... }` |
| Functions | ✅ | `fn main() { ... }` |

---

## Project Structure

```
knull-lang/
├── src/              # Bootstrap compiler (Rust)
│   ├── main.rs       # CLI entry
│   ├── lexer.rs      # Tokenizer
│   ├── parser.rs     # AST builder
│   └── compiler.rs   # Interpreter
├── self_hosted/      # Self-hosted compiler (Knull)
│   └── knullc.knull  # Entry point
├── examples/         # Example programs
│   ├── hello_world.knull
│   └── test_all.knull
├── docs/             # Documentation
└── README.md         # Main docs
```

---

## Running Examples

```bash
# Hello World
knull run examples/hello_world.knull

# Comprehensive test
knull run examples/test_all.knull

# Check syntax without running
knull check file.knull
```

---

For more details, see [STD_LIB.md](STD_LIB.md) and [SPECIFICATION.md](SPECIFICATION.md)
