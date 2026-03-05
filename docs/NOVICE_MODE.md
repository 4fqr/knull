# Knull — Novice Mode Guide

Welcome! This guide teaches you Knull from scratch, step by step.

---

## What is Knull?

Knull is a modern programming language that's easy to learn but powerful enough for serious programs. You can use it for scripts, games, web servers, tools, and more.

---

## Your First Program

Create a file called `hello.knull`:

```knull
println("Hello, World!")
```

Run it:

```bash
knull run hello.knull
```

Output: `Hello, World!`

---

## Storing Values (Variables)

Use `let` to store values:

```knull
let name = "Alice"
let age = 25
let height = 1.75

println(name)    // Alice
println(age)     // 25
println(height)  // 1.75
```

You can change values later:

```knull
let score = 0
score = score + 10
println(score)   // 10
```

---

## Numbers

You can do math:

```knull
println(10 + 5)    // 15
println(10 - 5)    // 5
println(10 * 5)    // 50
println(10 / 3)    // 3.333...
println(10 % 3)    // 1   (remainder)
println(2 ** 10)   // 1024 (power)
```

---

## Text (Strings)

Strings are text in double quotes:

```knull
let greeting = "Hello"
let name = "Bob"

// combine strings with +
let message = greeting + ", " + name + "!"
println(message)   // Hello, Bob!

// string length
println(len(message))   // 12
```

---

## True or False (Booleans)

```knull
let is_raining = true
let is_sunny = false

println(is_raining)   // true
println(!is_raining)  // false (not)
```

---

## Making Decisions (if/else)

```knull
let temperature = 28

if temperature > 30 {
    println("It's hot!")
} else if temperature > 20 {
    println("It's warm.")
} else {
    println("It's cold.")
}
```

---

## Repeating Things (Loops)

Repeat while something is true:

```knull
let count = 1
while count <= 5 {
    println(count)
    count = count + 1
}
// prints 1 2 3 4 5
```

Loop over a list:

```knull
let fruits = ["apple", "banana", "cherry"]
for fruit in fruits {
    println(fruit)
}
```

---

## Lists (Arrays)

```knull
let numbers = [1, 2, 3, 4, 5]

println(numbers[0])      // 1  (first item)
println(numbers.len())   // 5  (count)

numbers.push(6)          // add to end
let last = numbers.pop() // remove from end
println(last)            // 6
```

---

## Writing Functions

Functions let you reuse code:

```knull
fn greet(name) {
    println("Hello, " + name + "!")
}

greet("Alice")   // Hello, Alice!
greet("Bob")     // Hello, Bob!
```

Functions can return values:

```knull
fn add(a, b) {
    a + b
}

let result = add(3, 7)
println(result)   // 10
```

The last line of a function is automatically returned.

---

## A Complete Example

```knull
fn is_even(n) {
    n % 2 == 0
}

let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
let evens = numbers.filter(fn(n) { is_even(n) })

println("Even numbers:")
for n in evens {
    println(n)
}
```

---

## Getting User Input

```knull
let name = input("What is your name? ")
println("Nice to meet you, " + name + "!")

let age_str = input("How old are you? ")
let age = to_int(age_str)
println("In 10 years you will be " + to_string(age + 10))
```

---

## Handling Errors

Sometimes things go wrong. Use `try`/`catch`:

```knull
try {
    let num = parse_int("not a number")
} catch error {
    println("Something went wrong: " + error)
}
```

---

## Next Steps

- Read `docs/GUIDE.md` for the complete language guide
- Try the examples in `examples/`
- Start the REPL for interactive exploration: `knull repl`
- Read `docs/STD_LIB.md` for all built-in functions
