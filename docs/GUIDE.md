# Knull Language Guide

The comprehensive guide to programming in Knull.

---

## Introduction

Knull is a versatile programming language designed to scale with you.

---

## The Three Modes

```
+=========================================================+
|                        KNULL                             |
|  +-----------+   +-----------+   +-----------+         |
|  |  NOVICE   |-> |  EXPERT   |-> |   GOD     |         |
|  | (Script)  |   | (Systems) |   | (Hacking) |         |
|  +-----------+   +-----------+   +-----------+         |
+=========================================================+
```

---

## Chapter 1: Novice Mode (Scripting)

Perfect for beginners and automation tasks.

### Syntax

```knull
// Variables
name = "Alice"
age = 25

// Print
println "Hello, " + name
println "Age: " + age
```

---

## Chapter 2: Expert Mode (Systems)

For building high-performance applications and systems software.

### Syntax

```knull
// Functions
fn add(a, b) {
    a + b
}

fn main() {
    let x = 5 + 3
    let y = x * 2
    println y
}
```

### Variables

```knull
// Mutable variable
let mut counter = 0
counter = counter + 1
```

### Structs

```knull
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let p = Point { x: 10, y: 20 }
    println p.x
}
```

---

## Chapter 3: God Mode (Bare Metal)

For operating system development and low-level programming.

### Unsafe Blocks

```knull
fn main() {
    let ptr = alloc(8)
    unsafe {
        // Direct memory access
    }
    free(ptr)
}
```

### VGA Memory

```knull
fn main() {
    let vga = 0xB8000 as *mut u16
    unsafe {
        *vga = 'K' as u16 | 0x0F00
    }
}
```

---

## Basic Examples

### Hello World

```knull
fn main() {
    println "Hello, World!"
}
```

### Variables and Math

```knull
fn main() {
    let x = 10
    let y = 20
    let sum = x + y
    println sum
}
```

### Loops

```knull
fn main() {
    let i = 0
    while i < 10 {
        println i
        i = i + 1
    }
}
```

### Functions

```knull
fn double(n) {
    n * 2
}

fn main() {
    let result = double(21)
    println result
}
```

---

## Error Handling

### Option Type

```knull
fn find_user(id) {
    // Returns option
}

fn main() {
    user = find_user(1)
    if user.is_some() {
        println user
    }
}
```

---

## Collections

### Vec

```knull
fn main() {
    let v = Vec::new()
    v.push(1)
    v.push(2)
    v.push(3)
    println v.len()
}
```

---

## Summary

Knull adapts to your needs:

| Mode | Use Case | Memory |
|------|----------|--------|
| Novice | Scripting | GC |
| Expert | Systems | Manual |
| God | OS Dev | None |

Start with simple scripts and graduate to higher modes as you grow.

---

For API reference, see [STD_LIB.md](STD_LIB.md)
