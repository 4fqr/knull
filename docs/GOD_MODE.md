# Knull — God Mode Guide

Maximum power. For when you need to build things that shouldn't be possible.

---

## Systems Programming

### Manual memory patterns

```knull
fn arena_allocator(size) {
    let buffer = []
    let pos = 0

    {
        alloc: fn(n) {
            let start = pos
            pos = pos + n
            start
        },
        reset: fn() { pos = 0 },
        size: fn() { pos },
    }
}
```

### Custom data structures

```knull
struct Node { value, next }

fn linked_list() {
    let head = null
    {
        push: fn(v) { head = Node { value: v, next: head } },
        pop: fn() {
            if head == null { throw "empty list" }
            let v = head.value
            head = head.next
            v
        },
        to_array: fn() {
            let arr = []
            let curr = head
            while curr != null {
                arr.push(curr.value)
                curr = curr.next
            }
            arr
        }
    }
}

let list = linked_list()
list.push(1)
list.push(2)
list.push(3)
println(list.to_array())   // [3, 2, 1]
```

---

## Metaprogramming

### Dispatch tables

```knull
let ops = {
    "+": fn(a, b) { a + b },
    "-": fn(a, b) { a - b },
    "*": fn(a, b) { a * b },
    "/": fn(a, b) { a / b },
}

fn calculate(a, op, b) {
    if !ops.has(op) { throw "Unknown operator: " + op }
    ops[op](a, b)
}

println(calculate(10, "+", 5))   // 15
println(calculate(10, "*", 3))   // 30
```

### AST interpreter (meta-circular)

```knull
fn eval_expr(expr) {
    match expr["type"] {
        "num" => expr["value"],
        "add" => eval_expr(expr["left"]) + eval_expr(expr["right"]),
        "mul" => eval_expr(expr["left"]) * eval_expr(expr["right"]),
        _ => throw "Unknown expr type",
    }
}

let ast = {
    "type": "add",
    "left": {"type": "num", "value": 3},
    "right": {
        "type": "mul",
        "left": {"type": "num", "value": 4},
        "right": {"type": "num", "value": 5}
    }
}

println(eval_expr(ast))   // 23
```

---

## OS / Systems-Level

### Micro shell

```knull
fn run_shell() {
    println("KnullSH v1.0 — type 'exit' to quit")
    let running = true
    while running {
        let line = input("$ ")
        let parts = line.trim().split(" ")
        let cmd = parts[0]
        match cmd {
            "exit" => {
                running = false
                println("Goodbye")
            },
            "echo" => println(parts.slice(1, parts.len()).join(" ")),
            "env" => println(env(parts[1])),
            _ => println("Unknown command: " + cmd),
        }
    }
}

run_shell()
```

---

## Compiler Techniques

### Lexer

```knull
fn tokenize(source) {
    let tokens = []
    let i = 0
    while i < len(source) {
        let ch = source[i]
        if ch == " " || ch == "\n" {
            i = i + 1
        } else if ch >= "0" && ch <= "9" {
            let start = i
            while i < len(source) && source[i] >= "0" && source[i] <= "9" {
                i = i + 1
            }
            tokens.push({"type": "num", "value": to_int(source.slice(start, i))})
        } else if ch == "+" {
            tokens.push({"type": "plus"})
            i = i + 1
        } else {
            throw "Unexpected char: " + ch
        }
    }
    tokens
}

let tokens = tokenize("42 + 7 + 13")
println(tokens)
```

---

## Networking — Advanced

### HTTP server skeleton

```knull
fn http_server(host, port) {
    println("Listening on " + host + ":" + to_string(port))
    let server = tcp_listen(host, port)

    fn handle(conn) {
        let request = conn.recv()
        let lines = request.split("\n")
        let first = lines[0].trim()

        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from Knull!"
        conn.send(response)
        conn.close()
    }

    while true {
        let conn = server.accept()
        spawn { handle(conn) }
    }
}

http_server("127.0.0.1", 8080)
```

---

## Embedding and FFI

### Calling C functions (via FFI)

```knull
// Low-level C interop
let libc = ffi_load("libc.so.6")
let strlen = ffi_fn(libc, "strlen", ["ptr"], "size_t")

let s = ffi_string("hello world")
println(strlen(s))   // 11
```

---

## Advanced Concurrency

### Work-stealing pool

```knull
fn thread_pool(n_workers) {
    let ch = channel()
    let workers = range(0, n_workers).map(fn(_) {
        spawn {
            while true {
                let task = ch.recv()
                if task == null { break }
                task()
            }
        }
    })

    {
        submit: fn(task) { ch.send(task) },
        shutdown: fn() {
            for _ in workers { ch.send(null) }
            for h in workers { h.join() }
        }
    }
}

let pool = thread_pool(4)
let results = []
let m = mutex()

for i in range(0, 20) {
    let n = i
    pool.submit(fn() {
        let r = n * n
        let g = m.lock()
        results.push(r)
        g.unlock()
    })
}

pool.shutdown()
println(results.sort())
```

---

## Self-Modifying Patterns

### Plugin system

```knull
let plugins = {}

fn register_plugin(name, handler) {
    plugins[name] = handler
}

fn dispatch(event, data) {
    if plugins.has(event) {
        plugins[event](data)
    }
}

register_plugin("on_start", fn(data) {
    println("Started with: " + to_string(data))
})

register_plugin("on_data", fn(data) {
    println("Data received: " + to_string(data))
})

dispatch("on_start", {"version": "1.0"})
dispatch("on_data", [1, 2, 3])
```
