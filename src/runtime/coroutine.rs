//! Knull Coroutine Runtime
//!
//! Provides lightweight coroutine support for cooperative multitasking

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Coroutine state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoroutineState {
    /// Ready to run
    Ready,
    /// Currently executing
    Running,
    /// Suspended (yielded)
    Suspended,
    /// Completed
    Completed,
}

/// Coroutine handle
pub struct Coroutine {
    id: u64,
    state: CoroutineState,
    stack: *mut u8,
    stack_size: usize,
}

unsafe impl Send for Coroutine {}

impl Coroutine {
    pub fn new<F>(func: F, stack_size: usize) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let layout = std::alloc::Layout::from_size_align(stack_size, 16).unwrap();
        let stack = unsafe { std::alloc::alloc(layout) };

        Coroutine {
            id: 0,
            state: CoroutineState::Ready,
            stack,
            stack_size,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn is_completed(&self) -> bool {
        self.state == CoroutineState::Completed
    }

    pub fn is_suspended(&self) -> bool {
        self.state == CoroutineState::Suspended
    }
}

impl Drop for Coroutine {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::from_size_align(self.stack_size, 16).unwrap();
        unsafe { std::alloc::dealloc(self.stack, layout) };
    }
}

/// Coroutine local storage
pub struct CoroutineLocal<T: Send + 'static> {
    value: Mutex<Option<T>>,
}

impl<T: Send + 'static> CoroutineLocal<T> {
    pub fn new() -> Self {
        CoroutineLocal {
            value: Mutex::new(None),
        }
    }

    pub fn set(&self, value: T) {
        let mut guard = self.value.lock().unwrap();
        *guard = Some(value);
    }

    pub fn get(&self) -> Option<T> {
        let guard = self.value.lock().unwrap();
        guard.clone()
    }
}

/// Coroutine scheduler
pub struct CoroutineScheduler {
    ready_queue: Mutex<VecDeque<Arc<CoroutineTask>>>,
    suspended: Mutex<VecDeque<Arc<CoroutineTask>>>,
    completed: Mutex<VecDeque<Arc<CoroutineTask>>>,
    current: Mutex<Option<Arc<CoroutineTask>>>,
    next_id: AtomicUsize,
    active: AtomicBool,
}

struct CoroutineTask {
    id: u64,
    state: CoroutineState,
    func: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    stack: *mut u8,
    stack_size: usize,
}

unsafe impl Send for CoroutineTask {}

impl CoroutineScheduler {
    pub fn new() -> Self {
        CoroutineScheduler {
            ready_queue: Mutex::new(VecDeque::new()),
            suspended: Mutex::new(VecDeque::new()),
            completed: Mutex::new(VecDeque::new()),
            current: Mutex::new(None),
            next_id: AtomicUsize::new(0),
            active: AtomicBool::new(false),
        }
    }

    pub fn spawn<F>(&self, func: F) -> u64
    where
        F: FnOnce() + Send + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as u64;

        let stack_size = 65536; // 64KB default
        let layout = std::alloc::Layout::from_size_align(stack_size, 16).unwrap();
        let stack = unsafe { std::alloc::alloc(layout) };

        let task = Arc::new(CoroutineTask {
            id,
            state: CoroutineState::Ready,
            func: Mutex::new(Some(Box::new(func))),
            stack,
            stack_size,
        });

        self.ready_queue.lock().unwrap().push_back(task);
        id
    }

    pub fn spawn_with_stack<F>(&self, func: F, stack_size: usize) -> u64
    where
        F: FnOnce() + Send + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as u64;

        let layout = std::alloc::Layout::from_size_align(stack_size, 16).unwrap();
        let stack = unsafe { std::alloc::alloc(layout) };

        let task = Arc::new(CoroutineTask {
            id,
            state: CoroutineState::Ready,
            func: Mutex::new(Some(Box::new(func))),
            stack,
            stack_size,
        });

        self.ready_queue.lock().unwrap().push_back(task);
        id
    }

    pub fn resume(&self, id: u64) {
        let mut ready = self.ready_queue.lock().unwrap();
        let mut suspended = self.suspended.lock().unwrap();

        // Find and resume the coroutine
        if let Some(pos) = suspended.iter().position(|t| t.id == id) {
            let task = suspended.remove(pos).unwrap();
            task.state = CoroutineState::Running;
            ready.push_back(task);
        }
    }

    pub fn yield_now(&self) {
        let mut current = self.current.lock().unwrap();

        if let Some(task) = current.take() {
            task.state = CoroutineState::Suspended;
            self.suspended.lock().unwrap().push_back(task);
        }
    }

    pub fn run(&self) {
        self.active.store(true, Ordering::SeqCst);

        while self.active.load(Ordering::SeqCst) {
            let mut ready = self.ready_queue.lock().unwrap();

            if ready.is_empty() {
                break;
            }

            let task = ready.pop_front();
            drop(ready);

            if let Some(task) = task {
                {
                    let mut current = self.current.lock().unwrap();
                    *current = Some(task.clone());
                }

                task.state = CoroutineState::Running;

                // Execute the coroutine
                let func = task.func.lock().unwrap().take();
                if let Some(f) = func {
                    f();
                }

                task.state = CoroutineState::Completed;
                self.completed.lock().unwrap().push_back(task);

                let mut current = self.current.lock().unwrap();
                *current = None;
            }

            // Clean up completed tasks
            self.completed.lock().unwrap().clear();
        }
    }

    pub fn shutdown(&self) {
        self.active.store(false, Ordering::SeqCst);

        // Clean up
        self.ready_queue.lock().unwrap().clear();
        self.suspended.lock().unwrap().clear();
        self.completed.lock().unwrap().clear();
    }
}

/// Spawn a coroutine
pub fn spawn<F>(func: F) -> u64
where
    F: FnOnce() + Send + 'static,
{
    get_scheduler().spawn(func)
}

/// Spawn a coroutine with custom stack size
pub fn spawn_with_stack<F>(func: F, stack_size: usize) -> u64
where
    F: FnOnce() + Send + 'static,
{
    get_scheduler().spawn_with_stack(func, stack_size)
}

/// Yield to scheduler
pub fn yield_now() {
    get_scheduler().yield_now();
}

/// Resume a suspended coroutine
pub fn resume(id: u64) {
    get_scheduler().resume(id);
}

/// Run the scheduler
pub fn run() {
    get_scheduler().run();
}

/// Shutdown the scheduler
pub fn shutdown() {
    get_scheduler().shutdown();
}

/// Get the global scheduler
fn get_scheduler() -> &'static CoroutineScheduler {
    static SCHEDULER: CoroutineScheduler = CoroutineScheduler::new();
    &SCHEDULER
}

/// Channel for coroutine communication
pub struct Channel<T: Send + 'static> {
    queue: Mutex<VecDeque<T>>,
    closed: AtomicBool,
    send_waiting: Mutex<VecDeque<std::sync::mpsc::Sender<T>>>,
    recv_waiting: Mutex<VecDeque<std::sync::mpsc::Receiver<T>>>,
}

impl<T: Send + 'static> Channel<T> {
    pub fn new(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (tx, rx) = std::sync::mpsc::channel();

        let channel = Arc::new(Channel {
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            closed: AtomicBool::new(false),
            send_waiting: Mutex::new(VecDeque::new()),
            recv_waiting: Mutex::new(VecDeque::new()),
        });

        (
            Sender {
                channel: channel.clone(),
                tx,
            },
            Receiver {
                channel: channel.clone(),
                rx,
            },
        )
    }

    pub fn send(&self, value: T) -> bool {
        if self.closed.load(Ordering::SeqCst) {
            return false;
        }

        self.queue.lock().unwrap().push_back(value);
        true
    }

    pub fn recv(&self) -> Option<T> {
        self.queue.lock().unwrap().pop_front()
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

pub struct Sender<T: Send + 'static> {
    channel: Arc<Channel<T>>,
    tx: std::sync::mpsc::Sender<T>,
}

impl<T: Send + 'static> Sender<T> {
    pub fn send(&self, value: T) -> bool {
        self.channel.send(value)
    }

    pub fn close(&self) {
        self.channel.close();
    }
}

pub struct Receiver<T: Send + 'static> {
    channel: Arc<Channel<T>>,
    rx: std::sync::mpsc::Receiver<T>,
}

impl<T: Send + 'static> Receiver<T> {
    pub fn recv(&self) -> Option<T> {
        self.channel.recv()
    }
}

/// Sleep in a coroutine
pub fn sleep(duration: std::time::Duration) {
    std::thread::sleep(duration);
}
