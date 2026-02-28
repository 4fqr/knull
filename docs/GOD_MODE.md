# God Mode Guide

## Overview

God mode enables unsafe operations, direct memory access, and inline assembly. It's designed for bare-metal programming, operating system development, embedded systems, and security research.

## When to Use

- Operating system kernels
- Bootloaders
- Embedded systems
- Device drivers
- Security research
- Performance-critical code
- Interfacing with hardware

## Mode Declaration

```knull
mode god

fn main() {
    // Your unsafe code here
}
```

## Unsafe Blocks

### Basic Unsafe Operations

```knull
mode god

fn main() {
    unsafe {
        // Raw pointer manipulation
        let mut num = 5
        let r1 = &num as *const i32
        let r2 = &mut num as *mut i32
        
        // Dereference raw pointers
        println!("r1 is: {}", *r1)
        *r2 = 10
        println!("r2 is now: {}", *r2)
    }
}
```

### Direct Memory Access

```knull
mode god

fn main() {
    unsafe {
        // Write to a specific memory address
        let ptr = 0x1000 as *mut u8
        *ptr = 0xFF
        
        // Read from hardware register
        let vga_buffer = 0xB8000 as *mut u16
        *vga_buffer = 'H' as u16 | 0x0F00  // White 'H' on black
    }
}
```

### Raw Pointer Arithmetic

```knull
mode god

fn main() {
    unsafe {
        let arr: [i32; 5] = [1, 2, 3, 4, 5]
        let ptr = arr.as_ptr()
        
        // Pointer arithmetic
        println!("First: {}", *ptr)
        println!("Second: {}", *ptr.offset(1))
        println!("Third: {}", *ptr.offset(2))
    }
}
```

## Inline Assembly

### AT&T Syntax (Default)

```knull
mode god

fn main() {
    let result: i32
    
    unsafe {
        asm!(
            "movl $42, %eax",
            out("eax") result
        )
    }
    
    println!("Result: {}", result)  // 42
}
```

### Intel Syntax

```knull
mode god

fn main() {
    unsafe {
        // Exit with code 0 using syscall
        asm intel!(
            "mov rax, 60",   // sys_exit
            "xor rdi, rdi",  // exit code 0
            "syscall"
        )
    }
}
```

### Assembly with Inputs

```knull
mode god

fn add_asm(a: i64, b: i64) -> i64 {
    let result: i64
    
    unsafe {
        asm!(
            "add {0}, {1}",
            inout(reg) a => result,
            in(reg) b,
        )
    }
    
    result
}

fn main() {
    println!("10 + 20 = {}", add_asm(10, 20))  // 30
}
```

### Assembly with Clobbers

```knull
mode god

fn main() {
    unsafe {
        asm!(
            "cpuid",
            in("eax") 0,  // Get vendor string
            out("ebx") _,
            out("ecx") _,
            out("edx") _,
            clobber("eax")
        )
    }
}
```

### Volatile Assembly

```knull
mode god

fn main() {
    unsafe {
        // Volatile: compiler won't optimize this away
        asm volatile!(
            "nop",
            "nop",
            "nop"
        )
    }
}
```

## System Calls

### Direct Syscalls

```knull
mode god

fn main() {
    // Write to stdout
    let msg = "Hello from syscall\n"
    let result = syscall(1, 1, msg.as_ptr(), msg.len(), 0, 0, 0)
    
    // Exit with code 0
    syscall(60, 0, 0, 0, 0, 0, 0)
}
```

### Common Syscall Numbers (x86_64 Linux)

| Syscall | Number | Arguments |
|---------|--------|-----------|
| `read` | 0 | fd, buf, count |
| `write` | 1 | fd, buf, count |
| `open` | 2 | pathname, flags, mode |
| `close` | 3 | fd |
| `stat` | 4 | pathname, statbuf |
| `mmap` | 9 | addr, length, prot, flags, fd, offset |
| `exit` | 60 | error_code |
| `getdents` | 78 | fd, dirent, count |
| `getpid` | 39 | - |

### Syscall Wrapper Example

```knull
mode god

mod sys {
    const SYS_WRITE: i64 = 1
    const SYS_EXIT: i64 = 60
    
    pub fn write(fd: i32, buf: &[u8]) -> i64 {
        syscall(SYS_WRITE, fd, buf.as_ptr(), buf.len(), 0, 0, 0)
    }
    
    pub fn exit(code: i32) -> ! {
        syscall(SYS_EXIT, code, 0, 0, 0, 0, 0)
        unreachable!()
    }
}

fn main() {
    let msg = b"Hello, World!\n"
    sys::write(1, msg)
    sys::exit(0)
}
```

## Bare-Metal Programming

### No-Std Entry Point

```knull
mode god
#![no_std]
#![no_main]

use core::panic::PanicInfo

// Custom panic handler
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Entry point for bare metal
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u16
    
    unsafe {
        // Write "Knull" to VGA buffer
        *vga_buffer.offset(0) = 'K' as u16 | 0x0F00
        *vga_buffer.offset(1) = 'n' as u16 | 0x0F00
        *vga_buffer.offset(2) = 'u' as u16 | 0x0F00
        *vga_buffer.offset(3) = 'l' as u16 | 0x0F00
        *vga_buffer.offset(4) = 'l' as u16 | 0x0F00
    }
    
    loop {}
}
```

### Memory-Mapped I/O

```knull
mode god

// UART 16550 serial port
const UART_BASE: u16 = 0x3F8

fn uart_init() {
    unsafe {
        outb(UART_BASE + 1, 0x00)  // Disable interrupts
        outb(UART_BASE + 3, 0x80)  // Enable DLAB
        outb(UART_BASE + 0, 0x03)  // Set divisor to 3 (38400 baud)
        outb(UART_BASE + 1, 0x00)
        outb(UART_BASE + 3, 0x03)  // 8 bits, no parity, one stop
        outb(UART_BASE + 2, 0xC7)  // Enable FIFO
        outb(UART_BASE + 4, 0x0B)  // Enable interrupts
    }
}

fn uart_putc(c: u8) {
    unsafe {
        while (inb(UART_BASE + 5) & 0x20) == 0 {}  // Wait for transmit empty
        outb(UART_BASE, c)
    }
}

fn uart_puts(s: &str) {
    for c in s.bytes() {
        uart_putc(c)
    }
}

// Port I/O functions
unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out %al, %dx",
        in("dx") port,
        in("al") value,
    )
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8
    asm!(
        "in %dx, %al",
        in("dx") port,
        out("al") value,
    )
    value
}
```

### CPUID

```knull
mode god

#[repr(C)]
struct CpuidResult {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
}

fn cpuid(leaf: u32) -> CpuidResult {
    let mut result: CpuidResult = unsafe { core::mem::zeroed() }
    
    unsafe {
        asm!(
            "cpuid",
            inout("eax") leaf => result.eax,
            out("ebx") result.ebx,
            out("ecx") result.ecx,
            out("edx") result.edx,
        )
    }
    
    result
}

fn get_vendor_string() -> [u8; 12] {
    let result = cpuid(0)
    
    let mut vendor = [0u8; 12]
    
    // Vendor string is in ebx, edx, ecx (in that order!)
    unsafe {
        core::ptr::copy_nonoverlapping(
            &result.ebx as *const _ as *const u8,
            vendor.as_mut_ptr(),
            4
        )
        core::ptr::copy_nonoverlapping(
            &result.edx as *const _ as *const u8,
            vendor.as_mut_ptr().offset(4),
            4
        )
        core::ptr::copy_nonoverlapping(
            &result.ecx as *const _ as *const u8,
            vendor.as_mut_ptr().offset(8),
            4
        )
    }
    
    vendor
}
```

## FFI (Foreign Function Interface)

### Calling C Functions

```knull
mode god

// Link with C library
#[link(name = "c")]
extern "C" {
    fn printf(format: *const u8, ...) -> i32
    fn malloc(size: usize) -> *mut u8
    fn free(ptr: *mut u8)
    fn strlen(s: *const u8) -> usize
}

fn main() {
    unsafe {
        // Call C printf
        printf("Hello from %s\n".as_ptr(), "Knull".as_ptr())
        
        // Allocate memory using C malloc
        let ptr = malloc(1024)
        if !ptr.is_null() {
            // Use memory...
            free(ptr)
        }
    }
}
```

### Creating C-Compatible Libraries

```knull
mode god

// Export function for C
#[no_mangle]
pub extern "C" fn knull_add(a: i32, b: i32) -> i32 {
    a + b
}

// Export with specific ABI
#[no_mangle]
pub extern "system" fn knull_process(data: *const u8, len: usize) -> i32 {
    if data.is_null() {
        return -1
    }
    
    unsafe {
        let slice = core::slice::from_raw_parts(data, len)
        // Process data...
        0 // Success
    }
}
```

## Unsafe Traits

### Implementing Unsafe Traits

```knull
mode god

unsafe trait TrustedData {
    fn as_bytes(&self) -> &[u8]
}

unsafe impl TrustedData for i32 {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const i32 as *const u8,
                4
            )
        }
    }
}
```

## Union Types

```knull
mode god

union Data {
    int: i32,
    float: f32,
    bytes: [u8; 4],
}

fn main() {
    let mut data = Data { int: 42 }
    
    unsafe {
        println!("As int: {}", data.int)
        println!("As bytes: {:?}", data.bytes)
        
        data.float = 3.14
        println!("As float: {}", data.float)
    }
}
```

## Volatile Memory

```knull
mode god

use core::ptr::{read_volatile, write_volatile}

// Hardware register
const TIMER_STATUS: *mut u32 = 0x1000_0000 as *mut u32
const TIMER_VALUE: *mut u32 = 0x1000_0004 as *mut u32

fn read_timer() -> u32 {
    unsafe {
        // Compiler won't optimize this away
        read_volatile(TIMER_VALUE)
    }
}

fn wait_for_timer() {
    unsafe {
        // Poll until timer completes
        while read_volatile(TIMER_STATUS) & 1 == 0 {}
    }
}
```

## Transmute

```knull
mode god

use core::mem::transmute

fn main() {
    // Convert between types of same size
    let f: f32 = 3.14
    let i: i32 = unsafe { transmute(f) }
    
    println!("f32: {} -> i32: {}", f, i)
    
    // View bytes as different type
    let bytes: [u8; 4] = [0x00, 0x00, 0x48, 0x41]  // 12.0 in f32
    let float: f32 = unsafe { transmute(bytes) }
    println!("Bytes as f32: {}", float)
}
```

## Alignment Control

```knull
mode god

#[repr(align(4096))]  // Page-aligned
struct Page {
    data: [u8; 4096]
}

#[repr(packed)]  // No padding
struct PackedHeader {
    magic: u16,
    version: u8,
    flags: u8,
}

fn main() {
    let page = Page { data: [0; 4096] }
    
    // Check alignment
    assert_eq!(&page as *const _ as usize % 4096, 0)
}
```

## Complete OS Example

```knull
mode god
#![no_std]
#![no_main]

use core::panic::PanicInfo

// VGA text mode buffer
const VGA_BUFFER: *mut u16 = 0xb8000 as *mut u16
const VGA_WIDTH: usize = 80
const VGA_HEIGHT: usize = 25

static mut VGA_ROW: usize = 0
static mut VGA_COL: usize = 0

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print("PANIC: ")
    if let Some(msg) = _info.message() {
        // Would print message if we had formatting
    }
    loop {}
}

fn vga_putc(c: u8) {
    unsafe {
        if c == b'\n' {
            VGA_ROW += 1
            VGA_COL = 0
            return
        }
        
        let offset = VGA_ROW * VGA_WIDTH + VGA_COL
        *VGA_BUFFER.offset(offset as isize) = (c as u16) | 0x0F00
        
        VGA_COL += 1
        if VGA_COL >= VGA_WIDTH {
            VGA_COL = 0
            VGA_ROW += 1
        }
        
        if VGA_ROW >= VGA_HEIGHT {
            // Simple scroll (would clear screen in real OS)
            VGA_ROW = 0
        }
    }
}

fn print(s: &str) {
    for c in s.bytes() {
        vga_putc(c)
    }
}

fn print_hex(n: u64) {
    const HEX: &[u8] = b"0123456789ABCDEF"
    
    print("0x")
    for i in (0..16).rev() {
        let digit = ((n >> (i * 4)) & 0xF) as usize
        vga_putc(HEX[digit])
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Clear screen
    unsafe {
        for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
            *VGA_BUFFER.offset(i as isize) = 0
        }
    }
    
    print("Knull OS v0.1.0\n")
    print("==============\n\n")
    
    // Detect memory (simplified)
    print("Memory: ")
    print_hex(0x100000)  // Report 1MB
    print("\n")
    
    // CPU info
    print("CPU: x86_64\n")
    
    print("\nSystem ready.\n")
    
    loop {
        // Halt CPU until next interrupt
        unsafe {
            asm!("hlt")
        }
    }
}
```

## Safety Guidelines

Even in God mode, follow these principles:

1. **Minimize unsafe blocks** - Keep them small and focused
2. **Provide safe abstractions** - Wrap unsafe code in safe APIs
3. **Document invariants** - Explain what must be true for safety
4. **Use checked operations** - Validate inputs before unsafe operations
5. **Test thoroughly** - Unsafe bugs are often security vulnerabilities

### Safe Wrapper Example

```knull
mode god

pub struct MemoryMappedRegister<T> {
    ptr: *mut T,
}

impl<T> MemoryMappedRegister<T> {
    /// Create new MMIO register at given address
    /// 
    /// # Safety
    /// Address must point to valid MMIO region
    pub unsafe fn new(addr: usize) -> Self {
        MemoryMappedRegister {
            ptr: addr as *mut T,
        }
    }
    
    /// Read from register
    /// 
    /// # Safety
    /// Must not violate hardware access rules
    pub unsafe fn read(&self) -> T {
        core::ptr::read_volatile(self.ptr)
    }
    
    /// Write to register
    /// 
    /// # Safety
    /// Must not violate hardware access rules
    pub unsafe fn write(&self, value: T) {
        core::ptr::write_volatile(self.ptr, value)
    }
}

// Safe wrapper
pub struct Uart {
    data: MemoryMappedRegister<u8>,
}

impl Uart {
    pub fn new(base: usize) -> Self {
        Uart {
            data: unsafe { MemoryMappedRegister::new(base) },
        }
    }
    
    pub fn send(&self, c: u8) {
        unsafe {
            self.data.write(c)
        }
    }
    
    pub fn receive(&self) -> u8 {
        unsafe {
            self.data.read()
        }
    }
}
```

---

*God mode puts you in control of the hardware. With great power comes great responsibility.*
