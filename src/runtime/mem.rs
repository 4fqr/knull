//! Knull Memory Management Runtime
//!
//! Implements the memory management system for all three Knull modes:
//! - Novice: Garbage collection with reference counting
//! - Expert: Ownership-based with compile-time checks
//! - God: Manual memory management with unsafe blocks

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Memory block header for runtime tracking
#[repr(C)]
pub struct MemoryHeader {
    pub size: usize,
    pub refcount: AtomicUsize,
    pub flags: u32, // GC flags, ownership info, etc.
}

/// Memory management mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryMode {
    /// Garbage collected (reference counting)
    GarbageCollected,
    /// Ownership-based (manual with compile-time checks)
    Ownership,
    /// Manual (completely unsafe)
    Manual,
}

/// Global memory manager
pub struct MemoryManager {
    mode: MemoryMode,
    allocations: HashMap<usize, MemoryHeader>,
    total_allocated: AtomicUsize,
    total_freed: AtomicUsize,
}

impl MemoryManager {
    pub fn new(mode: MemoryMode) -> Self {
        MemoryManager {
            mode,
            allocations: HashMap::new(),
            total_allocated: AtomicUsize::new(0),
            total_freed: AtomicUsize::new(0),
        }
    }

    /// Allocate memory with header
    pub unsafe fn allocate(&mut self, size: usize) -> *mut u8 {
        let layout = Layout::from_size_align(
            size + std::mem::size_of::<MemoryHeader>(),
            std::mem::align_of::<MemoryHeader>(),
        )
        .unwrap();

        let ptr = alloc(layout);
        if ptr.is_null() {
            panic!("Memory allocation failed");
        }

        // Initialize header
        let header = ptr as *mut MemoryHeader;
        (*header).size = size;
        (*header).refcount = AtomicUsize::new(1);
        (*header).flags = match self.mode {
            MemoryMode::GarbageCollected => 0x1,
            MemoryMode::Ownership => 0x2,
            MemoryMode::Manual => 0x4,
        };

        let data_ptr = ptr.add(std::mem::size_of::<MemoryHeader>());
        self.allocations.insert(data_ptr as usize, *header);
        self.total_allocated.fetch_add(size, Ordering::SeqCst);

        data_ptr
    }

    /// Deallocate memory
    pub unsafe fn deallocate(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let header_ptr = ptr.sub(std::mem::size_of::<MemoryHeader>()) as *mut MemoryHeader;
        let size = (*header_ptr).size;

        let layout = Layout::from_size_align(
            size + std::mem::size_of::<MemoryHeader>(),
            std::mem::align_of::<MemoryHeader>(),
        )
        .unwrap();

        dealloc(header_ptr as *mut u8, layout);
        self.allocations.remove(&(ptr as usize));
        self.total_freed.fetch_add(size, Ordering::SeqCst);
    }

    /// Reallocate memory
    pub unsafe fn reallocate(&mut self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            return self.allocate(new_size);
        }

        let header_ptr = ptr.sub(std::mem::size_of::<MemoryHeader>()) as *mut MemoryHeader;
        let old_size = (*header_ptr).size;
        let refcount = (*header_ptr).refcount.load(Ordering::SeqCst);
        let flags = (*header_ptr).flags;

        let old_layout = Layout::from_size_align(
            old_size + std::mem::size_of::<MemoryHeader>(),
            std::mem::align_of::<MemoryHeader>(),
        )
        .unwrap();

        let new_ptr = realloc(
            header_ptr as *mut u8,
            old_layout,
            new_size + std::mem::size_of::<MemoryHeader>(),
        );

        if new_ptr.is_null() {
            panic!("Memory reallocation failed");
        }

        // Update header
        let new_header = new_ptr as *mut MemoryHeader;
        (*new_header).size = new_size;
        (*new_header).refcount = AtomicUsize::new(refcount);
        (*new_header).flags = flags;

        let data_ptr = new_ptr.add(std::mem::size_of::<MemoryHeader>());
        self.allocations.remove(&(ptr as usize));
        self.allocations.insert(data_ptr as usize, *new_header);
        self.total_allocated
            .fetch_add(new_size - old_size, Ordering::SeqCst);

        data_ptr
    }

    /// Increment reference count
    pub unsafe fn retain(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let header_ptr = ptr.sub(std::mem::size_of::<MemoryHeader>()) as *mut MemoryHeader;
        (*header_ptr).refcount.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement reference count and free if zero
    pub unsafe fn release(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let header_ptr = ptr.sub(std::mem::size_of::<MemoryHeader>()) as *mut MemoryHeader;
        let new_count = (*header_ptr).refcount.fetch_sub(1, Ordering::SeqCst) - 1;

        if new_count == 0 && self.mode == MemoryMode::GarbageCollected {
            self.deallocate(ptr);
        }
    }

    /// Get allocation statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_allocated: self.total_allocated.load(Ordering::SeqCst),
            total_freed: self.total_freed.load(Ordering::SeqCst),
            active_allocations: self.allocations.len(),
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub total_freed: usize,
    pub active_allocations: usize,
}

/// Thread-local memory manager
thread_local! {
    static MEMORY_MANAGER: std::cell::RefCell<Option<MemoryManager>> = std::cell::RefCell::new(None);
}

/// Initialize memory manager for current thread
pub fn init_memory_manager(mode: MemoryMode) {
    MEMORY_MANAGER.with(|mm| {
        *mm.borrow_mut() = Some(MemoryManager::new(mode));
    });
}

/// Allocate memory (thread-safe wrapper)
pub fn knull_alloc(size: usize) -> *mut u8 {
    MEMORY_MANAGER.with(|mm| {
        if let Some(ref mut manager) = *mm.borrow_mut() {
            unsafe { manager.allocate(size) }
        } else {
            panic!("Memory manager not initialized");
        }
    })
}

/// Deallocate memory (thread-safe wrapper)
pub fn knull_free(ptr: *mut u8) {
    MEMORY_MANAGER.with(|mm| {
        if let Some(ref mut manager) = *mm.borrow_mut() {
            unsafe {
                manager.deallocate(ptr);
            }
        }
    });
}

/// Reallocate memory (thread-safe wrapper)
pub fn knull_realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
    MEMORY_MANAGER.with(|mm| {
        if let Some(ref mut manager) = *mm.borrow_mut() {
            unsafe { manager.reallocate(ptr, new_size) }
        } else {
            panic!("Memory manager not initialized");
        }
    })
}

/// Retain reference (thread-safe wrapper)
pub fn knull_retain(ptr: *mut u8) {
    MEMORY_MANAGER.with(|mm| {
        if let Some(ref manager) = *mm.borrow() {
            unsafe {
                manager.retain(ptr);
            }
        }
    });
}

/// Release reference (thread-safe wrapper)
pub fn knull_release(ptr: *mut u8) {
    MEMORY_MANAGER.with(|mm| {
        if let Some(ref mut manager) = *mm.borrow_mut() {
            unsafe {
                manager.release(ptr);
            }
        }
    });
}
