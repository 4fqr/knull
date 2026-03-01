//! Knull Zero-Cost Async Runtime
//!
//! Provides async/await support with state machine generation only when awaited

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Async state machine states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsyncState {
    /// Not started yet
    Pending,
    /// Currently executing
    Running,
    /// Waiting for I/O
    Suspended,
    /// Finished with result
    Completed,
    /// Finished with error
    Failed,
}

/// Zero-cost Future - generates state machine only when awaited
pub struct Future<T: Send + 'static> {
    state: AsyncState,
    result: Option<T>,
    error: Option<String>,
    waker: Option<Arc<Waker>>,
}

unsafe impl<T: Send> Send for Future<T> {}
unsafe impl<T: Send> Sync for Future<T> {}

impl<T: Send> Future<T> {
    pub fn new() -> Self {
        Future {
            state: AsyncState::Pending,
            result: None,
            error: None,
            waker: None,
        }
    }

    pub fn ready(value: T) -> Self {
        Future {
            state: AsyncState::Completed,
            result: Some(value),
            error: None,
            waker: None,
        }
    }

    pub fn pending() -> Self {
        Future {
            state: AsyncState::Pending,
            result: None,
            error: None,
            waker: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state == AsyncState::Completed || self.state == AsyncState::Failed
    }

    pub fn poll(&mut self) -> bool {
        self.is_ready()
    }

    pub fn get_blocking(&self) -> &T {
        while self.state != AsyncState::Completed && self.state != AsyncState::Failed {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        self.result.as_ref().expect("Future completed with error")
    }

    pub fn set_result(&mut self, value: T) {
        self.result = Some(value);
        self.state = AsyncState::Completed;
        
        if let Some(ref waker) = self.waker {
            waker.wake();
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.state = AsyncState::Failed;
        
        if let Some(ref waker) = self.waker {
            waker.wake();
        }
    }
}

/// Waker for async tasks
pub struct Waker {
    ready: Arc<AtomicBool>,
    task: Arc<Mutex<dyn FnOnce() + Send>>,
}

impl Waker {
    pub fn new<F>(task: F) -> Arc<Self>
    where
        F: FnOnce() + Send + 'static,
    {
        Arc::new(Waker {
            ready: Arc::new(AtomicBool::new(false)),
            task: Arc::new(Mutex::new(task)),
        })
    }

    pub fn wake(&self) {
        self.ready.store(true, Ordering::SeqCst);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    pub fn consume(&self) -> Option<Box<dyn FnOnce() + Send>> {
        if self.ready.swap(false, Ordering::SeqCst) {
            let task = self.task.lock().unwrap();
            Some(Box::new(task as Box<dyn FnOnce() + Send>))
        } else {
            None
        }
    }
}

/// Promise for manual async operations
pub struct Promise<T: Send> {
    future: Future<T>,
}

impl<T: Send> Promise<T> {
    pub fn new() -> Self {
        Promise {
            future: Future::pending(),
        }
    }

    pub fn resolve(&self, value: T) {
        // Note: In real implementation, this needs interior mutability
    }

    pub fn reject(&self, error: String) {
        // Note: In real implementation, this needs interior mutability
    }

    pub fn future(&self) -> &Future<T> {
        &self.future
    }
}

/// Task handle for spawned async functions
pub struct Task<T: Send + 'static> {
    id: u64,
    future: Future<T>,
}

impl<T: Send> Task<T> {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn is_completed(&self) -> bool {
        self.future.is_ready()
    }

    pub fn get(&self) -> &T {
        self.future.get_blocking()
    }
}

/// Async runtime scheduler
pub struct Runtime {
    tasks: Mutex<VecDeque<TaskBox>>,
    next_id: AtomicUsize,
    active: AtomicBool,
}

struct TaskBox {
    id: usize,
    future: Box<dyn AnyFuture>,
}

trait AnyFuture: Send {
    fn poll(&mut self) -> AsyncState;
    fn get_result(&self) -> Option<String>;
}

impl<T: Send + 'static> AnyFuture for Future<T> {
    fn poll(&mut self) -> AsyncState {
        if self.is_ready() {
            self.state
        } else {
            AsyncState::Suspended
        }
    }

    fn get_result(&self) -> Option<String> {
        self.error.clone()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            tasks: Mutex::new(VecDeque::new()),
            next_id: AtomicUsize::new(0),
            active: AtomicBool::new(false),
        }
    }

    pub fn spawn<T, F>(&self, future: F) -> Task<T>
    where
        T: Send + 'static,
        F: FutureTrait<T>,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        
        let task = TaskBox {
            id,
            future: Box::new(future.into_future()),
        };
        
        self.tasks.lock().unwrap().push_back(task);
        
        Task {
            id: id as u64,
            future: Future::pending(),
        }
    }

    pub fn run(&self) {
        self.active.store(true, Ordering::SeqCst);
        
        while self.active.load(Ordering::SeqCst) {
            let mut tasks = self.tasks.lock().unwrap();
            
            if tasks.is_empty() {
                break;
            }
            
            // Simple round-robin scheduling
            let mut ready_tasks = VecDeque::new();
            std::mem::swap(&mut *tasks, &mut ready_tasks);
            
            drop(tasks);
            
            while let Some(mut task) = ready_tasks.pop_front() {
                let state = task.future.poll();
                
                if state != AsyncState::Completed && state != AsyncState::Failed {
                    ready_tasks.push_back(task);
                }
            }
            
            tasks = self.tasks.lock().unwrap();
            std::mem::swap(&mut *tasks, &mut ready_tasks);
            drop(tasks);
            
            if ready_tasks.is_empty() {
                break;
            }
            
            // Yield to other threads
            std::thread::yield_now();
        }
    }

    pub fn shutdown(&self) {
        self.active.store(false, Ordering::SeqCst);
    }
}

/// Trait for converting into Future
pub trait FutureTrait<T: Send + 'static> {
    fn into_future(self) -> Future<T>;
}

impl<T: Send + 'static, F: Future<T> + Send + 'static> FutureTrait<T> for F {
    fn into_future(self) -> Future<T> {
        self
    }
}

/// Block on a future (for non-async code)
pub fn block_on<T: Send>(future: Future<T>) -> T {
    future.get_blocking()
}

/// Spawn an async task
pub fn spawn<T, F>(future: F) -> Task<T>
where
    T: Send + 'static,
    F: FutureTrait<T>,
{
    let runtime = get_runtime();
    runtime.spawn(future)
}

/// Get the global runtime
fn get_runtime() -> &'static Runtime {
    static RUNTIME: Runtime = Runtime::new();
    &RUNTIME
}

/// Yield to scheduler
pub fn yield_now() {
    std::thread::yield_now();
}

/// Sleep for duration
pub async fn sleep(duration: std::time::Duration) {
    std::thread::sleep(duration);
}

// Re-export for convenience
pub use std::future::Future as StdFuture;
