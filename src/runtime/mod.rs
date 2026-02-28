//! Knull Runtime Library
//! 
//! Core runtime support for the Knull programming language.
//! Provides memory management, syscalls, and inline assembly support.

pub mod mem;
pub mod syscall;
pub mod asm;

/// Initialize the runtime
pub fn init() {
    // Initialize memory manager with default settings
    mem::init_memory_manager(mem::MemoryMode::GarbageCollected);
}

/// Runtime version
pub const VERSION: &str = "1.0.0";

/// Runtime capabilities
pub struct RuntimeInfo {
    pub version: &'static str,
    pub has_llvm: bool,
    pub memory_mode: mem::MemoryMode,
}

impl RuntimeInfo {
    pub fn new() -> Self {
        RuntimeInfo {
            version: VERSION,
            has_llvm: cfg!(feature = "llvm-backend"),
            memory_mode: mem::MemoryMode::GarbageCollected,
        }
    }
}
