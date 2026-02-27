# Knull Usage Guide

This guide covers how to get started with the Knull programming language, from installation to writing your first program.

## Installation

### Building from Source

```bash
# Clone the repository
git clone https://github.com/knull-lang/knull.git
cd knull

# Build the bootstrap compiler
cd src
cargo build --release

# Add to PATH (add to ~/.bashrc for permanent)
export PATH="$PWD/target/release:$PATH"

# Verify installation
knull --version
```

## Your First Knull Program

### Step 1: Create a File

Create a file called `hello.knull`:

```knull
// hello.knull
module main

fn main() {
    println("Hello, Knull!")
}
```

### Step 2: Run the Program

```bash
knull run hello.knull
```

Output:
```
Hello, Knull!
```

## Compiler Commands

### Build

Compile a Knull file to an executable:

```bash
knull build hello.knull -o hello
```

### Run

Compile and run in one step:

```bash
knull run hello.knull
```

### Check

Syntax check without building:

```bash
knull check hello.knull
```

### Format

Auto-format code:

```bash
knull fmt hello.knull
```

## Language Basics

### Variables

```knull
let x: i32 = 42           // Immutable
let mut y: i32 = 100      // Mutable

let name = "Knull"        // Type inferred
```

### Functions

```knull
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Closure
let multiply = |a: i32, b: i32| -> i32 { a * b }
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

// For range
for i in 0..10 {
    println(i)
}
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

### Error Handling

```knull
// Option
let result = find_item("name")
if let Some(item) = result {
    println(item)
}

// Result
match try_parse("42") {
    Ok(n) => println("Parsed: {}", n),
    Err(e) => println("Error: {}", e),
}
```

## Standard Library

### Print

```knull
println("Hello!")           // With newline
print("No newline")         // Without newline
eprintln("Error!")          // To stderr
```

### File I/O

```knull
let file = std.io.open("data.txt", std.io.READ)!
let content = file.read_all()!
file.close()
```

### Networking

```knull
// TCP client
let stream = std.net.tcp_connect("localhost", 8080)!

// TCP server
let listener = std.net.tcp_bind(8080)!
while let Some(stream) = listener.accept() {
    // Handle connection
}
```

### Memory

```knull
let ptr = std.alloc::<u8>(1024)
defer std.free(ptr)

// Write
unsafe { *ptr = 42 }

// Read
let value = unsafe { *ptr }
```

## Editor Setup

### VS Code

1. Open VS Code
2. Install the Knull extension from the marketplace (or use the provided syntax in `syntax/knull.tmLanguage.json`)
3. Create a `.knull` file
4. Enjoy syntax highlighting and IntelliSense

### Syntax Highlighting

Copy `syntax/knull.tmLanguage.json` to your editor's syntax folder:

- **VS Code**: `~/.vscode/extensions/`
- **Sublime Text**: `~/.config/sublime-text/Packages/User/`
- **Vim**: `~/.vim/syntax/`

Example for Vim, add to `~/.vim/filetype.vim`:
```vim
augroup filetypedetect
  au! BufNewFile,BufRead *.knull set filetype=knull
augroup END
```

## Package Management

### Create a Package

```bash
knull new my-project
```

This creates:

```
my-project/
├── knull.toml
└── src/
    └── main.knull
```

### Add Dependencies

Edit `knull.toml`:

```toml
[dependencies]
http = "^0.2"
json = "^1.0"
```

Install:

```bash
knull deps install
```

### Publish

```bash
knull publish
```

## Testing

### Run Tests

```bash
knull test
```

### Write Tests

```knull
#[test]
fn test_addition() {
    assert(2 + 2 == 4, "Math should work")
}
```

## Debugging

### Print Debug

```knull
let value = compute()
std.dbg(value)  // Prints: <file>:<line>: value
```

### Panic

```knull
panic("Something went wrong!")

assert(x > 0, "x must be positive")
```

## Performance Tips

1. Use `--release` for production builds
2. Pre-allocate collections with `Vec::with_capacity()`
3. Avoid allocations in hot loops
4. Use references instead of moving when possible

## Common Issues

### Unknown type 'X'

Solution: Import the module:
```knull
use std::X
```

### Cannot find function 'Y'

Solution: Check the function signature and ensure the module is imported.

### Mismatched types

Solution: Explicitly annotate types:
```knull
let x: i32 = 42
```

## Next Steps

- Read the [Specification](docs/SPECIFICATION.md) for language details
- Check [Standard Library](docs/STD_LIB.md) for API docs
- Explore [Examples](examples/) for sample programs

## Getting Help

- GitHub Issues: Report bugs
- Check docs/SPECIFICATION.md for detailed language reference

---

For more information, visit https://github.com/knull-lang/knull
