# Knull Toolchain Guide

> Installation, usage, and development workflow for Knull.

---

## Installation

### Prerequisites

- **Rust** (for building the bootstrap compiler)
- **Git** (for version control and package management)
- **Linux** (x86_64) - Primary target OS
- **A text editor** (VS Code recommended)

---

### Building from Source

```bash
# Clone the repository
git clone https://github.com/knull-lang/knull.git
cd knull

# Build the compiler
cd src
cargo build --release

# Add to PATH
export PATH="$PWD/target/release:$PATH"

# Verify installation
knull --version
```

---

## Project Structure

A typical Knull project:

```
my-project/
├── knull.toml          # Package manifest
├── src/
│   └── main.knull      # Entry point
├── tests/              # Integration tests
├── examples/           # Example programs
└── build/              # Build output
```

---

## The knull Command

### Basic Commands

```bash
# Compile a file
knull build <file.knull>

# Run a file (compiles and executes)
knull run <file.knull>

# Start REPL
knull repl

# Check/lint without building
knull check <file.knull>

# Format code
knull fmt <file.knull>

# Show version
knull --version
```

---

## Package Manager

### Creating a New Package

```bash
knull new my-package
```

This creates:

```
my-package/
├── knull.toml
└── src/
    └── main.knull
```

### Adding Dependencies

Edit `knull.toml`:

```toml
[dependencies]
http = "^0.2"
json = "^1.0"
```

Then run:

```bash
knull deps install
```

This resolves and downloads dependencies to `.knull/cache`.

### Publishing a Package

```bash
knull publish
```

---

## Language Modes

Knull has three modes for different skill levels:

### Novice Mode

Simple scripting with garbage collection.

```knull
// novice.knull
// Garbage collected, safe by default

let name = read_line()
print("Hello, " + name + "!")

// No manual memory management needed
let data = [1, 2, 3, 4, 5]
let doubled = data.map(|x| x * 2)
```

---

### Expert Mode

Systems programming with manual memory management.

```knull
// expert.knull
// Manual memory, zero-cost abstractions

// Allocate with explicit lifetime
let ptr = alloc::<u8>(1024)
defer free(ptr)

// Raw pointers, no GC
let raw = ptr as *mut u8

// Unsafe blocks when needed
unsafe {
    raw.write(0, 42)
}
```

---

### God Mode

Bare-metal programming without std.

```knull
// god.knull
// No std, no runtime, pure metal

// Entry point - no main
fn _start() noreturn {
    // Write directly to VGA memory
    let vga = 0xB8000 as *mut u16
    vga.write(0, 'H' as u16 | 0x0F00)
    
    // Infinite loop
    loop {}
}
```

---

## Writing Code

### Hello World

```knull
// hello.knull
module main

fn main() {
    println("Hello, Knull!")
}
```

### Basic Types

```knull
let x: i32 = 42           // Signed 32-bit
let y: u64 = 100           // Unsigned 64-bit
let z: f64 = 3.14          // Float
let b: bool = true         // Boolean
let c: char = 'A'          // Character
let s: string = "hello"    // String (owned)
let p: *i32 = &x           // Raw pointer
let r: &i32 = &x           // Reference
```

### Collections

```knull
// Vector
let mut v = Vec::new()
v.push(1)
v.push(2)

// HashMap
let mut map = HashMap::new()
map.insert("key", "value")
```

### Control Flow

```knull
// If expression
let max = if a > b { a } else { b }

// Match
match value {
    1 => println("one"),
    2 => println("two"),
    _ => println("other"),
}

// Loop
loop {
    if done { break }
}

// For
for i in 0..10 {
    println(i)
}
```

### Functions

```knull
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Closure
let add = |a: i32, b: i32| -> i32 { a + b }
```

### Error Handling

```knull
// Option
let value = maybe_number()  // option<i32>

if let Some(n) = value {
    println(n)
}

// Result
let result = try_parse("42")
match result {
    Ok(n) => println(n),
    Err(e) => eprintln("Error: {}", e),
}
```

---

## Building for Production

### Release Build

```bash
knull build --release main.knull
```

### Cross-Compilation

```bash
# Target specific architecture
knull build --target aarch64-unknown-linux-knull main.knull
```

### Link-Time Optimization

```bash
knull build --release --lto main.knull
```

---

## Editor Setup

### VS Code

1. Install the Knull extension from the marketplace
2. Open a `.knull` file
3. The extension provides:
   - Syntax highlighting
   - IntelliSense
   - Error diagnostics
   - Go to definition

### Other Editors

Use the TextMate grammar in `syntax/knull.tmLanguage.json`:

- **Sublime Text**: Install via Package Control
- **Vim/Neovim**: Use tree-sitter-knull
- **Emacs**: Use knull-mode

---

## Debugging

### Print Debugging

```knull
let value = compute_something()
std.dbg(value)  // Prints: <file>:<line>: value
```

### Panic Messages

```knull
// Custom panic
panic("Unexpected state!")

// Assert
assert(x > 0, "x must be positive")
```

---

## Performance Tips

1. **Use `--release`** for production - enables optimizations
2. **Pre-allocate collections** with `Vec::with_capacity()`
3. **Avoid allocations in hot loops** - reuse buffers
4. **Use references** instead of moving when possible
5. **Profile first** - don't optimize prematurely

---

## Common Errors

### "Unknown type 'X'"

- Import the module: `use std::X`

### "Cannot find function 'Y'"

- Check the function signature
- Module might not be imported

### "Mismatched types"

- Explicitly annotate types: `let x: i32 = 42`

### "Cannot borrow as mutable"

- Use `mut`: `let mut v = Vec::new()`

---

## Advanced Topics

### FFI (Foreign Function Interface)

```knull
// Call C function
extern "C" {
    fn printf(format: *const u8, ...) -> i32
}

// Use
printf("Hello %s\n".as_ptr(), "world".as_ptr())
```

### Inline Assembly

```knull
unsafe {
    asm intel {
        "mov rax, 60"
        "xor rdi, rdi"
        "syscall"
    }
}
```

### Custom Allocators

```knull
// Override default allocator
fn __knull_alloc(size: usize) -> *mut u8 {
    // Custom allocation logic
}
```

---

## Getting Help

- **GitHub Issues**: Report bugs
- **Discord**: Community discussion
- **Documentation**: Check docs/SPECIFICATION.md
- **REPL**: Experiment interactively

```bash
knull repl
> let x = 42
x: i32 = 42
> x * 2
84
```

---

## Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial release, basic types, functions |
| 0.2.0 | Added collections, error handling |
| 0.3.0 | Package manager, networking |
| 0.4.0 | God mode, bare-metal support |

---

*For more information, see docs/SPECIFICATION.md*
