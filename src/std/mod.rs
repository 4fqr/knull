//! Knull Standard Library
//!
//! Core library providing essential functionality for Knull programs.

pub mod collections;
pub mod io;
pub mod string;

/// Standard library version
pub const VERSION: &str = "1.0.0";

/// Panic handler
pub fn panic(msg: &str) -> ! {
    io::eprintln(&format!("PANIC: {}", msg));
    std::process::exit(1);
}

/// Assert condition
pub fn assert(condition: bool, msg: &str) {
    if !condition {
        panic(msg);
    }
}

/// Get current time in milliseconds
pub fn current_time_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Sleep for milliseconds
pub fn sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

/// Generate random number
pub fn random() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let mut hasher = DefaultHasher::new();
    time.hash(&mut hasher);
    hasher.finish()
}

/// Generate random number in range
pub fn random_range(min: i64, max: i64) -> i64 {
    min + (random() as i64 % (max - min))
}

/// Exit program
pub fn exit(code: i32) -> ! {
    std::process::exit(code)
}

/// Get command line arguments
pub fn args() -> Vec<String> {
    std::env::args().collect()
}

/// Get environment variable
pub fn env_var(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// Set environment variable
pub fn set_env_var(key: &str, value: &str) {
    std::env::set_var(key, value);
}
