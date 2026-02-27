# Knull Standard Library Documentation

> The API bible for Knull programmers.

---

## Module Index

| Module | Description |
|--------|-------------|
| `std` | Root module, prelude, common functions |
| `std.mem` | Memory operations: alloc, free, copy |
| `std.sys` | System calls: write, read, fork, exec |
| `std.net` | Networking: TCP, UDP, sockets |
| `std.io` | File I/O, buffered streams |
| `std.string` | String manipulation |
| `std.collections` | Vec, HashMap, Set |
| `std.time` | Time, dates, timers |
| `std.crypto` | Hashing, encryption, RNG |
| `std.thread` | Threads, channels, mutexes |

---

## std - Core Module

### Print Functions

#### `std.print(msg: string)`

Prints a message to stdout.

```knull
std.print("Hello, world!")
```

**Safety:** Safe.

---

#### `std.println(msg: string)`

Prints a message to stdout with a newline.

```knull
std.println("Hello, world!")
```

**Safety:** Safe.

---

#### `std.eprint(msg: string)`

Prints to stderr.

```knull
std.eprint("Error occurred")
```

**Safety:** Safe.

---

#### `std.eprintln(msg: string)`

Prints to stderr with newline.

```knull
std.eprintln("Fatal error!")
```

**Safety:** Safe.

---

### Input Functions

#### `std.read_line() -> string`

Reads a single line from stdin.

```knull
let name = std.read_line()
std.println("You entered: " + name)
```

**Returns:** The line without the trailing newline.

**Safety:** Safe. May block.

---

### Program Control

#### `std.exit(code: i32) -> never`

Exits the program with the given exit code.

```knull
std.exit(0)  // Success
std.exit(1)  // Error
```

**Safety:** Safe. Never returns.

---

#### `std.panic(msg: string) -> never`

Triggers a panic with the given message.

```knull
std.panic("This should never happen")
```

**Safety:** Safe. Never returns.

---

### Option/Result Helpers

#### `std.unwrap<T>(opt: option<T>, msg: string) -> T`

Unwraps an option, panicking if None.

```knull
let value = std.unwrap(some_option, "Expected a value")
```

**Safety:** Unsafe if option may be None.

---

#### `std.unwrap_or<T>(opt: option<T>, default: T) -> T`

Unwraps or returns default.

```knull
let value = std.unwrap_or(some_option, 42)
```

**Safety:** Safe.

---

### Memory Helpers

#### `std.alloc<T>(count: usize) -> *mut T`

Allocates uninitialized memory for `count` elements of type T.

```knull
let ptr = std.alloc::<i32>(10)  // Allocate 10 i32s
```

**Returns:** Raw pointer to zeroed memory.

**Safety:** 
- **Danger:** Must be freed manually to avoid leaks.
- Use `defer std.free(ptr)` for automatic cleanup.

```knull
let ptr = std.alloc::<u8>(1024)
defer std.free(ptr)
```

---

#### `std.free<T>(ptr: *mut T)`

Frees memory allocated with `alloc`.

```knull
let ptr = std.alloc::<i32>(10)
std.free(ptr)
```

**Safety:** 
- **Danger:** Double-free causes undefined behavior.
- **Danger:** Use-after-free causes undefined behavior.

---

#### `std.copy<T>(dest: *mut T, src: *const T, count: usize)`

Copies `count` elements from src to dest.

```knull
std.copy(dest, src, len)
```

**Safety:** 
- **Danger:** Regions must not overlap.
- Use `std.move` for overlapping regions.

---

#### `std.move<T>(dest: *mut T, src: *mut T, count: usize)`

Moves `count` elements, handling overlap.

```knull
std.move(dest, src, len)
```

**Safety:** Safe. Handles overlapping regions.

---

#### `std.zero<T>(ptr: *mut T, count: usize)`

Zeros memory.

```knull
std.zero(ptr, count)
```

**Safety:** Safe.

---

### Array Helpers

#### `std.vec<T>(items: ...T) -> Vec<T>`

Creates a vector from variadic arguments.

```knull
let v = std.vec(1, 2, 3, 4, 5)
```

**Safety:** Safe.

---

#### `std.make<T>(count: usize, value: T) -> Vec<T>`

Creates a vector filled with `count` copies of `value`.

```knull
let zeros = std.make(100, 0)
let threes = std.make(5, 3)
```

**Safety:** Safe.

---

### HashMap Helpers

#### `std.hashmap<K, V>(entries: ...(K, V)) -> HashMap<K, V>`

Creates a hashmap from entries.

```knull
let scores = std.hashmap(
    "Alice" -> 100,
    "Bob" -> 95,
    "Carol" -> 88
)
```

**Safety:** Safe.

---

### String Helpers

#### `std.to_string<T>(value: T) -> string`

Converts a value to string.

```knull
let s = std.to_string(42)        // "42"
let s = std.to_string(true)      // "true"
let s = std.to_string(3.14)      // "3.14"
```

**Safety:** Safe.

---

#### `std.format(template: string, args: ...any) -> string`

Formats a string with arguments.

```knull
let s = std.format("Hello, {}!", "world")
let s = std.format("{} + {} = {}", 1, 2, 3)
```

**Safety:** Safe.

---

### Assertions

#### `std.assert(condition: bool, msg: string)`

Asserts that condition is true, panics with message if false.

```knull
std.assert(x > 0, "x must be positive")
```

**Safety:** Safe (panics if assertion fails).

---

#### `std.assert_eq<T>(a: T, b: T)`

Asserts two values are equal.

```knull
std.assert_eq(actual, expected)
```

**Safety:** Safe.

---

### Debug Helpers

#### `std.dbg<T>(value: T) -> T`

Prints debug info and returns the value.

```knull
let result = std.dbg(compute_something())
// Prints: <file>:<line>: <value>
```

**Safety:** Safe.

---

## std.mem - Memory Module

### Functions

#### `std.mem::alloc<T>(count: usize) -> *mut T`

Allocates zeroed memory for `count` elements.

```knull
let ptr = std.mem::alloc::<u8>(1024)
```

**Safety:** Must free manually.

---

#### `std.mem::free<T>(ptr: *mut T)`

Frees allocated memory.

```knull
std.mem::free(ptr)
```

**Safety:** 
- **Danger:** Don't free twice.
- **Danger:** Don't use after free.

---

#### `std.mem::copy<T>(dest: *mut T, src: *const T, count: usize)`

Copy memory (non-overlapping).

```knull
std.mem::copy(dest, src, n)
```

**Safety:** 
- **Danger:** No overlap allowed.

---

#### `std.mem::move<T>(dest: *mut T, src: *mut T, count: usize)`

Move memory (any overlap).

```knull
std.mem::move(dest, src, n)
```

**Safety:** Safe.

---

#### `std.mem::zero<T>(ptr: *mut T, count: usize)`

Zero memory.

```knull
std.mem::zero(ptr, n)
```

**Safety:** Safe.

---

#### `std.mem::set<T>(ptr: *mut T, value: T, count: usize)`

Set memory to a value.

```knull
std.mem::set(ptr, 0xFF, n)
```

**Safety:** Safe.

---

## std.sys - System Calls

### File Descriptors

```knull
std.sys.STDIN   // 0
std.sys.STDOUT  // 1
std.sys.STDERR  // 2
```

---

### I/O Functions

#### `std.sys.write(fd: i32, data: *u8, count: usize) -> isize`

Write to a file descriptor.

```knull
let msg = "Hello"
let n = std.sys.write(1, msg.as_ptr(), msg.len())
```

**Returns:** Bytes written, or -1 on error.

**Safety:** Low-level, no buffering.

---

#### `std.sys.read(fd: i32, buf: *mut u8, count: usize) -> isize`

Read from a file descriptor.

```knull
let mut buf = [0u8; 1024]
let n = std.sys.read(0, &mut buf, 1024)
```

**Returns:** Bytes read, 0 on EOF, -1 on error.

**Safety:** Low-level, no buffering.

---

#### `std.sys.open(path: *u8, flags: i32, mode: i32) -> i32`

Open a file.

```knull
let fd = std.sys.open("file.txt".as_ptr(), std.sys.O_RDONLY, 0)
```

**Returns:** File descriptor, or -1 on error.

**Safety:** Low-level.

---

#### `std.sys.close(fd: i32)`

Close a file descriptor.

```knull
std.sys.close(fd)
```

**Safety:** Low-level.

---

### Process Functions

#### `std.sys.exit(code: i32) -> never`

Exit the process.

```knull
std.sys.exit(0)
```

**Safety:** Never returns.

---

#### `std.sys.fork() -> i32`

Fork the process.

```knull
let pid = std.sys.fork()
if pid == 0 {
    // Child
} else {
    // Parent
}
```

**Returns:** Child PID in parent, 0 in child, -1 on error.

**Safety:** Unsafe.

---

#### `std.sys.execve(path: *u8, argv: *const *const u8, envp: *const *const u8)`

Execute a program.

```knull
std.sys.execve("/bin/ls".as_ptr(), args, env)
```

**Safety:** Never returns on success. Unsafe.

---

### Memory Functions

#### `std.sys.mmap(addr: *mut u8, length: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8`

Map memory.

```knull
let ptr = std.sys.mmap(null, 4096, 
    std.sys.PROT_READ | std.sys.PROT_WRITE,
    std.sys.MAP_PRIVATE | std.sys.MAP_ANONYMOUS,
    -1, 0)
```

**Safety:** Low-level, unsafe.

---

#### `std.sys.munmap(addr: *mut u8, length: usize) -> i32`

Unmap memory.

```knull
std.sys.munmap(ptr, 4096)
```

**Safety:** Low-level, unsafe.

---

## std.net - Networking

### Constants

```knull
std.net.AF_INET      // IPv4
std.net.AF_INET6     // IPv6
std.net.SOCK_STREAM  // TCP
std.net.SOCK_DGRAM   // UDP
```

---

### Functions

#### `std.net.tcp_connect(host: string, port: u16) -> option<TcpStream>`

Connect to a TCP server.

```knull
let stream = std.net.tcp_connect("127.0.0.1", 8080)!
std.println("Connected!")
```

**Safety:** Safe. May block.

---

#### `std.net.tcp_bind(port: u16) -> option<TcpListener>`

Create a TCP server listening on a port.

```knull
let listener = std.net.tcp_bind(8080)!
std.println("Listening on port 8080")

while let Some(stream) = listener.accept() {
    // Handle connection
}
```

**Safety:** Safe. May block.

---

#### `std.net.tcp_socket() -> option<TcpSocket>`

Create a TCP socket.

```knull
let sock = std.net.tcp_socket()!
```

**Safety:** Safe.

---

### Types

#### `TcpStream`

A connected TCP stream.

```knull
pub struct TcpStream {
    socket: TcpSocket,
}
```

**Methods:**
- `write(data: []u8) -> isize`
- `read(buf: &mut [u8]) -> isize`
- `close()`

---

#### `TcpListener`

A listening TCP socket.

```knull
pub struct TcpListener {
    socket: TcpSocket,
}
```

**Methods:**
- `accept() -> option<TcpStream>`
- `close()`

---

#### `IpAddr`

IPv4 address.

```knull
let ip = std.net.ipv4(192, 168, 1, 1)
let local = std.net.LOCALHOST  // 127.0.0.1
```

**Methods:**
- `to_string() -> string`

---

#### `SocketAddr`

IP address + port.

```knull
let addr = std.net.socket_addr(std.net.LOCALHOST, 8080)
```

---

## std.io - I/O Module

### Types

#### `File`

A file handle.

```knull
let file = std.io.open("data.txt", std.io.READ)!
```

**Methods:**
- `read(buf: &mut [u8]) -> isize`
- `write(data: []u8) -> isize`
- `seek(offset: i64, whence: SeekWhence) -> i64`
- `close()`

---

#### `BufReader<T>`

Buffered reader.

```knull
let reader = std.io.buffered_reader(file)
let line = reader.read_line()!
```

---

#### `BufWriter<T>`

Buffered writer.

```knull
let writer = std.io.buffered_writer(file)
writer.write_string("Hello")!
writer.flush()!
```

---

## std.string - String Module

### Methods on `string`

#### `string::len() -> usize`

Get string length.

```knull
let len = "hello".len()  // 5
```

---

#### `string::is_empty() -> bool`

Check if empty.

```knull
if s.is_empty() { ... }
```

---

#### `string::contains(sub: string) -> bool`

Check if contains substring.

```knull
if "hello world".contains("world") { ... }
```

---

#### `string::split(delimiter: string) -> []string`

Split by delimiter.

```knull
let parts = "a,b,c".split(",")  // ["a", "b", "c"]
```

---

#### `string::trim() -> string`

Trim whitespace.

```knull
let s = "  hello  ".trim()  // "hello"
```

---

#### `string::to_upper() -> string`

Convert to uppercase.

```knull
let s = "hello".to_upper()  // "HELLO"
```

---

#### `string::to_lower() -> string`

Convert to lowercase.

---

#### `string::starts_with(prefix: string) -> bool`

Check prefix.

---

#### `string::ends_with(suffix: string) -> bool`

Check suffix.

---

#### `string::replace(old: string, new: string) -> string`

Replace substring.

```knull
let s = "foo bar".replace("bar", "baz")  // "foo baz"
```

---

## std.collections - Collections Module

### Vec<T>

A growable array.

```knull
let mut v = Vec::new()
v.push(1)
v.push(2)
v.push(3)

let first = v[0]       // 1
let len = v.len()      // 3
let pop = v.pop()      // Some(3)
```

**Methods:**
- `new() -> Vec<T>`
- `with_capacity(cap: usize) -> Vec<T>`
- `push(item: T)`
- `pop() -> option<T>`
- `get(index: usize) -> option<&T>`
- `len() -> usize`
- `is_empty() -> bool`
- `clear()`
- `reserve(additional: usize)`
- `as_slice() -> &[T]`
- `as_mut_slice() -> &mut [T]`

---

### HashMap<K, V>

A hash map.

```knull
let mut map = HashMap::new()
map.insert("key", "value")
let val = map.get("key")  // Some(&"value")
map.remove("key")
```

**Methods:**
- `new() -> HashMap<K, V>`
- `insert(key: K, value: V) -> option<V>`
- `get(key: &K) -> option<&V>`
- `remove(key: &K) -> option<V>`
- `len() -> usize`
- `contains(key: &K) -> bool`
- `clear()`

---

### HashSet<T>

A hash set.

```knull
let mut set = HashSet::new()
set.insert(1)
set.insert(2)
let has = set.contains(1)
```

---

## std.time - Time Module

### Functions

#### `std.time::sleep(duration: Duration)`

Sleep for a duration.

```knull
std.time.sleep(std.time::Duration::from_secs(1))
std.time.sleep(std.time::Duration::from_millis(500))
```

---

#### `std.time::now() -> Instant`

Get current time.

```knull
let start = std.time::now()
// ... do work ...
let elapsed = start.elapsed()
```

---

### Types

#### `Duration`

A time duration.

```knull
std.time::Duration::from_secs(1)
std.time::Duration::from_millis(100)
std.time::Duration::from_micros(50)
```

---

#### `Instant`

A point in time.

```knull
let now = std.time::now()
let later = std.time::now()
let dur = later.since(now)
```

---

## std.crypto - Cryptography

### Functions

#### `std.crypto::sha256(data: []u8) -> [32]u8`

Compute SHA-256 hash.

```knull
let hash = std.crypto::sha256("hello".as_bytes())
```

---

#### `std.crypto::random(len: usize) -> []u8`

Generate random bytes.

```knull
let key = std.crypto::random(32)
```

---

#### `std.crypto::aes_encrypt(data: []u8, key: []u8) -> []u8`

AES encryption.

---

#### `std.crypto::aes_decrypt(data: []u8, key: []u8) -> []u8`

AES decryption.

---

## std.thread - Threading

### Functions

#### `std.thread::spawn(f: fn() -> T) -> thread::JoinHandle<T>`

Spawn a thread.

```knull
let handle = std.thread::spawn(|| {
    std.println("Hello from thread!")
})
handle.join()
```

---

#### `std.thread::current() -> ThreadId`

Get current thread ID.

---

### Types

#### `JoinHandle<T>`

A handle to a spawned thread.

```knull
let handle = std.thread::spawn(|| 42)
let result = handle.join()  // 42
```

**Methods:**
- `join() -> T`

---

#### `Mutex<T>`

A mutual exclusion lock.

```knull
let mutex = Mutex::new(0)
{
    let mut guard = mutex.lock()
    *guard += 1
}
```

---

#### `Channel<T>`

A message channel.

```knull
let (tx, rx) = std.thread::channel()
tx.send(42)
let value = rx.recv()
```

---

## Safety Notes Summary

| Function | Safety Level | Notes |
|----------|--------------|-------|
| `std.print` | Safe | |
| `std.alloc` | **Danger** | Must free manually |
| `std.free` | **Danger** | No double-free |
| `std.copy` | **Danger** | No overlap |
| `std.sys.write` | Low-level | Raw syscall |
| `std.sys.mmap` | **Danger** | Raw memory mapping |
| `std.thread::spawn` | Safe | |

---

*End of Standard Library Documentation*
