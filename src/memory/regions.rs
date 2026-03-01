use crate::{Allocator, DEFAULT_ALIGN};
use core::alloc::Layout;
use core::cell::RefCell;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct Region {
    allocator: RegionAllocator,
    allocations: RefCell<Vec<(*mut u8, Layout)>>,
}

struct RegionAllocator {
    memory: *mut u8,
    offset: AtomicUsize,
    capacity: usize,
}

impl Region {
    pub fn new() -> Self {
        Self::with_capacity(1024 * 1024)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        unsafe {
            let memory = libc::malloc(capacity) as *mut u8;
            Self {
                allocator: RegionAllocator {
                    memory,
                    offset: AtomicUsize::new(0),
                    capacity,
                },
                allocations: RefCell::new(Vec::new()),
            }
        }
    }

    pub fn alloc(&self, size: usize) -> *mut u8 {
        let align = DEFAULT_ALIGN;
        let layout = Layout::from_size_align(size, align).unwrap();

        let current_offset = self.allocator.offset.load(Ordering::Relaxed);
        let aligned_offset = (current_offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.allocator.capacity {
            return core::ptr::null_mut();
        }

        self.allocator
            .offset
            .store(aligned_offset + size, Ordering::Relaxed);

        let ptr = unsafe { self.allocator.memory.add(aligned_offset) };
        self.allocations.borrow_mut().push((ptr, layout));

        ptr
    }

    pub fn alloc_with_layout(&self, layout: Layout) -> Option<NonNull<u8>> {
        let current_offset = self.allocator.offset.load(Ordering::Relaxed);
        let align = layout.align();
        let aligned_offset = (current_offset + align - 1) & !(align - 1);

        if aligned_offset + layout.size() > self.allocator.capacity {
            return None;
        }

        self.allocator
            .offset
            .store(aligned_offset + layout.size(), Ordering::Relaxed);

        let ptr = unsafe { self.allocator.memory.add(aligned_offset) };
        self.allocations.borrow_mut().push((ptr, layout));

        Some(unsafe { NonNull::new_unchecked(ptr) })
    }

    pub fn reset(&self) {
        self.allocator.offset.store(0, Ordering::Relaxed);
        self.allocations.borrow_mut().clear();
    }

    pub fn used(&self) -> usize {
        self.allocator.offset.load(Ordering::Relaxed)
    }

    pub fn capacity(&self) -> usize {
        self.allocator.capacity
    }

    pub fn available(&self) -> usize {
        self.allocator.capacity - self.used()
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.allocator.memory as *mut libc::c_void);
        }
    }
}

impl Default for Region {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ScopedRegion<'a> {
    parent: &'a Region,
    checkpoint: usize,
}

impl<'a> ScopedRegion<'a> {
    pub fn new(parent: &'a Region) -> Self {
        let checkpoint = parent.used();
        Self { parent, checkpoint }
    }

    pub fn alloc(&self, size: usize) -> *mut u8 {
        self.parent.alloc(size)
    }

    pub fn reset(&self) {
        while self.parent.used() > self.checkpoint {
            if let Some((ptr, layout)) = self.parent.allocations.borrow_mut().pop() {
                unsafe {
                    libc::free(ptr as *mut libc::c_void);
                }
            }
        }
        self.parent
            .allocator
            .offset
            .store(self.checkpoint, Ordering::Relaxed);
    }
}

impl<'a> Drop for ScopedRegion<'a> {
    fn drop(&mut self) {
        self.reset();
    }
}

pub struct ArenaRegion {
    regions: Vec<Region>,
    current_index: usize,
    region_capacity: usize,
}

impl ArenaRegion {
    pub fn new() -> Self {
        Self::with_capacity(1024 * 1024)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut regions = Vec::new();
        regions.push(Region::with_capacity(capacity));

        Self {
            regions,
            current_index: 0,
            region_capacity: capacity,
        }
    }

    pub fn alloc(&mut self, size: usize) -> *mut u8 {
        if let Some(ptr) = self.regions[self.current_index].alloc(size) {
            return ptr;
        }

        self.regions
            .push(Region::with_capacity(self.region_capacity));
        self.current_index = self.regions.len() - 1;

        self.regions[self.current_index].alloc(size)
    }

    pub fn reset(&mut self) {
        for region in &mut self.regions {
            region.reset();
        }
        self.current_index = 0;
    }

    pub fn total_used(&self) -> usize {
        self.regions.iter().map(|r| r.used()).sum()
    }
}

impl Default for ArenaRegion {
    fn default() -> Self {
        Self::new()
    }
}
