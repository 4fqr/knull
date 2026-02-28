# Knull Usage Guide

Complete guide for using the Knull programming language and command-line interface.

---

## Table of Contents

1. [Installation](#installation)
2. [Command Reference](#command-reference)
3. [Writing Knull Programs](#writing-knull-programs)
4. [Project Management](#project-management)
5. [Editor Integration](#editor-integration)
6. [Troubleshooting](#troubleshooting)

---

## Installation

### Prerequisites

- Rust compiler (1.70 or later)
- Git

### Building from Source

```bash
# Clone the repository
git clone https://github.com/4fqr/knull.git
cd knull

# Build the compiler
cd src
cargo build --release --no-default-features

# The binary is located at:
# Linux/macOS: target/release/knull
# Windows: target/release/knull.exe
```

### Adding to PATH

**Linux/macOS:**
```bash
# Temporary (current session only)
export PATH="$PWD/target/release:$PATH"

# Permanent (add to ~/.bashrc or ~/.zshrc)
echo 'export PATH="$HOME/knull/src/target/release:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

**Windows:**
```powershell
# Add to system PATH through System Properties
# Or copy to a directory already in PATH:
copy target\release\knull.exe C:\Windows\System32\
```

### Verification

```bash
knull --version
```

Expected output:
```
knull 1.0.0
```

---

## Command Reference

### knull run

Execute a Knull source file.

```bash
knull run <file>
```

**Example:**
```bash
knull run hello.knull
knull run examples/fibonacci.knull
```

**Output:**
```
Running: hello.knull
Lexed 12 tokens
Parsed successfully
Hello, World!
```

### knull build

Compile a Knull file to binary (preparation for native compilation).

```bash
knull build <file> [OPTIONS]
```

**Options:**
- `-o, --output <path>`: Specify output file name
- `-r, --release`: Build in release mode (optimized)

**Example:**
```bash
knull build hello.knull
knull build hello.knull -o myprogram
knull build hello.knull --release
```

### knull check

Validate syntax without executing.

```bash
knull check <file>
```

**Example:**
```bash
knull check hello.knull
```

**Success output:**
```
No errors found
```

**Error output:**
```
Error: Parse error: Expected LBrace, got Identifier
```

### knull fmt

Format a Knull source file.

```bash
knull fmt <file>
```

**Example:**
```bash
knull fmt hello.knull
```

### knull repl

Start the interactive Read-Eval-Print Loop.

```bash
knull repl
```

**Example session:**
```
Knull REPL v1.0.0
Type :quit to exit

knull> let x = 42
knull> println x
42
knull> :quit
```

### knull new

Create a new Knull project.

```bash
knull new <project-name>
```

**Example:**
```bash
knull new myproject
```

**Creates:**
```
myproject/
├── knull.toml          # Project manifest
├── README.md           # Project documentation
└── src/
    └── main.knull      # Entry point
```

### knull add

Add a dependency to the current project.

```bash
knull add <package> [version]
```

**Example:**
```bash
knull add json
knull add http 1.2.0
```

### knull test

Run tests in the current project.

```bash
knull test
```

Looks for test files in:
- `tests/`
- `test/`
- `src/tests/`

### knull version

Display version information.

```bash
knull version
```

### knull help

Display help information.

```bash
knull help
```

---

## Writing Knull Programs

### Basic Program Structure

Every Knull program requires a `main` function as the entry point:

```knull
fn main() {
    println "Hello, World!"
}
```

### Variables

Declare variables using `let`:

```knull
let integer = 42
let floating = 3.14
let text = "Hello"
```

### Functions

Define functions with parameters:

```knull
fn add(a, b) {
    return a + b
}

fn main() {
    let result = add(5, 3)
    println result
}
```

### Control Flow

If-else statements:

```knull
if condition {
    // true branch
} else {
    // false branch
}
```

While loops:

```knull
let i = 0
while i < 10 {
    println i
    i = i + 1
}
```

### Comments

Single-line comments:

```knull
// This is a comment
let x = 5  // This is also a comment
```

### Complete Example

```knull
// Calculate factorial
fn factorial(n) {
    if n <= 1 {
        return 1
    }
    
    let result = 1
    let i = 2
    
    while i <= n {
        result = result * i
        i = i + 1
    }
    
    return result
}

fn main() {
    println "Factorial of 5: " + factorial(5)
}
```

---

## Project Management

### knull.toml Format

```toml
[package]
name = "myproject"
version = "0.1.0"
edition = "2024"
entry = "src/main.knull"
authors = ["Your Name <email@example.com>"]
description = "A brief description"
license = "MIT"

[dependencies]
# std = "1.0"

[build]
opt-level = 3
lto = true
```

### Project Structure

Standard project layout:

```
myproject/
├── knull.toml          # Project configuration
├── README.md           # Documentation
├── src/
│   ├── main.knull      # Entry point
│   └── lib.knull       # Library code
├── examples/           # Example programs
└── tests/              # Test files
```

### Building Projects

```bash
# Navigate to project directory
cd myproject

# Run the main entry point
knull run src/main.knull

# Check all files for syntax errors
knull check src/main.knull
```

---

## Editor Integration

### Visual Studio Code

1. Install the Knull extension (when available)
2. Or use the provided syntax file:
   - Copy `syntax/knull.tmLanguage.json` to `.vscode/extensions/`

### Vim

Add to `~/.vim/filetype.vim`:

```vim
augroup filetypedetect
  au! BufNewFile,BufRead *.knull set filetype=knull
augroup END
```

Create `~/.vim/syntax/knull.vim`:

```vim
if exists("b:current_syntax")
  finish
endif

syntax keyword knullKeyword fn let if else while for return
syntax keyword knullType int float string bool
syntax match knullComment "//.*$"
syntax match knullNumber "\d\+"
syntax match knullString "\"[^\"]*\""

highlight link knullKeyword Keyword
highlight link knullType Type
highlight link knullComment Comment
highlight link knullNumber Number
highlight link knullString String

let b:current_syntax = "knull"
```

### Sublime Text

Copy `syntax/knull.tmLanguage.json` to:

```
~/.config/sublime-text/Packages/User/
```

### Emacs

Add to `~/.emacs` or `~/.emacs.d/init.el`:

```elisp
(add-to-list 'auto-mode-alist '("\\.knull\\'" . knull-mode))

(define-derived-mode knull-mode prog-mode "Knull"
  "Major mode for editing Knull source files."
  (setq comment-start "// ")
  (setq comment-end "")
  (font-lock-add-keywords nil
    '(("\\<\\(fn\\|let\\|if\\|else\\|while\\|for\\|return\\)\\>" . font-lock-keyword-face)
      ("\\<\\(int\\|float\\|string\\|bool\\)\\>" . font-lock-type-face)
      ("//.*$" . font-lock-comment-face))))
```

---

## Troubleshooting

### Command Not Found

**Problem:** `knull: command not found`

**Solutions:**
1. Verify the binary exists: `ls target/release/knull`
2. Add to PATH: `export PATH="$PWD/target/release:$PATH"`
3. Use full path: `./target/release/knull`

### Parse Errors

**Problem:** `Error: Expected LBrace, got Identifier`

**Solution:** Check for syntax errors. Common issues:
- Missing braces: `fn main()` should be `fn main() { }`
- Incorrect function call syntax: use `println "text"` not `println("text")`

### Build Failures

**Problem:** Cargo build fails

**Solutions:**
1. Update Rust: `rustup update`
2. Build without LLVM: `cargo build --release --no-default-features`
3. Check for missing dependencies

### Runtime Errors

**Problem:** `Error: undefined variable 'x'`

**Solution:** Ensure variables are declared with `let` before use.

**Problem:** Infinite loops

**Solution:** Verify loop conditions will eventually be false:

```knull
// Correct
let i = 0
while i < 10 {
    i = i + 1  // Increment ensures termination
}

// Incorrect (infinite loop)
let i = 0
while i < 10 {
    // Missing increment
}
```

---

## Quick Reference

### File Extensions

- `.knull` - Knull source files

### Entry Point

- `fn main()` - Required in executable programs

### Common Patterns

**Loop n times:**
```knull
let i = 0
while i < n {
    // Loop body
    i = i + 1
}
```

**Iterate array:**
```knull
let arr = [1, 2, 3, 4, 5]
let i = 0
while i < 5 {
    println arr[i]
    i = i + 1
}
```

**Function with default behavior:**
```knull
fn greet(name) {
    if name == "" {
        return "Hello, World!"
    }
    return "Hello, " + name + "!"
}
```

---

For additional information, consult the Language Guide (docs/GUIDE.md) or Standard Library Reference (docs/STD_LIB.md).
