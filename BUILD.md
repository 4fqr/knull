# Building Knull

---

## Prerequisites

- **Rust** 1.70+ (`rustup` recommended)
- **Git**
- **LLVM** 14+ (optional, for `--features llvm-backend`)
- **GCC / Clang** (for C codegen backend)

---

## Quick Build (No LLVM)

```bash
git clone https://github.com/4fqr/knull.git
cd knull/src
cargo build --release --no-default-features
./target/release/knull version
```

## Build with LLVM

```bash
# Install LLVM (Ubuntu/Debian)
sudo apt install llvm-14 llvm-14-dev

cd knull/src
cargo build --release
```

## Debug Build

```bash
cd src
cargo build --no-default-features
# Binary: target/debug/knull
```

## Run Tests

```bash
# Rust unit tests
cd src && cargo test --no-default-features

# Knull integration tests
cd ..
bash test_all.sh
# Expected: 33 passed, 0 failed
```

## Build Targets

| Target | Command |
|--------|---------|
| Native (interpreter) | `cargo build --no-default-features` |
| Native (LLVM) | `cargo build --features llvm-backend` |
| WASM output | `knull build --target wasm32 file.knull` |
| C codegen | `knull build file.knull` (default) |

## Install Locally

```bash
# Linux/macOS
cp src/target/release/knull ~/.local/bin/

# Or use the install script
bash install.sh
```

## Uninstall

```bash
bash uninstall.sh
# or
rm ~/.local/bin/knull
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `llvm-backend` | Enable LLVM compilation |
| `lsp` | Enable LSP language server |
| `debugger` | Enable interactive debugger |

Default (no flags): interpreter + C codegen only.
