//! Knull Standard Library - Collections

use std::collections::HashMap as StdHashMap;

/// Dynamic array (Vector)
pub struct Vec<T> {
    data: std::vec::Vec<T>,
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        Vec {
            data: std::vec::Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Vec {
            data: std::vec::Vec::with_capacity(cap),
        }
    }

    pub fn push(&mut self, item: T) {
        self.data.push(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    pub fn set(&mut self, index: usize, item: T) {
        if index < self.data.len() {
            self.data[index] = item;
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn contains(&self, item: &T) -> bool
    where
        T: PartialEq,
    {
        self.data.contains(item)
    }

    pub fn remove(&mut self, index: usize) -> T {
        self.data.remove(index)
    }

    pub fn insert(&mut self, index: usize, item: T) {
        self.data.insert(index, item);
    }
}

/// Hash map
pub struct HashMap<K, V> {
    data: StdHashMap<K, V>,
}

impl<K, V> HashMap<K, V>
where
    K: std::cmp::Eq + std::hash::Hash,
{
    pub fn new() -> Self {
        HashMap {
            data: StdHashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.data.insert(key, value)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.data.remove(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn keys(&self) -> Vec<&K> {
        self.data.keys().collect()
    }

    pub fn values(&self) -> Vec<&V> {
        self.data.values().collect()
    }
}

/// Fixed-size array wrapper
pub struct Array<T, const N: usize> {
    data: [T; N],
}

impl<T: Default + Copy, const N: usize> Array<T, N> {
    pub fn new() -> Self {
        Array {
            data: [T::default(); N],
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    pub fn set(&mut self, index: usize, value: T) {
        if index < N {
            self.data[index] = value;
        }
    }

    pub fn len(&self) -> usize {
        N
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data
    }
}

/// Queue (FIFO)
pub struct Queue<T> {
    data: std::collections::VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            data: std::collections::VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, item: T) {
        self.data.push_back(item);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        self.data.pop_front()
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.front()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Stack (LIFO)
pub struct Stack<T> {
    data: std::vec::Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack {
            data: std::vec::Vec::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        self.data.push(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    pub fn peek(&self) -> Option<&T> {
        self.data.last()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
