# Expert Mode Guide

## Overview

Expert mode adds static typing, ownership, and compile-time checks to Knull. It provides memory safety without garbage collection through an ownership system inspired by Rust.

## When to Use

- Systems programming
- High-performance applications
- Memory-constrained environments
- When you need deterministic performance
- Production software requiring reliability

## Mode Declaration

```knull
mode expert

fn main() {
    // Your code here
}
```

## Static Typing

### Type Annotations

All variables must have explicit or inferred types:

```knull
mode expert

fn main() {
    let x: i32 = 42
    let name: String = String::from("Knull")
    let items: Vec<i32> = vec![1, 2, 3]
    let flag: bool = true
}
```

### Function Signatures

Functions must declare parameter and return types:

```knull
mode expert

fn add(a: i32, b: i32) -> i32 {
    a + b  // Implicit return (no semicolon)
}

fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

fn process(data: Vec<u8>) -> Result<String, Error> {
    // Process and return result
}
```

## Ownership System

### Core Rules

1. Each value has exactly one owner
2. When the owner goes out of scope, the value is dropped
3. Ownership can be moved or borrowed

### Move Semantics

When you assign a value to another variable, ownership is moved:

```knull
mode expert

fn main() {
    let s1 = String::from("hello")
    let s2 = s1          // s1 moved to s2
    
    // println!("{}", s1)  // ERROR: s1 no longer valid
    println!("{}", s2)     // OK
} // s2 dropped here
```

### Copy Types

Primitive types implement `Copy`, so they're copied instead of moved:

```knull
mode expert

fn main() {
    let x: i32 = 5
    let y = x            // x is copied to y
    
    println!("x = {}", x)  // OK: i32 is Copy
    println!("y = {}", y)  // OK
}
```

Types that implement `Copy`:
- All integers (`i8` through `u128`)
- All floats (`f32`, `f64`)
- `bool`
- `char`
- Tuples of Copy types
- Arrays of Copy types

### Clone

For non-Copy types, use `.clone()` to make a deep copy:

```knull
mode expert

fn main() {
    let s1 = String::from("hello")
    let s2 = s1.clone()  // Explicit deep copy
    
    println!("s1 = {}", s1)  // OK: we cloned
    println!("s2 = {}", s2)  // OK
}
```

## Borrowing

### Immutable References

Borrow without taking ownership:

```knull
mode expert

fn calculate_length(s: &String) -> usize {
    s.len()  // Can read but not modify
}

fn main() {
    let s = String::from("hello")
    let len = calculate_length(&s)  // Borrow s
    
    println!("'{}' has length {}", s, len)  // s still valid
}
```

### Mutable References

Borrow with permission to modify:

```knull
mode expert

fn change(s: &mut String) {
    s.push_str(", world")
}

fn main() {
    let mut s = String::from("hello")
    change(&mut s)
    println!("{}", s)  // "hello, world"
}
```

### Borrowing Rules

1. You can have **any number** of immutable references (`&T`)
2. You can have **exactly one** mutable reference (`&mut T`)
3. You **cannot** have both mutable and immutable references at the same time

```knull
mode expert

fn main() {
    let mut s = String::from("hello")
    
    let r1 = &s
    let r2 = &s
    println!("{} {}", r1, r2)  // OK: multiple immutable borrows
    
    let r3 = &mut s  // OK: r1 and r2 not used after this point
    r3.push_str("!")
}
```

## Lifetimes

### Explicit Lifetimes

When the compiler can't infer lifetimes, annotate them:

```knull
mode expert

// 'a is a lifetime parameter
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

fn main() {
    let string1 = String::from("long string is long")
    {
        let string2 = String::from("xyz")
        let result = longest(&string1, &string2)
        println!("Longest: {}", result)
    }
}
```

### Lifetime Elision

The compiler often infers lifetimes automatically:

```knull
mode expert

// These are equivalent:
fn first_word(s: &str) -> &str { }          // Elided
fn first_word<'a>(s: &'a str) -> &'a str { } // Explicit
```

### Structs with Lifetimes

```knull
mode expert

struct Book<'a> {
    title: &'a str,  // Reference lives as long as Book
}

impl<'a> Book<'a> {
    fn get_title(&self) -> &'a str {
        self.title
    }
}

fn main() {
    let title = String::from("The Rust Book")
    let book = Book { title: &title }
    println!("{}", book.get_title())
} // title dropped here, so book can't outlive this scope
```

## Smart Pointers

### Box<T>

Heap allocation with single ownership:

```knull
mode expert

fn main() {
    // Store large data on heap
    let data: Box<Vec<u8>> = Box::new(vec![0; 1000000])
    
    // Box is dereferenced automatically
    println!("First byte: {}", data[0])
    
    // Ownership moves
    let data2 = data
    // println!("{}", data[0])  // ERROR: moved
}
```

### Rc<T> (Reference Counted)

Shared ownership (single-threaded only):

```knull
mode expert

use std::rc::Rc

fn main() {
    let data = Rc::new(String::from("shared"))
    
    let data2 = Rc::clone(&data)
    let data3 = Rc::clone(&data)
    
    println!("Reference count: {}", Rc::strong_count(&data))  // 3
    
    // All three Rcs point to the same data
    println!("{}", data2)
}
```

### RefCell<T>

Interior mutability:

```knull
mode expert

use std::cell::RefCell

fn main() {
    let data = RefCell::new(String::from("hello"))
    
    {
        let mut s = data.borrow_mut()
        s.push_str(" world")
    }
    
    println!("{}", data.borrow())  // "hello world"
}
```

## Error Handling

### Result Type

```knull
mode expert

fn read_file(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?  // ? propagates errors
    let mut contents = String::new()
    file.read_to_string(&mut contents)?
    Ok(contents)
}

fn main() {
    match read_file("config.txt") {
        Ok(contents) => println!("File contents: {}", contents),
        Err(e) => eprintln!("Error reading file: {}", e),
    }
}
```

### The ? Operator

```knull
mode expert

fn may_fail() -> Result<i32, Error> {
    let x = operation1()?  // Returns Err early if operation1 fails
    let y = operation2(x)?  // Returns Err early if operation2 fails
    Ok(y)
}
```

### Option Type

```knull
mode expert

fn find(arr: &[i32], target: i32) -> Option<usize> {
    for (i, &item) in arr.iter().enumerate() {
        if item == target {
            return Some(i)
        }
    }
    None
}

fn main() {
    let numbers = [1, 2, 3, 4, 5]
    
    // Using match
    match find(&numbers, 3) {
        Some(index) => println!("Found at {}", index),
        None => println!("Not found"),
    }
    
    // Using if-let
    if let Some(index) = find(&numbers, 3) {
        println!("Found at {}", index)
    }
    
    // Using unwrap_or
    let index = find(&numbers, 10).unwrap_or(0)
}
```

## Traits

### Defining Traits

```knull
mode expert

trait Drawable {
    fn draw(&self)
    fn bounds(&self) -> Rect
}

struct Circle {
    x: f64,
    y: f64,
    radius: f64,
}

impl Drawable for Circle {
    fn draw(&self) {
        println!("Drawing circle at ({}, {}) with radius {}", 
                 self.x, self.y, self.radius)
    }
    
    fn bounds(&self) -> Rect {
        Rect {
            x: self.x - self.radius,
            y: self.y - self.radius,
            width: self.radius * 2.0,
            height: self.radius * 2.0,
        }
    }
}
```

### Trait Bounds

```knull
mode expert

fn draw_all(items: &[impl Drawable]) {
    for item in items {
        item.draw()
    }
}

// Equivalent with explicit trait bound
fn draw_all<T: Drawable>(items: &[T]) {
    for item in items {
        item.draw()
    }
}

// Multiple trait bounds
fn process<T: Drawable + Clone + Send>(item: T) { }
```

### Associated Types

```knull
mode expert

trait Iterator {
    type Item
    fn next(&mut self) -> Option<Self::Item>
}

struct Counter {
    count: u32,
}

impl Iterator for Counter {
    type Item = u32
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.count < 5 {
            self.count += 1
            Some(self.count)
        } else {
            None
        }
    }
}
```

## Unsafe Code (Preview)

Even in expert mode, `unsafe` blocks are available for low-level operations. See [GOD_MODE.md](GOD_MODE.md) for full details.

```knull
mode expert

fn main() {
    // Safe wrapper around unsafe operations
    let mut num = 5
    
    unsafe {
        let r1 = &num as *const i32
        let r2 = &mut num as *mut i32
        
        // Dereference raw pointer
        println!("r1 is: {}", *r1)
        *r2 = 10
    }
}
```

## Performance Tips

### Zero-Cost Abstractions

Expert mode features have no runtime cost:

```knull
mode expert

// No runtime overhead for these abstractions
let iter = (0..100)
    .filter(|x| x % 2 == 0)
    .map(|x| x * x)
    .collect::<Vec<_>>()
```

### Stack vs Heap

Prefer stack allocation when possible:

```knull
mode expert

fn main() {
    // Stack allocation (fast)
    let arr: [i32; 1000] = [0; 1000]
    
    // Heap allocation (slower)
    let vec: Vec<i32> = vec![0; 1000]
}
```

### Avoiding Clones

Use references to avoid expensive copies:

```knull
mode expert

// Expensive: clones the entire string
fn process_owned(s: String) { }

// Cheap: just borrows
fn process_borrowed(s: &str) { }

fn main() {
    let data = String::from("large data...")
    
    process_owned(data.clone())  // Expensive
    process_borrowed(&data)       // Cheap
}
```

## Common Patterns

### RAII (Resource Acquisition Is Initialization)

```knull
mode expert

struct FileGuard {
    handle: File,
}

impl FileGuard {
    fn open(path: &str) -> Result<FileGuard, Error> {
        let handle = File::open(path)?
        Ok(FileGuard { handle })
    }
}

impl Drop for FileGuard {
    fn drop(&mut self) {
        // Automatically closes file when guard goes out of scope
        println!("Closing file automatically")
    }
}

fn main() {
    {
        let guard = FileGuard::open("data.txt").unwrap()
        // Use guard.handle
    } // File automatically closed here
}
```

### Builder Pattern

```knull
mode expert

struct HttpRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

struct HttpRequestBuilder {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl HttpRequestBuilder {
    fn new(url: &str) -> Self {
        HttpRequestBuilder {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: vec![],
            body: None,
        }
    }
    
    fn method(mut self, m: &str) -> Self {
        self.method = m.to_string()
        self
    }
    
    fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()))
        self
    }
    
    fn build(self) -> HttpRequest {
        HttpRequest {
            method: self.method,
            url: self.url,
            headers: self.headers,
            body: self.body,
        }
    }
}

fn main() {
    let request = HttpRequestBuilder::new("https://api.example.com")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer token123")
        .build()
}
```

## Complete Example

```knull
mode expert

use std::fs::File
use std::io::{self, Read, Write}

// Custom error type
#[derive(Debug)]
enum AppError {
    Io(io::Error),
    Parse(String),
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err)
    }
}

struct Config {
    path: String,
    verbose: bool,
}

impl Config {
    fn from_args(args: &[String]) -> Result<Config, AppError> {
        if args.len() < 2 {
            return Err(AppError::Parse("Usage: program <file>".to_string()))
        }
        
        Ok(Config {
            path: args[1].clone(),
            verbose: args.contains(&"--verbose".to_string()),
        })
    }
}

fn read_file(path: &str) -> Result<String, AppError> {
    let mut file = File::open(path)?
    let mut contents = String::new()
    file.read_to_string(&mut contents)?
    Ok(contents)
}

fn process_content(content: &str) -> String {
    content.to_uppercase()
}

fn main() -> Result<(), AppError> {
    let args: Vec<String> = std::env::args().collect()
    let config = Config::from_args(&args)?
    
    if config.verbose {
        eprintln!("Reading from: {}", config.path)
    }
    
    let content = read_file(&config.path)?
    let processed = process_content(&content)
    
    println!("{}", processed)
    Ok(())
}
```

---

*Expert mode provides the power of systems programming with the safety of modern language design.*
