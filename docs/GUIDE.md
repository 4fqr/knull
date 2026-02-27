# Knull Language Guide

The comprehensive guide to programming in Knull.

---

## Introduction

Knull is a versatile programming language designed to scale with you. Whether you're a beginner learning to code, a systems programmer building operating systems, or a hacker pushing the boundaries of what's possible, Knull has a mode for you.

## The Three Modes

Knull operates in three distinct modes, each designed for different skill levels and use cases:

```
┌─────────────────────────────────────────────────────────────┐
│                        KNULL                                │
│  ┌───────────┐   ┌───────────┐   ┌───────────┐            │
│  │  NOVICE   │ → │  EXPERT   │ → │   GOD     │            │
│  │ (Script)  │   │ (Systems) │   │ (Hacking) │            │
│  └───────────┘   └───────────┘   └───────────┘            │
│       ↓               ↓               ↓                     │
│  +---------+     +---------+     +---------+               │
│  │  Safe   │     │ Manual  │     │ Raw     │               │
│  │  GC     │     │ Memory  │     │ Metal   │               │
│  └─────────┘     └─────────┘     └─────────┘               │
└─────────────────────────────────────────────────────────────┘
```

---

## Chapter 1: Novice Mode (Scripting)

Perfect for beginners and automation tasks.

### Syntax

```knull
// Variables (dynamic)
name = "Alice"
age = 25
pi = 3.14

// Print
println "Hello, " + name + "!"
println "Age: " + age

// Arrays
numbers = [1, 2, 3, 4, 5]
println numbers[0]

// Loops
for n in numbers {
    println n
}
```

### Features

- **Dynamic Typing**: No type annotations needed
- **Garbage Collection**: Automatic memory management
- **Simple Syntax**: No semicolons, minimal punctuation

### Example: File Automation

```knull
// Read a file
content = read_file("data.txt")
println content

// Write to a file  
write_file("output.txt", "Hello from Knull!")

// List directory
files = list_dir(".")
for f in files {
    println f
}
```

---

## Chapter 2: Expert Mode (Systems)

For building high-performance applications and systems software.

### Syntax

```knull
// Explicit types
let x: i32 = 42
let mut y: i32 = 100

// Functions
fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Structs
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let p = Point { x: 10, y: 20 }
    println p.x
}
```

### Ownership & Borrowing

```knull
// Ownership
let s1 = String::new("hello")
let s2 = s1  // s1 is moved to s2

// Borrowing
fn len(s: &String) -> usize {
    s.len()
}

let s = String::new("world")
let l = len(&s)  // Borrow s
```

### Example: Web Server

```knull
struct HttpRequest {
    method: string,
    path: string,
    headers: HashMap<string, string>,
}

struct HttpResponse {
    status: i32,
    body: string,
}

fn handle_request(req: HttpRequest) -> HttpResponse {
    HttpResponse {
        status: 200,
        body: "Hello, World!",
    }
}
```

---

## Chapter 3: God Mode (Bare Metal)

For operating system development and low-level programming.

### Unsafe Blocks

```knull
// Direct pointer manipulation
fn main() {
    let ptr = alloc(8)
    
    unsafe {
        // Write directly to memory
        ptr.write(42)
        
        // Read back
        value = ptr.read()
    }
    
    free(ptr)
}
```

### Direct Syscalls

```knull
// Linux syscalls
fn write(fd: i32, buf: *u8, count: usize) -> isize {
    unsafe {
        syscall(1, fd, buf, count)
    }
}

// Exit program
fn exit(code: i32) {
    unsafe {
        syscall(60, code)
    }
}
```

### VGA Memory (Bare Metal)

```knull
// Write to VGA text buffer
fn main() {
    let vga = 0xB8000 as *mut u16
    
    // Write 'K' with green color
    unsafe {
        *vga = 'K' as u16 | 0x0F00
    }
}
```

---

## Chapter 4: Metaprogramming

### Macros

```knull
// Compile-time code generation
macro assert(condition, message) {
    if !condition {
        panic(message)
    }
}

// Usage
assert(x > 0, "x must be positive")
```

### Compile-Time Evaluation

```knull
// Const expressions
const SIZE = 1024 * 1024

// Compile-time function
comptime fn double(x: i32) -> i32 {
    x * 2
}
```

---

## Chapter 5: Networking

### TCP Server

```knull
fn main() {
    let listener = tcp_bind(8080)
    println "Listening on port 8080"
    
    while true {
        let client = listener.accept()
        handle_client(client)
    }
}

fn handle_client(stream: TcpStream) {
    let request = stream.read()
    let response = "HTTP/1.1 200 OK\r\n\r\nHello!"
    stream.write(response)
}
```

### HTTP Client

```knull
fn main() {
    let response = fetch("https://api.github.com")
    println response.body
}
```

---

## Chapter 6: File I/O

### Reading Files

```knull
fn main() {
    // Read entire file
    content = read_file("data.txt")
    
    // Read lines
    lines = read_lines("data.txt")
    for line in lines {
        println line
    }
}
```

### Writing Files

```knull
fn main() {
    // Write string
    write_file("output.txt", "Hello, World!")
    
    // Append
    append_file("log.txt", "New entry\n")
}
```

---

## Error Handling

### Option Type

```knull
fn find_user(id: i32) -> option<User> {
    // Returns Some(user) or None
}

// Usage
user = find_user(123)
if user.is_some() {
    println user.unwrap().name
}
```

### Result Type

```knull
fn parse_number(s: string) -> result<i32, string> {
    // Returns Ok(value) or Err(error)
}

result = parse_number("42")
match result {
    Ok(n) => println "Parsed: " + n,
    Err(e) => println "Error: " + e,
}
```

---

## Collections

### Vec (Dynamic Array)

```knull
fn main() {
    let mut v = Vec::new()
    v.push(1)
    v.push(2)
    v.push(3)
    
    println v.len()
    println v[0]
    
    // Iterate
    for x in v {
        println x
    }
}
```

### HashMap

```knull
fn main() {
    let mut scores = HashMap::new()
    scores.insert("Alice", 100)
    scores.insert("Bob", 95)
    
    alice_score = scores.get("Alice")
    println alice_score
}
```

---

## Summary

Knull adapts to your needs:

| Mode | Use Case | Memory | Syntax |
|------|----------|--------|--------|
| Novice | Scripting, Automation | GC | Simple |
| Expert | Systems, Web, Games | Manual | Strict |
| God | OS Dev, Hacking | None | Raw |

Start with Novice mode and graduate to higher modes as you grow.

---

For API reference, see [STD_LIB.md](STD_LIB.md)
