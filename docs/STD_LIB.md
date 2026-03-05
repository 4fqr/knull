# Knull Standard Library Reference v2.0.0

---

## I/O

| Function | Signature | Description |
|----------|-----------|-------------|
| `println` | `(value)` | Print value + newline |
| `print` | `(value)` | Print value, no newline |
| `input` | `(prompt) -> string` | Read line from stdin |
| `eprintln` | `(msg)` | Print to stderr + newline |

```knull
println("Hello!")
print("no newline")
let name = input("Name: ")
```

---

## Types / Conversion

| Function | Returns | Notes |
|----------|---------|-------|
| `type_of(v)` | string | "number", "string", "bool", "array", "map", "null", "fn" |
| `to_string(v)` | string | any value to string |
| `to_int(v)` | number | float truncate, string parse |
| `to_float(v)` | number | int or string to float |
| `parse_int(s)` | number | throws on failure |
| `parse_float(s)` | number | throws on failure |
| `is_null(v)` | bool | |
| `is_number(v)` | bool | |
| `is_string(v)` | bool | |
| `is_array(v)` | bool | |
| `is_map(v)` | bool | |
| `is_fn(v)` | bool | |

---

## Math

| Function | Description |
|----------|-------------|
| `abs(x)` | absolute value |
| `sqrt(x)` | square root |
| `floor(x)` / `ceil(x)` / `round(x)` | rounding |
| `pow(base, exp)` | same as `base ** exp` |
| `log(x)` | natural log |
| `log2(x)` / `log10(x)` | log base 2 / 10 |
| `sin(x)` / `cos(x)` / `tan(x)` | trig (radians) |
| `min(a, b)` / `max(a, b)` | 2-arg min/max |
| `min(arr)` / `max(arr)` | array min/max |
| `clamp(x, lo, hi)` | clamp to range |
| `random()` | float in [0,1) |
| `random_int(lo, hi)` | integer in [lo,hi] |

Constants: `PI`, `E`, `TAU`, `INFINITY`, `NAN`

---

## Strings

Methods called as `s.method()`:

| Method / Function | Description |
|------------------|-------------|
| `len(s)` | length in characters |
| `s.to_upper()` / `s.to_lower()` | case change |
| `s.trim()` / `s.trim_start()` / `s.trim_end()` | strip whitespace |
| `s.contains(sub)` | boolean |
| `s.starts_with(p)` / `s.ends_with(s)` | boolean |
| `s.find(sub)` | index or null |
| `s.replace(from, to)` | replace all |
| `s.split(sep)` | array of strings |
| `s.chars()` | array of characters |
| `s.bytes()` | array of byte values |
| `s.slice(start, end)` | substring |
| `s.repeat(n)` | repeat n times |
| `s.pad_start(len, ch)` / `s.pad_end(len, ch)` | padding |
| `join(arr, sep)` | join array with sep |
| `format(tmpl, ..args)` | format with {} |

---

## Arrays

Methods called as `arr.method()`:

| Method / Function | Description |
|------------------|-------------|
| `arr.len()` | length |
| `arr.push(v)` | append (in-place) |
| `arr.pop()` | remove and return last |
| `arr.shift()` | remove and return first |
| `arr.unshift(v)` | prepend |
| `arr.contains(v)` | boolean |
| `arr.index_of(v)` | index or null |
| `arr.map(f)` | transform each element |
| `arr.filter(f)` | keep matching elements |
| `arr.reduce(init, f)` | fold with accumulator |
| `arr.for_each(f)` | iterate (no return) |
| `arr.find(f)` | first matching value or null |
| `arr.any(f)` / `arr.all(f)` | boolean |
| `arr.sort()` | sorted copy (ascending) |
| `arr.sort_by(f)` | sorted by comparator |
| `arr.reverse()` | reversed copy |
| `arr.slice(s, e)` | sub-array |
| `arr.concat(b)` | concatenate |
| `arr.flat()` / `arr.flatten()` | flatten one level |
| `arr.unique()` | deduplicate |
| `arr.sum()` | sum of numbers |
| `arr.join(sep)` | join as string |
| `range(start, end)` | [start, end) |
| `range(start, end, step)` | with step |
| `zip(a, b)` | zip two arrays |
| `min(arr)` / `max(arr)` | min/max value |

---

## Maps

| Method | Description |
|--------|-------------|
| `m.keys()` | array of keys |
| `m.values()` | array of values |
| `m.has(k)` | boolean |
| `m.get(k)` | value or null |
| `m.remove(k)` | remove key |
| `m.len()` | count |
| `m.entries()` | array of [k, v] pairs |

---

## System

| Function | Description |
|----------|-------------|
| `env(name)` | get env var or null |
| `env_set(name, v)` | set env var |
| `clock()` | ms since epoch |
| `time()` | alias for clock() |
| `sleep(ms)` | sleep milliseconds |
| `exit(code)` | terminate process |
| `args()` | CLI arguments array |

---

## File System

| Function | Description |
|----------|-------------|
| `fs_read(path)` | read file as string |
| `fs_write(path, s)` | write string to file |
| `fs_append(path, s)` | append to file |
| `fs_exists(path)` | boolean |
| `fs_delete(path)` | delete file |
| `fs_mkdir(path)` | create directory |
| `fs_ls(path)` | list directory |

---

## JSON

| Function | Description |
|----------|-------------|
| `json_parse(s)` | string → value |
| `json_stringify(v)` | value → string |

---

## Network

| Function | Description |
|----------|-------------|
| `http_get(url)` | GET → string |
| `http_post(url, body)` | POST → string |
| `http_post_json(url, v)` | POST JSON → string |
| `tcp_connect(host, port)` | → connection |
| `conn.send(data)` | send data |
| `conn.recv()` | receive string |
| `conn.close()` | close connection |
| `tcp_listen(host, port)` | → server |
| `server.accept()` | → connection |

---

## Concurrency

| Function | Description |
|----------|-------------|
| `spawn { block }` | start thread, returns handle |
| `handle.join()` | wait for result |
| `channel()` | create channel |
| `ch.send(v)` | send value (blocking) |
| `ch.recv()` | receive value (blocking) |
| `mutex()` | create mutex |
| `m.lock()` | acquire lock, returns guard |
| `guard.unlock()` | release lock |

---

## Debug / Assert

| Function | Description |
|----------|-------------|
| `debug(v)` | print with type info |
| `assert(cond, msg)` | panic if false |
| `assert_eq(a, b)` | assert equality |
| `panic(msg)` | terminate with message |
| `trace(v)` | print and return v |
