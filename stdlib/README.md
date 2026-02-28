# Knull Standard Library

The Knull Standard Library provides essential functionality for building applications in Knull.

## Modules

### Core (`stdlib/core/`)
Essential types and functions:
- `Option<T>` - Nullable values
- `Result<T, E>` - Error handling
- Memory management utilities
- Panic and assertion functions
- String utilities (strlen, strcmp, strcpy, etc.)
- Range iterators
- Comparison utilities (min, max, clamp, abs)

### IO (`stdlib/io/`)
Input/Output operations:
- File operations (open, read, write, close)
- Buffered reader/writer
- Path operations
- Standard streams

### Math (`stdlib/math/`)
Mathematical functions:
- Constants (PI, E, TAU, PHI)
- Trigonometry (sin, cos, tan)
- Hyperbolic functions (sinh, cosh, tanh)
- Exponential and logarithmic (exp, ln, pow, sqrt)
- Rounding functions (floor, ceil, round)
- Number theory (gcd, lcm, is_prime, factorial, fibonacci)
- Random number generation

### Collections (`stdlib/collections/`)
Data structures:
- `Vec<T>` - Dynamic array
- `Stack<T>` - LIFO stack
- `Queue<T>` - FIFO queue
- `List<T>` - Linked list
- `Map<K, V>` - Hash map
- `Set<T>` - Hash set

### Net (`stdlib/net/`)
Network programming:
- TCP sockets (TcpStream, TcpListener)
- UDP sockets (UdpSocket)
- HTTP client and server
- URL parsing

### Async (`stdlib/async/`)
Asynchronous programming:
- Futures and Promises
- Task scheduler
- Channels for message passing
- Timeout and delay functions

## Usage

```knull
use core
use collections
use math

fn main() {
    // Using Option
    let opt = Some(42)
    if is_some(opt) {
        println("Value: " + unwrap(opt))
    }
    
    // Using Vec
    let v = vec_new<i64>()
    vec_push(&v, 10)
    vec_push(&v, 20)
    
    // Using math
    let angle = PI / 4.0
    println("sin(45°) = " + sin(angle))
}
```

## Building

The standard library is included with the Knull compiler. To use it:

```bash
knull run --stdlib myprogram.knull
```

Or import specific modules:

```knull
use stdlib/core
use stdlib/collections/vec
```
