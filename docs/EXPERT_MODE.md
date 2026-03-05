# Knull — Expert Mode Guide

For experienced developers. This guide covers advanced features in depth.

---

## Advanced Functions

### Closures and Captures

```knull
fn make_counter(start) {
    let count = start
    fn() {
        count = count + 1
        count
    }
}

let counter = make_counter(0)
println(counter())   // 1
println(counter())   // 2
println(counter())   // 3
```

### Higher-Order Functions

```knull
fn compose(f, g) { fn(x) { f(g(x)) } }
fn pipe(..fns) {
    fn(x) {
        let result = x
        for f in fns { result = f(result) }
        result
    }
}

let process = pipe(
    fn(x) { x * 2 },
    fn(x) { x + 10 },
    fn(x) { x * x }
)

println(process(5))   // ((5*2)+10)^2 = 400
```

### Memoization

```knull
fn memoize(f) {
    let cache = {}
    fn(x) {
        let key = to_string(x)
        if cache.has(key) {
            cache[key]
        } else {
            let result = f(x)
            cache[key] = result
            result
        }
    }
}

fn slow_fib(n) {
    if n <= 1 { n } else { slow_fib(n-1) + slow_fib(n-2) }
}

let fib = memoize(slow_fib)
println(fib(35))
```

---

## Pattern Matching — Advanced

### OR-patterns

```knull
fn classify(x) {
    match x {
        0 => "zero",
        1 | 2 | 3 => "small",
        n if n < 0 => "negative",
        n if n > 1000 => "huge",
        _ => "medium",
    }
}
```

### Nested struct matching

```knull
struct Vec2 { x, y }
struct Rect { top_left, bottom_right }

fn describe_rect(r) {
    match r {
        Rect { top_left: Vec2 { x: 0, y: 0 }, bottom_right } =>
            "rect from origin",
        Rect { top_left, bottom_right } =>
            "rect at " + to_string(top_left.x),
    }
}
```

---

## Structs and Methods — Advanced

### Builder pattern

```knull
struct Request {
    url,
    method,
    headers,
    body
}

impl Request {
    fn new(url) {
        Request { url: url, method: "GET", headers: {}, body: null }
    }

    fn with_method(self, method) {
        Request { url: self.url, method: method, headers: self.headers, body: self.body }
    }

    fn with_header(self, key, value) {
        self.headers[key] = value
        self
    }

    fn with_body(self, body) {
        Request { url: self.url, method: self.method, headers: self.headers, body: body }
    }

    fn send(self) {
        if self.method == "GET" {
            http_get(self.url)
        } else {
            http_post(self.url, self.body)
        }
    }
}

let response = Request.new("https://api.example.com/data")
    .with_method("POST")
    .with_header("Content-Type", "application/json")
    .with_body('{"key": "value"}')
    .send()
```

---

## Error Handling — Advanced

### Custom error types

```knull
struct AppError {
    code,
    message
}

fn validate_age(age) {
    if age < 0 { throw AppError { code: "INVALID", message: "Age cannot be negative" } }
    if age > 150 { throw AppError { code: "INVALID", message: "Age seems too large" } }
    age
}

try {
    let age = validate_age(-5)
} catch e {
    match e {
        AppError { code: "INVALID", message } => println("Validation: " + message),
        _ => println("Unknown error: " + to_string(e)),
    }
}
```

### Error propagation pattern

```knull
fn try_parse(s) {
    try { parse_int(s) } catch _ { null }
}

fn process_numbers(strings) {
    let results = []
    for s in strings {
        let n = try_parse(s)
        if n != null { results.push(n) }
    }
    results
}

println(process_numbers(["1", "abc", "3", "xyz", "5"]))
// [1, 3, 5]
```

---

## Concurrency — Advanced

### Producer/consumer pattern

```knull
fn producer(ch, items) {
    for item in items {
        ch.send(item)
    }
    ch.send(null)  // sentinel
}

fn consumer(ch) {
    let results = []
    while true {
        let item = ch.recv()
        if item == null { break }
        results.push(item * item)
    }
    results
}

let ch = channel()
let items = range(1, 6)

spawn { producer(ch, items) }
let squares = consumer(ch)
println(squares)   // [1, 4, 9, 16, 25]
```

### Parallel map

```knull
fn parallel_map(arr, f) {
    let handles = arr.map(fn(item) {
        spawn { f(item) }
    })
    handles.map(fn(h) { h.join() })
}

let results = parallel_map([1, 2, 3, 4], fn(n) {
    sleep(100)   // simulate work
    n * n
})
println(results)
```

---

## Functional Patterns

### Maybe/Option pattern

```knull
fn find_user(id) {
    let users = [
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"},
    ]
    users.find(fn(u) { u["id"] == id })
}

fn get_user_name(id) {
    let user = find_user(id)
    if user == null { "Unknown" } else { user["name"] }
}

println(get_user_name(1))   // Alice
println(get_user_name(99))  // Unknown
```

### Lazy evaluation

```knull
fn lazy(f) { fn() { f() } }
fn force(thunk) { thunk() }

let expensive = lazy(fn() {
    println("computing...")
    42 * 42
})

// Computed only when forced:
println(force(expensive))  // computing... 1764
```

---

## Performance Tips

1. **Avoid string concatenation in loops** — build an array and `join()` at end:
   ```knull
   let parts = []
   for i in range(0, 1000) { parts.push(to_string(i)) }
   let result = parts.join(", ")
   ```

2. **Cache repeated calls** — use memoization for expensive pure functions

3. **Use `for` over `while`** when iterating arrays

4. **Prefer functional pipelines** for data transformation:
   ```knull
   let result = data
       .filter(fn(x) { x > 0 })
       .map(fn(x) { x * 2 })
       .reduce(0, fn(a, b) { a + b })
   ```
