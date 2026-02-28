# Tutorial: FFI - Foreign Function Interface

This tutorial covers using Knull's FFI capabilities to interoperate with C libraries, with a practical example using SQLite.

## Overview

By the end of this tutorial, you'll be able to:
- Load and use C libraries
- Call C functions from Knull
- Handle C strings and data structures
- Create a complete SQLite wrapper

## Prerequisites

- Knull installed
- Basic C knowledge
- SQLite development libraries installed

## Part 1: FFI Basics

### What is FFI?

FFI (Foreign Function Interface) allows Knull code to call functions written in other languages, primarily C. This enables:
- Using existing C libraries
- System-level programming
- Performance-critical code
- Hardware interaction

### Mode Declaration

FFI operations require god mode:

```knull
mode god

// FFI declarations
extern "C" {
    // C functions go here
}
```

## Part 2: Calling C Standard Library

### Step 1: Basic C Functions

Create `src/ffi_basics.knull`:

```knull
mode god

// Link with C standard library
#[link(name = "c")]
extern "C" {
    // Standard I/O
    fn printf(format: *const u8, ...) -> i32
    fn puts(s: *const u8) -> i32
    fn putchar(c: i32) -> i32
    
    // Memory management
    fn malloc(size: usize) -> *mut u8
    fn free(ptr: *mut u8)
    fn calloc(nmemb: usize, size: usize) -> *mut u8
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8
    fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8
    fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8
    fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32
    
    // String functions
    fn strlen(s: *const u8) -> usize
    fn strcpy(dest: *mut u8, src: *const u8) -> *mut u8
    fn strncpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8
    fn strcat(dest: *mut u8, src: *const u8) -> *mut u8
    fn strcmp(s1: *const u8, s2: *const u8) -> i32
    fn strstr(haystack: *const u8, needle: *const u8) -> *const u8
    
    // Time
    fn time(tloc: *mut i64) -> i64
    fn sleep(seconds: u32) -> u32
}

fn main() {
    unsafe {
        // Print using C printf
        printf("Hello from %s\n".as_ptr(), "Knull".as_ptr())
        
        // Simple string output
        puts("Using C puts function".as_ptr())
        
        // Get string length
        let s = "Hello, World!"
        let len = strlen(s.as_ptr())
        printf("Length: %zu\n".as_ptr(), len)
        
        // Allocate memory
        let ptr = malloc(1024)
        if !ptr.is_null() {
            // Use memory...
            strcpy(ptr, "Allocated with malloc".as_ptr())
            puts(ptr)
            free(ptr)
        }
    }
}
```

### Step 2: Safe Wrappers

Create safe wrappers around unsafe C functions:

```knull
mode god

// Safe wrapper for C strings
struct CString {
    ptr: *mut u8,
}

impl CString {
    fn new(s: &str) -> Option<CString> {
        unsafe {
            let ptr = malloc(s.len() + 1) as *mut u8
            if ptr.is_null() {
                return None
            }
            
            // Copy string bytes + null terminator
            memcpy(ptr, s.as_ptr(), s.len())
            *ptr.offset(s.len() as isize) = 0
            
            Some(CString { ptr })
        }
    }
    
    fn as_ptr(&self) -> *const u8 {
        self.ptr
    }
    
    fn to_string(&self) -> String {
        unsafe {
            let len = strlen(self.ptr)
            let bytes = std::slice::from_raw_parts(self.ptr, len)
            String::from_utf8_lossy(bytes).to_string()
        }
    }
}

impl Drop for CString {
    fn drop(&mut self) {
        unsafe {
            free(self.ptr)
        }
    }
}

// Safe wrapper for C allocated memory
struct CBuffer {
    ptr: *mut u8,
    size: usize,
}

impl CBuffer {
    fn new(size: usize) -> Option<CBuffer> {
        unsafe {
            let ptr = malloc(size)
            if ptr.is_null() {
                return None
            }
            Some(CBuffer { ptr, size })
        }
    }
    
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }
    
    fn as_slice(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.ptr, self.size)
        }
    }
    
    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr, self.size)
        }
    }
}

impl Drop for CBuffer {
    fn drop(&mut self) {
        unsafe {
            free(self.ptr)
        }
    }
}
```

## Part 3: Dynamic Library Loading

### Step 3: Dynamic Loading

```knull
mode god

#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const u8, flag: i32) -> *mut u8
    fn dlsym(handle: *mut u8, symbol: *const u8) -> *mut u8
    fn dlclose(handle: *mut u8) -> i32
    fn dlerror() -> *const u8
}

const RTLD_LAZY: i32 = 0x00001
const RTLD_NOW: i32 = 0x00002

struct DynamicLibrary {
    handle: *mut u8,
}

impl DynamicLibrary {
    fn open(path: &str) -> Result<DynamicLibrary, String> {
        let c_path = CString::new(path)
            .ok_or("Failed to create CString")?
        
        unsafe {
            let handle = dlopen(c_path.as_ptr(), RTLD_NOW)
            
            if handle.is_null() {
                let err = CStr::from_ptr(dlerror())
                    .to_string_lossy()
                    .to_string()
                return Err(err)
            }
            
            Ok(DynamicLibrary { handle })
        }
    }
    
    fn get_symbol<T>(&self, name: &str) -> Option<*mut T> {
        let c_name = CString::new(name)?
        
        unsafe {
            let symbol = dlsym(self.handle, c_name.as_ptr())
            if symbol.is_null() {
                return None
            }
            Some(symbol as *mut T)
        }
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        unsafe {
            dlclose(self.handle)
        }
    }
}
```

## Part 4: SQLite Wrapper

### Step 4: SQLite Types

Create `src/sqlite.knull`:

```knull
mode god

// SQLite Opaque Types
type Sqlite3 = u8
type Sqlite3Stmt = u8
type Sqlite3Value = u8

// SQLite Constants
const SQLITE_OK: i32 = 0
const SQLITE_ROW: i32 = 100
const SQLITE_DONE: i32 = 101
const SQLITE_INTEGER: i32 = 1
const SQLITE_FLOAT: i32 = 2
const SQLITE_TEXT: i32 = 3
const SQLITE_BLOB: i32 = 4
const SQLITE_NULL: i32 = 5

#[link(name = "sqlite3")]
extern "C" {
    // Database connection
    fn sqlite3_open(filename: *const u8, db: *mut *mut Sqlite3) -> i32
    fn sqlite3_close(db: *mut Sqlite3) -> i32
    fn sqlite3_errmsg(db: *mut Sqlite3) -> *const u8
    fn sqlite3_errcode(db: *mut Sqlite3) -> i32
    
    // Statement execution
    fn sqlite3_prepare_v2(
        db: *mut Sqlite3,
        sql: *const u8,
        nByte: i32,
        ppStmt: *mut *mut Sqlite3Stmt,
        pzTail: *mut *const u8
    ) -> i32
    fn sqlite3_step(stmt: *mut Sqlite3Stmt) -> i32
    fn sqlite3_finalize(stmt: *mut Sqlite3Stmt) -> i32
    fn sqlite3_reset(stmt: *mut Sqlite3Stmt) -> i32
    
    // Parameter binding
    fn sqlite3_bind_int(stmt: *mut Sqlite3Stmt, idx: i32, value: i32) -> i32
    fn sqlite3_bind_int64(stmt: *mut Sqlite3Stmt, idx: i32, value: i64) -> i32
    fn sqlite3_bind_double(stmt: *mut Sqlite3Stmt, idx: i32, value: f64) -> i32
    fn sqlite3_bind_text(
        stmt: *mut Sqlite3Stmt,
        idx: i32,
        value: *const u8,
        n: i32,
        destructor: *const u8
    ) -> i32
    fn sqlite3_bind_null(stmt: *mut Sqlite3Stmt, idx: i32) -> i32
    fn sqlite3_clear_bindings(stmt: *mut Sqlite3Stmt) -> i32
    
    // Result retrieval
    fn sqlite3_column_count(stmt: *mut Sqlite3Stmt) -> i32
    fn sqlite3_column_name(stmt: *mut Sqlite3Stmt, col: i32) -> *const u8
    fn sqlite3_column_type(stmt: *mut Sqlite3Stmt, col: i32) -> i32
    fn sqlite3_column_int(stmt: *mut Sqlite3Stmt, col: i32) -> i32
    fn sqlite3_column_int64(stmt: *mut Sqlite3Stmt, col: i32) -> i64
    fn sqlite3_column_double(stmt: *mut Sqlite3Stmt, col: i32) -> f64
    fn sqlite3_column_text(stmt: *mut Sqlite3Stmt, col: i32) -> *const u8
    fn sqlite3_column_bytes(stmt: *mut Sqlite3Stmt, col: i32) -> i32
}
```

### Step 5: Safe SQLite API

```knull
mode god

struct Database {
    db: *mut Sqlite3,
}

impl Database {
    fn open(path: &str) -> Result<Database, String> {
        let c_path = CString::new(path)
            .ok_or("Invalid path")?
        
        unsafe {
            let mut db: *mut Sqlite3 = std::ptr::null_mut()
            let result = sqlite3_open(c_path.as_ptr(), &mut db)
            
            if result != SQLITE_OK {
                let msg = if db.is_null() {
                    "Failed to allocate database handle".to_string()
                } else {
                    let err = CStr::from_ptr(sqlite3_errmsg(db))
                        .to_string_lossy()
                        .to_string()
                    sqlite3_close(db)
                    err
                }
                return Err(msg)
            }
            
            Ok(Database { db })
        }
    }
    
    fn execute(&self, sql: &str) -> Result<(), String> {
        unsafe {
            let c_sql = CString::new(sql).ok_or("Invalid SQL")?
            
            let mut stmt: *mut Sqlite3Stmt = std::ptr::null_mut()
            let result = sqlite3_prepare_v2(
                self.db,
                c_sql.as_ptr(),
                -1,  // Read until null terminator
                &mut stmt,
                std::ptr::null_mut()
            )
            
            if result != SQLITE_OK {
                return Err(self.last_error())
            }
            
            // Execute statement
            let step_result = sqlite3_step(stmt)
            sqlite3_finalize(stmt)
            
            if step_result != SQLITE_DONE && step_result != SQLITE_ROW {
                return Err(self.last_error())
            }
            
            Ok(())
        }
    }
    
    fn query(&self, sql: &str) -> Result<Statement, String> {
        unsafe {
            let c_sql = CString::new(sql).ok_or("Invalid SQL")?
            
            let mut stmt: *mut Sqlite3Stmt = std::ptr::null_mut()
            let result = sqlite3_prepare_v2(
                self.db,
                c_sql.as_ptr(),
                -1,
                &mut stmt,
                std::ptr::null_mut()
            )
            
            if result != SQLITE_OK {
                return Err(self.last_error())
            }
            
            Ok(Statement { stmt })
        }
    }
    
    fn last_error(&self) -> String {
        unsafe {
            CStr::from_ptr(sqlite3_errmsg(self.db))
                .to_string_lossy()
                .to_string()
        }
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.db)
        }
    }
}

struct Statement {
    stmt: *mut Sqlite3Stmt,
}

impl Statement {
    fn bind_int(&mut self, idx: i32, value: i32) -> Result<(), String> {
        unsafe {
            let result = sqlite3_bind_int(self.stmt, idx, value)
            if result != SQLITE_OK {
                return Err("Failed to bind int".to_string())
            }
            Ok(())
        }
    }
    
    fn bind_text(&mut self, idx: i32, value: &str) -> Result<(), String> {
        unsafe {
            let c_value = CString::new(value).ok_or("Invalid text")?
            let result = sqlite3_bind_text(
                self.stmt,
                idx,
                c_value.as_ptr(),
                -1,
                std::ptr::null()  // SQLite makes its own copy
            )
            if result != SQLITE_OK {
                return Err("Failed to bind text".to_string())
            }
            Ok(())
        }
    }
    
    fn step(&mut self) -> Result<RowState, String> {
        unsafe {
            match sqlite3_step(self.stmt) {
                SQLITE_ROW => Ok(RowState::Row),
                SQLITE_DONE => Ok(RowState::Done),
                _ => Err("Step failed".to_string()),
            }
        }
    }
    
    fn column_int(&self, col: i32) -> i32 {
        unsafe { sqlite3_column_int(self.stmt, col) }
    }
    
    fn column_text(&self, col: i32) -> String {
        unsafe {
            let ptr = sqlite3_column_text(self.stmt, col)
            CStr::from_ptr(ptr)
                .to_string_lossy()
                .to_string()
        }
    }
    
    fn column_name(&self, col: i32) -> String {
        unsafe {
            CStr::from_ptr(sqlite3_column_name(self.stmt, col))
                .to_string_lossy()
                .to_string()
        }
    }
    
    fn column_count(&self) -> i32 {
        unsafe { sqlite3_column_count(self.stmt) }
    }
    
    fn reset(&mut self) {
        unsafe {
            sqlite3_reset(self.stmt)
            sqlite3_clear_bindings(self.stmt)
        }
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        unsafe {
            sqlite3_finalize(self.stmt)
        }
    }
}

enum RowState {
    Row,
    Done,
}
```

### Step 6: Using SQLite

```knull
mode god

fn main() {
    // Open database
    let db = match Database::open("test.db") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e)
            return
        }
    }
    
    // Create table
    db.execute(r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            age INTEGER
        )
    "#).expect("Failed to create table")
    
    // Insert data using prepared statement
    let mut stmt = db.query("INSERT INTO users (name, email, age) VALUES (?, ?, ?)")
        .expect("Failed to prepare insert")
    
    // Insert user 1
    stmt.bind_text(1, "Alice")?
    stmt.bind_text(2, "alice@example.com")?
    stmt.bind_int(3, 30)?
    stmt.step()
    stmt.reset()
    
    // Insert user 2
    stmt.bind_text(1, "Bob")?
    stmt.bind_text(2, "bob@example.com")?
    stmt.bind_int(3, 25)?
    stmt.step()
    stmt.reset()
    
    println!("Users inserted successfully")
    
    // Query data
    let mut query = db.query("SELECT id, name, email, age FROM users")
        .expect("Failed to prepare query")
    
    println!("\nUsers:")
    println!("{:<5} {:<15} {:<25} {:<5}", "ID", "Name", "Email", "Age")
    println!("{}", "-".repeat(60))
    
    while let Ok(RowState::Row) = query.step() {
        let id = query.column_int(0)
        let name = query.column_text(1)
        let email = query.column_text(2)
        let age = query.column_int(3)
        
        println!("{:<5} {:<15} {:<25} {:<5}", id, name, email, age)
    }
}
```

## Part 5: Advanced FFI

### Step 7: Callbacks

```knull
mode god

// Define callback type
type CompareFn = extern "C" fn(*const u8, *const u8) -> i32

#[link(name = "c")]
extern "C" {
    fn qsort(
        base: *mut u8,
        nmemb: usize,
        size: usize,
        compar: CompareFn
    )
}

// Callback function
extern "C" fn compare_ints(a: *const u8, b: *const u8) -> i32 {
    unsafe {
        let val_a = *(a as *const i32)
        let val_b = *(b as *const i32)
        
        if val_a < val_b { -1 }
        else if val_a > val_b { 1 }
        else { 0 }
    }
}

fn main() {
    let mut numbers: [i32; 5] = [5, 2, 8, 1, 9]
    
    println!("Before: {:?}", numbers)
    
    unsafe {
        qsort(
            numbers.as_mut_ptr() as *mut u8,
            numbers.len(),
            std::mem::size_of::<i32>(),
            compare_ints
        )
    }
    
    println!("After: {:?}", numbers)
}
```

### Step 8: Structs and Unions

```knull
mode god

// C struct
#[repr(C)]
struct Person {
    name: [u8; 64],
    age: i32,
    height: f32,
}

// C union
#[repr(C)]
union Value {
    int_val: i32,
    float_val: f32,
    bytes: [u8; 4],
}

#[link(name = "example")]
extern "C" {
    fn process_person(p: *const Person) -> i32
    fn process_value(v: *const Value, type_tag: i32) -> f64
}

fn main() {
    let person = Person {
        name: {
            let mut name = [0u8; 64]
            let bytes = "Alice".as_bytes()
            name[..bytes.len()].copy_from_slice(bytes)
            name
        },
        age: 30,
        height: 5.6,
    }
    
    unsafe {
        let result = process_person(&person)
        println!("Process result: {}", result)
    }
    
    // Using union
    let mut value = Value { int_val: 42 }
    
    unsafe {
        println!("As int: {}", value.int_val)
        println!("As bytes: {:?}", value.bytes)
        
        value.float_val = 3.14
        println!("As float: {}", value.float_val)
    }
}
```

## Building and Running

```bash
# Install SQLite development files
# Ubuntu/Debian:
sudo apt-get install libsqlite3-dev

# macOS:
brew install sqlite3

# Compile with SQLite
knull build --release src/main.knull -l sqlite3

# Run
./main
```

## Common Issues

### Linking Errors
- Ensure library is installed (`libsqlite3-dev`)
- Check library path (`-L` flag)
- Verify library name (no "lib" prefix or extension)

### ABI Mismatches
- Use `#[repr(C)]` for structs
- Match C types exactly (`c_int`, `c_char`, etc.)
- Be careful with bool (use `c_bool` or i32)

### Memory Safety
- Always free C-allocated memory
- Don't use Knull pointers after C modifies them
- Be careful with string lifetimes

## Exercises

1. **Complete SQLite ORM**: Build a macro-based ORM
2. **libcurl Wrapper**: Create HTTP client using libcurl
3. **OpenSSL Bindings**: Implement TLS/SSL support
4. **GTK Interface**: Build GUI using GTK bindings
5. **System Library**: Interface with platform-specific APIs

---

*FFI is powerful but dangerous. Always wrap unsafe code in safe abstractions.*
