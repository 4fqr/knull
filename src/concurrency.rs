#![allow(dead_code)]

use std::collections::VecDeque;
use std::hint;
use std::marker::PhantomData;
use std::mem::{self, ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::ptr::{self, NonNull};
use std::sync::atomic::{
   AtomicBool, AtomicI32, AtomicI64, AtomicIsize, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize,
    Ordering,
};
use std::sync::mpsc::{self, Sender as MpscSender};
use std::sync::{Arc, Condvar, Mutex as StdMutex, RwLock as StdRwLock};
use std::task::{Context, Poll as TaskPoll, RawWaker, RawWakerVTable, Waker};
use std::thread::{self, JoinHandle, ThreadId};
use std::time::Duration;

#[cfg(feature = "llvm-backend")]
use std::arch::asm;

mod green;
mod work_stealing;
mod channels;
mod atomics;
mod parallel;

pub use green::*;
pub use work_stealing::*;
pub use channels::*;
pub use atomics::*;
pub use parallel::*;

#[cfg(feature = "llvm-backend")]
pub mod codegen {
    #[cfg(feature = "llvm-backend")]
    use crate::ast::*;
    
    #[allow(dead_code)]
    pub fn generate_async_state_machine(
        _fn_name: &str,
        state_count: usize,
    ) -> String {
        format!(
            r#"
; Knull Async State Machine
; Generated for function with {} states
define void @async_state_machine_{}(i64 %state_ptr, i64 %future_ptr) {{
entry:
  %state = load i64, i64* %state_ptr
  switch i64 %state, label %done [
    {}
  ]
done:
  ret void
}}
"#,
            state_count, _fn_name,
            (0..state_count)
                .map(|i| format!("    i64 {}, label .state_{}", i, i))
                .collect::<Vec<_>>()
                .join(",\n")
        )
    }

    #[allow(dead_code)]
    pub fn generate_green_thread_entry(fn_name: &str) -> String {
        format!(
            r#"
; Green Thread Entry Point
define void @{}_green_entry(i64 %stack_ptr, i64 %func_ptr) {{
entry:
  %func = inttoptr i64 %func_ptr to void ()*
  call void %func()
  call void @knull_green_thread_exit()
  ret void
}}

declare void @knull_green_thread_exit()
"#,
            fn_name
        )
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn generate_work_stealing_enqueue() -> String {
        r#"
; Work Stealing Queue Enqueue (Lock-free)
define i64 @ws_enqueue(i64 %queue_ptr, i64 %task_ptr) {
entry:
  %old_head = load i64, i64* %queue_ptr
  %new_head = add i64 %old_head, 1
  %cmp = icmp ult i64 %new_head, %old_head
  br i1 %cmp, label %overflow, label %success

overflow:
  ret i64 -1

success:
  store i64 %new_head, i64* %queue_ptr
  ret i64 0
}
"#
        .to_string()
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn generate_atomic_compare_exchange() -> String {
        r#"
; Lock-free CAS loop for channels
define i64 @knull_cas(i64* %ptr, i64 %old, i64 %new) {
entry:
  %res = cmpxchg i64* %ptr, i64 %old, i64 %new acq_rel monotonic
  %val = extractvalue { i64, i1 } %res, 0
  ret i64 %val
}
"#
        .to_string()
    }

    #[inline(always)]
    pub fn generate_spin_loop_hint(module: &mut LLVMModule) -> String {
        r#"
; Pause instruction for spin loops
define void @knull_pause() {
entry:
  tail call void @llvm.x86.sse2.pause()
  ret void
}

declare void @llvm.x86.sse2.pause() nounwind
"#
        .to_string()
    }
}

#[cfg(not(feature = "llvm-backend"))]
pub mod codegen {
    pub fn generate_async_state_machine(_state_count: usize) -> String {
        String::new()
    }
}

#[cfg(feature = "llvm-backend")]
pub fn optimize_concurrency_patterns(module: &mut LLVMModule) {
    let pass_manager = inkwell::passes::PassManager::create(());
    pass_manager.add_constant_propagation_pass();
    pass_manager.add_instruction_combining_pass();
    pass_manager.add_reassociate_pass();
    pass_manager.add_gvn_pass();
    pass_manager.add_cfg_simplification_pass();
    pass_manager.add_loop_unroll_pass();
    pass_manager.add_loop_vectorize_pass();
    pass_manager.add_sccp_pass();
    pass_manager.add_scalar_repl_aggregates_pass();
    pass_manager.run_on(module);
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConcurrencyLevel {
    None = 0,
    GreenThreads = 1,
    WorkStealing = 2,
    FullAsync = 3,
    Parallel = 4,
}

pub struct ConcurrencyConfig {
    pub green_thread_stack_size: usize,
    pub work_stealing_workers: usize,
    pub work_stealing_queue_size: usize,
    pub channel_buffer_size: usize,
    pub spinloop_pause_threshold: usize,
    pub async_task_cache_size: usize,
    pub parallel_chunk_size: usize,
    pub level: ConcurrencyLevel,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            green_thread_stack_size: 64 * 1024,
            work_stealing_workers: 4,
            work_stealing_queue_size: 1024,
            channel_buffer_size: 64,
            spinloop_pause_threshold: 1000,
            async_task_cache_size: 256,
            parallel_chunk_size: 256,
            level: ConcurrencyLevel::FullAsync,
        }
    }
}

impl ConcurrencyConfig {
    pub fn high_performance() -> Self {
        Self {
            green_thread_stack_size: 128 * 1024,
            work_stealing_workers: num_cpus::get(),
            work_stealing_queue_size: 4096,
            channel_buffer_size: 256,
            spinloop_pause_threshold: 5000,
            async_task_cache_size: 1024,
            parallel_chunk_size: 512,
            level: ConcurrencyLevel::Parallel,
        }
    }

    pub fn embedded() -> Self {
        Self {
            green_thread_stack_size: 8 * 1024,
            work_stealing_workers: 2,
            work_stealing_queue_size: 128,
            channel_buffer_size: 16,
            spinloop_pause_threshold: 100,
            async_task_cache_size: 32,
            parallel_chunk_size: 32,
            level: ConcurrencyLevel::GreenThreads,
        }
    }
}

#[cfg(feature = "llvm-backend")]
mod num_cpus {
    pub fn get() -> usize {
        unsafe {
            let mut len: libc::size_t = 0;
            libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize
        }
    }
}

#[cfg(not(feature = "llvm-backend"))]
mod num_cpus {
    pub fn get() -> usize {
        4
    }
}

#[inline(always)]
fn pause() {
    #[cfg(feature = "llvm-backend")]
    unsafe {
        asm!("pause");
    }
    #[cfg(not(feature = "llvm-backend"))]
    {
        hint::spin_loop();
    }
}

#[inline(always)]
fn fast_yield() {
    thread::yield_now();
}

#[cold]
fn backoff(iteration: usize) {
    if iteration < 4 {
        pause();
    } else if iteration < 16 {
        for _ in 0..4 {
            pause();
        }
    } else {
        fast_yield();
    }
}

unsafe impl<T: ?Sized> Send for Mutex<T> {}
unsafe impl<T: ?Sized> Sync for Mutex<T> {}

pub struct Mutex<T: ?Sized> {
    state: AtomicU32,
    queue: MaybeUninit<StdMutex<Vec<ThreadId>>>,
    data: T,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            queue: MaybeUninit::new(StdMutex::new(Vec::new())),
            data,
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| MutexGuard {
                mutex: self,
                guard: PhantomData,
            })
    }

    pub fn lock(&self) -> MutexGuard<T> {
        loop {
            match self.try_lock() {
                Some(guard) => return guard,
                None => {
                    let queue = unsafe { &*self.queue.as_ptr() };
                    let mut q = queue.lock().unwrap();
                    q.push(thread::current().id());
                }
            }
            fast_yield();
        }
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
    guard: PhantomData<std::sync::MutexGuard<'a, T>>,
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.state.store(0, Ordering::Release);
    }
}

#[derive(Debug)]
pub struct RwLock<T> {
    state: AtomicU32,
    write_lock: StdMutex<()>,
    readers: StdRwLock<Vec<ThreadId>>,
    data: T,
}

impl<T> RwLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            write_lock: StdMutex::new(()),
            readers: StdRwLock::new(Vec::new()),
            data,
        }
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if state & 0x80000000 == 0 {
                if self
                    .state
                    .compare_exchange(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
            }
            fast_yield();
        }
        RwLockReadGuard { rwlock: self }
    }

    pub fn write(&self) -> RwLockWriteGuard<T> {
        self.write_lock.lock().unwrap();
        self.state.fetch_or(0x80000000, Ordering::Acquire);
        RwLockWriteGuard { rwlock: self }
    }
}

pub struct RwLockReadGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

pub struct RwLockWriteGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.rwlock.data
    }
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.rwlock.data
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.rwlock.data
    }
}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.rwlock.state.fetch_sub(1, Ordering::Release);
    }
}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.rwlock.state.fetch_and(!0x80000000, Ordering::Release);
    }
}

pub struct Semaphore {
    permits: AtomicI32,
    waiters: StdMutex<Vec<ThreadId>>,
    signal: Condvar,
}

impl Semaphore {
    pub fn new(permits: i32) -> Self {
        Self {
            permits: AtomicI32::new(permits),
            waiters: StdMutex::new(Vec::new()),
            signal: Condvar::new(),
        }
    }

    pub fn try_acquire(&self) -> bool {
        let mut permits = self.permits.load(Ordering::Relaxed);
        loop {
            if permits <= 0 {
                return false;
            }
            match self.permits.compare_exchange_weak(
                permits,
                permits - 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(p) => permits = p,
            }
        }
    }

    pub fn acquire(&self) {
        while !self.try_acquire() {
            let mut waiters = self.waiters.lock().unwrap();
            waiters.push(thread::current().id());
            waiters = self.signal.wait(waiters).unwrap();
        }
    }

    pub fn release(&self) {
        self.permits.fetch_add(1, Ordering::Release);
        let mut waiters = self.waiters.lock().unwrap();
        if !waiters.is_empty() {
            self.signal.notify_one();
        }
    }
}

pub struct Barrier {
    count: AtomicUsize,
    generation: AtomicUsize,
    waiters: StdMutex<Vec<ThreadId>>,
    signal: Condvar,
}

impl Barrier {
    pub fn new(count: usize) -> Self {
        Self {
            count: AtomicUsize::new(count),
            generation: AtomicUsize::new(0),
            waiters: StdMutex::new(Vec::new()),
            signal: Condvar::new(),
        }
    }

    pub fn wait(&self) -> bool {
        let gen = self.generation.load(Ordering::Acquire);
        let mut waiters = self.waiters.lock().unwrap();
        waiters.push(thread::current().id());

        if waiters.len() >= self.count.load(Ordering::Acquire) {
            waiters.clear();
            self.generation.fetch_add(1, Ordering::Release);
            self.signal.notify_all();
            true
        } else {
            while gen == self.generation.load(Ordering::Acquire) {
                waiters = self.signal.wait(waiters).unwrap();
            }
            false
        }
    }
}

pub struct Once<T> {
    cell: AtomicPtr<T>,
    lock: StdMutex<()>,
}

impl<T> Once<T> {
    pub const fn new() -> Self {
        Self {
            cell: AtomicPtr::new(ptr::null_mut()),
            lock: StdMutex::new(()),
        }
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        let ptr = self.cell.load(Ordering::Acquire);
        if !ptr.is_null() {
            return unsafe { &*ptr };
        }

        let _guard = self.lock.lock().unwrap();
        let ptr = self.cell.load(Ordering::Acquire);
        if !ptr.is_null() {
            return unsafe { &*ptr };
        }

        let value = Box::into_raw(Box::new(f()));
        self.cell.store(value, Ordering::Release);
        unsafe { &*value }
    }
}

pub struct Lazy<T> {
    cell: MaybeUninit<Arc<T>>,
    lock: StdMutex<()>,
    inited: AtomicBool,
}

impl<T> Lazy<T> {
    pub const fn new() -> Self {
        Self {
            cell: MaybeUninit::uninit(),
            lock: StdMutex::new(()),
            inited: AtomicBool::new(false),
        }
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> Arc<T> {
        if self.inited.load(Ordering::Acquire) {
            return unsafe { Arc::clone(&*self.cell.as_ptr()) };
        }

        let _guard = self.lock.lock().unwrap();
        if self.inited.load(Ordering::Acquire) {
            return unsafe { Arc::clone(&*self.cell.as_ptr()) };
        }

        let value = Arc::new(f());
        unsafe {
            self.cell.as_mut_ptr().write(value.clone());
        }
        self.inited.store(true, Ordering::Release);
        value
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &**self.get_or_init(|| unreachable!())
    }
}

pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    thread::spawn(f)
}

pub fn spawn_scoped<'a, F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'a,
    T: Send + 'static,
{
    thread::spawn(f)
}

pub fn scope<'a, T, F>(f: F) -> T
where
    F: FnOnce(&Scope<'a>) -> T,
{
    let scope = Scope::new();
    let result = f(&scope);
    scope.join();
    result
}

pub struct Scope<'a> {
    handles: Mutex<Vec<JoinHandle<()>>>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Scope<'a> {
    fn new() -> Self {
        Self {
            handles: Mutex::new(Vec::new()),
            _marker: PhantomData,
        }
    }

    fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'a,
    {
        let handle = thread::spawn(f);
        self.handles.lock().push(handle);
    }

    fn join(self) {}
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        let mut handles = self.handles.lock();
        for handle in handles.drain(..) {
            let _ = handle.join();
        }
    }
}

pub trait FnBox: Send + 'static {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce() + Send + 'static> FnBox for F {
    fn call_box(self: Box<Self>) {
        (*self)()
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: MpscSender<Job>,
}

struct Worker {
    thread: Option<JoinHandle<()>>,
}

type Job = Box<dyn FnBox + Send>;

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            let recv = Arc::clone(&receiver);
            workers.push(Worker {
                thread: Some(thread::spawn(move || {
                    loop {
                        let job = {
                            let recv = recv.lock();
                            recv.recv()
                        };
                        match job {
                            Ok(job) => job.call_box(),
                            Err(_) => break,
                        }
                    }
                })),
            });
        }

        Self { workers, sender }
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender);
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutex() {
        let mutex = Mutex::new(0i32);
        {
            let mut guard = mutex.lock();
            *guard = 42;
        }
        assert_eq!(*mutex.lock(), 42);
    }

    #[test]
    fn test_rwlock() {
        let rwlock = RwLock::new(vec![1, 2, 3]);
        {
            let read = rwlock.read();
            assert_eq!(read.len(), 3);
        }
        {
            let mut write = rwlock.write();
            write.push(4);
        }
        assert_eq!(rwlock.read().len(), 4);
    }

    #[test]
    fn test_semaphore() {
        let sem = Semaphore::new(2);
        assert!(sem.try_acquire());
        assert!(sem.try_acquire());
        assert!(!sem.try_acquire());
        sem.release();
        assert!(sem.try_acquire());
    }

    #[test]
    fn test_barrier() {
        let barrier = Barrier::new(3);
        let handles: Vec<_> = (0..3)
            .map(|_| {
                thread::spawn(move || {
                    barrier.wait();
                    true
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert!(results.iter().all(|&x| x));
    }

    #[test]
    fn test_thread_pool() {
        let pool = ThreadPool::new(4);
        let counter = Arc::new(Mutex::new(0i32));
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let c = Arc::clone(&counter);
                pool.spawn(move || {
                    let mut guard = c.lock();
                    *guard += 1;
                })
            })
            .collect();
        drop(handles);
        assert_eq!(*counter.lock(), 10);
    }

    #[test]
    fn test_scope() {
        let result = scope(|s| {
            let counter = Mutex::new(0i32);
            for _ in 0..10 {
                s.spawn(|| {
                    let mut guard = counter.lock();
                    *guard += 1;
                });
            }
            *counter.lock()
        });
        assert_eq!(result, 10);
    }
}
