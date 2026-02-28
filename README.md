# Knull Programming Language

```
.____/\ .______  .____     .___    .___    
:   /  \:      \ |    |___ |   |   |   |   
|.  ___/|       ||    |   ||   |   |   |   
|     \ |   |   ||    :   ||   |/\ |   |/\ 
|      \|___|   ||        ||   /  \|   /  \
|___\  /    |___||. _____/ |______/|______/
     \/           :/                       
                  :                        
                                           
```

**The God Programming Language**

Simple as Python. Fast as C. Powerful as Assembly.

---

## Overview

Knull is a unified programming language designed to span the entire spectrum of software development, from beginner scripting to bare-metal systems programming. It eliminates the need to learn multiple languages by providing three integrated modes that adapt to your expertise level.

---

## The Three Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **Novice** | Dynamic typing, automatic memory management | Beginners, prototyping, scripting |
| **Expert** | Static typing, ownership system, compile-time checks | Systems programming, high-performance applications |
| **God** | Unsafe blocks, direct hardware access, inline assembly | Operating systems, embedded systems, security research |

---

## Installation

### Quick Install (Linux / macOS)

```bash
curl -sSL https://raw.githubusercontent.com/4fqr/knull/master/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"
```

### Build from Source

```bash
git clone https://github.com/4fqr/knull.git
cd knull/src
cargo build --release --no-default-features
./target/release/knull --version
```

### Windows

```powershell
git clone https://github.com/4fqr/knull.git
cd knull\src
cargo build --release --no-default-features
```

---

## Quick Start

### Hello World

```knull
fn main() {
    println "Hello, World!"
}
```

### Variables and Types

```knull
fn main() {
    let integer = 42
    let floating = 3.14159
    let text = "Knull"
    let flag = true
    
    println "Integer: " + integer
    println "Float: " + floating
    println "String: " + text
}
```

### Functions

```knull
fn add(a, b) {
    return a + b
}

fn main() {
    let result = add(5, 3)
    println "5 + 3 = " + result
}
```

### Control Flow

```knull
fn main() {
    let x = 10
    
    if x > 5 {
        println "x is greater than 5"
    } else {
        println "x is not greater than 5"
    }
    
    let i = 0
    while i < 5 {
        println "Iteration: " + i
        i = i + 1
    }
}
```

---

## Command Line Interface

```bash
knull run <file>        # Execute a Knull file
knull build <file>      # Compile to binary
knull check <file>      # Syntax validation
knull fmt <file>        # Format code
knull repl              # Interactive shell
knull new <name>        # Create new project
```

---

## Language Features

### Supported Constructs

- **Variables**: `let x = value`
- **Functions**: `fn name(params) { ... }`
- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Comparison**: `==`, `!=`, `<`, `>`, `<=`, `>=`
- **Logical**: `&&`, `||`, `!`
- **Control Flow**: `if`/`else`, `while`, `for`
- **Arrays**: `[1, 2, 3]` with indexing `arr[i]`
- **Strings**: Concatenation with `+` operator
- **Comments**: Single-line with `//`

### Operator Precedence

1. Parentheses `()`
2. Unary operators `!`, `-`
3. Multiplicative `*`, `/`, `%`
4. Additive `+`, `-`
5. Comparison `==`, `!=`, `<`, `>`, `<=`, `>=`
6. Logical `&&`, `||`
7. Assignment `=`

---

## Examples

See the `examples/` directory for comprehensive demonstrations:

- `showcase.knull` - Complete language feature overview
- `fibonacci.knull` - Mathematical sequence generation
- `primes.knull` - Prime number sieve
- `calculator.knull` - Mathematical operations
- `patterns.knull` - Pattern printing with loops
- `sorting.knull` - Bubble sort visualization
- `benchmark.knull` - Performance testing

---

## Documentation

- [Build Instructions](BUILD.md) - Compiling from source
- [Language Guide](docs/GUIDE.md) - Tutorial and concepts
- [Standard Library](docs/STD_LIB.md) - API reference
- [Specification](docs/SPECIFICATION.md) - Complete language specification

---

## Project Structure

```
knull/
├── src/                    # Bootstrap compiler (Rust)
│   ├── main.rs            # CLI entry point
│   ├── lexer.rs           # Tokenizer
│   ├── parser.rs          # AST builder
│   ├── compiler.rs        # Interpreter
│   └── cli.rs             # Command implementations
├── examples/              # Example programs
├── runtime/               # Standard library (Knull)
├── self_hosted/           # Self-hosted compiler
├── docs/                  # Documentation
└── tests/                 # Test suite
```

---

## Current Status

| Component | Status |
|-----------|--------|
| Lexer | Complete |
| Parser | Complete |
| Interpreter | Complete |
| Standard Library | Partial |
| Self-Hosted Compiler | In Development |
| Package Manager | In Development |

---

## Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

MIT License - See LICENSE file for details.

---

**Knull** - From hello world to operating system, one language.
