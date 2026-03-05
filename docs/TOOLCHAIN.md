# Knull Toolchain v2.0.0

---

## CLI Reference

```
knull [OPTIONS] <COMMAND>

Commands:
  run     <file>              Run a .knull file
  eval    <expr>              Evaluate an expression inline
  repl                        Start interactive REPL
  build   <file>              Compile to binary
  check   <file>              Check syntax only
  fmt     <file>              Format source code
  new     <name>              Create new project
  add     <pkg> <version>     Add dependency
  test                        Run test suite
  version                     Show version
  help                        Show help

Options:
  -v, --verbose               Verbose output
```

---

## Project Management

### Creating a project

```bash
knull new my-app
cd my-app
```

Generated structure:
```
my-app/
├── knull.toml
├── src/
│   └── main.knull
└── tests/
```

### knull.toml

```toml
[package]
name    = "my-app"
version = "0.1.0"
author  = "Your Name"
description = "My Knull app"

[dependencies]
json = "^1.0"
```

### Dependencies

```bash
knull add json ^1.0
knull add crypto ^2.0
knull add sqlite ^1.0
```

Available packages: `json`, `http`, `crypto`, `sqlite`

---

## Building

### Debug build

```bash
knull build src/main.knull
./main
```

### Release build

```bash
knull build --release src/main.knull -o myapp
```

### WebAssembly

```bash
knull build --target wasm32 src/main.knull -o app.wasm
```

---

## Testing

```bash
knull test                    # runs all tests/ *.knull files
knull test tests/math.knull   # single file
```

Test file example:

```knull
// tests/math.knull
assert_eq(1 + 1, 2)
assert_eq(10 % 3, 1)
assert(sqrt(4.0) == 2.0, "sqrt failed")
println("All math tests passed")
```

---

## Formatting

```bash
knull fmt src/main.knull       # format in-place
knull fmt --check src/main.knull   # check without modifying
```

---

## REPL

```bash
knull repl
```

### REPL commands

| Command | Description |
|---------|-------------|
| `:help` | Show help |
| `:quit` | Exit |
| `:vars` | List variables |
| `:fns` | List functions |
| `:reset` | Clear state |
| `:load <file>` | Load file into session |
| `:type <expr>` | Show type |
| `:clear` | Clear screen |

---

## Editor Integration

### VS Code

```bash
cd editor
npm install
npm run compile
code --install-extension knull-lang-*.vsix
```

### LSP (Language Server)

```bash
knull lsp     # starts LSP on stdio
```

Configure your editor to use `knull lsp` as the language server for `.knull` files.

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `KNULL_PATH` | Additional search paths for packages |
| `KNULL_DEBUG` | Enable debug output (1/0) |
| `KNULL_COLOR` | Force/disable color output (auto/always/never) |
