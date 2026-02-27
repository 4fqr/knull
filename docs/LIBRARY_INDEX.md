# Knull Library Index

Complete index of all standard library modules and their use cases.

---

## Core Modules

| Module | Description | Use Case |
|--------|-------------|----------|
| `std` | Root module, prelude | Everything |
| `std.io` | Input/Output | File I/O, console |
| `std.mem` | Memory operations | Allocation, pointers |
| `std.sys` | System calls | OS interaction |

---

## Networking

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.net` | TCP/UDP sockets | Servers, clients |
| `std.http` | HTTP protocol | REST APIs, web |
| `std.json` | JSON parsing | Data exchange |
| `std.url` | URL handling | URI parsing |

### Example

```knull
// HTTP server
let server = std.net.tcp_bind(8080)
```

---

## Web Development

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.web` | Web framework | Full-stack apps |
| `std.tmpl` | Templates | HTML rendering |
| `std.cookie` | Cookie handling | Sessions |
| `std.static` | Static files | Assets |

---

## Graphics & Games

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.gfx` | Graphics | 2D/3D rendering |
| `std.window` | Windowing | GUI applications |
| `std.audio` | Audio | Sound playback |
| `std.game` | Game engine | Game development |

---

## Systems Programming

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.fs` | Filesystem | File operations |
| `std.process` | Processes | Spawn, fork, exec |
| `std.thread` | Threading | Concurrency |
| `std.env` | Environment | ENV vars, args |

---

## Security & Hacking

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.crypto` | Cryptography | Encryption, hashing |
| `std.sec` | Security | Permissions, capabilities |
| `std.hack` | Hacking tools | Analysis, injection |
| `std.net.raw` | Raw sockets | Packet crafting |

### Example

```knull
// Port scanner
fn scan_port(host: string, port: u16) -> bool {
    let conn = std.net.tcp_connect(host, port)
    conn.is_some()
}
```

---

## Mathematics

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.math` | Basic math | Arithmetic, trig |
| `std.rand` | Random numbers | RNG, seeding |
| `std.complex` | Complex numbers | Scientific computing |
| `std.matrix` | Matrices | Linear algebra |

---

## Data Structures

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.vec` | Vector | Dynamic arrays |
| `std.map` | Hash map | Key-value storage |
| `std.set` | Hash set | Unique collections |
| `std.tree` | Trees | Hierarchical data |

---

## Text & Strings

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.str` | String operations | Manipulation |
| `std.regex` | Regular expressions | Pattern matching |
| `std.utf8` | UTF-8 handling | Unicode |
| `std.format` | Formatting | String interpolation |

---

## Time & Date

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.time` | Time operations | Timestamps |
| `std.date` | Date handling | Calendars |
| `std.tz` | Timezones | Internationalization |

---

## Async & Concurrency

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.async` | Async runtime | Non-blocking I/O |
| `std.channel` | Message passing | Actor model |
| `std.sync` | Synchronization | Mutex, atomic |
| `std.future` | Futures | Async computation |

---

## Debugging

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.dbg` | Debug output | Logging |
| `std.prof` | Profiling | Performance |
| `std.trap` | Breakpoints | Debugging |

---

## FFI (Foreign Function Interface)

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.ffi` | C interop | Calling C libraries |
| `std.asm` | Inline assembly | CPU-specific code |

### Example

```knull
// Call C function
extern "C" {
    fn printf(fmt: *const u8, ...) -> i32
}
```

---

## God Mode (Bare Metal)

| Module | Description | Use Case |
|--------|-------------|----------|
| `std.boot` | Bootloader | System startup |
| `std.pic` | PIC interrupts | Hardware interrupts |
| `std.paging` | Memory paging | Virtual memory |
| `std.gdt` | GDT | Segmentation |

---

## Summary by Application

| Domain | Modules |
|--------|---------|
| **Web APIs** | `std.net`, `std.http`, `std.json`, `std.web` |
| **CLI Tools** | `std.io`, `std.fs`, `std.process`, `std.args` |
| **Systems** | `std.sys`, `std.thread`, `std.fs`, `std.process` |
| **Games** | `std.gfx`, `std.audio`, `std.window`, `std.math` |
| **Hacking** | `std.net.raw`, `std.hack`, `std.crypto`, `std.asm` |
| **OS Dev** | `std.boot`, `std.paging`, `std.gdt`, `std.asm` |
| **Science** | `std.math`, `std.matrix`, `std.complex` |
| **Scripts** | `std.io`, `std.fs`, `std.str`, `std.regex` |

---

For detailed API documentation, see [STD_LIB.md](STD_LIB.md)
