# Building Knull from Source

This guide covers how to build the Knull compiler from source.

## Prerequisites

- **Rust** (1.70+)
- **Git**
- **Linux/macOS/Windows**

## Quick Install (All OS)

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/4fqr/knull/master/install.sh | bash
```

Then add to PATH:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Windows

```powershell
# Install Rust from https://rustup.rs
git clone https://github.com/4fqr/knull.git
cd knull\src
cargo build --release --no-default-features
copy target\release\knull.exe C:\Windows\System32\
```

---

## Manual Build

### Linux / macOS

```bash
# Clone the repository
git clone https://github.com/4fqr/knull.git
cd knull

# Build the compiler
cd src
cargo build --release --no-default-features

# Verify
../target/release/knull --version
```

### Windows

```powershell
# Clone the repository
git clone https://github.com/4fqr/knull.git
cd knull\src

# Build
cargo build --release --no-default-features

# Verify
..\target\release\knull.exe --version
```

---

## Build Commands

```bash
# Development build (faster, no optimizations)
cargo build

# Release build (slower, optimized)
cargo build --release

# Build without default features
cargo build --no-default-features

# Run tests
cargo test

# Format code
cargo fmt
```

---

## Output

The compiled binary is located at:

- **Linux/macOS**: `target/release/knull`
- **Windows**: `target/release/knull.exe`

---

## Installation

### Local (Linux/macOS)

```bash
# Copy to ~/.local/bin
cp target/release/knull ~/.local/bin/

# Or add to PATH
export PATH="$PWD/target/release:$PATH"
```

### System-wide (Linux/macOS)

```bash
# Requires sudo
sudo cp target/release/knull /usr/local/bin/knull
```

### Windows

```cmd
copy target\release\knull.exe C:\Windows\System32\
```

---

## Troubleshooting

### "command not found: knull"

Add to PATH:
```bash
# Linux/macOS
export PATH="$HOME/.local/bin:$PATH"

# Add to ~/.bashrc or ~/.zshrc for permanent
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
```

### Slow build times

Use cached builds:
```bash
cargo build
```

---

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

---

For more help, see [docs/GUIDE.md](docs/GUIDE.md)
