# Knull Package Manifest Specification

This document defines the `knull.toml` format for Knull packages.

## File Format

The manifest file must be named `knull.toml` and placed at the root of a Knull package.

## Structure

```toml
[package]
name = "package-name"
version = "0.1.0"
edition = "2024"  # Knull edition (optional, defaults to latest)
entry = "src/main.knull"

[dependencies]
# ...

[dev-dependencies]
# ...

[build]
script = "build.knull"  # Build script (optional)

[features]
feature1 = []
feature2 = []
default = ["feature1"]
```

## Sections

### `[package]`

| Field     | Type   | Required | Description                          |
|-----------|--------|----------|--------------------------------------|
| name      | string | Yes      | Package name (kebab-case)            |
| version   | string | Yes      | Semantic version (semver)            |
| edition   | string | No       | Knull edition (default: latest)      |
| entry     | string | Yes      | Entry point file                     |
| authors   | array  | No       | List of author names/emails           |
| license   | string | No       | SPDX license identifier              |
| desc      | string | No       | Short package description            |
| readme    | string | No       | README file path                      |
| homepage  | string | No       | Project homepage URL                 |
| repository| string | No       | Repository URL                       |

### `[dependencies]`

Dependencies are specified as key-value pairs where:
- Key: package name (kebab-case)
- Value: version string or dependency spec

#### Version Specifiers

| Format           | Description                          |
|------------------|--------------------------------------|
| "1.2.3"          | Exact version                        |
| "^1.2.3"         | Compatible (>=1.2.3, <2.0.0)          |
| "~1.2.3"         | Tilde (>=1.2.3, <1.3.0)              |
| ">=1.0.0"        | Minimum version                      |
| ">=1.0.0 <2.0.0" | Range                                |
| "*"              | Any version                          |

#### Dependency Sources

```toml
# Crate registry
serde = "^1.0"

# Git repository
http = { git = "https://github.com/4fqr/http" }

# Git with branch/ref
http = { git = "https://github.com/4fqr/http", branch = "main" }

# Local path
mylib = { path = "../mylib" }

# Features
tokio = { version = "^1.0", features = ["full", "rt-multi-thread"] }
```

### `[dev-dependencies]`

Development-only dependencies (tests, examples, benchmarks).

```toml
[dev-dependencies]
criterion = "^0.5"
```

### `[build]`

Build scripts run before compilation.

```toml
[build]
script = "build.knull"  # Build script
output = "build.rs"     # Generated file (optional)
```

### `[features]`

Conditional compilation flags.

```toml
[features]
default = ["std"]

std = []           # Standard library (enabled by default)
no-alloc = []      # Disable allocations
simd = []          # SIMD support
experimental = []  # Experimental features
```

### `[targets]`

Platform-specific configuration.

```toml
[target.x86_64-unknown-linux-knull]
opt-level = 3
lto = true
```

## Workspace

A workspace contains multiple packages.

```toml
[workspace]
members = [
    "crates/foo",
    "crates/bar",
]
resolver = "2"

[workspace.dependencies]
serde = "^1.0"
tokio = "^1.0"
```

Workspace members can reference shared dependencies:

```toml
[dependencies]
serde = { workspace = true }
```

## Publishing

When publishing to a registry, additional fields are available:

```toml
[package]
name = "my-package"
version = "1.0.0"

[package.metadata.knull]
# Registry-specific metadata
registry = "https://crates.io"
```

## Examples

### Minimal Package

```toml
[package]
name = "hello-world"
version = "0.1.0"
entry = "src/main.knull"
```

### Full Package

```toml
[package]
name = "my-web-server"
version = "1.0.0"
edition = "2024"
entry = "src/server.knull"
authors = ["Alice <alice@example.com>"]
license = "MIT"
desc = "A fast web server written in Knull"
homepage = "https://github.com/user/my-web-server"
repository = "https://github.com/user/my-web-server"
readme = "README.md"

[dependencies]
http = "^0.2"
router = "^1.0"
templates = "^0.1"

[dev-dependencies]
bench = "^0.1"

[features]
default = ["server"]
server = []
client = []

[build]
script = "build.knull"
```

### Library Package

```toml
[package]
name = "json"
version = "2.0.0"
entry = "src/lib.knull"  # Note: lib.knull for libraries

[dependencies]
unicode = "^0.3"

[features]
std = []
```

## Notes

- All paths are relative to the package root
- Version must follow semver
- Package names must be unique within a workspace
- Cyclic dependencies are not allowed
