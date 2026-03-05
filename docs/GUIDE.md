# Knull Language Guide v2.0.0

A comprehensive guide to the Knull programming language.

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Variables](#variables)
3. [Types](#types)
4. [Operators](#operators)
5. [Control Flow](#control-flow)
6. [Functions](#functions)
7. [Arrays](#arrays)
8. [Maps](#maps)
9. [Structs](#structs)
10. [Pattern Matching](#pattern-matching)
11. [Error Handling](#error-handling)
12. [Closures and Functional Programming](#closures)
13. [Concurrency](#concurrency)
14. [Standard Library Overview](#standard-library-overview)

---

## Getting Started

Your first Knull program:

```knull
println("Hello, World!")
```

Run it:

```bash
knull run hello.knull
```

Or evaluate inline:

```bash
knull eval 'println("Hello, Knull!")'
```

---

## Variables

```knull
let x = 42
let name = "Knull"
let pi = 3.14159
let flag = true

x = 100          // reassignment

let y = x + 10
println(y)       // 110
```

---

## Types

| Type | Example | Notes |
|------|---------|-------|
| number | 42, 3.14 | integer or float |
| string | "hello" | Unicode UTF-8 |
| bool | true, false | |
| array | [1, 2, 3] | heterogeneous |
| map | {"k": v} | string keys |
| null | null | |
| fn | fn(x){ x } | first-class function |

Type inspection:

```knull
println(type_of(42))        // number
println(type_of("hi"))      // string
println(type_of([1,2,3]))   // array
println(type_of(null))      // null
```

---

## Operators

### Arithmetic

```knull
println(10 + 3)    // 13
println(10 - 3)    // 7
println(10 * 3)    // 30
println(10 / 3)    // 3.333...
println(10 % 3)    // 1
println(2 ** 8)    // 256
```

### Comparison

```knull
println(5 > 3)     // true
println(5 < 3)     // false
println(5 >= 5)    // true
println(5 <= 4)    // false
println(5 == 5)    // true
println(5 != 4)    // true
```

### Logical

```knull
println(true && false)   // false
println(true || false)   // true
println(!true)           // false
```

### String concatenation

```knull
let s = "Hello" + ", " + "World!"
println(s)   // Hello, World!
```

---

## Control Flow

### if / else if / else

```knull
let x = 42
if x > 100 {
    println("big")
} else if x > 10 {
    println("medium")
} else {
    println("small")
}
```

`if` as expression:

```knull
let label = if x > 50 { "high" } else { "low" }
```

### while loop

```knull
let i = 0
while i < 5 {
    println(i)
    i = i + 1
}
```

### for loop

```knull
for item in [10, 20, 30] {
    println(item)
}

for i in range(0, 5) {
    println(i)
}
```

### break / continue

```knull
let i = 0
while true {
    if i >= 3 { break }
    i = i + 1
}
```

---

## Functions

### Basic definition

```knull
fn add(a, b) {
    a + b
}

println(add(3, 4))   // 7
```

Last expression is the implicit return value.

### Explicit return

```knull
fn clamp(x, lo, hi) {
    if x < lo { return lo }
    if x > hi { return hi }
    x
}
```

### Recursive functions

```knull
fn fib(n) {
    if n <= 1 { n }
    else { fib(n-1) + fib(n-2) }
}

println(fib(10))   // 55
```

### Default parameters

```knull
fn greet(name, greeting = "Hello") {
    greeting + ", " + name + "!"
}

println(greet("Alice"))          // Hello, Alice!
println(greet("Bob", "Hi"))      // Hi, Bob!
```

### Variadic

```knull
fn sum(..nums) {
    let total = 0
    for n in nums { total = total + n }
    total
}

println(sum(1, 2, 3, 4))   // 10
```

---

## Arrays

```knull
let arr = [1, 2, 3, 4, 5]

println(arr[0])            // 1
println(arr.len())         // 5

arr.push(6)
let top = arr.pop()        // 6
let first = arr.shift()    // 1

let doubled = arr.map(fn(x) { x * 2 })
let evens   = arr.filter(fn(x) { x % 2 == 0 })
let sum     = arr.reduce(0, fn(acc, x) { acc + x })
let sorted  = arr.sort()
let rev     = arr.reverse()
let slice   = arr.slice(1, 3)

println(min(arr))    // minimum value in array
println(max(arr))    // maximum value in array
```

---

## Maps

```knull
let m = {"name": "Alice", "age": 30}

println(m["name"])       // Alice
m["city"] = "London"
println(m.keys())        // ["name", "age", "city"]
println(m.has("name"))   // true
m.remove("city")
println(m.len())         // 2
```

---

## Structs

```knull
struct Point {
    x,
    y
}

let p = Point { x: 10, y: 20 }
println(p.x)    // 10
```

### With methods

```knull
struct Circle {
    radius
}

impl Circle {
    fn area(self) {
        3.14159 * self.radius * self.radius
    }
}

let c = Circle { radius: 5 }
println(c.area())
```

---

## Pattern Matching

```knull
let x = 3

match x {
    1 => println("one"),
    2 | 3 => println("two or three"),
    n if n > 10 => println("big"),
    _ => println("other"),
}
```

### Struct destructuring

```knull
match p {
    Point { x: 0, y: 0 } => println("origin"),
    Point { x, y }        => println(to_string(x) + "," + to_string(y)),
}
```

---

## Error Handling

```knull
fn divide(a, b) {
    if b == 0 { throw "division by zero" }
    a / b
}

try {
    println(divide(10, 0))
} catch e {
    println("Error: " + e)
}
```

---

## Closures

```knull
let double = fn(x) { x * 2 }
println(double(7))    // 14

// capturing outer variable
let factor = 3
let triple = fn(x) { x * factor }
println(triple(5))    // 15

// higher-order
println([1,2,3].map(fn(n) { n * n }))
```

---

## Concurrency

```knull
let handle = spawn {
    42
}
println(handle.join())   // 42

let ch = channel()
spawn { ch.send(99) }
println(ch.recv())       // 99
```

---

## Standard Library Overview

See `docs/STD_LIB.md` for full reference.

| Category | Key functions |
|----------|--------------|
| I/O | println, print, input |
| Math | sqrt, abs, floor, ceil, pow, sin, cos, log, min, max |
| Strings | len, to_upper, to_lower, trim, split, contains, replace |
| Arrays | push, pop, map, filter, reduce, sort, reverse, slice, zip |
| Maps | keys, values, has, remove, len |
| System | env, clock, exit |
| Files | fs_read, fs_write, fs_exists, fs_delete |
| JSON | json_parse, json_stringify |
| Network | http_get, http_post, tcp_connect |
