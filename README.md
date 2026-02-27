# Knull

```
██╗  ██╗ █████╗ ███╗   ██╗██████╗ 
██║ ██╔╝██╔══██╗████╗  ██║██╔══██╗
█████╔╝ ███████║██╔██╗ ██║██║  ██║
██╔═██╗ ██╔══██║██║╚██╗██║██║  ██║
██║  ██╗██║  ██║██║ ╚████║██████╔╝
╚═╝  ╚═╝╚═╝  ╚═╝╚═╝ ╚═══╝╚═════╝ 
```

**The Ultimate Programming Language**

Simple as Python. Fast as C. Powerful as Assembly.

---

## The Three Pillars

Knull adapts to your skill level:

| Mode | Description | Target User |
|------|-------------|-------------|
| **Novice** | Dynamic typing, garbage collected | Beginners, Scripting |
| **Expert** | Static types, ownership, manual memory | Systems Programming |
| **God** | `unsafe`, direct syscalls, bare-metal | Hackers, OS Dev |

---

## Installation

```bash
# Quick install (Linux/macOS)
curl -sSL https://raw.githubusercontent.com/4fqr/knull/main/install.sh | sh

# Build from source
git clone https://github.com/4fqr/knull.git
cd knull
cargo build --release
./target/release/knull --version
```

---

## Quick Start

```bash
# Create your first program
echo 'println "Hello, World!"' > hello.knull

# Run it
knull run hello.knull
```

---

## Examples

### Web Development
```knull
// A simple HTTP server
fn main() {
    let server = std.net.tcp_bind(8080)
    println "Server listening on port 8080"
}
```

### Systems Programming
```knull
// Direct memory access
fn main() {
    let vga = 0xB8000 as *mut u16
    unsafe { *vga = 'K' as u16 | 0x0F00 }
}
```

### Scripting
```knull
// Easy automation
fn main() {
    let files = std.os.list_dir(".")
    for f in files {
        println f
    }
}
```

### Hacking
```knull
// Port scanner
fn scan(host: string, port: u16) -> bool {
    let conn = std.net.tcp_connect(host, port)
    conn.is_some()
}
```

---

## Features

- **Three Language Modes** - Novice to God Mode
- **Self-Hosted Compiler** - Written in Knull, compiling Knull
- **Zero-Cost Abstractions** - High-level without runtime overhead
- **Type Safety** - Powerful type system with generics
- **Package Manager** - Integrated dependency management
- **Cross-Platform** - Linux, macOS, Windows

---

## Documentation

- [Installation Guide](BUILD.md) - Building from source
- [Language Guide](docs/GUIDE.md) - Complete tutorial
- [Standard Library](docs/STD_LIB.md) - API reference
- [Library Index](docs/LIBRARY_INDEX.md) - Module overview

---

## Status

| Component | Status |
|-----------|--------|
| Lexer | Working |
| Parser | Working |
| Code Generator | Working |
| Runtime | Partial |
| Package Manager | Planning |

---

## License

MIT License - See [LICENSE](LICENSE) for details.

---

**Knull** - The language that scales with you from hello world to operating system.
