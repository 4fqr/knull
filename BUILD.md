# Building Knull from Source

This guide covers how to build the Knull compiler from source.

## Prerequisites

- **Rust** (1.70+)
- **Git**
- **Linux/macOS/Windows**

## Quick Build

```bash
# Clone the repository
git clone https://github.com/4fqr/knull.git
cd knull

# Build the compiler
cd src
cargo build --release

# Verify installation
../target/release/knull --version
```

## Requirements

### Required

- **Rust**: Install via [rustup.rs](https://rustup.rs)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Git**: Pre-installed on most systems

### Optional

- **LLVM**: For advanced JIT compilation (optional, not required for basic build)

## Build Commands

```bash
# Development build (faster, no optimizations)
cargo build

# Release build (slower, optimized)
cargo build --release

# With debug symbols
cargo build --profile debug

# Build without default features
cargo build --no-default-features

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## Output

The compiled binary is located at:

- **Debug**: `target/debug/knull`
- **Release**: `target/release/knull`

## Installation

### Local

```bash
# Copy to ~/.local/bin
cp target/release/knull ~/.local/bin/knull

# Or add to PATH
export PATH="$PWD/target/release:$PATH"
```

### System-wide (Linux/macOS)

```bash
# Requires sudo
sudo cp target/release/knull /usr/local/bin/knull
```

## Cross-Compilation

### Linux x86_64 to ARM64

```bash
# Install target
rustup target add aarch64-unknown-linux-gnu

# Build
cargo build --release --target aarch64-unknown-linux-gnu
```

### Windows (from Linux)

```bash
# Install target
rustup target add x86_64-pc-windows-gnu

# Build with MinGW
cargo build --release --target x86_64-pc-windows-gnu
```

## Troubleshooting

### "cc: command not found"

Install a C compiler:
- **Linux**: `sudo apt install build-essential` or `sudo dnf install gcc`
- **macOS**: `xcode-select --install`

### "linker not found"

Ensure LLVM or GCC is installed and in PATH.

### Slow build times

Use cached builds:
```bash
# Keep build artifacts
cargo build

# Clean and rebuild
cargo clean && cargo build --release
```

## Verification

```bash
# Check version
knull --version

# Run a test file
echo 'println "Knull works!"' > test.knull
knull run test.knull
```

Expected output:
```
Running: test.knull
Lexed X tokens
Parsed successfully
Knull works!
```

## Editor Setup

### VS Code

1. Install the Knull extension
2. Or use the syntax file in `editor/syntaxes/knull.tmLanguage.json`

### Vim

Add to `~/.vim/filetype.vim`:
```vim
augroup filetypedetect
  au! BufNewFile,BufRead *.knull set filetype=knull
augroup END
```

---

For more help, see [docs/GUIDE.md](docs/GUIDE.md)
