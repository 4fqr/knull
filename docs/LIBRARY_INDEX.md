# Knull Built-in Library Index v2.0.0

Quick index of all built-in functions and methods. See `docs/STD_LIB.md` for full documentation.

---

## By Category

### I/O
`println` `print` `input` `eprintln` `eprint`

### Types & Conversion
`type_of` `to_string` `to_int` `to_float` `parse_int` `parse_float`
`is_null` `is_number` `is_string` `is_bool` `is_array` `is_map` `is_fn`

### Math
`abs` `sqrt` `floor` `ceil` `round` `pow` `log` `log2` `log10`
`sin` `cos` `tan` `asin` `acos` `atan` `atan2`
`min` `max` `clamp` `random` `random_int`
**Constants**: `PI` `E` `TAU` `INFINITY` `NAN`

### Strings
`len` `join` `format`
**Methods**: `to_upper` `to_lower` `trim` `trim_start` `trim_end`
`contains` `starts_with` `ends_with` `find` `replace`
`split` `chars` `bytes` `slice` `repeat` `pad_start` `pad_end`

### Arrays
`range` `zip` `min` `max`
**Methods**: `len` `push` `pop` `shift` `unshift` `contains` `index_of`
`map` `filter` `reduce` `for_each` `find` `any` `all`
`sort` `sort_by` `reverse` `slice` `concat` `flat` `flatten`
`unique` `sum` `join`

### Maps
**Methods**: `keys` `values` `has` `get` `remove` `len` `entries`

### System
`env` `env_set` `clock` `time` `sleep` `exit` `args`

### File System
`fs_read` `fs_write` `fs_append` `fs_exists` `fs_delete` `fs_mkdir` `fs_ls`

### JSON
`json_parse` `json_stringify`

### Network (HTTP)
`http_get` `http_post` `http_post_json`

### Network (TCP)
`tcp_connect` `tcp_listen`
**Connection methods**: `send` `recv` `close`
**Server methods**: `accept`

### Concurrency
`channel` `mutex`
**spawn** `{ block }` → handle
**Handle methods**: `join`
**Channel methods**: `send` `recv`
**Mutex methods**: `lock` `unlock`

### Debug / Assert
`debug` `assert` `assert_eq` `panic` `trace`

---

## Alphabetical Index

| Function | Category |
|---------|---------|
| `abs` | Math |
| `acos` / `asin` / `atan` / `atan2` | Math |
| `args` | System |
| `assert` / `assert_eq` | Debug |
| `channel` | Concurrency |
| `ceil` | Math |
| `clamp` | Math |
| `clock` / `time` | System |
| `cos` | Math |
| `debug` | Debug |
| `env` / `env_set` | System |
| `eprintln` / `eprint` | I/O |
| `exit` | System |
| `floor` | Math |
| `format` | String |
| `fs_append` | Files |
| `fs_delete` | Files |
| `fs_exists` | Files |
| `fs_ls` | Files |
| `fs_mkdir` | Files |
| `fs_read` | Files |
| `fs_write` | Files |
| `http_get` | Network |
| `http_post` | Network |
| `http_post_json` | Network |
| `input` | I/O |
| `is_array` / `is_bool` / `is_fn` / `is_map` / `is_null` / `is_number` / `is_string` | Types |
| `join` | String |
| `json_parse` / `json_stringify` | JSON |
| `len` | String / Array |
| `log` / `log2` / `log10` | Math |
| `max` | Math / Array |
| `min` | Math / Array |
| `mutex` | Concurrency |
| `panic` | Debug |
| `parse_float` / `parse_int` | Types |
| `pow` | Math |
| `print` / `println` | I/O |
| `random` / `random_int` | Math |
| `range` | Array |
| `round` | Math |
| `sin` | Math |
| `sleep` | System |
| `sqrt` | Math |
| `tan` | Math |
| `tcp_connect` / `tcp_listen` | Network |
| `to_float` / `to_int` / `to_string` | Types |
| `trace` | Debug |
| `type_of` | Types |
| `zip` | Array |
