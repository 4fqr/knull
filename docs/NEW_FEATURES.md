# Knull v2.0.0 — What's New

---

## Summary

Knull v2.0.0 is a significant upgrade focused on developer experience, correctness, and power.

---

## New: `eval` Subcommand

Run Knull expressions directly from the shell:

```bash
knull eval 'println(40 + 2)'
knull eval 'let x = [1,2,3]; println(x.map(fn(n){n*n}))'
```

Useful for scripting, one-liners, and quick calculations.

---

## Upgraded REPL

The REPL now has persistent state across lines, multi-line block editing, and built-in commands:

```
knull❯ let x = 10
knull❯ let y = 20
knull❯ x + y
30 // number
knull❯ :vars
Variables:
  x = 10
  y = 20
knull❯ fn double(n) { n * 2 }
knull❯ double(x)
20 // number
knull❯ :fns
Functions:
  double
knull❯ :reset
State cleared.
knull❯ :quit
```

New REPL commands: `:vars`, `:fns`, `:reset`, `:load <file>`, `:type <expr>`, `:clear`

---

## `min()` / `max()` Array Form

`min` and `max` now accept a single array argument:

```knull
let nums = [5, 1, 8, 3, 9, 2]
println(min(nums))   // 1
println(max(nums))   // 9

// Two-arg form still works
println(min(3, 7))   // 3
println(max(3, 7))   // 7
```

---

## OR-Patterns in Match

Match multiple values in a single arm using `|`:

```knull
let x = 3
match x {
    1 | 2 | 3 => println("small"),
    4 | 5 | 6 => println("medium"),
    _ => println("large"),
}
```

Works with strings too:

```knull
match status {
    "ok" | "success" => println("good"),
    "error" | "fail" => println("bad"),
    _ => println("unknown"),
}
```

---

## Match Guards

Add conditions to match arms with `if`:

```knull
match score {
    n if n >= 90 => println("A"),
    n if n >= 80 => println("B"),
    n if n >= 70 => println("C"),
    n if n >= 60 => println("D"),
    _ => println("F"),
}
```

---

## Optional Type Annotations in Structs

Struct fields no longer require type annotations:

```knull
// v2.0.0 — clean, no annotations needed
struct Point {
    x,
    y
}

struct Person {
    name,
    age,
    email
}
```

---

## Recursive Implicit Return

Recursive functions now correctly propagate implicit return values through all branches:

```knull
fn fib(n) {
    if n <= 1 { n }
    else { fib(n-1) + fib(n-2) }
}

println(fib(10))   // 55  (was broken in v1.x for some cases)
```

---

## All Examples Updated

All 67 example `.knull` files have been updated to current syntax:
- `println(...)` with parentheses (mandatory since v1.5)
- `print(...)` with parentheses
- `spawn { block }` thread syntax
- Struct field syntax without type annotations

---

## Breaking Changes from v1.x

| Old syntax | New syntax |
|-----------|-----------|
| `println x` | `println(x)` |
| `print x` | `print(x)` |
| `spawn("fn_name")` | `spawn { ... }` |
| `struct S { x: int }` | `struct S { x }` |

---

## Full Changelog

- `eval` subcommand added (`knull eval 'expr'`)
- REPL: persistent state, multi-line, `:vars :fns :reset :load :type :clear`
- `min(array)` / `max(array)` single-argument form
- OR-patterns: `1 | 2 | 3 =>` in match arms
- Match guards: `n if n > 0 =>`
- Struct fields without type annotations
- Recursive functions with implicit return fixed
- 33/33 unit tests pass
- 67/67 example files verified working
