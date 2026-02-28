//! Knull Standard Library - String operations

/// Convert integer to string
pub fn int_to_string(n: i64) -> String {
    n.to_string()
}

/// Convert string to integer
pub fn string_to_int(s: &str) -> i64 {
    s.parse().unwrap_or(0)
}

/// String builder
pub struct StringBuilder {
    buffer: String,
}

impl StringBuilder {
    pub fn new() -> Self {
        StringBuilder {
            buffer: String::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        StringBuilder {
            buffer: String::with_capacity(cap),
        }
    }

    pub fn append(&mut self, s: &str) {
        self.buffer.push_str(s);
    }

    pub fn append_char(&mut self, c: char) {
        self.buffer.push(c);
    }

    pub fn append_int(&mut self, n: i64) {
        self.buffer.push_str(&n.to_string());
    }

    pub fn to_string(self) -> String {
        self.buffer
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Split string by delimiter
pub fn split(s: &str, delim: &str) -> Vec<String> {
    s.split(delim).map(|s| s.to_string()).collect()
}

/// Join strings with separator
pub fn join(parts: &[String], sep: &str) -> String {
    parts.join(sep)
}

/// Check if string contains substring
pub fn contains(s: &str, substr: &str) -> bool {
    s.contains(substr)
}

/// Replace substring
pub fn replace(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

/// Trim whitespace
pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

/// To lowercase
pub fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

/// To uppercase
pub fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

/// Get substring
pub fn substring(s: &str, start: usize, end: usize) -> String {
    s.chars().skip(start).take(end - start).collect()
}

/// String length
pub fn length(s: &str) -> usize {
    s.len()
}

/// Character count
pub fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Check if string starts with prefix
pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Check if string ends with suffix
pub fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}
