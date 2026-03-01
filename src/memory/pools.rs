use crate::{Allocator, DEFAULT_ALIGN};
use core::alloc::Layout;
use core::cell::RefCell;
use core::ptr::NonNull;

pub struct ObjectPool<T> {
    factory: Option<Box<dyn Fn() -> T>>,
    available: RefCell<Vec<*mut u8>>,
    in_use: RefCell<usize>,
    layout: Layout,
}

impl<T> ObjectPool<T> {
    pub fn new() -> Self {
        Self {
            factory: None,
            available: RefCell::new(Vec::new()),
            in_use: RefCell::new(0),
            layout: Layout::new::<T>(),
        }
    }

    pub fn with_factory<F>(factory: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        Self {
            factory: Some(Box::new(factory)),
            available: RefCell::new(Vec::new()),
            in_use: RefCell::new(0),
            layout: Layout::new::<T>(),
        }
    }

    pub fn preallocate(&self, count: usize) {
        for _ in 0..count {
            let ptr = self.allocate_slot();
            if !ptr.is_null() {
                self.available.borrow_mut().push(ptr);
            }
        }
    }

    fn allocate_slot(&self) -> *mut u8 {
        unsafe {
            let ptr = libc::malloc(self.layout.size()) as *mut u8;
            ptr
        }
    }

    pub fn get(&self) -> T {
        let ptr = self
            .available
            .borrow_mut()
            .pop()
            .unwrap_or_else(|| self.allocate_slot());

        *self.in_use.borrow_mut() += 1;

        if let Some(ref factory) = self.factory {
            unsafe {
                let ptr = NonNull::new_unchecked(ptr);
                ptr.cast::<T>().write(factory());
                return ptr.cast::<T>().read();
            }
        }

        unsafe { ptr.cast::<T>().read() }
    }

    pub fn put(&self, value: T) {
        unsafe {
            let ptr = self.allocate_slot();
            ptr.cast::<T>().write(value);
            self.available.borrow_mut().push(ptr);
            *self.in_use.borrow_mut() -= 1;
        }
    }

    pub fn available_count(&self) -> usize {
        self.available.borrow().len()
    }

    pub fn in_use_count(&self) -> usize {
        *self.in_use.borrow()
    }

    pub fn clear(&self) {
        let available = self.available.borrow();
        for ptr in available.iter() {
            unsafe {
                libc::free(*ptr as *mut libc::c_void);
            }
        }
        drop(available);
        self.available.borrow_mut().clear();
        *self.in_use.borrow_mut() = 0;
    }
}

impl<T> Drop for ObjectPool<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T> Default for ObjectPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
use std::sync::{Arc as StdArc, Mutex};

#[cfg(feature = "std")]
pub struct ThreadLocalObjectPool<T> {
    inner: StdArc<Mutex<ObjectPool<T>>>,
}

#[cfg(feature = "std")]
impl<T> ThreadLocalObjectPool<T> {
    pub fn new() -> Self {
        Self {
            inner: StdArc::new(Mutex::new(ObjectPool::new())),
        }
    }

    pub fn with_factory<F>(factory: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        Self {
            inner: StdArc::new(Mutex::new(ObjectPool::with_factory(factory))),
        }
    }

    pub fn get(&self) -> T {
        self.inner.lock().unwrap().get()
    }

    pub fn put(&self, value: T) {
        self.inner.lock().unwrap().put(value);
    }
}

#[cfg(feature = "std")]
impl<T> Default for ThreadLocalObjectPool<T> {
    fn default() -> Self {
        Self::new()
    }
}
