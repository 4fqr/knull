# Novice Mode Guide

## Overview

Novice mode is Knull's beginner-friendly mode with dynamic typing and garbage collection. It provides a gentle learning curve while still offering the full power of the Knull language.

## Features

- **Type inference at runtime** - No need to specify types
- **Automatic memory management (GC)** - No manual memory management
- **Bounds checking** - Safe array access
- **No unsafe code** - All operations are safe by default

## When to Use

- Learning programming concepts
- Rapid prototyping
- Scripting tasks
- Small utility programs
- Educational purposes

## Mode Declaration

```knull
mode novice

fn main() {
    // Your code here
}
```

The `mode novice` declaration is optional at the file level - if no mode is specified, novice mode is assumed for simple scripts.

## Variables and Types

### Type Inference

In novice mode, types are automatically inferred at runtime:

```knull
mode novice

fn main() {
    let x = 42          // Type inferred as i64
    let name = "Knull"  // Type inferred as String
    let items = [1, 2, 3]  // Type inferred as Vec<i64>
    let flag = true     // Type inferred as bool
    let pi = 3.14159    // Type inferred as f64
}
```

### Dynamic Typing

Variables can hold values of different types over time:

```knull
mode novice

fn main() {
    let value = 42      // Initially an integer
    value = "hello"     // Now a string
    value = [1, 2, 3]   // Now an array
    println(value)      // Prints: [1, 2, 3]
}
```

### Common Types

| Type | Example | Description |
|------|---------|-------------|
| `i64` | `42`, `-17` | 64-bit signed integer |
| `f64` | `3.14`, `-0.5` | 64-bit floating point |
| `String` | `"hello"` | UTF-8 string |
| `bool` | `true`, `false` | Boolean value |
| `Vec<T>` | `[1, 2, 3]` | Dynamic array |
| `HashMap<K, V>` | `{"a": 1}` | Key-value map |

## Memory Management

### Garbage Collection

Novice mode uses automatic garbage collection. You don't need to manually free memory:

```knull
mode novice

fn main() {
    // Memory is automatically allocated
    let data = [1, 2, 3, 4, 5]
    
    // Create nested structures
    let matrix = [
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9]
    ]
    
    // Memory is automatically reclaimed when no longer needed
    println("No manual memory management required!")
}
```

### GC Control (Optional)

While automatic, you can control the GC if needed:

```knull
mode novice

fn main() {
    // Force garbage collection
    gc_collect()
    
    // Get GC statistics
    let stats = gc_stats()
    println("GC stats: {} objects, {} bytes", stats.objects, stats.bytes)
}
```

## Control Flow

### If-Else

```knull
mode novice

fn main() {
    let score = 85
    
    if score >= 90 {
        println("Grade: A")
    } else if score >= 80 {
        println("Grade: B")
    } else if score >= 70 {
        println("Grade: C")
    } else {
        println("Grade: D or F")
    }
}
```

### Loops

```knull
mode novice

fn main() {
    // While loop
    let i = 0
    while i < 5 {
        println("Iteration: {}", i)
        i = i + 1
    }
    
    // For loop with range
    for i in 0..5 {
        println("For iteration: {}", i)
    }
    
    // For each
    let items = ["apple", "banana", "cherry"]
    for item in items {
        println("Item: {}", item)
    }
    
    // Loop forever (use break to exit)
    let count = 0
    loop {
        println("Count: {}", count)
        count = count + 1
        if count >= 3 {
            break
        }
    }
}
```

## Functions

### Defining Functions

```knull
mode novice

// Simple function
fn greet(name) {
    println("Hello, {}!", name)
}

// Function with return value
fn add(a, b) {
    return a + b
}

// Function with default parameter
fn greet_with_default(name = "World") {
    println("Hello, {}!", name)
}

fn main() {
    greet("Alice")
    
    let result = add(5, 3)
    println("5 + 3 = {}", result)
    
    greet_with_default()
    greet_with_default("Bob")
}
```

### First-Class Functions

Functions can be assigned to variables and passed as arguments:

```knull
mode novice

fn apply_twice(f, x) {
    return f(f(x))
}

fn main() {
    // Anonymous function (closure)
    let double = |x| x * 2
    
    println(double(5))              // 10
    println(apply_twice(double, 3)) // 12
    
    // Function as parameter
    let numbers = [1, 2, 3, 4, 5]
    let doubled = numbers.map(|n| n * 2)
    println(doubled)                // [2, 4, 6, 8, 10]
}
```

## Collections

### Arrays (Vec)

```knull
mode novice

fn main() {
    // Create array
    let fruits = ["apple", "banana", "cherry"]
    
    // Access elements
    println(fruits[0])      // "apple"
    println(fruits[1])      // "banana"
    
    // Modify elements
    fruits[0] = "apricot"
    
    // Add elements
    fruits.push("date")
    
    // Remove elements
    let last = fruits.pop()
    
    // Array operations
    println(fruits.len())           // 3
    println(fruits.is_empty())      // false
    
    // Bounds checking (safe)
    // println(fruits[10])          // Runtime error: index out of bounds
}
```

### Hash Maps

```knull
mode novice

fn main() {
    // Create hash map
    let scores = {
        "Alice": 95,
        "Bob": 87,
        "Carol": 92
    }
    
    // Access values
    println(scores["Alice"])        // 95
    
    // Check if key exists
    if scores.contains("Dave") {
        println("Dave's score: {}", scores["Dave"])
    }
    
    // Add/modify entries
    scores["Dave"] = 88
    scores["Alice"] = 96
    
    // Iterate
    for name, score in scores {
        println("{}: {}", name, score)
    }
}
```

## String Operations

```knull
mode novice

fn main() {
    let s = "Hello, World!"
    
    // String methods
    println(s.len())              // 13
    println(s.to_upper())         // "HELLO, WORLD!"
    println(s.to_lower())         // "hello, world!"
    println(s.contains("World"))  // true
    println(s.starts_with("Hello")) // true
    println(s.ends_with("!"))     // true
    
    // String manipulation
    let parts = s.split(", ")     // ["Hello", "World!"]
    let trimmed = "  hello  ".trim() // "hello"
    let replaced = s.replace("World", "Knull") // "Hello, Knull!"
    
    // String concatenation
    let name = "Alice"
    let greeting = "Hello, " + name + "!"
    
    // String interpolation
    let age = 30
    let message = "{} is {} years old".format(name, age)
}
```

## Input/Output

```knull
mode novice

fn main() {
    // Print output
    print("Enter your name: ")
    
    // Read input
    let name = read_line()
    
    // File I/O
    file_write("output.txt", "Hello, World!")
    let content = file_read("output.txt")
    println(content)
    
    // Check file exists
    if file_exists("output.txt") {
        println("File exists!")
    }
}
```

## Error Handling

### Option Type

```knull
mode novice

fn find_index(items, target) {
    let i = 0
    while i < items.len() {
        if items[i] == target {
            return Some(i)
        }
        i = i + 1
    }
    return None
}

fn main() {
    let numbers = [10, 20, 30, 40, 50]
    
    // Using Option
    let result = find_index(numbers, 30)
    match result {
        Some(index) => println("Found at index: {}", index),
        None => println("Not found"),
    }
    
    // Or use if-let
    if let Some(index) = find_index(numbers, 100) {
        println("Found at: {}", index)
    } else {
        println("100 not found")
    }
}
```

### Result Type

```knull
mode novice

fn divide(a, b) {
    if b == 0 {
        return Err("Cannot divide by zero")
    }
    return Ok(a / b)
}

fn main() {
    let result = divide(10, 2)
    match result {
        Ok(value) => println("Result: {}", value),
        Err(msg) => println("Error: {}", msg),
    }
    
    // Using ? operator (propagates errors)
    let value = divide(10, 0)?  // Returns early with Err if failed
}
```

## Pattern Matching

```knull
mode novice

fn describe_value(value) {
    match value {
        0 => "zero",
        1 => "one",
        n if n < 0 => "negative",
        n if n > 100 => "large",
        _ => "small positive",
    }
}

fn main() {
    println(describe_value(0))    // "zero"
    println(describe_value(-5))   // "negative"
    println(describe_value(50))   // "small positive"
    println(describe_value(200))  // "large"
}
```

## Best Practices

### Naming Conventions

- Use `snake_case` for functions and variables
- Use descriptive names

```knull
// Good
fn calculate_average(scores) { }
let total_score = 100

// Avoid
fn calc_avg(s) { }
let ts = 100
```

### Code Organization

```knull
mode novice

// Constants at the top
const MAX_RETRIES = 3
const DEFAULT_TIMEOUT = 30

// Helper functions
fn helper_function() { }

// Main logic
fn process_data(data) { }

// Entry point
fn main() {
    let data = load_data()
    process_data(data)
}
```

### Error Handling

Always handle potential errors gracefully:

```knull
mode novice

fn safe_file_read(path) {
    if !file_exists(path) {
        println("Warning: File '{}' not found", path)
        return ""
    }
    
    return file_read(path)
}

fn main() {
    let content = safe_file_read("config.txt")
    if content == "" {
        println("Using default configuration")
    }
}
```

## Complete Example

```knull
mode novice

// Simple calculator program

fn add(a, b) { a + b }
fn subtract(a, b) { a - b }
fn multiply(a, b) { a * b }
fn divide(a, b) {
    if b == 0 {
        println("Error: Cannot divide by zero")
        return 0
    }
    return a / b
}

fn main() {
    println("=== Knull Calculator ===")
    
    loop {
        print("Enter first number (or 'quit' to exit): ")
        let input = read_line()
        
        if input == "quit" {
            println("Goodbye!")
            break
        }
        
        let a = input.to_int()
        
        print("Enter operation (+, -, *, /): ")
        let op = read_line()
        
        print("Enter second number: ")
        let b = read_line().to_int()
        
        let result = match op {
            "+" => add(a, b),
            "-" => subtract(a, b),
            "*" => multiply(a, b),
            "/" => divide(a, b),
            _ => {
                println("Unknown operation: {}", op)
                continue
            }
        }
        
        println("Result: {}", result)
    }
}
```

## Transitioning to Expert Mode

When you're ready for more control:

1. **Static Typing**: Add type annotations
2. **Memory Management**: Learn ownership and borrowing
3. **Performance**: Optimize with manual memory control

See [EXPERT_MODE.md](EXPERT_MODE.md) for details.

---

*Novice mode makes programming accessible while preparing you for more advanced concepts.*
