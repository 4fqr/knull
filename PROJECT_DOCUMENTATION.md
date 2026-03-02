# Knull Programming Language - Complete Project Documentation

## Project Overview

**Knull** is a systems programming language project started with the goal of being a unified language spanning from beginner scripting to bare-metal systems programming. The project consists of:

1. A **bootstrap compiler** written in Rust (works, runs Knull programs)
2. A **tree-walking interpreter** (works for basic to moderate complexity)
3. **Standard library** files (many written but most don't parse yet)
4. A **self-hosted compiler** attempt (incomplete)

**Current Reality**: The language works as an interpreted language with a functional interpreter. Many advanced features claimed in documentation do not actually work yet.

---

## What Actually Works

### Working Features (Verified)

**Core Language:**
- Variables: `let x = 42`, `let name = "hello"`
- Types: Int (i64), Float (f64), String, Bool, Array, Null
- Functions: `fn add(a, b) { return a + b }`
- Control flow: `if/else`, `while`, `for item in arr`, `loop`
- Arrays: `[1, 2, 3]`, `arr[0]`, `len(arr)`
- Binary ops: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical ops: `&&`, `||`, `!`
- Print functions: `print(x)`, `println(x)` - space separated calls work
- Comments: `// line comments`, `/* block comments */`
- Hex literals: `0xFF`, `0xDEADBEEF`
- Const declarations: `const MAX = 100`
- Type casting: `x as f64`
- Structs: `struct Point { x, y }`, `let p = Point { x: 10, y: 20 }`
- Methods: `impl Point { fn get_x(self) { self.x } }`, `p.get_x()`
- Field access: `p.x`, `self.y`

**Built-in Functions (Actually Working):**
- `print(x)`, `println(x)`
- `len(arr)` or `len(str)`
- `typeof(x)`
- `time()`, `time_millis()`
- `env_get(key)`, `env_set(key, value)`
- `file_read(path)`, `file_write(path, content)`
- `file_append(path, content)`, `file_exists(path)`
- `mkdir(path)`, `dir_list(path)`
- `exec(command)`
- `spawn(code)` - spawns thread
- `sleep(millis)`
- `tcp_connect(address)`, `tcp_send(handle, data)`
- `tcp_recv(handle, size)`, `tcp_close(handle)`
- `tcp_listen(address)`, `tcp_accept(handle)`
- `get_hostname()`
- `exit(code)`

**Working Examples:**
- All 48 examples in `examples/` directory (except web_server which times out)
- hello_world.knull, fibonacci.knull, primes.knull
- calculator.knull, game_of_life.knull, adventure_game.knull
- Basic struct and method examples

---

## What Does NOT Work Yet

**Parser Limitations:**
- Generics (`Vec<T>`, `fn identity<T>(x: T) -> T`)
- Traits and trait implementations
- Complex type annotations beyond basic types
- Closures/lambdas (`|x| x * 2`)
- Pattern matching in match arms
- Advanced macro invocations

**Standard Library:**
- Most `.knull` files in `stdlib/` do NOT parse or run
- They use syntax the parser doesn't support
- Files like `ml.knull`, `blockchain.knull`, `security.knull` are essentially pseudocode

**Self-Hosted Compiler:**
- Files in `self_hosted/` don't actually compile working programs
- The self-hosted compiler is incomplete

**Claimed but Not Implemented:**
- Ownership system (borrow checker)
- Linear types
- Effect system
- Async/await execution
- WASM code generation (exists but doesn't work)
- LLVM backend (exists but requires LLVM)
- Package manager commands beyond basic
- LSP server (exists but incomplete)
- Debugger (exists but incomplete)

---

## Project Structure

### `/src/` - Main Compiler Source (Rust)

**Entry Point:**
- `main.rs` - CLI entry, argument parsing, subcommands (run, build, test, etc.)

**Core Components:**
- `lexer.rs` - Tokenizer, converts source to tokens
  - Supports: identifiers, numbers, strings, operators, keywords
  - Handles hex literals, block comments
- `parser.rs` - Recursive descent parser, converts tokens to AST
  - Supports: expressions, statements, functions, structs, impl blocks
  - Handles operator precedence
  - 1200+ lines
- `interpreter.rs` - Tree-walking interpreter (the working runtime)
  - 1200+ lines
  - Executes AST directly
  - Contains all built-in functions
- `ast.rs` - AST node definitions
- `type_system.rs` - Type definitions (i32, i64, f32, String, etc.)

**Code Generation (Partial):**
- `c_codegen.rs` - Generates C code (works for basic programs)
- `wasm_codegen.rs` - WASM generation (stub)
- `llvm_codegen.rs` - LLVM IR generation (requires LLVM feature)

**Other Modules:**
- `compiler.rs` - Compilation orchestration
- `cli.rs` - Command line interface
- `ownership.rs` - Ownership tracking (stub)
- `gc.rs` - Garbage collector integration
- `ffi.rs` - Foreign function interface bindings
- `effects.rs` - Effect system (stub)
- `macros.rs` - Macro system (basic support)
- `concurrency.rs` - Threading primitives
- `debugger.rs` - Debug adapter protocol (incomplete)
- `lsp.rs` - Language server protocol (incomplete)
- `incremental.rs` - Incremental compilation
- `linear_check.rs` - Linear type checking
- `doc.rs` - Documentation generation
- `comptime.rs` - Compile-time execution

**Package Management:**
- `pkg/` - Package manager
  - `mod.rs` - Package manager interface
  - `manifest.rs` - knull.toml parsing
  - `lockfile.rs` - Dependency locking
  - `resolver.rs` - Dependency resolution
  - `manager.rs` - Package operations

**Runtime:**
- `runtime/` - Runtime support files

### `/examples/` - Working Example Programs

All 48 examples work with the interpreter:
- Basic: hello_world.knull, fibonacci.knull, primes.knull
- Math: calculator.knull, math_demo.knull, sorting.knull
- Games: guessing_game.knull, game_of_life.knull, adventure_game.knull
- Advanced: os_kernel.knull, gui_framework.knull, sdl_demo.knull
- Network: network_demo.knull, tcp_example.knull
- Tests: test_*.knull (16 test files)

### `/stdlib/` - Standard Library Files (Most Don't Work)

**Status: These exist but mostly don't parse:**

- `core/` - Core types (Option, Result)
- `collections/` - Vec, Map, Set (collections.knull)
- `io/` - File I/O operations
- `math/` - Math functions
- `net/` - Networking (http, websocket)
- `thread/` - Concurrency primitives
- `async/` - Async runtime
- `mem/` - Memory management (memory.knull - uses unsupported syntax)
- `crypto/` - Cryptography (crypto.knull, security.knull)
- `db/` - Database (sqlite.knull, database.knull)
- `game/` - Game engine (engine.knull, physics.knull, sprite.knull)
- `gui/` - GUI framework (framework.knull, sdl2.knull)
- `embedded/` - Embedded/bare metal
- `ai/` - AI/ML (ml.knull - extensive but doesn't parse)
- `json/` - JSON parsing (json.knull)

**Reality Check:**
These files demonstrate what the language SHOULD be able to do, but most use syntax the parser doesn't support yet (generics, complex types, advanced patterns).

### `/bootstrap/` - Bootstrap Compiler

Alternative Rust-based compiler:
- `src/main.rs` - Entry point
- `src/lexer.rs` - Lexer
- `src/parser.rs` - Parser
- `src/ccodegen.rs` - C code generator (actually works)
- `src/compiler.rs` - Compilation logic

This can compile simple Knull programs to native binaries via C.

### `/self_hosted/` - Self-Hosted Compiler (Incomplete)

- `knullc.knull` - Main compiler (doesn't work)
- `lexer.knull` - Lexer in Knull (doesn't work)
- `parser.knull` - Parser in Knull (doesn't work)
- `codegen.knull` - Code generator in Knull (doesn't work)

### `/docs/` - Documentation

- `SPECIFICATION.md` - Language specification
- `TOOLCHAIN.md` - Tool usage guide
- `GUIDE.md` - User guide
- `NOVICE_MODE.md`, `EXPERT_MODE.md`, `GOD_MODE.md` - Mode docs
- `TUTORIALS/` - Tutorial files

### `/tests/` - Test Suite

- Test files for the compiler

### `/packages/` - Package Registry (Empty)

### Root Files

- `README.md` - Main documentation
- `IMPLEMENTATION.md` - Implementation status
- `BUILD.md` - Build instructions
- `USAGE.md` - Usage guide
- `MANIFESTO.md` - Project philosophy
- `CONTRIBUTING.md` - Contribution guidelines
- `LICENSE-MIT`, `LICENSE-APACHE` - Dual licensing
- `install.sh` - Installation script
- `uninstall.sh` - Uninstallation script
- `test_all.sh` - Test runner
- `Makefile` - Build automation
- `knull.toml` - Project manifest example

---

## Complete Syntax Specification (What Actually Parses)

### Literals
```knull
42          // integer
3.14        // float
"hello"     // string
true        // boolean
false       // boolean
null        // null
[1, 2, 3]   // array
0xFF        // hex integer
```

### Variables
```knull
let x = 42
let y: i32 = 42          // type annotation (optional)
let name = "hello"
const MAX = 100          // constant
```

### Functions
```knull
fn add(a, b) {           // implicit return of last expression
    a + b
}

fn subtract(a, b) {      // explicit return
    return a - b
}

fn greet(name) {         // void function
    print("Hello, ")
    println(name)
}

// Space-separated calls work:
println "Hello World"
print x
```

### Structs
```knull
struct Point {
    x: f32,
    y: f32
}

let p = Point { x: 10, y: 20 }
print(p.x)              // field access

impl Point {
    fn get_x(self) {    // method with self
        return self.x
    }
    
    fn distance(self, other) {
        // method with self and other params
        return self.x - other.x
    }
}

print(p.get_x())        // method call
```

### Control Flow
```knull
if condition {
    // code
} else if other_condition {
    // code
} else {
    // code
}

while condition {
    // code
}

for item in array {
    // code
}

loop {
    // infinite loop
    if condition { break }
    if other { continue }
}
```

### Operators
```knull
// Arithmetic
+  -  *  /  %

// Comparison
==  !=  <  >  <=  >=

// Logical
&&  ||  !

// Assignment
=  +=  -=  *=  /=

// Index
arr[0]

// Field access
obj.field

// Type cast
value as f64
```

### Comments
```knull
// Line comment

/*
 * Block comment
 */
```

---

## Current Implementation Status

### Working (Verified)
| Component | Status | Notes |
|-----------|--------|-------|
| Lexer | Working | All features |
| Parser | Working | Basic to moderate complexity |
| Interpreter | Working | Executes programs correctly |
| Structs | Working | Just added |
| Methods | Working | Just added |
| Built-in functions | Working | 40+ functions |
| File I/O | Working | Via interpreter |
| Networking | Working | TCP via interpreter |
| Threading | Working | Basic spawn |
| Examples | 48/49 | All except web_server |

### Partially Working
| Component | Status | Notes |
|-----------|--------|-------|
| C Codegen | Partial | Works for basic programs |
| Bootstrap | Partial | Simple programs only |
| FFI | Partial | Basic C interop |

### Not Working / Stubs
| Component | Status | Notes |
|-----------|--------|-------|
| Self-hosted compiler | Not working | Files exist but don't compile |
| Stdlib .knull files | Most broken | Use unsupported syntax |
| Ownership checker | Stub | Exists but not enforced |
| Borrow checker | Not implemented | |
| Generics | Not working | Parser doesn't support |
| Closures | Not working | Parser doesn't support |
| Pattern matching | Partial | Basic match works |
| LSP | Incomplete | Stub implementation |
| Debugger | Incomplete | Stub implementation |
| WASM target | Stub | Exists but broken |
| LLVM backend | Requires LLVM | Feature-gated |

---

## Build and Run

### Build the Compiler
```bash
cd src
cargo build --release --no-default-features
# Binary: src/target/release/knull
```

### Run a Program
```bash
knull run file.knull
```

### Run Tests
```bash
cd /home/foufqr/Documents/knull-lang
./test_all.sh
```

---

## Requirements for Full Implementation

### Parser Enhancements Needed
1. Generics parsing (`Vec<T>`, `fn foo<T>(x: T)`)
2. Closure/lambda parsing (`|x| x * 2`)
3. Trait parsing (`trait Foo { ... }`)
4. Complex type annotations
5. Pattern matching in all contexts

### Interpreter Enhancements Needed
1. Ownership tracking
2. Borrow checking at runtime
3. Linear type enforcement
4. Effect checking
5. Async execution

### Standard Library
1. Fix all .knull files to use supported syntax
2. Implement core data structures
3. Make stdlib actually loadable

### Self-Hosted Compiler
1. Complete the lexer
2. Complete the parser
3. Complete the code generator
4. Make it self-hosting

### Tooling
1. Complete LSP implementation
2. Complete debugger
3. Package manager registry
4. IDE plugins

---

## Testing

### What to Test
```bash
# Core examples
cd /home/foufqr/Documents/knull-lang
knull run examples/hello_world.knull
knull run examples/calculator.knull
knull run examples/fibonacci.knull

# Struct tests
echo 'struct P { x } fn main() { let p = P { x: 5 } print(p.x) }' | knull run -

# Method tests  
echo 'struct P { x } impl P { fn get(self) { self.x } } fn main() { let p = P { x: 5 } print(p.get()) }' | knull run -

# All examples
for f in examples/*.knull; do knull run "$f"; done
```

### Test Results (Current)
- 48/49 examples pass
- All basic language features work
- Structs and methods work
- File I/O works
- Network I/O works
- Threading works

---

## Contributing

### Where to Start
1. Fix stdlib files to use supported syntax
2. Add missing built-in functions
3. Extend parser for generics
4. Improve interpreter performance
5. Fix self-hosted compiler

### Code Guidelines
1. Test every change with examples
2. Don't claim features work until verified
3. Update documentation honestly
4. Keep backwards compatibility

---

## Honest Assessment

**What Knull Is:**
A functional interpreted language with:
- Solid basic syntax
- Working structs and methods (newly added)
- Good built-in function library
- 48 working examples

**What Knull Is NOT (Yet):**
- A production-ready systems language
- Self-hosting
- Fully featured as claimed in some docs
- Able to run most stdlib files

**Path Forward:**
1. Fix stdlib to use supported syntax
2. Extend parser incrementally
3. Verify every claimed feature
4. Complete self-hosted compiler
5. Add ownership system for real

---

## File Manifest

### Critical Files (Working)
- `src/main.rs` - Entry point
- `src/lexer.rs` - Tokenizer
- `src/parser.rs` - Parser
- `src/interpreter.rs` - Interpreter (main runtime)
- `src/ast.rs` - AST definitions
- `examples/*.knull` - Working examples

### Important Files (Partial)
- `src/c_codegen.rs` - C backend
- `bootstrap/src/*.rs` - Bootstrap compiler
- `src/pkg/*.rs` - Package manager

### Stub Files (Not Working)
- `self_hosted/*.knull` - Self-hosted (incomplete)
- Most `stdlib/*/*.knull` files
- `src/lsp.rs` - LSP (incomplete)
- `src/debugger.rs` - Debugger (incomplete)
- `src/llvm_codegen.rs` - Requires LLVM

---

## Contact / Repository

Repository: https://github.com/4fqr/knull

**Important**: This documentation reflects the actual state as of the last commit. Many features claimed in README.md and other docs are aspirational and don't work yet.
