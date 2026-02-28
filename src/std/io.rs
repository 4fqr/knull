//! Knull Standard Library - IO Module
//!
//! File and console I/O operations

use crate::runtime::syscall;
use std::fs;
use std::io::{self, Read, Write};

/// Standard file descriptors
pub const STDIN_FILENO: u32 = 0;
pub const STDOUT_FILENO: u32 = 1;
pub const STDERR_FILENO: u32 = 2;

/// Print to stdout
pub fn print(s: &str) {
    syscall::sys_write(STDOUT_FILENO, s.as_bytes());
}

/// Print to stdout with newline
pub fn println(s: &str) {
    let line = format!("{}\n", s);
    syscall::sys_write(STDOUT_FILENO, line.as_bytes());
}

/// Print to stderr
pub fn eprint(s: &str) {
    syscall::sys_write(STDERR_FILENO, s.as_bytes());
}

/// Print to stderr with newline
pub fn eprintln(s: &str) {
    let line = format!("{}\n", s);
    syscall::sys_write(STDERR_FILENO, line.as_bytes());
}

/// Read line from stdin
pub fn read_line() -> String {
    let mut buffer = [0u8; 1024];
    let n = syscall::sys_read(STDIN_FILENO, &mut buffer) as usize;
    String::from_utf8_lossy(&buffer[..n]).to_string()
}

/// Read entire file to string
pub fn read_file(path: &str) -> Result<String, io::Error> {
    fs::read_to_string(path)
}

/// Write string to file
pub fn write_file(path: &str, contents: &str) -> Result<(), io::Error> {
    fs::write(path, contents)
}

/// File handle
pub struct File {
    fd: i32,
    path: String,
}

impl File {
    /// Open file for reading
    pub fn open(path: &str) -> Result<Self, io::Error> {
        // Use Rust's std for now, syscall version would need more work
        let _ = fs::File::open(path)?;
        Ok(File {
            fd: -1, // Placeholder
            path: path.to_string(),
        })
    }

    /// Create file for writing
    pub fn create(path: &str) -> Result<Self, io::Error> {
        let _ = fs::File::create(path)?;
        Ok(File {
            fd: -1,
            path: path.to_string(),
        })
    }

    /// Read all contents
    pub fn read_all(&self) -> Result<String, io::Error> {
        read_file(&self.path)
    }

    /// Write contents
    pub fn write(&mut self, contents: &str) -> Result<(), io::Error> {
        write_file(&self.path, contents)
    }
}
