# Knull Language Guide

A comprehensive guide to the Knull programming language.

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Language Basics](#language-basics)
4. [Variables and Types](#variables-and-types)
5. [Operators](#operators)
6. [Control Flow](#control-flow)
7. [Functions](#functions)
8. [Arrays](#arrays)
9. [Standard Library](#standard-library)
10. [Best Practices](#best-practices)

---

## Introduction

Knull is a multi-paradigm programming language designed to adapt to programmer expertise. It provides a unified syntax that scales from simple scripting to complex systems programming.

### Philosophy

- **Simplicity**: Clear, readable syntax
- **Performance**: Zero-cost abstractions
- **Safety**: Memory safety without garbage collection
- **Flexibility**: Three modes for different use cases

---

## Getting Started

### Hello World

```knull
fn main() {
    println "Hello, World!"
}
```

Save this to `hello.knull` and run:

```bash
knull run hello.knull
```

### Program Structure

Every Knull program consists of functions. The entry point is the `main` function:

```knull
fn main() {
    // Program execution starts here
}
```

---

## Language Basics

### Comments

```knull
// This is a single-line comment

// Comments are ignored by the compiler
// Use them to document your code
```

### Statements

Statements in Knull do not require semicolons. Each statement appears on its own line:

```knull
let x = 5           // Variable declaration
println x           // Function call
x = x + 1           // Assignment
```

---

## Variables and Types

### Variable Declaration

Use the `let` keyword to declare variables:

```knull
let variable_name = value
```

### Basic Types

#### Integers

```knull
let count = 42
let negative = -17
let zero = 0
```

Integers are 64-bit signed numbers.

#### Floating Point

```knull
let pi = 3.14159
let negative_float = -2.5
```

Floating point numbers use double precision (64-bit).

#### Strings

```knull
let greeting = "Hello, World!"
let empty = ""
```

Strings are immutable sequences of characters.

#### Booleans

Booleans are represented as integers:

```knull
let true_value = 1
let false_value = 0
```

### Variable Assignment

Variables declared with `let` can be reassigned:

```knull
let x = 10
x = 20              // Reassignment
x = x + 5           // Increment by 5
```

---

## Operators

### Arithmetic Operators

| Operator | Description | Example | Result |
|----------|-------------|---------|--------|
| `+` | Addition | `5 + 3` | `8` |
| `-` | Subtraction | `5 - 3` | `2` |
| `*` | Multiplication | `5 * 3` | `15` |
| `/` | Division | `5 / 3` | `1` |
| `%` | Modulo | `5 % 3` | `2` |

```knull
let a = 17
let b = 5

println a + b       // 22
println a - b       // 12
println a * b       // 85
println a / b       // 3
println a % b       // 2
```

### Comparison Operators

| Operator | Description | Example | Result |
|----------|-------------|---------|--------|
| `==` | Equal to | `5 == 5` | `true` |
| `!=` | Not equal to | `5 != 3` | `true` |
| `<` | Less than | `3 < 5` | `true` |
| `>` | Greater than | `5 > 3` | `true` |
| `<=` | Less than or equal | `5 <= 5` | `true` |
| `>=` | Greater than or equal | `5 >= 3` | `true` |

```knull
let x = 10
let y = 20

println x == y      // false
println x != y      // true
println x < y       // true
println x > y       // false
println x <= y      // true
println x >= y      // false
```

### Logical Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `&&` | Logical AND | `a && b` |
| `||` | Logical OR | `a || b` |
| `!` | Logical NOT | `!a` |

### String Concatenation

The `+` operator concatenates strings:

```knull
let first = "Hello"
let second = "World"
let message = first + " " + second    // "Hello World"
```

Strings can be concatenated with numbers:

```knull
let count = 42
let message = "The count is " + count    // "The count is 42"
```

### Operator Precedence

Operators are evaluated in the following order (highest to lowest):

1. Parentheses `()`
2. Unary operators `!`, `-`
3. Multiplicative `*`, `/`, `%`
4. Additive `+`, `-`
5. Comparison `==`, `!=`, `<`, `>`, `<=`, `>=`
6. Logical AND `&&`
7. Logical OR `||`
8. Assignment `=`

Use parentheses to override precedence:

```knull
let result = (a + b) * (c - d)
```

---

## Control Flow

### If Statements

Execute code conditionally:

```knull
if condition {
    // Execute if condition is true
}
```

### If-Else Statements

Choose between two branches:

```knull
if condition {
    // Execute if true
} else {
    // Execute if false
}
```

### Nested If Statements

Chain multiple conditions:

```knull
if score >= 90 {
    println "Grade: A"
} else {
    if score >= 80 {
        println "Grade: B"
    } else {
        if score >= 70 {
            println "Grade: C"
        } else {
            println "Grade: D or F"
        }
    }
}
```

### While Loops

Execute code repeatedly while a condition is true:

```knull
while condition {
    // Execute repeatedly
}
```

Example:

```knull
let i = 0
while i < 10 {
    println i
    i = i + 1
}
```

### For Loops

Iterate over a range or collection:

```knull
for variable in iterable {
    // Execute for each item
}
```

---

## Functions

### Function Definition

Define functions using the `fn` keyword:

```knull
fn function_name(parameters) {
    // Function body
    return value
}
```

### Functions Without Parameters

```knull
fn get_greeting() {
    return "Hello, Knull!"
}
```

### Functions With Parameters

```knull
fn add(a, b) {
    return a + b
}
```

### Functions With Multiple Parameters

```knull
fn calculate_volume(width, height, depth) {
    return width * height * depth
}
```

### Calling Functions

```knull
let result = add(5, 3)
println result        // 8
```

### Void Functions

Functions that don't return values:

```knull
fn print_message(msg) {
    println msg
}
```

### Recursive Functions

Functions that call themselves:

```knull
fn factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
```

---

## Arrays

### Array Literals

Create arrays using square brackets:

```knull
let numbers = [1, 2, 3, 4, 5]
let empty = []
```

### Array Indexing

Access elements by index (0-based):

```knull
let numbers = [10, 20, 30, 40, 50]
let first = numbers[0]      // 10
let third = numbers[2]      // 30
let last = numbers[4]       // 50
```

### Iterating Over Arrays

Use a while loop to iterate:

```knull
let numbers = [10, 20, 30, 40, 50]
let i = 0
while i < 5 {
    println numbers[i]
    i = i + 1
}
```

---

## Standard Library

### Output Functions

#### println

Prints a value followed by a newline:

```knull
println "Hello, World!"
println 42
println "The answer is " + 42
```

#### print

Prints a value without a newline:

```knull
print "Loading"
print "."
print "."
println ". Done!"
```

### Built-in Functions

| Function | Description | Example |
|----------|-------------|---------|
| `println` | Print with newline | `println "text"` |
| `print` | Print without newline | `print "text"` |

---

## Best Practices

### Naming Conventions

- Use `snake_case` for function and variable names
- Use descriptive names that convey purpose

```knull
// Good
let user_count = 42
fn calculate_total() { }

// Avoid
let uc = 42
fn calc() { }
```

### Code Organization

- Group related functions together
- Add comments to explain complex logic
- Keep functions focused on a single task

### Error Prevention

- Initialize variables before use
- Check array bounds manually
- Validate function inputs

### Example: Well-Structured Program

```knull
// Calculate the sum of squares from 1 to n
fn sum_of_squares(n) {
    if n < 1 {
        return 0
    }
    
    let sum = 0
    let i = 1
    
    while i <= n {
        sum = sum + (i * i)
        i = i + 1
    }
    
    return sum
}

fn main() {
    let result = sum_of_squares(10)
    println "Sum of squares from 1 to 10: " + result
}
```

---

## Examples

### Fibonacci Sequence

```knull
fn fibonacci(n) {
    if n <= 0 {
        return 0
    }
    if n == 1 {
        return 1
    }
    
    let a = 0
    let b = 1
    let i = 2
    
    while i <= n {
        let temp = a + b
        a = b
        b = temp
        i = i + 1
    }
    
    return b
}

fn main() {
    let i = 0
    while i < 10 {
        println fibonacci(i)
        i = i + 1
    }
}
```

### Prime Number Check

```knull
fn is_prime(n) {
    if n < 2 {
        return 0
    }
    if n == 2 {
        return 1
    }
    if n % 2 == 0 {
        return 0
    }
    
    let i = 3
    while i * i <= n {
        if n % i == 0 {
            return 0
        }
        i = i + 2
    }
    
    return 1
}

fn main() {
    let num = 17
    if is_prime(num) {
        println num + " is prime"
    } else {
        println num + " is not prime"
    }
}
```

---

## Additional Resources

- [Standard Library Reference](STD_LIB.md)
- [Language Specification](SPECIFICATION.md)
- [Examples Directory](../examples/)

---

For questions or issues, consult the project repository or documentation.
