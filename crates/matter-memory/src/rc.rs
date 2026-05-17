//! Reference counted smart pointers

use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Reference counted object wrapper
struct RcBox<T> {
    /// Strong reference count
    strong: AtomicUsize,

    /// Weak reference count
    weak: AtomicUsize,

    /// The actual data
    value: T,
}

/// Strong reference counted pointer
pub struct Rc<T> {
    ptr: NonNull<RcBox<T>>,
}

impl<T> Rc<T> {
    /// Create a new reference counted value
    pub fn new(value: T) -> Self {
        let boxed = Box::new(RcBox {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(0),
            value,
        });

        Rc {
            ptr: unsafe { NonNull::new_unchecked(Box::into_raw(boxed)) },
        }
    }

    /// Get the number of strong references
    pub fn strong_count(&self) -> usize {
        self.inner().strong.load(Ordering::Relaxed)
    }

    /// Get the number of weak references
    pub fn weak_count(&self) -> usize {
        self.inner().weak.load(Ordering::Relaxed)
    }

    /// Create a weak reference
    pub fn downgrade(&self) -> Weak<T> {
        self.inner().weak.fetch_add(1, Ordering::Relaxed);
        Weak { ptr: self.ptr }
    }

    /// Get a mutable reference if this is the only strong reference
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.strong_count() == 1 {
            Some(unsafe { &mut self.ptr.as_mut().value })
        } else {
            None
        }
    }

    /// Try to unwrap the value if this is the only strong reference
    pub fn try_unwrap(self) -> Result<T, Self> {
        if self.strong_count() == 1 {
            unsafe {
                let val = std::ptr::read(&self.ptr.as_ref().value);

                // Don't run the Drop impl
                std::mem::forget(self);

                Ok(val)
            }
        } else {
            Err(self)
        }
    }

    fn inner(&self) -> &RcBox<T> {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        self.inner().strong.fetch_add(1, Ordering::Relaxed);
        Rc { ptr: self.ptr }
    }
}

impl<T> Deref for Rc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner().value
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        let old_strong = self.inner().strong.fetch_sub(1, Ordering::Release);

        if old_strong == 1 {
            // This was the last strong reference
            std::sync::atomic::fence(Ordering::Acquire);

            // Check if there are any weak references
            if unsafe { self.ptr.as_ref().weak.load(Ordering::Relaxed) } == 0 {
                // No weak references, deallocate everything
                unsafe {
                    drop(Box::from_raw(self.ptr.as_ptr()));
                }
            } else {
                // There are weak references, only drop the value
                unsafe {
                    std::ptr::drop_in_place(&mut self.ptr.as_mut().value);
                }
            }
        }
    }
}

unsafe impl<T: Send> Send for Rc<T> {}
unsafe impl<T: Sync> Sync for Rc<T> {}

impl<T: PartialEq> PartialEq for Rc<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Rc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

/// Weak reference counted pointer
pub struct Weak<T> {
    ptr: NonNull<RcBox<T>>,
}

impl<T> Weak<T> {
    /// Attempt to upgrade to a strong reference
    pub fn upgrade(&self) -> Option<Rc<T>> {
        let inner = unsafe { self.ptr.as_ref() };
        let strong = inner.strong.load(Ordering::Relaxed);

        if strong == 0 {
            None
        } else {
            inner.strong.fetch_add(1, Ordering::Relaxed);
            Some(Rc { ptr: self.ptr })
        }
    }

    /// Get the number of strong references
    pub fn strong_count(&self) -> usize {
        unsafe { self.ptr.as_ref().strong.load(Ordering::Relaxed) }
    }

    /// Get the number of weak references
    pub fn weak_count(&self) -> usize {
        unsafe { self.ptr.as_ref().weak.load(Ordering::Relaxed) }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.ptr.as_ref().weak.fetch_add(1, Ordering::Relaxed);
        }
        Weak { ptr: self.ptr }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        let old_weak = unsafe { self.ptr.as_ref().weak.fetch_sub(1, Ordering::Release) };

        if old_weak == 1 {
            // This was the last weak reference
            std::sync::atomic::fence(Ordering::Acquire);

            unsafe {
                // Check if there are any strong references
                if self.ptr.as_ref().strong.load(Ordering::Relaxed) == 0 {
                    // No strong references, deallocate
                    drop(Box::from_raw(self.ptr.as_ptr()));
                }
            }
        }
    }
}

unsafe impl<T: Send> Send for Weak<T> {}
unsafe impl<T: Sync> Sync for Weak<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc_creation() {
        let rc = Rc::new(42);
        assert_eq!(*rc, 42);
        assert_eq!(rc.strong_count(), 1);
        assert_eq!(rc.weak_count(), 0);
    }

    #[test]
    fn test_rc_clone() {
        let rc1 = Rc::new(42);
        let rc2 = rc1.clone();

        assert_eq!(*rc1, 42);
        assert_eq!(*rc2, 42);
        assert_eq!(rc1.strong_count(), 2);
        assert_eq!(rc2.strong_count(), 2);
    }

    #[test]
    fn test_rc_drop() {
        let rc1 = Rc::new(42);
        let rc2 = rc1.clone();

        assert_eq!(rc1.strong_count(), 2);

        drop(rc2);

        assert_eq!(rc1.strong_count(), 1);
    }

    #[test]
    fn test_weak_reference() {
        let rc = Rc::new(42);
        let weak = rc.downgrade();

        assert_eq!(rc.weak_count(), 1);
        assert_eq!(weak.strong_count(), 1);

        let upgraded = weak.upgrade();
        assert!(upgraded.is_some());
        assert_eq!(*upgraded.unwrap(), 42);
    }

    #[test]
    fn test_weak_upgrade_after_drop() {
        let rc = Rc::new(42);
        let weak = rc.downgrade();

        drop(rc);

        let upgraded = weak.upgrade();
        assert!(upgraded.is_none());
    }

    #[test]
    fn test_rc_get_mut() {
        let mut rc = Rc::new(42);

        if let Some(val) = rc.get_mut() {
            *val = 100;
        }

        assert_eq!(*rc, 100);
    }

    #[test]
    fn test_rc_get_mut_fails_with_multiple_refs() {
        let mut rc1 = Rc::new(42);
        let _rc2 = rc1.clone();

        assert!(rc1.get_mut().is_none());
    }

    #[test]
    fn test_rc_try_unwrap() {
        let rc = Rc::new(42);

        match rc.try_unwrap() {
            Ok(val) => assert_eq!(val, 42),
            Err(_) => panic!("should have unwrapped"),
        }
    }

    #[test]
    fn test_rc_try_unwrap_fails_with_multiple_refs() {
        let rc1 = Rc::new(42);
        let rc2 = rc1.clone();

        match rc1.try_unwrap() {
            Ok(_) => panic!("should not have unwrapped"),
            Err(rc) => {
                assert_eq!(*rc, 42);
                assert_eq!(rc.strong_count(), 2);
            }
        }

        drop(rc2);
    }

    #[test]
    fn test_weak_clone() {
        let rc = Rc::new(42);
        let weak1 = rc.downgrade();
        let weak2 = weak1.clone();

        assert_eq!(rc.weak_count(), 2);
        assert_eq!(weak1.weak_count(), 2);
        assert_eq!(weak2.weak_count(), 2);
    }

    #[test]
    fn test_circular_reference_with_weak() {
        struct Node {
            _value: i32,
            _next: Option<Weak<Node>>,
        }

        let node1 = Rc::new(Node {
            _value: 1,
            _next: None,
        });

        let node2 = Rc::new(Node {
            _value: 2,
            _next: Some(node1.downgrade()),
        });

        // This creates a cycle, but using Weak prevents memory leak
        assert_eq!(node1.strong_count(), 1);
        assert_eq!(node1.weak_count(), 1);
        assert_eq!(node2.strong_count(), 1);
    }
}
