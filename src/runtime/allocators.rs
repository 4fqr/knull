//! Knull Pluggable Allocators Runtime
//!
//! Provides pluggable allocator implementations for high-performance scenarios

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Allocator trait for runtime polymorphism
pub trait RuntimeAllocator: Send + Sync {
    fn allocate(&self, size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, size: usize);
    fn reallocate(&self, ptr: *mut u8, old_size: usize, new_size: usize) -> *mut u8;
}

/// Arena allocator - bulk deallocation
pub struct ArenaAllocator {
    buffer: *mut u8,
    offset: AtomicUsize,
    capacity: usize,
}

impl ArenaAllocator {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, 16).unwrap();
        let buffer = unsafe { alloc(layout) };
        
        ArenaAllocator {
            buffer,
            offset: AtomicUsize::new(0),
            capacity,
        }
    }

    pub fn alloc(&self, size: usize) -> *mut u8 {
        let align = 16;
        let aligned_offset = (self.offset.load(Ordering::SeqCst) + align - 1) & !(align - 1);
        let new_offset = aligned_offset + size;
        
        if new_offset > self.capacity {
            return std::ptr::null_mut();
        }
        
        self.offset.store(new_offset, Ordering::SeqCst);
        unsafe { self.buffer.offset(aligned_offset as isize) }
    }

    pub fn reset(&self) {
        self.offset.store(0, Ordering::SeqCst);
    }

    pub fn used(&self) -> usize {
        self.offset.load(Ordering::SeqCst)
    }

    pub fn remaining(&self) -> usize {
        self.capacity - self.offset.load(Ordering::SeqCst)
    }
}

impl Drop for ArenaAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.capacity, 16).unwrap();
        unsafe { dealloc(self.buffer, layout) };
    }
}

impl RuntimeAllocator for ArenaAllocator {
    fn allocate(&self, size: usize) -> *mut u8 {
        self.alloc(size)
    }

    fn deallocate(&self, _ptr: *mut u8, _size: usize) {
        // No-op: bulk deallocation only
    }

    fn reallocate(&self, _ptr: *mut u8, _old_size: usize, new_size: usize) -> *mut u8 {
        self.alloc(new_size)
    }
}

/// Bump pointer allocator - fastest, no free
pub struct BumpPointerAllocator {
    start: *mut u8,
    end: *mut u8,
    current: AtomicUsize,
}

mut u8,
impl BumpPointerAllocator {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, 8).unwrap();
        let start = unsafe { alloc(layout) };
        let end = unsafe { start.offset(capacity as isize) };
        
        BumpPointerAllocator {
            start,
            end,
            current: AtomicUsize::new(start as usize),
        }
    }

    pub fn alloc(&self, size: usize) -> *mut u8 {
        let align = 8;
        let current = self.current.load(Ordering::SeqCst);
        let aligned = (current + align - 1) & !(align - 1);
        let next = aligned + size;
        
        if next > self.end as usize {
            return std::ptr::null_mut();
        }
        
        self.current.store(next, Ordering::SeqCst);
        aligned as *mut u8
    }

    pub fn reset(&self) {
        self.current.store(self.start as usize, Ordering::SeqCst);
    }

    pub fn allocated(&self) -> usize {
        self.current.load(Ordering::SeqCst) - self.start as usize
    }
}

impl Drop for BumpPointerAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(
            self.end as usize - self.start as usize,
            8,
        ).unwrap();
        unsafe { dealloc(self.start, layout) };
    }
}

impl RuntimeAllocator for BumpPointerAllocator {
    fn allocate(&self, size: usize) -> *mut u8 {
        self.alloc(size)
    }

    fn deallocate(&self, _ptr: *mut u8, _size: usize) {
        // No-op: bump pointer cannot free individual allocations
    }

    fn reallocate(&self, _ptr: *mut u8, _old_size: usize, new_size: usize) -> *mut u8 {
        self.alloc(new_size)
    }
}

/// Pool allocator - fixed-size objects
pub struct PoolAllocator {
    block_size: usize,
    free_list: *mut *mut u8,
    memory: *mut u8,
    total_blocks: usize,
    allocated: AtomicUsize,
}

impl PoolAllocator {
    pub fn new(block_size: usize, num_blocks: usize) -> Self {
        let align = 16;
        let aligned_size = (block_size + align - 1) & !(align - 1);
        
        let layout = Layout::from_size_align(aligned_size * num_blocks, align).unwrap();
        let memory = unsafe { alloc(layout) };
        
        // Build free list
        let free_list = memory as *mut *mut u8;
        
        unsafe {
            let mut current = memory;
            for i in 0..num_blocks - 1 {
                let next = current.add((i + 1) * aligned_size);
                *current.add(i * aligned_size) = next;
            }
            *memory.add((num_blocks - 1) * aligned_size) = std::ptr::null_mut();
        }
        
        PoolAllocator {
            block_size: aligned_size,
            free_list,
            memory,
            total_blocks: num_blocks,
            allocated: AtomicUsize::new(0),
        }
    }

    pub fn alloc(&self) -> *mut u8 {
        unsafe {
            if self.free_list.is_null() {
                return std::ptr::null_mut();
            }
            
            let ptr = *self.free_list;
            if ptr.is_null() {
                return std::ptr::null_mut();
            }
            
            self.free_list = *ptr as *mut *mut u8;
            self.allocated.fetch_add(1, Ordering::SeqCst);
            ptr
        }
    }

    pub fn free(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        
        unsafe {
            *(ptr as *mut *mut u8) = self.free_list;
            self.free_list = ptr;
        }
        self.allocated.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn used(&self) -> usize {
        self.allocated.load(Ordering::SeqCst)
    }

    pub fn available(&self) -> usize {
        self.total_blocks - self.allocated.load(Ordering::SeqCst)
    }
}

impl Drop for PoolAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(
            self.block_size * self.total_blocks,
            16,
        ).unwrap();
        unsafe { dealloc(self.memory, layout) };
    }
}

impl RuntimeAllocator for PoolAllocator {
    fn allocate(&self, _size: usize) -> *mut u8 {
        self.alloc()
    }

    fn deallocate(&self, ptr: *mut u8, _size: usize) {
        self.free(ptr)
    }

    fn reallocate(&self, ptr: *mut u8, _old_size: usize, _new_size: usize) -> *mut u8 {
        // Pool allocator doesn't support reallocation
        ptr
    }
}

/// Malloc allocator - system default
pub struct MallocAllocator;

impl RuntimeAllocator for MallocAllocator {
    fn allocate(&self, size: usize) -> *mut u8 {
        let layout = Layout::from_size_align(size, 1).unwrap();
        unsafe { alloc(layout) }
    }

    fn deallocate(&self, ptr: *mut u8, size: usize) {
        let layout = Layout::from_size_align(size, 1).unwrap();
        unsafe { dealloc(ptr, layout) };
    }

    fn reallocate(&self, ptr: *mut u8, old_size: usize, new_size: usize) -> *mut u8 {
        let old_layout = Layout::from_size_align(old_size, 1).unwrap();
        let new_layout = Layout::from_size_align(new_size, 1).unwrap();
        unsafe { realloc(ptr, old_layout, new_layout) }
    }
}

/// Global allocator selection
pub static CURRENT_ALLOCATOR: std::sync::RwLock<Box<dyn RuntimeAllocator>> = 
    std::sync::RwLock::new(Box::new(MallocAllocator));

/// Set the global allocator
pub fn set_allocator<A: RuntimeAllocator + 'static>(allocator: A) {
    if let Ok(mut current) = CURRENT_ALLOCATOR.write() {
        *current = Box::new(allocator);
    }
}

/// Get the current allocator
pub fn get_allocator() -> std::sync::RwLockReadGuard<'static, Box<dyn RuntimeAllocator>> {
    CURRENT_ALLOCATOR.read().unwrap()
}

/// Allocate using current allocator
pub fn runtime_alloc(size: usize) -> *mut u8 {
    get_allocator().allocate(size)
}

/// Deallocate using current allocator
pub fn runtime_free(ptr: *mut u8, size: usize) {
    get_allocator().deallocate(ptr, size)
}

/// Reallocate using current allocator
pub fn runtime_realloc(ptr: *mut u8, old_size: usize, new_size: usize) -> *mut u8 {
    get_allocator().reallocate(ptr, old_size, new_size)
}
