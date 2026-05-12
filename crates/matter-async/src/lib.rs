//! Matter Core Async Runtime
//!
//! Provides async/await, channels, spawn/join and thread safety primitives.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex as StdMutex};

fn lock_unpoison<T>(mutex: &StdMutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Task ID
pub type TaskId = usize;

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Created,
    Scheduled,
    Running,
    Waiting,
    Completed,
    Failed,
}

/// Task handle
#[derive(Debug, Clone)]
pub struct TaskHandle {
    pub id: TaskId,
    pub state: Arc<StdMutex<TaskState>>,
}

impl TaskHandle {
    pub fn new(id: TaskId) -> Self {
        Self {
            id,
            state: Arc::new(StdMutex::new(TaskState::Created)),
        }
    }

    pub fn get_state(&self) -> TaskState {
        *lock_unpoison(&self.state)
    }

    pub fn set_state(&self, state: TaskState) {
        *lock_unpoison(&self.state) = state;
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.get_state(), TaskState::Completed | TaskState::Failed)
    }
}

/// Channel for communication between tasks
pub struct Channel<T> {
    queue: Arc<StdMutex<VecDeque<T>>>,
    closed: Arc<StdMutex<bool>>,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(StdMutex::new(VecDeque::new())),
            closed: Arc::new(StdMutex::new(false)),
        }
    }

    pub fn send(&self, value: T) -> Result<(), &'static str> {
        if *lock_unpoison(&self.closed) {
            return Err("Channel is closed");
        }
        lock_unpoison(&self.queue).push_back(value);
        Ok(())
    }

    pub fn recv(&self) -> Option<T> {
        lock_unpoison(&self.queue).pop_front()
    }

    pub fn close(&self) {
        *lock_unpoison(&self.closed) = true;
    }

    pub fn is_closed(&self) -> bool {
        *lock_unpoison(&self.closed)
    }

    pub fn len(&self) -> usize {
        lock_unpoison(&self.queue).len()
    }

    pub fn is_empty(&self) -> bool {
        lock_unpoison(&self.queue).is_empty()
    }
}

impl<T> Clone for Channel<T> {
    fn clone(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
            closed: Arc::clone(&self.closed),
        }
    }
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Mutex for thread-safe access
pub struct MatterMutex<T> {
    value: Arc<StdMutex<T>>,
}

impl<T> MatterMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(StdMutex::new(value)),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, T> {
        lock_unpoison(&self.value)
    }
}

impl<T> Clone for MatterMutex<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
        }
    }
}

/// Async runtime
pub struct AsyncRuntime {
    next_task_id: StdMutex<TaskId>,
    tasks: Arc<StdMutex<Vec<TaskHandle>>>,
}

impl AsyncRuntime {
    pub fn new() -> Self {
        Self {
            next_task_id: StdMutex::new(0),
            tasks: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    pub fn spawn(&self) -> TaskHandle {
        let mut id = lock_unpoison(&self.next_task_id);
        let task_id = *id;
        *id += 1;

        let handle = TaskHandle::new(task_id);
        lock_unpoison(&self.tasks).push(handle.clone());

        handle
    }

    pub fn get_task(&self, id: TaskId) -> Option<TaskHandle> {
        self.tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    pub fn active_tasks(&self) -> usize {
        self.tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|t| !t.is_completed())
            .count()
    }

    pub fn completed_tasks(&self) -> usize {
        self.tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|t| t.is_completed())
            .count()
    }
}

impl Default for AsyncRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_handle() {
        let handle = TaskHandle::new(1);
        assert_eq!(handle.id, 1);
        assert_eq!(handle.get_state(), TaskState::Created);

        handle.set_state(TaskState::Running);
        assert_eq!(handle.get_state(), TaskState::Running);

        handle.set_state(TaskState::Completed);
        assert!(handle.is_completed());
    }

    #[test]
    fn test_channel() {
        let ch = Channel::new();

        assert!(ch.send(42).is_ok());
        assert_eq!(ch.recv(), Some(42));
        assert_eq!(ch.recv(), None);
    }

    #[test]
    fn test_channel_close() {
        let ch = Channel::new();

        ch.send(1).unwrap();
        ch.close();

        assert!(ch.is_closed());
        assert!(ch.send(2).is_err());
        assert_eq!(ch.recv(), Some(1));
    }

    #[test]
    fn test_mutex() {
        let mutex = MatterMutex::new(0);

        {
            let mut value = mutex.lock();
            *value = 42;
        }

        assert_eq!(*mutex.lock(), 42);
    }

    #[test]
    fn test_async_runtime() {
        let runtime = AsyncRuntime::new();

        let task1 = runtime.spawn();
        let task2 = runtime.spawn();

        assert_eq!(task1.id, 0);
        assert_eq!(task2.id, 1);
        assert_eq!(runtime.active_tasks(), 2);

        task1.set_state(TaskState::Completed);
        assert_eq!(runtime.completed_tasks(), 1);
        assert_eq!(runtime.active_tasks(), 1);
    }

    #[test]
    fn test_channel_clone() {
        let ch1 = Channel::new();
        let ch2 = ch1.clone();

        ch1.send(42).unwrap();
        assert_eq!(ch2.recv(), Some(42));
    }

    #[test]
    fn test_channel_len() {
        let ch = Channel::new();

        assert_eq!(ch.len(), 0);
        assert!(ch.is_empty());

        ch.send(1).unwrap();
        ch.send(2).unwrap();

        assert_eq!(ch.len(), 2);
        assert!(!ch.is_empty());
    }

    #[test]
    fn test_runtime_get_task() {
        let runtime = AsyncRuntime::new();
        let task = runtime.spawn();

        let found = runtime.get_task(task.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, task.id);

        let not_found = runtime.get_task(999);
        assert!(not_found.is_none());
    }
}
