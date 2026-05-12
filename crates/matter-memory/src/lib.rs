//! Matter Memory - Memory Management System
//!
//! This crate provides memory management for Matter Core using
//! reference counting with automatic cycle detection and memory pools.

pub mod cycle;
pub mod pool;
pub mod rc;
pub mod stats;

pub use cycle::{
    next_object_id, CycleDetectionResult, CycleDetector, CycleDetectorStats, Traceable,
};
pub use pool::{MemoryPool, PoolError, PoolStats};
pub use rc::{Rc, Weak};
pub use stats::MemoryStats;

use std::fmt;

/// Memory management error types
#[derive(Debug, Clone)]
pub enum MemoryError {
    AllocationFailed(String),
    InvalidReference,
    CycleDetected,
    OutOfMemory,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::AllocationFailed(msg) => write!(f, "Allocation failed: {}", msg),
            MemoryError::InvalidReference => write!(f, "Invalid reference"),
            MemoryError::CycleDetected => write!(f, "Cycle detected in object graph"),
            MemoryError::OutOfMemory => write!(f, "Out of memory"),
        }
    }
}

impl std::error::Error for MemoryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_error_display() {
        let error = MemoryError::AllocationFailed("test".to_string());
        assert_eq!(error.to_string(), "Allocation failed: test");
    }
}
