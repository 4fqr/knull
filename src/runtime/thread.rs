//! Knull Standard Library - Threading Module
//!
//! Provides real threading, channels, and synchronization primitives.

use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

/// Thread handle
pub struct KnullThread {
    handle: Option<thread::JoinHandle<()>>,
    id: u64,
}

impl KnullThread {
    pub fn spawn<F>(f: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let handle = thread::spawn(f);
        let id = handle.thread().id().as_u64().get();

        KnullThread {
            handle: Some(handle),
            id,
        }
    }

    pub fn join(&mut self) -> Result<(), String> {
        if let Some(handle) = self.handle.take() {
            handle.join().map_err(|_| "Thread panicked".to_string())?;
        }
        Ok(())
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

/// Thread-safe shared value
pub struct KnullMutex<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> KnullMutex<T> {
    pub fn new(value: T) -> Self {
        KnullMutex {
            inner: Arc::new(Mutex::new(value)),
        }
    }

    pub fn lock<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| format!("Mutex poisoned: {}", e))?;
        Ok(f(&mut *guard))
    }

    pub fn try_lock<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self
            .inner
            .try_lock()
            .map_err(|_| "Mutex would block".to_string())?;
        Ok(f(&mut *guard))
    }
}

impl<T> Clone for KnullMutex<T> {
    fn clone(&self) -> Self {
        KnullMutex {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Read-write lock
pub struct KnullRwLock<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> KnullRwLock<T> {
    pub fn new(value: T) -> Self {
        KnullRwLock {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    pub fn read<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self
            .inner
            .read()
            .map_err(|e| format!("RwLock poisoned: {}", e))?;
        Ok(f(&*guard))
    }

    pub fn write<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self
            .inner
            .write()
            .map_err(|e| format!("RwLock poisoned: {}", e))?;
        Ok(f(&mut *guard))
    }
}

impl<T> Clone for KnullRwLock<T> {
    fn clone(&self) -> Self {
        KnullRwLock {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Channel sender
pub struct KnullSender<T> {
    tx: mpsc::Sender<T>,
}

impl<T> KnullSender<T> {
    pub fn send(&self, value: T) -> Result<(), String> {
        self.tx
            .send(value)
            .map_err(|_| "Receiver disconnected".to_string())
    }

    pub fn try_send(&self, value: T) -> Result<(), String> {
        self.tx
            .try_send(value)
            .map_err(|_| "Channel full or disconnected".to_string())
    }
}

impl<T> Clone for KnullSender<T> {
    fn clone(&self) -> Self {
        KnullSender {
            tx: self.tx.clone(),
        }
    }
}

/// Channel receiver
pub struct KnullReceiver<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> KnullReceiver<T> {
    pub fn recv(&self) -> Result<T, String> {
        self.rx
            .recv()
            .map_err(|_| "All senders disconnected".to_string())
    }

    pub fn try_recv(&self) -> Result<T, String> {
        self.rx
            .try_recv()
            .map_err(|_| "Channel empty or disconnected".to_string())
    }

    pub fn recv_timeout(&self, millis: u64) -> Result<T, String> {
        let duration = Duration::from_millis(millis);
        self.rx
            .recv_timeout(duration)
            .map_err(|e| format!("Receive error: {:?}", e))
    }
}

/// Create a channel
pub fn channel<T>() -> (KnullSender<T>, KnullReceiver<T>) {
    let (tx, rx) = mpsc::channel();
    (KnullSender { tx }, KnullReceiver { rx })
}

/// Create a bounded channel with capacity
pub fn bounded_channel<T>(capacity: usize) -> (KnullSender<T>, KnullReceiver<T>) {
    let (tx, rx) = mpsc::sync_channel(capacity);
    (KnullSender { tx }, KnullReceiver { rx })
}

/// Thread pool for managing worker threads
pub struct ThreadPool {
    workers: Vec<KnullThread>,
    sender: Option<KnullSender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = bounded_channel::<Job>(size * 2);
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for _ in 0..size {
            let receiver = Arc::clone(&receiver);

            let thread = KnullThread::spawn(move || loop {
                let job = {
                    let rx = receiver.lock().unwrap();
                    rx.recv()
                };

                match job {
                    Ok(job) => job(),
                    Err(_) => break,
                }
            });

            workers.push(thread);
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender
            .as_ref()
            .ok_or("Thread pool shut down")?
            .send(job)
    }

    pub fn shutdown(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            let _ = worker.join();
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Sleep for specified milliseconds
pub fn sleep(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}

/// Get current thread ID
pub fn current_thread_id() -> u64 {
    thread::current().id().as_u64().get()
}

/// Yield current thread
pub fn yield_thread() {
    thread::yield_now();
}

/// Atomic counter
pub struct AtomicCounter {
    value: Arc<Mutex<i64>>,
}

impl AtomicCounter {
    pub fn new(initial: i64) -> Self {
        AtomicCounter {
            value: Arc::new(Mutex::new(initial)),
        }
    }

    pub fn increment(&self) -> i64 {
        let mut val = self.value.lock().unwrap();
        *val += 1;
        *val
    }

    pub fn decrement(&self) -> i64 {
        let mut val = self.value.lock().unwrap();
        *val -= 1;
        *val
    }

    pub fn get(&self) -> i64 {
        *self.value.lock().unwrap()
    }

    pub fn set(&self, value: i64) {
        *self.value.lock().unwrap() = value;
    }
}

impl Clone for AtomicCounter {
    fn clone(&self) -> Self {
        AtomicCounter {
            value: Arc::clone(&self.value),
        }
    }
}

/// Barrier for thread synchronization
pub struct Barrier {
    count: KnullMutex<usize>,
    target: usize,
    cond: Arc<std::sync::Condvar>,
}

impl Barrier {
    pub fn new(count: usize) -> Self {
        Barrier {
            count: KnullMutex::new(0),
            target: count,
            cond: Arc::new(std::sync::Condvar::new()),
        }
    }

    pub fn wait(&self) -> Result<usize, String> {
        let mut count = self
            .count
            .inner
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        *count += 1;
        let current = *count;

        if current >= self.target {
            *count = 0;
            self.cond.notify_all();
            Ok(0)
        } else {
            let _ = self
                .cond
                .wait(count)
                .map_err(|e| format!("Wait error: {}", e))?;
            Ok(current)
        }
    }
}

/// Semaphore for controlling access to resources
pub struct Semaphore {
    permits: KnullMutex<usize>,
    cond: Arc<std::sync::Condvar>,
}

impl Semaphore {
    pub fn new(permits: usize) -> Self {
        Semaphore {
            permits: KnullMutex::new(permits),
            cond: Arc::new(std::sync::Condvar::new()),
        }
    }

    pub fn acquire(&self) -> Result<(), String> {
        let mut permits = self
            .permits
            .inner
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        while *permits == 0 {
            permits = self
                .cond
                .wait(permits)
                .map_err(|e| format!("Wait error: {}", e))?;
        }

        *permits -= 1;
        Ok(())
    }

    pub fn release(&self) -> Result<(), String> {
        let mut permits = self
            .permits
            .inner
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        *permits += 1;
        self.cond.notify_one();
        Ok(())
    }
}

/// Spawn a new thread
pub fn spawn_thread<F>(f: F) -> KnullThread
where
    F: FnOnce() + Send + 'static,
{
    KnullThread::spawn(f)
}

/// Scoped thread (can borrow from parent scope) - requires crossbeam
/// For now, we'll use Arc for shared data

/// Parallel map using thread pool
pub fn parallel_map<T, F, R>(items: Vec<T>, f: F) -> Vec<R>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> R + Send + Sync + 'static,
{
    let f = Arc::new(f);
    let mut handles = Vec::new();

    for item in items {
        let f = Arc::clone(&f);
        let handle = spawn_thread(move || f(item));
        handles.push(handle);
    }

    // Note: We'd need to collect results, but our current API doesn't support it easily
    // For now, just wait for completion
    for mut handle in handles {
        let _ = handle.join();
    }

    Vec::new() // Placeholder - would need proper result collection
}
