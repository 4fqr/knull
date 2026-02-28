# Knull

```
                                                       
       ,--.                                            
   ,--/  /|                            ,--,    ,--,    
,---,': / '                          ,--.'|  ,--.'|    
:   : '/ /       ,---,          ,--, |  | :  |  | :    
|   '   ,    ,-+-. /  |       ,'_ /| :  : '  :  : '    
'   |  /    ,--.'|'   |  .--. |  | : |  ' |  |  ' |    
|   ;  ;   |   |  ,"' |,'_ /| :  . | '  | |  '  | |    
:   '   \  |   | /  | ||  ' | |  . . |  | :  |  | :    
|   |    ' |   | |  | ||  | ' |  | | '  : |__'  : |__  
'   : |.  \|   | |  |/ :  | : ;  ; | |  | '.'|  | '.'| 
|   | '_\.'|   | |--'  '  :  `--'   \;  :    ;  :    ; 
'   : |    |   |/      :  ,      .-./|  ,   /|  ,   /  
;   |,'    '---'        `--`----'     ---`-'  ---`-'   
'---'                                                  
                                                       
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
# Install Rust first from https://rustup.rs
git clone https://github.com/4fqr/knull.git
cd knull\src
cargo build --release --no-default-features
copy target\release\knull.exe C:\Windows\System32\
```

### Build from Source

```bash
git clone https://github.com/4fqr/knull.git
cd knull/src
cargo build --release --no-default-features
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
    println "Server listening on port 8080"
    // Full implementation in examples/demo_web.knull
}
```

### Systems Programming
```knull
// Direct memory access
fn main() {
    println "VGA buffer at 0xB8000"
    // Full implementation in examples/micro_kernel.knull
}
```

### Scripting
```knull
// Easy automation
fn main() {
    println "Hello from Knull!"
    let x = 42
    println "x = " + x
}
```

### Hacking
```knull
// Port scanner
fn scan(host, port) {
    println "Scanning " + host + ":" + port
    // Full implementation in examples/demo_hack.knull
}

fn main() {
    scan("127.0.0.1", 8080)
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
