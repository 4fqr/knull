use core::cell::Cell;
use core::fmt;
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct Arc<T: ?Sized> {
    ptr: NonNull<ArcInner<T>>,
}

struct ArcInner<T: ?Sized> {
    refcount: AtomicUsize,
    weak_count: AtomicUsize,
    data: T,
}

unsafe impl<T: Send + ?Sized> Send for Arc<T> {}
unsafe impl<T: Sync + ?Sized> Sync for Arc<T> {}

impl<T> Arc<T> {
    #[inline]
    pub fn new(value: T) -> Arc<T> {
        unsafe {
            let layout = Layout::new::<ArcInner<T>>();
            let ptr = libc::malloc(layout.size()) as *mut ArcInner<T>;
            ptr.write(ArcInner {
                refcount: AtomicUsize::new(1),
                weak_count: AtomicUsize::new(0),
                data: value,
            });

            Arc {
                ptr: NonNull::new_unchecked(ptr),
            }
        }
    }

    #[inline]
    pub fn try_new(value: T) -> Option<Arc<T>> {
        Some(Arc::new(value))
    }

    #[inline]
    pub fn into_raw(this: Arc<T>) -> *const T {
        this.deref() as *const T
    }

    #[inline]
    pub unsafe fn from_raw(ptr: *const T) -> Arc<T> {
        let align = core::mem::align_of::<ArcInner<T>>();
        let offset = core::mem::size_of::<ArcInner<T>>() - core::mem::size_of::<T>();
        let arc_ptr = (ptr as *const u8).sub(offset) as *mut ArcInner<T>;

        Arc {
            ptr: NonNull::new_unchecked(arc_ptr as *mut ArcInner<T>),
        }
    }

    #[inline]
    pub fn get_mut(this: &mut Arc<T>) -> Option<&mut T> {
        if Arc::strong_count(this) == 1 {
            unsafe { Some(&mut (*this.ptr.as_ptr()).data) }
        } else {
            None
        }
    }

    #[inline]
    pub fn make_mut(this: &mut Arc<T>) -> &mut T {
        if Arc::strong_count(this) != 1 {
            this.clone();
        }
        unsafe { &mut (*this.ptr.as_ptr()).data }
    }
}

impl<T: ?Sized> Arc<T> {
    #[inline]
    pub fn clone(this: &Arc<T>) -> Arc<T> {
        this.ptr.as_ref().refcount.fetch_add(1, Ordering::Relaxed);
        Arc { ptr: this.ptr }
    }

    #[inline]
    pub fn strong_count(this: &Arc<T>) -> usize {
        this.ptr.as_ref().refcount.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn weak_count(this: &Arc<T>) -> usize {
        this.ptr.as_ref().weak_count.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn downgrade(this: &Arc<T>) -> Weak<T> {
        this.ptr.as_ref().weak_count.fetch_add(1, Ordering::Relaxed);
        Weak { ptr: this.ptr }
    }

    #[inline]
    pub fn as_ptr(this: &Arc<T>) -> *const T {
        &this.ptr.as_ref().data as *const T
    }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T: ?Sized> Drop for Arc<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if self.ptr.as_ref().refcount.fetch_sub(1, Ordering::Release) == 1 {
                core::sync::atomic::fence(Ordering::Acquire);
                ptr::drop_in_place(&mut self.ptr.as_mut().data);

                let weak_count = self.ptr.as_ref().weak_count.load(Ordering::Relaxed);
                if weak_count == 0 {
                    libc::free(self.ptr.as_ptr() as *mut libc::c_void);
                }
            }
        }
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Weak(...)")
    }
}

pub struct Weak<T: ?Sized> {
    ptr: NonNull<ArcInner<T>>,
}

impl<T> Weak<T> {
    #[inline]
    pub fn new() -> Weak<T> {
        Weak {
            ptr: NonNull::dangling(),
        }
    }

    #[inline]
    pub fn upgrade(&self) -> Option<Arc<T>> {
        let old_count = self.ptr.as_ref().refcount.load(Ordering::Relaxed);
        if old_count == 0 {
            return None;
        }

        if self
            .ptr
            .as_ref()
            .refcount
            .compare_exchange(
                old_count,
                old_count + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            Some(Arc { ptr: self.ptr })
        } else {
            None
        }
    }
}

impl<T: ?Sized> Deref for Weak<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T: ?Sized> Drop for Weak<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if self.ptr.as_ref().weak_count.fetch_sub(1, Ordering::Release) == 1 {
                core::sync::atomic::fence(Ordering::Acquire);

                let strong_count = self.ptr.as_ref().refcount.load(Ordering::Relaxed);
                if strong_count == 0 {
                    libc::free(self.ptr.as_ptr() as *mut libc::c_void);
                }
            }
        }
    }
}

impl<T> Default for Weak<T> {
    fn default() -> Weak<T> {
        Weak::new()
    }
}

pub struct Rc<T: ?Sized> {
    ptr: NonNull<RcInner<T>>,
}

struct RcInner<T: ?Sized> {
    refcount: Cell<usize>,
    weak_count: Cell<usize>,
    data: T,
}

impl<T> Rc<T> {
    #[inline]
    pub fn new(value: T) -> Rc<T> {
        unsafe {
            let layout = Layout::new::<RcInner<T>>();
            let ptr = libc::malloc(layout.size()) as *mut RcInner<T>;
            ptr.write(RcInner {
                refcount: Cell::new(1),
                weak_count: Cell::new(0),
                data: value,
            });

            Rc {
                ptr: NonNull::new_unchecked(ptr),
            }
        }
    }

    #[inline]
    pub fn clone(this: &Rc<T>) -> Rc<T> {
        this.ptr
            .as_ref()
            .refcount
            .set(this.ptr.as_ref().refcount.get() + 1);
        Rc { ptr: this.ptr }
    }

    #[inline]
    pub fn strong_count(this: &Rc<T>) -> usize {
        this.ptr.as_ref().refcount.get()
    }

    #[inline]
    pub fn weak_count(this: &Rc<T>) -> usize {
        this.ptr.as_ref().weak_count.get()
    }

    #[inline]
    pub fn downgrade(this: &Rc<T>) -> WeakRc<T> {
        let weak = WeakRc { ptr: this.ptr };
        this.ptr
            .as_ref()
            .weak_count
            .set(this.ptr.as_ref().weak_count.get() + 1);
        weak
    }
}

impl<T: ?Sized> Deref for Rc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T: ?Sized> Drop for Rc<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let count = self.ptr.as_ref().refcount.get();
            if count == 1 {
                ptr::drop_in_place(&mut self.ptr.as_mut().data);
                self.ptr.as_ref().refcount.set(0);

                let weak_count = self.ptr.as_ref().weak_count.get();
                if weak_count == 0 {
                    libc::free(self.ptr.as_ptr() as *mut libc::c_void);
                }
            } else {
                self.ptr.as_ref().refcount.set(count - 1);
            }
        }
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

pub struct WeakRc<T: ?Sized> {
    ptr: NonNull<RcInner<T>>,
}

impl<T> WeakRc<T> {
    #[inline]
    pub fn upgrade(&self) -> Option<Rc<T>> {
        let count = self.ptr.as_ref().refcount.get();
        if count == 0 {
            None
        } else {
            self.ptr.as_ref().refcount.set(count + 1);
            Some(Rc { ptr: self.ptr })
        }
    }
}

impl<T: ?Sized> Deref for WeakRc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T: ?Sized> Drop for WeakRc<T> {
    #[inline]
    fn drop(&mut self) {
        let count = self.ptr.as_ref().weak_count.get();
        if count == 1 {
            let strong_count = self.ptr.as_ref().refcount.get();
            if strong_count == 0 {
                unsafe {
                    libc::free(self.ptr.as_ptr() as *mut libc::c_void);
                }
            }
        } else {
            self.ptr.as_ref().weak_count.set(count - 1);
        }
    }
}

impl<T> Default for WeakRc<T> {
    fn default() -> WeakRc<T> {
        WeakRc {
            ptr: NonNull::dangling(),
        }
    }
}

use core::alloc::Layout;
use core::ptr;
