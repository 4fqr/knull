use core::alloc::{GlobalAlloc, Layout};
use core::cell::{Cell, RefCell, UnsafeCell};
use core::fmt;
use core::mem::{self, Align, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "std")]
use std::collections::HashMap;

pub mod allocators;
pub mod pools;
pub mod regions;
pub mod rc;
pub mod zero_copy;

pub use allocators::{Arena, BumpAllocator, HeapAllocator, Pool, PoolAllocator, Slab, SlabAllocator};
pub use pools::ObjectPool;
pub use regions::Region;
pub use rc::{Arc, Rc, Weak};
pub use zero_copy::{ByteSlice, StrView, ZeroCopy};

const DEFAULT_ALIGN: usize = 16;
const CACHE_LINE_SIZE: usize = 64;

pub trait Allocator: Send + Sync {
    fn alloc(&self, layout: Layout) -> Option<NonNull<u8>>;
    fn dealloc(&self, ptr: NonNull<u8>, layout: Layout);
    fn allocated_size(&self, ptr: NonNull<u8>) -> usize;
    fn total_allocated(&self) -> usize;
    fn total_deallocated(&self) -> usize;
}

pub struct TrackingAllocator<A: Allocator> {
    inner: A,
    allocated: AtomicUsize,
    deallocated: AtomicUsize,
    allocations: RefCell<HashMap<usize, Layout>>,
}

impl<A: Allocator> TrackingAllocator<A> {
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            allocated: AtomicUsize::new(0),
            deallocated: AtomicUsize::new(0),
            allocations: RefCell::new(HashMap::new()),
        }
    }

    pub fn total_allocated(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    pub fn total_deallocated(&self) -> usize {
        self.deallocated.load(Ordering::Relaxed)
    }

    pub fn live_allocations(&self) -> usize {
        self.allocated.load(Ordering::Relaxed) - self.deallocated.load(Ordering::Relaxed)
    }
}

impl<A: Allocator> Allocator for TrackingAllocator<A> {
    #[inline]
    fn alloc(&self, layout: Layout) -> Option<NonNull<u8>> {
        let ptr = self.inner.alloc(layout)?;
        self.allocated.fetch_add(layout.size(), Ordering::Relaxed);
        self.allocations
            .borrow_mut()
            .insert(ptr.as_ptr() as usize, layout);
        Some(ptr)
    }

    #[inline]
    fn dealloc(&self, ptr: NonNull<u8>, layout: Layout) {
        self.inner.dealloc(ptr, layout);
        self.deallocated.fetch_add(layout.size(), Ordering::Relaxed);
        self.allocations.borrow_mut().remove(&(ptr.as_ptr() as usize));
    }

    #[inline]
    fn allocated_size(&self, ptr: NonNull<u8>) -> usize {
        self.allocations
            .borrow()
            .get(&(ptr.as_ptr() as usize))
            .map(|l| l.size())
            .unwrap_or(0)
    }

    #[inline]
    fn total_allocated(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    #[inline]
    fn total_deallocated(&self) -> usize {
        self.deallocated.load(Ordering::Relaxed)
    }
}

pub struct Box<T: ?Sized> {
    ptr: NonNull<T>,
}

impl<T> Box<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            ptr: Self::allocate(value),
        }
    }

    #[inline]
    pub fn new_in(value: T, alloc: &'static dyn Allocator) -> Self {
        Self {
            ptr: Self::allocate_in(value, alloc),
        }
    }

    #[inline]
    fn allocate(value: T) -> NonNull<T> {
        let layout = Layout::new::<T>();
        unsafe {
            let ptr = alloc(Layout::new::<T>()).expect("allocation failed");
            ptr.cast::<T>().write(value);
            ptr.cast()
        }
    }

    #[inline]
    fn allocate_in(value: T, alloc: &'static dyn Allocator) -> NonNull<T> {
        unsafe {
            let layout = Layout::new::<T>();
            let ptr = alloc.alloc(layout).expect("allocation failed");
            ptr.cast::<T>().write(value);
            ptr.cast()
        }
    }

    #[inline]
    pub fn into_raw(b: Box<T>) -> *mut T {
        let ptr = b.ptr;
        mem::forget(b);
        ptr.as_ptr()
    }

    #[inline]
    pub unsafe fn from_raw(ptr: *mut T) -> Box<T> {
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }

    #[inline]
    pub fn leak(b: Box<T>) -> &'static mut T {
        let ptr = b.ptr;
        mem::forget(b);
        unsafe { &mut *ptr.as_ptr() }
    }
}

impl<T: ?Sized> Deref for Box<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for Box<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.ptr.as_ptr());
            dealloc(self.ptr.cast::<u8>(), Layout::for_value(&*self.ptr));
        }
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

fn alloc(layout: Layout) -> Option<NonNull<u8>> {
    unsafe {
        let ptr = libc::malloc(layout.size()) as *mut u8;
        NonNull::new(ptr)
    }
}

fn dealloc(ptr: NonNull<u8>, _layout: Layout) {
    unsafe {
        libc::free(ptr.as_ptr() as *mut libc::c_void);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box() {
        let b = Box::new(42);
        assert_eq!(*b, 42);
    }

    #[test]
    fn test_arc() {
        let arc = Arc::new(42);
        assert_eq!(*arc, 42);
        let arc2 = arc.clone();
        assert_eq!(*arc2, 42);
    }

    #[test]
    fn test_rc() {
        let rc = Rc::new(42);
        assert_eq!(*rc, 42);
    }

    #[test]
    fn test_object_pool() {
        let pool = ObjectPool::new();
        let _obj = pool.get();
    }

    #[test]
    fn test_region() {
        let region = Region::new();
        let _ptr = region.alloc(100);
    }

    #[test]
    fn test_bump_allocator() {
        let arena = Arena::new();
        let _ptr = arena.alloc(1024);
    }

    #[test]
    fn test_zero_copy() {
        let data = vec![72, 101, 108, 108, 111];
        let view = StrView::from_bytes(&data).unwrap();
        assert_eq!(view.as_str(), "Hello");
    }
}
