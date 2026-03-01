use crate::{Allocator, CACHE_LINE_SIZE, DEFAULT_ALIGN};
use core::alloc::Layout;
use core::cell::Cell;
use core::cmp;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct BumpAllocator {
    start: Cell<*mut u8>,
    end: *mut u8,
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
}

impl BumpAllocator {
    pub fn new(size: usize) -> Self {
        unsafe {
            let layout = Layout::from_size_align_aligned(size, CACHE_LINE_SIZE).unwrap();
            let ptr = libc::malloc(layout.size()) as *mut u8;
            Self {
                start: Cell::new(ptr),
                end: ptr.add(size),
                allocations: AtomicUsize::new(0),
                deallocations: AtomicUsize::new(0),
            }
        }
    }

    pub fn reset(&self) {
        self.start.set(self.end.sub(self.used()));
    }

    pub fn used(&self) -> usize {
        self.start.get() as usize - self.end as usize
    }

    pub fn remaining(&self) -> usize {
        self.end as usize - self.start.get() as usize
    }
}

impl Drop for BumpAllocator {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.start.get() as *mut libc::c_void);
        }
    }
}

impl Allocator for BumpAllocator {
    fn alloc(&self, layout: Layout) -> Option<NonNull<u8>> {
        let align = cmp::max(layout.align(), DEFAULT_ALIGN);

        unsafe {
            let current = self.start.get();
            let aligned = (current as *mut u8).add((align - (current as usize) % align) % align);

            if aligned.add(layout.size()) > self.end {
                return None;
            }

            self.start.set(aligned.add(layout.size()));
            self.allocations.fetch_add(layout.size(), Ordering::Relaxed);
            Some(NonNull::new_unchecked(aligned))
        }
    }

    fn dealloc(&self, _ptr: NonNull<u8>, layout: Layout) {
        self.deallocations
            .fetch_add(layout.size(), Ordering::Relaxed);
    }

    fn allocated_size(&self, _ptr: NonNull<u8>) -> usize {
        0
    }

    fn total_allocated(&self) -> usize {
        self.allocations.load(Ordering::Relaxed)
    }

    fn total_deallocated(&self) -> usize {
        self.deallocations.load(Ordering::Relaxed)
    }
}

pub struct Arena {
    bump: BumpAllocator,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            bump: BumpAllocator::new(1024 * 1024),
        }
    }

    pub fn new_with_size(size: usize) -> Self {
        Self {
            bump: BumpAllocator::new(size),
        }
    }

    pub fn alloc(&self, size: usize) -> *mut u8 {
        let layout = Layout::from_size_align(size, DEFAULT_ALIGN).unwrap();
        self.bump
            .alloc(layout)
            .map(|p| p.as_ptr())
            .unwrap_or(ptr::null_mut())
    }

    pub fn reset(&self) {
        self.bump.reset();
    }

    pub fn used(&self) -> usize {
        self.bump.used()
    }

    pub fn remaining(&self) -> usize {
        self.bump.remaining()
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PoolAllocator {
    block_size: usize,
    free_list: Cell<*mut u8>,
    total_blocks: usize,
    used_blocks: Cell<usize>,
    alignment: usize,
}

impl PoolAllocator {
    pub fn new(block_size: usize, total_blocks: usize) -> Self {
        let align = DEFAULT_ALIGN.max(core::mem::size_of::<*mut u8>());
        let size = block_size * total_blocks;

        unsafe {
            let memory = libc::malloc(size) as *mut u8;
            let free_list = memory;

            let mut ptr = free_list;
            for i in 0..(total_blocks - 1) {
                *ptr.cast::<*mut u8>() = ptr.add(block_size);
                ptr = ptr.add(block_size);
            }
            *ptr.cast::<*mut u8>() = ptr::null_mut();

            Self {
                block_size,
                free_list: Cell::new(free_list),
                total_blocks,
                used_blocks: Cell::new(0),
                alignment: align,
            }
        }
    }

    pub fn alloc(&self) -> Option<NonNull<u8>> {
        unsafe {
            let ptr = self.free_list.get();
            if ptr.is_null() {
                return None;
            }

            let next = *ptr.cast::<*mut u8>();
            self.free_list.set(next);
            self.used_blocks.set(self.used_blocks.get() + 1);

            Some(NonNull::new_unchecked(ptr))
        }
    }

    pub fn dealloc(&self, ptr: NonNull<u8>) {
        unsafe {
            let ptr = ptr.as_ptr();
            *ptr.cast::<*mut u8>() = self.free_list.get();
            self.free_list.set(ptr);
            self.used_blocks.set(self.used_blocks.get() - 1);
        }
    }

    pub fn used(&self) -> usize {
        self.used_blocks.get()
    }

    pub fn available(&self) -> usize {
        self.total_blocks - self.used_blocks.get()
    }
}

pub struct Pool {
    pool: PoolAllocator,
    block_size: usize,
}

impl Pool {
    pub fn new(block_size: usize) -> Self {
        Self {
            pool: PoolAllocator::new(block_size, 1000),
            block_size,
        }
    }

    pub fn new_with_blocks(block_size: usize, blocks: usize) -> Self {
        Self {
            pool: PoolAllocator::new(block_size, blocks),
            block_size,
        }
    }

    pub fn alloc(&self) -> Option<NonNull<u8>> {
        self.pool.alloc()
    }

    pub fn dealloc(&self, ptr: NonNull<u8>) {
        self.pool.dealloc(ptr);
    }

    pub fn used(&self) -> usize {
        self.pool.used()
    }
}

pub struct SlabAllocator {
    object_size: usize,
    slabs: Vec<Slab>,
}

struct Slab {
    memory: *mut u8,
    free_list: Cell<*mut u8>,
    capacity: usize,
    used: Cell<usize>,
}

impl SlabAllocator {
    pub fn new(object_size: usize) -> Self {
        Self {
            object_size,
            slabs: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        for slab in &self.slabs {
            if let Some(ptr) = slab.alloc() {
                return Some(ptr);
            }
        }

        let new_slab = Slab::new(self.object_size, 64);
        let ptr = new_slab.alloc();
        self.slabs.push(new_slab);
        ptr
    }

    pub fn dealloc(&self, ptr: NonNull<u8>) {
        for slab in &self.slabs {
            if slab.contains(ptr.as_ptr()) {
                slab.dealloc(ptr);
                return;
            }
        }
    }

    pub fn total_used(&self) -> usize {
        self.slabs.iter().map(|s| s.used()).sum()
    }
}

impl Slab {
    fn new(object_size: usize, capacity: usize) -> Self {
        unsafe {
            let size = object_size * capacity;
            let memory = libc::malloc(size) as *mut u8;

            let free_list = memory;
            let mut ptr = free_list;
            for i in 0..(capacity - 1) {
                *ptr.cast::<*mut u8>() = ptr.add(object_size);
                ptr = ptr.add(object_size);
            }
            *ptr.cast::<*mut u8>() = ptr::null_mut();

            Self {
                memory,
                free_list: Cell::new(free_list),
                capacity,
                used: Cell::new(0),
            }
        }
    }

    fn alloc(&self) -> Option<NonNull<u8>> {
        unsafe {
            let ptr = self.free_list.get();
            if ptr.is_null() {
                return None;
            }

            let next = *ptr.cast::<*mut u8>();
            self.free_list.set(next);
            self.used.set(self.used.get() + 1);

            Some(NonNull::new_unchecked(ptr))
        }
    }

    fn dealloc(&self, ptr: NonNull<u8>) {
        unsafe {
            let ptr = ptr.as_ptr();
            *ptr.cast::<*mut u8>() = self.free_list.get();
            self.free_list.set(ptr);
            self.used.set(self.used.get() - 1);
        }
    }

    fn contains(&self, ptr: *mut u8) -> bool {
        unsafe { ptr >= self.memory && ptr < self.memory.add(self.capacity * self.object_size) }
    }

    fn used(&self) -> usize {
        self.used.get()
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.memory as *mut libc::c_void);
        }
    }
}

pub struct Slab {
    inner: SlabAllocator,
}

impl Slab {
    pub fn new(object_size: usize) -> Self {
        Self {
            inner: SlabAllocator::new(object_size),
        }
    }

    pub fn alloc(&mut self) -> *mut u8 {
        self.inner
            .alloc()
            .map(|p| p.as_ptr())
            .unwrap_or(ptr::null_mut())
    }

    pub fn dealloc(&self, ptr: *mut u8) {
        self.inner.dealloc(NonNull::new_unchecked(ptr));
    }

    pub fn used(&self) -> usize {
        self.inner.total_used()
    }
}

pub struct HeapAllocator {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
}

impl HeapAllocator {
    pub fn new() -> Self {
        Self {
            allocations: AtomicUsize::new(0),
            deallocations: AtomicUsize::new(0),
        }
    }
}

impl Default for HeapAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Allocator for HeapAllocator {
    fn alloc(&self, layout: Layout) -> Option<NonNull<u8>> {
        unsafe {
            let ptr = libc::malloc(layout.size()) as *mut u8;
            if ptr.is_null() {
                None
            } else {
                self.allocations.fetch_add(layout.size(), Ordering::Relaxed);
                Some(NonNull::new_unchecked(ptr))
            }
        }
    }

    fn dealloc(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe {
            libc::free(ptr.as_ptr() as *mut libc::c_void);
            self.deallocations
                .fetch_add(layout.size(), Ordering::Relaxed);
        }
    }

    fn allocated_size(&self, _ptr: NonNull<u8>) -> usize {
        0
    }

    fn total_allocated(&self) -> usize {
        self.allocations.load(Ordering::Relaxed)
    }

    fn total_deallocated(&self) -> usize {
        self.deallocations.load(Ordering::Relaxed)
    }
}
