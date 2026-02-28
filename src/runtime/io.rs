//! Knull Standard Library - I/O Module
//!
//! Provides file system operations for Knull programs.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// File handle for Knull programs
pub struct KnullFile {
    path: String,
    contents: Option<String>,
}

/// Read entire file to string
pub fn file_read(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read file '{}': {}", path, e))
}

/// Read file to bytes
pub fn file_read_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Failed to read file '{}': {}", path, e))
}

/// Write string to file
pub fn file_write(path: &str, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|e| format!("Failed to write file '{}': {}", path, e))
}

/// Append string to file
pub fn file_append(path: &str, contents: &str) -> Result<(), String> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Failed to open file '{}': {}", path, e))?;

    file.write_all(contents.as_bytes())
        .map_err(|e| format!("Failed to append to file '{}': {}", path, e))
}

/// Check if file exists
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Create directory
pub fn dir_create(path: &str) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create directory '{}': {}", path, e))
}

/// List directory contents
pub fn dir_list(path: &str) -> Result<Vec<String>, String> {
    let entries =
        fs::read_dir(path).map_err(|e| format!("Failed to read directory '{}': {}", path, e))?;

    let mut result = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(name) = entry.file_name().to_str() {
                result.push(name.to_string());
            }
        }
    }

    Ok(result)
}

/// Remove file
pub fn file_remove(path: &str) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| format!("Failed to remove file '{}': {}", path, e))
}

/// Remove directory
pub fn dir_remove(path: &str) -> Result<(), String> {
    fs::remove_dir_all(path).map_err(|e| format!("Failed to remove directory '{}': {}", path, e))
}

/// Get file size
pub fn file_size(path: &str) -> Result<u64, String> {
    let metadata =
        fs::metadata(path).map_err(|e| format!("Failed to get metadata for '{}': {}", path, e))?;
    Ok(metadata.len())
}

/// Get current working directory
pub fn get_cwd() -> Result<String, String> {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get current directory: {}", e))
}

/// Change working directory
pub fn set_cwd(path: &str) -> Result<(), String> {
    std::env::set_current_dir(path)
        .map_err(|e| format!("Failed to change directory to '{}': {}", path, e))
}

/// Copy file
pub fn file_copy(src: &str, dst: &str) -> Result<(), String> {
    fs::copy(src, dst).map_err(|e| format!("Failed to copy '{}' to '{}': {}", src, dst, e))?;
    Ok(())
}

/// Rename/move file
pub fn file_rename(src: &str, dst: &str) -> Result<(), String> {
    fs::rename(src, dst).map_err(|e| format!("Failed to rename '{}' to '{}': {}", src, dst, e))
}

/// Execute file I/O operation from interpreter
pub fn execute_file_op(op: &str, args: &[String]) -> Result<String, String> {
    match op {
        "read" => {
            if args.is_empty() {
                return Err("read requires a path argument".to_string());
            }
            file_read(&args[0])
        }
        "write" => {
            if args.len() < 2 {
                return Err("write requires path and content arguments".to_string());
            }
            file_write(&args[0], &args[1])?;
            Ok(String::new())
        }
        "append" => {
            if args.len() < 2 {
                return Err("append requires path and content arguments".to_string());
            }
            file_append(&args[0], &args[1])?;
            Ok(String::new())
        }
        "exists" => {
            if args.is_empty() {
                return Err("exists requires a path argument".to_string());
            }
            Ok(file_exists(&args[0]).to_string())
        }
        "size" => {
            if args.is_empty() {
                return Err("size requires a path argument".to_string());
            }
            file_size(&args[0]).map(|s| s.to_string())
        }
        "remove" => {
            if args.is_empty() {
                return Err("remove requires a path argument".to_string());
            }
            file_remove(&args[0])?;
            Ok(String::new())
        }
        "mkdir" => {
            if args.is_empty() {
                return Err("mkdir requires a path argument".to_string());
            }
            dir_create(&args[0])?;
            Ok(String::new())
        }
        "list" => {
            let path = if args.is_empty() { "." } else { &args[0] };
            let entries = dir_list(path)?;
            Ok(entries.join("\n"))
        }
        _ => Err(format!("Unknown file operation: {}", op)),
    }
}
