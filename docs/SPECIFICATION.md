# KNULL PROGRAMMING LANGUAGE SPECIFICATION
## Version 1.0 — The Complete Definition

---

# PART I: LEXICAL SPECIFICATION

## 1.1 Source File Representation

- **Extension**: `.knull`
- **Encoding**: UTF-8
- **Line Endings**: LF (`\n`) or CRLF (`\r\n`)
- **Case Sensitivity**: Yes

## 1.2 Tokens

### 1.2.1 Identifiers

```
identifier     → identifier_start (identifier_start | digit)*
identifier_start → letter | underscore
letter         → 'A'..'Z' | 'a'..'z' | Unicode letter
digit          → '0'..'9'
```

**Keywords** (reserved, case-sensitive):

```
// Mode Keywords
unsafe     // God Mode: disables safety checks
comptime   // Compile-time execution
ghost      // Compile-time only, erased at runtime

// Declaration Keywords
fn         // Function
let        // Immutable binding (static typing)
var        // Mutable binding (dynamic typing)
own        // Ownership transfer (linear type)
mut        // Mutable reference
ref        // Borrowed reference
const      // Compile-time constant
static     // Global/static variable
struct     // Structure definition
enum       // Enumeration definition
union      // Union definition
trait      // Trait definition (interface)
impl       // Trait implementation
type       // Type alias
actor      // Concurrent process (green thread)
macro      // Macro definition

// Control Flow
if         // Conditional
else       // Alternative branch
match       // Pattern matching
loop        // Infinite loop
while       // Conditional loop
for         // Iterator loop
in          // Iterator binding
return      // Return from function
break       // Exit loop
continue    // Next iteration
defer       // Scoped cleanup
goto        // Unconditional jump (restricted)

// Special
embed       // Embed binary data
asm         // Inline assembly
syscall     // Direct system call
import      // Module import
export      // Module export
pub         // Public visibility
priv        // Private visibility
module      // Module declaration
as          // Type cast / alias
is          // Type check
null        // Null value
true        // Boolean true
false       // Boolean false
self        // Current instance
Super       // Parent module
```

### 1.2.2 Literals

```
literal → integer_literal
        | float_literal
        | string_literal
        | char_literal
        | byte_literal
        | bool_literal
        | raw_string_literal
        | byte_string_literal

integer_literal → decimal_literal
                 | hex_literal
                 | octal_literal
                 | binary_literal
                 | char_literal

decimal_literal → digit+ ('_' digit)*
hex_literal      → '0x' (hex_digit '_')* hex_digit
octal_literal    → '0o' (octal_digit '_')* octal_digit
binary_literal   → '0b' (binary_digit '_')* binary_digit

float_literal    → digit* '.' digit+ (exponent)?
                 | digit+ exponent
exponent         → ('e' | 'E') ('+' | '-')? digit+

string_literal   → '"' (string_char | escape_sequence)* '"'
raw_string_literal → 'r' '#'* '"' (any_except_newline)* '"' '#'*
byte_literal     → 'b' char_literal
byte_string_literal → 'b' string_literal

escape_sequence  → '\\' ('n' | 'r' | 't' | '\\' | '"' | "'" | '0'
                            | 'x' hex_digit hex_digit
                            | 'u' '{' hex_digit+ '}')
```

### 1.2.3 Operators

```
// Arithmetic
+    // Addition
-    // Subtraction
*    // Multiplication
/    // Division
%    // Modulo
**   // Exponentiation
<<   // Left shift
>>   // Right shift
&    // Bitwise AND
|    // Bitwise OR
^    // Bitwise XOR
~    // Bitwise NOT (unary)

// Comparison
==   // Equal
!=   // Not equal
<    // Less than
>    // Greater than
<=   // Less than or equal
>=   // Greater than or equal

// Logical
&&   // Logical AND
||   // Logical OR
!    // Logical NOT (unary)

// Assignment
=    // Simple assignment
+=   // Add and assign
-=   // Subtract and assign
*=   // Multiply and assign
/=   // Divide and assign
%=   // Modulo and assign
<<=  // Left shift and assign
>>=  // Right shift and assign
&=   // Bitwise AND and assign
|=   // Bitwise OR and assign
^=   // Bitwise XOR and assign

// Special Operators
?:   // Elvis operator (null-coalescing)
?.   // Safe navigation
|>   // Pipeline operator
..   // Range operator
..=  // Inclusive range
::   // Scope resolution
->   // Function return type / lambda
@    // Attribute / macro attribute
$    // Macro capture
```

### 1.2.4 Delimiters

```
( )       // Parentheses (grouping, function calls)
[ ]       // Brackets (arrays, indexing)
{ }       // Braces (blocks, structs)
< >       // Angle brackets (generics)
,         // Comma (separators)
;         // Semicolon (statement terminator)
:         // Colon (type annotation, object literal)
.         // Dot (member access)
::        // Double colon (scope resolution)
->        // Arrow (function return, lambda)
=         // Assignment
```

---

# PART II: GRAMMAR (EBNF)

## 2.1 Module Structure

```
module_file → (import_stmt | export_stmt | declaration)*

import_stmt → 'import' string_literal ('as' identifier)?
            | 'import' identifier ('::' identifier)* ('as' identifier)?

export_stmt → 'export' identifier (',' identifier)*

declaration → function_decl
            | struct_decl
            | enum_decl
            | union_decl
            | trait_decl
            | impl_decl
            | type_decl
            | constant_decl
            | static_decl
            | actor_decl
            | macro_decl
```

## 2.2 Type Declarations

### 2.2.1 Function Declaration

```
function_decl → 'fn' identifier generic_params? '(' param_list? ')' return_type? block
              | 'fn' identifier generic_params? '(' param_list? ')' return_type? '=>' expression

param_list → param (',' param)*
param → identifier ':' type_annotation ('=' expression)?

return_type → '->' type_annotation
```

### 2.2.2 Struct Declaration

```
struct_decl → 'struct' identifier generic_params? '{' field_list? '}'
field_list → field_decl (',' field_decl)*
field_decl → identifier ':' type_annotation
           | 'pub' identifier ':' type_annotation
```

### 2.2.3 Enum Declaration

```
enum_decl → 'enum' identifier generic_params? '{' variant_list? '}'
variant_list → variant_decl (',' variant_decl)*
variant_decl → identifier
             | identifier '(' type_annotation ')'
             | identifier '{' field_list '}'
```

### 2.2.4 Union Declaration

```
union_decl → 'union' identifier generic_params? '{' field_list '}'
```

### 2.2.5 Trait Declaration

```
trait_decl → 'trait' identifier generic_params? '{' trait_member* '}'
trait_member → function_decl
             | constant_decl
```

### 2.2.6 Implementation

```
impl_decl → 'impl' identifier generic_params? ('for' type_annotation)? '{' impl_member* '}'
impl_member → function_decl
            | constant_decl
```

### 2.2.7 Type Alias

```
type_decl → 'type' identifier generic_params? '=' type_annotation
```

### 2.2.8 Actor Declaration

```
actor_decl → 'actor' identifier generic_params? '{' actor_member* '}'
actor_member → function_decl
             | state_decl
state_decl → 'state' ':' type_annotation
```

## 2.3 Statements and Expressions

### 2.3.1 Statements

```
statement → declaration
          | expression_stmt
          | assignment_stmt
          | control_flow_stmt
          | defer_stmt
          | unsafe_block
          | asm_block
          | syscall_stmt
          | comptime_block

expression_stmt → expression ';'

assignment_stmt → lvalue '=' expression ';'
                | lvalue op_assign expression ';'
lvalue → identifier
       | identifier '[' expression ']'
       | lvalue '.' identifier
       | '*' lvalue
       | '(' lvalue ')'

control_flow_stmt → if_stmt
                  | match_stmt
                  | loop_stmt
                  | while_stmt
                  | for_stmt

defer_stmt → 'defer' block
           | 'defer' expression

unsafe_block → 'unsafe' '{' statement* '}'

asm_block → 'asm' ('intel' | 'att')? '{' asm_instruction* '}'

syscall_stmt → 'syscall' expression (',' expression)* ';'

comptime_block → 'comptime' '{' statement* '}'
               | 'comptime' expression
```

### 2.3.2 Expressions

```
expression → literal
           | identifier
           | lambda_expression
           | block_expression
           | if_expression
           | match_expression
           | loop_expression
           | break_expression
           | continue_expression
           | return_expression
           | unary_expression
           | binary_expression
           | ternary_expression
           | range_expression
           | type_cast_expression
           | type_check_expression
           | call_expression
           | index_expression
           | field_expression
           | tuple_expression
           | struct_expression
           | array_expression
           | pipeline_expression
           | unsafe_expression
           | comptime_expression

literal → integer_literal
        | float_literal
        | string_literal
        | char_literal
        | byte_literal
        | bool_literal
        | array_literal
        | tuple_literal

identifier → ('_' | letter) ('_' | letter | digit)*

lambda_expression → 'fn' '(' param_list? ')' return_type? '=>' expression
                  | 'fn' '(' param_list? ')' return_type? block

block_expression → '{' statement* expression? '}'

if_expression → 'if' expression block
             | 'if' expression block 'else' expression
             | 'if' expression block ('else if' expression block)* 'else' expression

match_expression → 'match' expression '{' match_arm* '}'

match_arm → pattern ('if' expression)? '=>' expression
          | pattern ('if' expression)? block

pattern → '_'  // wildcard
       | literal
       | identifier ':' type_annotation  // pattern binding
       | identifier ('(' pattern_list? ')')?  // enum variant
       | identifier '{' field_pattern_list? '}'  // struct pattern
       | pattern '|' pattern  // or pattern
       | pattern '..' pattern  // range pattern

loop_expression → 'loop' block
                | 'loop' identifier block

break_expression → 'break' identifier? ';'
                | 'break' identifier? expression

continue_expression → 'continue' identifier? ';'

return_expression → 'return' expression? ';'

unary_expression → unary_op expression
unary_op → '-' | '!' | '~' | '*' | '&' | 'ref'

binary_expression → expression binary_op expression
binary_op → '+' | '-' | '*' | '/' | '%' | '**'
          | '<<' | '>>' | '&' | '|' | '^'
          | '==' | '!=' | '<' | '>' | '<=' | '>='
          | '&&' | '||'

ternary_expression → expression '?' expression ':' expression

range_expression → expression '..' expression
                 | expression '..=' expression

type_cast_expression → expression 'as' type_annotation

type_check_expression → expression 'is' type_annotation

call_expression → expression '(' arg_list? ')'

arg_list → expression (',' expression)*

index_expression → expression '[' expression ']'

field_expression → expression '.' identifier

tuple_expression → '(' expression (',' expression)* ','? ')'

struct_expression → identifier '{' field_init (',' field_init)* ','? '}'
field_init → identifier ':' expression
          | identifier

array_expression → '[' expression (',' expression)* ','? ']'

pipeline_expression → expression '|>' expression

unsafe_expression → 'unsafe' expression

comptime_expression → 'comptime' expression
```

## 2.4 Type Annotations

```
type_annotation → type_name
                | generic_type
                | pointer_type
                | reference_type
                | array_type
                | slice_type
                | tuple_type
                | function_type
                | trait_object_type
                | option_type
                | result_type

type_name → identifier ('::' identifier)*

generic_type → type_name '<' type_list? '>'
type_list → type_annotation (',' type_annotation)*

pointer_type → '*' 'const'? type_annotation
             | '*' 'mut' type_annotation

reference_type → '&' 'const'? type_annotation
               | '&' 'mut' type_annotation

array_type → '[' type_annotation ';' expression ']'

slice_type → '[' type_annotation ']'

tuple_type → '(' type_annotation (',' type_annotation)* ','? ')'

function_type → 'fn' '(' type_list? ')' return_type?

trait_object_type → 'dyn' type_name ('+' type_name)*

option_type → 'option' '<' type_annotation '>'
            | type_annotation '?'

result_type → 'result' '<' type_annotation ',' type_annotation '>'
```

---

# PART III: TYPE SYSTEM

## 3.1 Primitive Types

### 3.1.1 Integer Types

| Type | Size | Range |
|------|------|-------|
| `i8` | 8-bit | -128 to 127 |
| `i16` | 16-bit | -32,768 to 32,767 |
| `i32` | 32-bit | -2,147,483,648 to 2,147,483,647 |
| `i64` | 64-bit | -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807 |
| `i128` | 128-bit | -170,141,183,460,469,231,731,687,303,715,884,105,728 to 170,141,183,460,469,231,731,687,303,715,884,105,727 |
| `i256` | 256-bit | Extended range |
| `u8` | 8-bit | 0 to 255 |
| `u16` | 16-bit | 0 to 65,535 |
| `u32` | 32-bit | 0 to 4,294,967,295 |
| `u64` | 64-bit | 0 to 18,446,744,073,709,551,615 |
| `u128` | 128-bit | 0 to 340,282,366,920,938,463,463,374,607,431,768,211,455 |
| `u256` | 256-bit | Extended range |
| `usize` | platform | Pointer-sized unsigned |
| `isize` | platform | Pointer-sized signed |

### 3.1.2 Floating-Point Types

| Type | Size | Precision |
|------|------|-----------|
| `f16` | 16-bit | IEEE 754 half-precision |
| `f32` | 32-bit | IEEE 754 single-precision |
| `f64` | 64-bit | IEEE 754 double-precision |
| `f128` | 128-bit | IEEE 754 quadruple-precision |

### 3.1.3 Other Primitives

| Type | Description |
|------|-------------|
| `bool` | Boolean (`true` / `false`) |
| `char` | Unicode scalar value (32-bit) |
| `byte` | Alias for `u8` |
| `void` | No value (used for functions that don't return) |
| `never` | Bottom type (never returns, e.g., `loop {}`) |

## 3.2 Composite Types

### 3.2.1 Structs

```knull
struct Point {
    x: f64,
    y: f64,
}

struct Config {
    host: String,
    port: u16,
    ssl: bool,
}
```

### 3.2.2 Enums

```knull
enum Result<T, E> {
    Ok(T),
    Err(E),
}

enum Option<T> {
    Some(T),
    None,
}

enum Color {
    Red,
    Green,
    Blue,
    RGB { r: u8, g: u8, b: u8 },
}
```

### 3.2.3 Unions

```knull
union Data {
    bytes: [u8; 8],
    as_u64: u64,
    as_f64: f64,
}
```

## 3.3 Special Types

### 3.3.1 Dynamic Type (`var`)

The `var` type is a dynamic type that can hold any value at runtime.

```knull
let dynamic: var = 42;
dynamic = "hello";  // Now a string
dynamic = [1, 2, 3];  // Now an array
```

### 3.3.2 Option Type

```knull
fn find_user(id: u32) -> option<User> {
    if exists(id) {
        Some(get_user(id))
    } else {
        None
    }
}

// Usage with safe navigation
let user = find_user(123)?;
```

### 3.3.3 Result Type

```knull
fn read_file(path: string) -> result<FileData, Error> {
    if path.exists() {
        Ok(FileData::read(path))
    } else {
        Err(Error::FileNotFound(path))
    }
}
```

### 3.3.4 Compile-Time Type (`comptime`)

Types that only exist at compile time.

```knull
comptime fn generate_table() -> [u8; 256] {
    // Compute at compile time
    let mut table = [0u8; 256];
    // ... generation logic
    table
}
```

## 3.4 Pointer Types

### 3.4.1 Safe References

```knull
let x: i32 = 42;
let r: ref i32 = &x;        // Shared reference
let m: ref mut i32 = &mut x;  // Mutable reference
```

### 3.4.2 Raw Pointers (unsafe)

```knull
unsafe {
    let ptr: *mut i32 = &mut x as *mut i32;
    ptr.write(100);
    let val = ptr.read();
}
```

## 3.5 Ownership Types

### 3.5.1 Owned Types (`own`)

```knull
fn process(data: own Vec<u8>) {
    // data is moved into function
    // automatically freed at end of scope
}
```

### 3.5.2 Borrowed Types

```knull
fn process(data: &[u8]) {
    // borrowed reference, no ownership
}
```

### 3.5.3 Reference-Counted Types

```knull
let shared = arc::new(SharedData::new());
let clone1 = arc::clone(&shared);
let clone2 = arc::clone(&shared);
// All three share ownership
```

---

# PART IV: MEMORY MODEL

## 4.1 The Three Memory Sovereignty Modes

### 4.1.1 Managed Mode (Novice)

In managed mode, Knull uses **Inference-Based Ownership (IBO)** with automatic reference counting.

```knull
// This code runs in managed mode by default
let names = vec!["Alice", "Bob"];
names.push("Charlie");
// names is automatically freed when it goes out of scope
// No memory leaks, no GC pauses
```

**Characteristics:**
- Automatic reference counting (`Rc<T>` internally)
- Zero-cost abstraction (compile-time determined)
- No garbage collector
- Deterministic destruction

### 4.1.2 Linear Mode (Expert)

In linear mode, values have exactly one owner and are moved, not copied.

```knull
fn process_buffer(buffer: own Buffer) -> own Buffer {
    // buffer is owned, moved
    let transformed = buffer.map(|b| b + 1);
    transformed  // moved out, caller owns
}
// Original buffer is dropped at end of scope
```

**Characteristics:**
- Move semantics by default
- No accidental copying
- Zero-cost safety
- Borrow checker active

### 4.1.3 Manual Mode (God)

In manual mode, the programmer has full control over memory.

```knull
unsafe {
    // Allocate raw memory
    let ptr = mem::alloc::<u8>(1024);
    
    // Write to memory
    ptr.write(0xFF);
    ptr.offset(512).write(0x00);
    
    // Read from memory
    let byte = ptr.read();
    
    // Free memory
    mem::free(ptr);
}
```

**Characteristics:**
- No safety checks
- Raw pointer arithmetic
- Manual memory management
- Undefined behavior possible

## 4.2 The Memory Namespace

### 4.2.1 Allocation Functions

```knull
// Allocate heap memory
fn alloc<T>(count: usize) -> *mut T

// Allocate and zero-initialize
fn alloc_zeroed<T>(count: usize) -> *mut T

// Reallocate memory
fn realloc<T>(ptr: *mut T, new_count: usize) -> *mut T

// Free memory
fn free<T>(ptr: *mut T)

// Stack allocation (compile-time size)
fn stack_alloc<T: Sized>(count: usize) -> [T; count]
```

### 4.2.2 Memory Operations

```knull
// Copy memory (overlapping safe, overlapping unsafe)
fn copy<T>(src: *const T, dest: *mut T, count: usize)
fn copy_nonoverlapping<T>(src: *const T, dest: *mut T, count: usize)

// Set memory
fn set<T>(ptr: *mut T, value: T, count: usize)
fn zero<T>(ptr: *mut T, count: usize)

// Volatile access (for MMIO)
fn read_volatile<T>(ptr: *const T) -> T
fn write_volatile<T>(ptr: *mut T, value: T)
```

## 4.3 Security Features

### 4.3.1 Stack Canaries

Knull automatically inserts stack canaries in debug builds:

```knull
fn vulnerable() {
    let buffer: [u8; 64];
    // Compiler inserts: canary = random()
    // ... function body ...
    // Compiler checks: if canary != original { panic!("stack overflow") }
}
```

### 4.3.2 ASLR Enforcement

Knull binaries are position-independent by default:
- All allocations use random offsets
- Function pointers are randomized
- vtable randomization

### 4.3.3 Bounds Checking

```knull
let arr = [1, 2, 3];
let x = arr[5];  // Panics with bounds error
```

In `unsafe` blocks, bounds checking is disabled:
```knull
unsafe {
    let x = arr.offset(5);  // No check, undefined behavior if OOB
}
```

---

# PART V: CONTROL FLOW

## 5.1 Conditionals

### 5.1.1 If/Else

```knull
if condition {
    // ...
} else if other_condition {
    // ...
} else {
    // ...
}

// Expression form
let result = if x > 0 { "positive" } else { "non-positive" };
```

### 5.1.2 Ternary Operator

```knull
let result = condition ? value_if_true : value_if_false;
```

## 5.2 Loops

### 5.2.1 Infinite Loop

```knull
loop {
    // infinite loop
    if should_exit {
        break;
    }
}

// With label
loop 'outer {
    loop 'inner {
        break 'outer;  // breaks outer loop
    }
}
```

### 5.2.2 While Loop

```knull
while condition {
    // ...
}

while let Some(value) = iterator.next() {
    // pattern matching in while
}
```

### 5.2.3 For Loop

```knull
for item in collection {
    // iterate by value
}

for item in &collection {
    // iterate by reference
}

for item in &mut collection {
    // iterate by mutable reference
}

// Range-based
for i in 0..10 {
    // 0 to 9
}

for i in 0..=10 {
    // 0 to 10 inclusive
}
```

## 5.3 Pattern Matching

### 5.3.1 Match Expression

```knull
match value {
    0 => "zero",
    1 | 2 => "one or two",
    n if n > 10 => "greater than ten",
    _ => "other",
}

// With destructuring
match point {
    Point { x: 0, y: 0 } => "origin",
    Point { x, y } if x == y => "diagonal",
    Point { x, y } => format!("({}, {})", x, y),
}

// With enum
match result {
    Ok(value) => process(value),
    Err(Error::NotFound) => handle_not_found(),
    Err(e) => handle_error(e),
}
```

## 5.4 Defer

### 5.4.1 Scoped Cleanup

```knull
fn read_and_close() {
    let file = File::open("data.txt");
    
    defer {
        file.close();
    }
    
    // ... use file ...
    // file.close() called automatically at end of scope
}

// Works with expressions too
fn with_cleanup() {
    defer { cleanup(); }
    
    let result = do_something();
    if result.is_error() {
        return;  // cleanup still runs
    }
    result
}
```

---

# PART VI: SPECIAL FEATURES

## 6.1 Unsafe Blocks

Unsafe blocks disable safety checks:

```knull
unsafe {
    // Raw pointer dereference
    let ptr = 0x1000 as *mut u8;
    ptr.write(0xFF);
    
    // Inline assembly
    asm { "mov rax, 60" "mov rdi, 0" "syscall" }
    
    // Direct syscall
    syscall(SYS_exit, 0);
    
    // Union field access
    let data: Data = union_ptr.read();
}
```

## 6.2 Inline Assembly

```knull
// Intel syntax (default)
asm intel {
    "mov rax, 60"
    "mov rdi, 0"
    "syscall"
}

// AT&T syntax
asm att {
    "mov $60, %rax"
    "mov $0, %rdi"
    "syscall"
}

// With operands
unsafe {
    let result: u64;
    asm intel {
        "rdtsc"
        "mov eax, edx"
        "mov [${0}], eax"
        : "=r"(result)
        :
        : "eax", "edx"
    };
}
```

## 6.3 Direct Syscalls

```knull
// Linux x86_64
syscall(SYS_write, STDOUT, msg_ptr, msg_len);

// Windows NT
syscall(NT_WriteFile, handle, buffer, length, &bytes_written, nullptr);
```

## 6.4 Compile-Time Execution

```knull
// Compile-time function evaluation
comptime fn fib(n: u32) -> u32 {
    if n <= 1 { n } else { fib(n-1) + fib(n-2) }
}

// Generate lookup table at compile time
comptime {
    let table = generate_crypto_table();
    embed_bytes(table);
}

// Type-level computation
type AssertSize<T, N> = comptime_assert!(size_of::<T>() == N);
```

## 6.5 Macros

```knull
// Declarative macro
macro! assert_eq(a, b) {
    if a != b {
        panic!("assertion failed: {} != {}", a, b);
    }
}

// Usage
assert_eq!(2 + 2, 4);

// Macro with repetition
macro! vec![..$($element:expr),*] = {
    let mut v = Vec::new();
    $(v.push($element);)*
    v
};
```

## 6.6 Embed Binary Data

```knull
// Embed file contents
let logo = embed!("assets/logo.png");

// Embed raw bytes
let shellcode = embed_bytes!([0x90, 0x90, 0xC3]);

// Embed assembly
let machine_code = asm! {
    "0xB800000000"  // mov eax, 0
    "0xC3"          // ret
};
```

## 6.7 Pipeline Operator

```knull
// Transform data through a chain of functions
let result = data
    |> filter(|x| x > 0)
    |> map(|x| x * 2)
    |> fold(0, |a, b| a + b);
```

## 6.8 Safe Navigation

```knull
// Null-safe member access
let length = user?.address?.city?.name?.length();

// Equivalent to:
let length = if let Some(addr) = user.address {
    if let Some(city) = addr.city {
        if let Some(name) = city.name {
            Some(name.length())
        } else { None }
    } else { None }
} else { None };
```

## 6.9 Elvis Operator

```knull
// Null-coalescing
let value = optional ?? default_value;

// Chained
let value = a ?? b ?? c ?? default;
```

---

# PART VII: STANDARD LIBRARY CORE

## 7.1 Option and Result

### 7.1.1 Option

```knull
enum option<T> {
    Some(T),
    None,
}

// Methods
fn is_some(&self) -> bool
fn is_none(&self) -> bool
fn unwrap(self) -> T
fn unwrap_or(self, default: T) -> T
fn unwrap_or_else(self, f: fn() -> T) -> T
fn map<U>(self, f: fn(T) -> U) -> option<U>
fn and_then<U>(self, f: fn(T) -> option<U>) -> option<U>
```

### 7.1.2 Result

```knull
enum result<T, E> {
    Ok(T),
    Err(E),
}

// Methods
fn is_ok(&self) -> bool
fn is_err(&self) -> bool
fn unwrap(self) -> T
fn unwrap_err(self) -> E
fn unwrap_or(self, default: T) -> T
fn map<U>(self, f: fn(T) -> U) -> result<U, E>
fn map_err<F>(self, f: fn(E) -> F) -> result<T, F>
fn and_then<U>(self, f: fn(T) -> result<U, E>) -> result<U, E>
fn or_else<F>(self, f: fn(E) -> result<T, F>) -> result<T, F>
```

## 7.2 Collections

### 7.2.1 Vec<T>

```knull
struct Vec<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
}

impl<T> Vec<T> {
    fn new() -> Self
    fn with_capacity(cap: usize) -> Self
    fn push(&mut self, value: T)
    fn pop(&mut self) -> option<T>
    fn len(&self) -> usize
    fn is_empty(&self) -> bool
    fn get(&self, index: usize) -> option<&T>
    fn get_mut(&mut self, index: usize) -> option<&mut T>
    fn clear(&mut self)
    fn capacity(&self) -> usize
    fn reserve(&mut self, additional: usize)
    fn truncate(&mut self, new_len: usize)
    fn swap_remove(&mut self, index: usize) -> T
    fn insert(&mut self, index: usize, value: T)
    fn remove(&mut self, index: usize) -> T
}
```

### 7.2.2 HashMap<K, V>

```knull
struct HashMap<K, V> { /* ... */ }

impl<K, V> HashMap<K, V> where K: Hash {
    fn new() -> Self
    fn insert(&mut self, key: K, value: V) -> option<V>
    fn get(&self, key: &K) -> option<&V>
    fn get_mut(&mut self, key: &K) -> option<&mut V>
    fn remove(&mut self, key: &K) -> option<V>
    fn contains_key(&self, key: &K) -> bool
    fn len(&self) -> usize
    fn is_empty(&self) -> bool
    fn clear(&mut self)
    fn keys(&self) -> Keys<K>
    fn values(&self) -> Values<V>
}
```

---

# APPENDIX A: KEYWORD REFERENCE

| Keyword | Description |
|---------|-------------|
| `fn` | Function declaration |
| `let` | Immutable binding |
| `var` | Mutable binding (dynamic) |
| `own` | Ownership transfer |
| `mut` | Mutable reference |
| `ref` | Borrowed reference |
| `const` | Compile-time constant |
| `static` | Global/static variable |
| `struct` | Structure definition |
| `enum` | Enumeration definition |
| `union` | Union definition |
| `trait` | Trait definition |
| `impl` | Implementation block |
| `type` | Type alias |
| `actor` | Concurrent process |
| `macro` | Macro definition |
| `unsafe` | Unsafe block |
| `comptime` | Compile-time execution |
| `ghost` | Compile-time only |
| `if` | Conditional |
| `else` | Alternative branch |
| `match` | Pattern matching |
| `loop` | Infinite loop |
| `while` | Conditional loop |
| `for` | Iterator loop |
| `in` | Iterator binding |
| `return` | Return value |
| `break` | Exit loop |
| `continue` | Next iteration |
| `defer` | Scoped cleanup |
| `goto` | Unconditional jump |
| `embed` | Embed binary |
| `asm` | Inline assembly |
| `syscall` | Direct syscall |
| `import` | Import module |
| `export` | Export symbol |
| `pub` | Public visibility |
| `priv` | Private visibility |
| `module` | Module declaration |
| `as` | Type cast |
| `is` | Type check |
| `null` | Null value |
| `true` | Boolean true |
| `false` | Boolean false |
| `self` | Current instance |
| `super` | Parent module |

---

# APPENDIX B: OPERATOR PRECEDENCE

| Precedence | Operators | Associativity |
|------------|-----------|----------------|
| 1 | `::` | Left |
| 2 | `a()` `a[]` `a.` | Left |
| 3 | `await` | Right |
| 4 | `**` | Right |
| 5 | `*` `/` `%` | Left |
| 6 | `+` `-` | Left |
| 7 | `<<` `>>` | Left |
| 8 | `&` | Left |
| 9 | `^` | Left |
| 10 | `|` | Left |
| 11 | `==` `!=` `<` `>` `<=` `>=` `is` | Left |
| 12 | `&&` | Left |
| 13 | `\|\|` | Left |
| 14 | `..` `..=` | Left |
| 15 | `?:` | Right |
| 16 | `=` `+=` `-=` `*=` `/=` `%=` `<<=` `>>=` `&=` `\|=` `^=` | Right |
| 17 | `,` | Left |

---

*Specification Version: 1.0*
*Last Updated: 2026*
