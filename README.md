# Knull Programming Language

```
.____/\ .______ .____     .___   .___
:   /  \:      \|    |___ |   |  |   |
|.  ___/|       ||    |   ||   |  |   |
|     \ |   |   ||    :   ||   |/\|   |/\
|      \|___|   ||        ||   /  \|   /  \
|___\  /    |___||. _____/ |______/|______/
     \/           :/                         
                  :                          
```

**The God Programming Language**

Simple as Python. Fast as C. Powerful as Assembly.

---

## Overview

Knull is a unified systems programming language designed to span the entire spectrum of software development, from beginner scripting to bare-metal systems programming. It eliminates the need to learn multiple languages by providing three integrated modes that adapt to your expertise level.

### Philosophy

1. **Unified Learning Curve**: Start simple, grow into complexity without switching languages
2. **Zero-Cost Abstractions**: What you don't use, you don't pay for
3. **Safety by Default**: Memory-safe unless explicitly unsafe
4. **Systems-First**: Designed for operating systems, drivers, and embedded systems
5. **C Interoperability**: Seamless FFI with C and assembly

---

## The Three Modes

| Mode | Description | Memory | Use Case |
|------|-------------|--------|----------|
| **Novice** | Dynamic typing, automatic memory management (GC) | Garbage collected | Beginners, prototyping, scripting |
| **Expert** | Static typing, ownership system, compile-time checks | RAII + borrow checker | Systems programming, high-performance applications |
| **God** | Unsafe blocks, direct hardware access, inline assembly | Manual + syscalls | Operating systems, embedded systems, security research |

### Mode Declaration

```knull
// Novice mode - implicit or explicit
mode novice
fn main() {
    let x = 42  // Type inferred, GC managed
    println(x)
}

// Expert mode - ownership tracking
mode expert
fn main() {
    let s = String::from("hello")  // Owned value
    let r = &s                      // Borrow
    println(r)
} // s dropped here automatically

// God mode - unsafe operations allowed
mode god
fn main() {
    unsafe {
        let ptr = 0x1000 as *mut u8
        *ptr = 0xFF  // Direct memory access
    }
    
    // Inline assembly
    asm volatile("mov $1, %eax")
    
    // Direct syscall
    syscall(60, 0)  // exit(0)
}
```

---

## Installation

### Prerequisites

- **Rust** (1.70+) for building the bootstrap compiler
- **LLVM** (optional, for optimized builds)
- **NASM** (for assembly output)
- **ld** or **lld** (for linking)

### Quick Install (Linux / macOS)

```bash
curl -sSL https://raw.githubusercontent.com/knull-lang/knull/main/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/knull-lang/knull.git
cd knull

# Build without LLVM (faster, no external deps)
cd src
cargo build --release --no-default-features

# Build with LLVM (optimized, requires LLVM installed)
cargo build --release

# Verify installation
./target/release/knull --version
```

### Windows

```powershell
git clone https://github.com/knull-lang/knull.git
cd knull\src
cargo build --release --no-default-features
```

---

## Quick Start

### Hello World

```knull
fn main() {
    println("Hello, World!")
}
```

```bash
knull run hello.knull
```

### Variables and Types

```knull
fn main() {
    // Type inference
    let integer = 42
    let floating = 3.14159
    let text = "Knull"
    let flag = true
    
    // Explicit types
    let x: i32 = 42
    let y: f64 = 3.14
    let name: String = "Knull"
    
    println("Integer: {}", integer)
    println("Float: {}", floating)
    println("String: {}", text)
}
```

### Functions

```knull
// Function with parameters and return type
fn add(a: i32, b: i32) -> i32 {
    return a + b
}

// Shorthand return (implicit)
fn multiply(a: i32, b: i32) -> i32 { a * b }

// Generic function
fn identity<T>(x: T) -> T { x }

// Function pointers
fn apply(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

fn main() {
    let result = add(5, 3)
    println("5 + 3 = {}", result)
    
    let doubled = apply(|x| x * 2, 21)
    println("Doubled: {}", doubled)
}
```

### Control Flow

```knull
fn main() {
    let x = 10
    
    // If/else
    if x > 5 {
        println("x is greater than 5")
    } else if x > 0 {
        println("x is positive but <= 5")
    } else {
        println("x is not positive")
    }
    
    // While loop
    let mut i = 0
    while i < 5 {
        println("Iteration: {}", i)
        i = i + 1
    }
    
    // For loop (iterator-based)
    for i in 0..5 {
        println("For iteration: {}", i)
    }
    
    // For each
    let items = [1, 2, 3, 4, 5]
    for item in items {
        println("Item: {}", item)
    }
    
    // Match expression
    let value = 3
    match value {
        1 => println("One"),
        2 => println("Two"),
        3 => println("Three"),
        _ => println("Other"),
    }
}
```

### Ownership and Borrowing (Expert/God Mode)

```knull
mode expert

fn main() {
    // Ownership
    let s1 = String::from("hello")
    let s2 = s1  // s1 moved to s2
    // println("{}", s1)  // ERROR: s1 no longer valid
    println("{}", s2)     // OK
    
    // Borrowing
    let s3 = String::from("world")
    let len = calculate_length(&s3)  // Borrow
    println("'{}' has length {}", s3, len)  // s3 still valid
    
    // Mutable borrow
    let mut s4 = String::from("hello")
    change(&mut s4)
    println("Changed: {}", s4)
}

fn calculate_length(s: &String) -> usize {
    s.len()
}

fn change(s: &mut String) {
    s.push_str(", world")
}
```

### Structs and Enums

```knull
// Struct definition
struct Point {
    x: f64,
    y: f64,
}

// Struct with methods
impl Point {
    // Constructor
    fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    
    // Method
    fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    
    // Mutable method
    fn translate(&mut self, dx: f64, dy: f64) {
        self.x = self.x + dx
        self.y = self.y + dy
    }
}

// Enum
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(i32, i32, i32),
}

impl Message {
    fn call(&self) {
        match self {
            Message::Quit => println("Quit"),
            Message::Move { x, y } => println("Move to ({}, {})", x, y),
            Message::Write(text) => println("Write: {}", text),
            Message::ChangeColor(r, g, b) => println("Color: {},{},{}", r, g, b),
        }
    }
}
```

### Unsafe Code and Syscalls (God Mode)

```knull
mode god

fn main() {
    unsafe {
        // Raw pointer manipulation
        let mut num = 5
        let r1 = &num as *const i32
        let r2 = &mut num as *mut i32
        
        println!("r1 is: {}", *r1)
        *r2 = 10
        println!("r2 is: {}", *r2)
    }
    
    // Direct syscall
    let pid = syscall(39, 0, 0, 0, 0, 0, 0)  // getpid
    println("PID: {}", pid)
    
    // File I/O via syscall
    let fd = syscall(2, "test.txt", 0o644, 0, 0, 0, 0)  // open
    if fd >= 0 {
        let msg = "Hello from syscall\n"
        syscall(1, fd, msg, msg.len(), 0, 0, 0)  // write
        syscall(3, fd, 0, 0, 0, 0, 0)  // close
    }
    
    // Inline assembly
    let result: i32
    asm volatile(
        "movl $42, %eax",
        out("eax") result
    )
    println("Assembly result: {}", result)
}
```

### Modules and Packages

```knull
// math.knull
pub mod math {
    pub const PI: f64 = 3.14159265359
    
    pub fn square(x: f64) -> f64 { x * x }
    
    pub fn factorial(n: u64) -> u64 {
        if n <= 1 { 1 } else { n * factorial(n - 1) }
    }
}

// main.knull
use math::{PI, square}

fn main() {
    println("PI = {}", PI)
    println("Square of 5 = {}", square(5))
    println("5! = {}", math::factorial(5))
}
```

### Networking

```knull
use std::net::{TcpListener, TcpStream}
use std::io::{Read, Write}

fn main() {
    // TCP Server
    let listener = TcpListener::bind("127.0.0.1:8080")
    println("Server listening on port 8080")
    
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                handle_client(&mut stream)
            }
            Err(e) => println("Error: {}", e),
        }
    }
}

fn handle_client(stream: &mut TcpStream) {
    let mut buffer = [0u8; 1024]
    match stream.read(&mut buffer) {
        Ok(n) => {
            let request = String::from_utf8_lossy(&buffer[..n])
            println!("Received: {}", request)
            
            let response = "HTTP/1.1 200 OK\r\n\r\nHello from Knull!"
            stream.write(response.as_bytes())
        }
        Err(e) => println("Error reading: {}", e),
    }
}
```

---

## Command Line Interface

```bash
# Run a Knull file directly (interpreted)
knull run <file>        

# Compile to executable
knull build <file>      

# Compile to assembly
knull build --emit=asm <file>

# Compile to LLVM IR
knull build --emit=llvm <file>

# Check syntax without running
knull check <file>      

# Format code
knull fmt <file>        

# Interactive REPL
knull repl              

# Create new project
knull new <name>        

# Package management
knull pkg add <name>    # Add dependency
knull pkg update        # Update dependencies
knull pkg build         # Build project

# Show version
knull --version

# Show help
knull --help
```

### Compilation Options

```bash
# Optimization levels
knull build -O0 <file>  # No optimization
knull build -O1 <file>  # Basic optimization
knull build -O2 <file>  # Standard optimization (default)
knull build -O3 <file>  # Aggressive optimization
knull build -Os <file>  # Optimize for size

# Target architecture
knull build --target=x86_64-linux-gnu <file>
knull build --target=aarch64-linux-gnu <file>
knull build --target=wasm32-wasi <file>

# Cross-compilation
knull build --target=x86_64-windows-gnu <file>
```

---

## Language Features

### Type System

| Type | Description | Size |
|------|-------------|------|
| `i8` to `i128` | Signed integers | 1-16 bytes |
| `u8` to `u128` | Unsigned integers | 1-16 bytes |
| `f32`, `f64` | Floating point | 4, 8 bytes |
| `bool` | Boolean | 1 byte |
| `char` | Unicode scalar | 4 bytes |
| `String` | UTF-8 string | variable |
| `&T` | Immutable reference | pointer size |
| `&mut T` | Mutable reference | pointer size |
| `*const T` | Raw const pointer | pointer size |
| `*mut T` | Raw mut pointer | pointer size |
| `[T; N]` | Fixed array | N * sizeof(T) |
| `[T]` | Slice | 2 * pointer size |
| `Vec<T>` | Dynamic array | 3 * pointer size |
| `Option<T>` | Optional value | sizeof(T) + 1 |
| `Result<T, E>` | Error handling | max(sizeof(T), sizeof(E)) + 1 |
| `fn` | Function pointer | pointer size |

### Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+`, `-`, `*`, `/`, `%` |
| Bitwise | `&`, `|`, `^`, `<<`, `>>`, `~` |
| Comparison | `==`, `!=`, `<`, `>`, `<=`, `>=` |
| Logical | `&&`, `\|\|`, `!` |
| Assignment | `=`, `+=`, `-=`, `*=`, `/=`, `%=` |
| Reference | `&`, `&mut`, `*` |
| Range | `..`, `..=` |

### Operator Precedence

1. Postfix: `()`, `[]`, `.`, `?.`
2. Unary: `!`, `-`, `*`, `&`, `&mut`
3. Multiplicative: `*`, `/`, `%`
4. Additive: `+`, `-`
5. Shift: `<<`, `>>`
6. Bitwise AND: `&`
7. Bitwise XOR: `^`
8. Bitwise OR: `|`
9. Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
10. Logical AND: `&&`
11. Logical OR: `\|\|`
12. Range: `..`
13. Assignment: `=`, `+=`, `-=`, etc.

---

## Examples

See the `examples/` directory for comprehensive demonstrations:

| Example | Description |
|---------|-------------|
| `hello.knull` | Hello world |
| `fibonacci.knull` | Mathematical sequence |
| `primes.knull` | Prime number sieve |
| `calculator.knull` | Mathematical operations |
| `patterns.knull` | Pattern printing |
| `sorting.knull` | Sorting algorithms |
| `benchmark.knull` | Performance testing |
| `ownership.knull` | Ownership examples |
| `unsafe.knull` | Unsafe code patterns |
| `network.knull` | TCP/UDP networking |
| `syscall.knull` | Direct system calls |
| `asm.knull` | Inline assembly |

---

## Project Structure

```
knull/
├── src/                    # Bootstrap compiler (Rust)
│   ├── main.rs            # CLI entry point
│   ├── lexer.rs           # Tokenizer
│   ├── parser.rs          # Parser
│   ├── ast.rs             # AST definitions
│   ├── type_system.rs     # Type checker
│   ├── ownership.rs       # Ownership checker
│   ├── llvm_codegen.rs    # LLVM backend
│   ├── compiler.rs        # Compiler driver
│   ├── pkg.rs             # Package manager
│   └── cli.rs             # CLI implementation
├── self_hosted/           # Self-hosted compiler (Knull)
│   ├── knullc.knull       # Main compiler
│   ├── lexer.knull        # Lexer module
│   ├── parser.knull       # Parser module
│   └── codegen.knull      # Code generator
├── runtime/               # Standard library (Knull)
│   ├── std.knull          # Standard library root
│   ├── builtins.knull     # Built-in functions
│   ├── sys.knull          # System calls
│   └── net.knull          # Networking
├── std/                   # Standard library (Rust implementation)
│   ├── io.rs              # I/O operations
│   ├── string.rs          # String utilities
│   └── collections.rs     # Vectors, HashMaps
├── pkg/                   # Package manager (Rust)
│   └── manager.rs         # Package management
├── examples/              # Example programs
├── tests/                 # Test suite
└── docs/                  # Documentation
```

---

## Current Status

| Component | Status | Notes |
|-----------|--------|-------|
| Lexer | Complete | Full tokenization |
| Parser | Complete | Full AST generation |
| Type System | Complete | Type inference, checking |
| Ownership System | Complete | Borrow checker |
| Interpreter | Complete | Tree-walk interpreter |
| LLVM Backend | Partial | Basic codegen working |
| Self-Hosted Compiler | In Progress | Lexer/parser complete |
| Standard Library | Partial | Core modules working |
| Package Manager | In Progress | Basic functionality |
| Networking | Complete | TCP/UDP/sockets |
| Documentation | In Progress | Core docs complete |

### Performance

| Benchmark | Knull | Python | C | Rust |
|-----------|-------|--------|---|------|
| Fibonacci (40) | ~0.8s | ~35s | ~0.6s | ~0.5s |
| Primes (1M) | ~0.3s | ~12s | ~0.2s | ~0.15s |
| String concat | ~0.1s | ~2s | ~0.05s | ~0.04s |

*Note: Benchmarks run on AMD Ryzen 9 5900X, compiled with -O2*

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone and build
git clone https://github.com/knull-lang/knull.git
cd knull/src
cargo build

# Run tests
cargo test

# Run with specific features
cargo run --features llvm-backend -- run example.knull

# Format code
cargo fmt
cargo clippy
```

### Areas for Contribution

- [ ] Complete LLVM backend optimization passes
- [ ] WebAssembly target support
- [ ] LSP (Language Server Protocol) implementation
- [ ] IDE plugins (VS Code, Vim, Emacs)
- [ ] Additional standard library modules
- [ ] Documentation translations
- [ ] More examples and tutorials

---

## Roadmap

### Phase 1: Bootstrap (Complete)
- [x] Bootstrap compiler in Rust
- [x] Lexer and parser
- [x] Type system
- [x] Ownership system
- [x] Basic LLVM backend
- [x] Standard library foundation

### Phase 2: Self-Hosting (In Progress)
- [x] Self-hosted lexer
- [x] Self-hosted parser
- [x] Self-hosted type checker
- [ ] Self-hosted code generator
- [ ] Self-compilation

### Phase 3: Ecosystem (Planned)
- [ ] Package registry
- [ ] LSP implementation
- [ ] Debugger support
- [ ] WASM target
- [ ] Embedded targets

### Phase 4: Production (Future)
- [ ] Stable 1.0 release
- [ ] Verified compiler
- [ ] Formal specification
- [ ] Industry adoption

---

## Community

- **Discord**: [discord.gg/knull](https://discord.gg/knull)
- **Forum**: [forum.knull-lang.org](https://forum.knull-lang.org)
- **Twitter**: [@knull_lang](https://twitter.com/knull_lang)
- **Mastodon**: [@knull@fosstodon.org](https://fosstodon.org/@knull)

---

## Acknowledgments

Knull draws inspiration from:
- **Rust** - Ownership system, zero-cost abstractions
- **C** - Systems programming, simplicity
- **Python** - Readability, ease of use
- **Zig** - Comptime, C interoperability
- **Jai** - Data-oriented design, pragmatism
- **C++** - Template metaprogramming (concepts)

---

## License

Knull is dual-licensed under:

- **MIT License** - See [LICENSE-MIT](LICENSE-MIT)
- **Apache License 2.0** - See [LICENSE-APACHE](LICENSE-APACHE)

You may use Knull under either license at your option.

SPDX-License-Identifier: MIT OR Apache-2.0

---

## Disclaimer

Knull is currently in development. The language design and implementation may change.

---

## Built-in Functions Reference

### File I/O
- `file_read(path)` - Read entire file to string
- `file_write(path, content)` - Write string to file
- `file_append(path, content)` - Append to file
- `file_exists(path)` - Check if file exists
- `file_remove(path)` - Delete file
- `mkdir(path)` - Create directory
- `dir_list(path)` - List directory contents

### Networking
- `tcp_connect(address)` - Connect to TCP server
- `tcp_send(handle, data)` - Send data over TCP
- `tcp_recv(handle, size)` - Receive data from TCP
- `tcp_close(handle)` - Close TCP connection
- `tcp_listen(address)` - Start TCP server
- `tcp_accept(handle)` - Accept TCP connection
- `get_hostname()` - Get system hostname

### Threading
- `spawn(code)` - Spawn new thread
- `sleep(millis)` - Sleep for milliseconds
- `thread_id()` - Get current thread ID

### Time
- `time()` - Get Unix timestamp (seconds)
- `time_millis()` - Get Unix timestamp (milliseconds)

### Environment
- `env_get(key)` - Get environment variable
- `env_set(key, value)` - Set environment variable
- `exec(command)` - Execute shell command

### String/Array Utilities
- `len(arr_or_str)` - Get length
- `strlen(str)` - Get string length
- `substring(str, start, end)` - Extract substring
- `push(arr, element)` - Add element to array

### FFI (Foreign Function Interface)
- `ffi_open(path)` - Load dynamic library
- `ffi_get_symbol(lib, name)` - Get symbol from library
- `ffi_malloc(size)` - Allocate C memory
- `ffi_free(ptr)` - Free C memory

### Garbage Collection
- `gc_collect()` - Run garbage collector
- `gc_stats()` - Get GC statistics (returns array)

---

## Project Statistics

- **40+ Working Examples**: All `.knull` example files execute correctly
- **Standard Library**: 6 modules (core, io, math, collections, net, async)
- **Self-Hosted Compiler**: 1700+ lines of Knull code
- **Bootstrap Compiler**: Rust-based compiler with C backend
- **Package Manager**: Create, build, and manage projects
- **Native Compilation**: Compiles to native binaries via C backend

---

**Knull** - From hello world to operating system, one language.

*Made with ❤️ by the Knull community*
