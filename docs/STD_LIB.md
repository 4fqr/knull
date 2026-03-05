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
| `json_parse(s)` | JSON string → value (map/array/number/string/bool/null) |
| `json_stringify(v)` | value → compact JSON string |
| `json_encode(v)` | alias for `json_stringify` |
| `parse_json(s)` | alias for `json_parse` |

---

## Database (SQLite)

Built-in SQLite via rusqlite — no external dependencies needed.

| Function | Description |
|----------|-------------|
| `db_open(path)` | open or create SQLite file, returns handle |
| `db_open_memory()` | open in-memory SQLite database |
| `db_exec(h, sql)` | run INSERT / CREATE / UPDATE / DELETE |
| `db_query(h, sql)` | SELECT → array of maps |
| `db_query_one(h, sql)` | SELECT → single map (or null) |
| `db_last_insert_id(h)` | last auto-increment row id |

Package wrapper at `packages/sqlite/src/lib.knull` adds: `connect`, `create_table`, `db_insert`, `select_all`, `select_where`, `update_where`, `delete_where`, `db_count`, `begin_transaction`, `commit`, `rollback`.

---

## GUI / Graphics (minifb)

Hardware-accelerated framebuffer window via minifb.

| Function | Description |
|----------|-------------|
| `gui_window(title, w, h)` | open window, returns int handle |
| `gui_fill(h, rgb)` | fill entire buffer with colour |
| `gui_set_pixel(h, x, y, rgb)` | set single pixel |
| `gui_rect(h, x, y, w, h, rgb)` | filled rectangle |
| `gui_rect_outline(h, x, y, w, h, rgb)` | rectangle outline |
| `gui_line(h, x0, y0, x1, y1, rgb)` | Bresenham line |
| `gui_circle(h, cx, cy, r, rgb)` | filled circle |
| `gui_circle_outline(h, cx, cy, r, rgb)` | circle outline (midpoint) |
| `gui_present(h)` | flush buffer to screen |
| `gui_is_open(h)` | → bool (window still open) |
| `gui_get_keys(h)` | → array of key name strings |
| `gui_get_mouse(h)` | → `{x, y, left, right, middle}` |
| `gui_rgb(r, g, b)` | pack r,g,b (0-255) into 0xRRGGBB |
| `gui_size(h)` | → `{w, h}` pixel dimensions |
| `gui_set_title(h, title)` | update window title |
| `gui_close(h)` | close and destroy window |

Colors are packed `int` values: `0xRRGGBB`. Use `gui_rgb(r,g,b)` for convenience.

---

## Image Processing

| Function | Description |
|----------|-------------|
| `img_new(w, h)` | create blank RGBA image handle |
| `img_load(path)` | load PNG/JPEG/BMP/GIF from file |
| `img_save(h, path)` | save image to file |
| `img_width(h)` | width in pixels |
| `img_height(h)` | height in pixels |
| `img_resize(h, w, h)` | resize image |
| `img_get_pixel(h, x, y)` | → `{r, g, b, a}` |
| `img_set_pixel(h, x, y, r, g, b, a)` | set pixel RGBA |

---

## Crypto

| Function | Description |
|----------|-------------|
| `sha256(s)` | SHA-256 hex string |
| `md5(s)` | MD5 hex string |
| `base64_encode(s)` | base64 encode |
| `base64_decode(s)` | base64 decode |
| `random_bytes(n)` | n random bytes as hex string |

Package wrapper at `packages/crypto/src/lib.knull` adds: `hash_sha256`, `hash_md5`, `encode_base64`, `decode_base64`, `hash_password`, `verify_password`, `hmac_simple`, `token_generate`.

---

## Network

| Function | Description |
|----------|-------------|
| `http_get(url)` | GET → body string |
| `http_post(url, body, content_type)` | POST → body string |
| `http_put(url, body, content_type)` | PUT → body string |
| `http_delete(url)` | DELETE → body string |
| `tcp_connect(host, port)` | → connection handle |
| `tcp_listen(host, port)` | → server handle |

Package wrapper at `packages/http/src/lib.knull` adds: `get_json`, `post_json`, `put_json`, `fetch`, `download`, `build_query`.

---

## Concurrency

| Function | Description |
|----------|-------------|
| `spawn { block }` | start thread, returns int handle (also usable as statement) |
| `thread_join(h)` | wait for result from spawned block or thread_spawn |
| `thread_spawn(fn, args...)` | spawn a function in a new thread, returns int handle |
| `thread_try_recv(h)` | non-blocking poll; returns null if not done |
| `chan_create()` | create channel, returns `{ "id": N }` |
| `chan_send(id, v)` | send value (blocking) |
| `chan_recv(id)` | receive value (blocking) |
| `sleep_ms(ms)` | sleep current thread |

---

## Debug / Assert

| Function | Description |
|----------|-------------|
| `debug(v)` | print with type info |
| `assert(cond, msg)` | panic if false |
| `assert_eq(a, b)` | assert equality |
| `panic(msg)` | terminate with message |
| `trace(v)` | print and return v |

