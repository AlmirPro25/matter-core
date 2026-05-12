//! Memory pool (arena allocator) for fast allocation and reduced fragmentation.
//!
//! This module implements arena-based allocation where memory is allocated
//! in large chunks and individual allocations are served from these chunks.

use std::alloc::{alloc, dealloc, Layout};
use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Default chunk size (1MB)
const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Minimum alignment for allocations
const MIN_ALIGN: usize = 8;

/// A chunk of memory in the pool
struct Chunk {
    /// Pointer to the start of the chunk
    data: NonNull<u8>,

    /// Total size of the chunk
    size: usize,

    /// Current offset in the chunk
    offset: usize,

    /// Layout used for allocation
    layout: Layout,
}

impl Chunk {
    /// Create a new chunk with the given size
    fn new(size: usize) -> Result<Self, PoolError> {
        let layout =
            Layout::from_size_align(size, MIN_ALIGN).map_err(|_| PoolError::InvalidLayout)?;

        let data = unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(PoolError::AllocationFailed);
            }
            NonNull::new_unchecked(ptr)
        };

        Ok(Chunk {
            data,
            size,
            offset: 0,
            layout,
        })
    }

    /// Allocate memory from this chunk
    fn allocate(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        // Align the current offset
        let aligned_offset = (self.offset + align - 1) & !(align - 1);

        // Check if we have enough space
        if aligned_offset + size > self.size {
            return None;
        }

        // Calculate pointer
        let ptr = unsafe { NonNull::new_unchecked(self.data.as_ptr().add(aligned_offset)) };

        // Update offset
        self.offset = aligned_offset + size;

        Some(ptr)
    }

    /// Reset this chunk (reuse memory)
    fn reset(&mut self) {
        self.offset = 0;
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.data.as_ptr(), self.layout);
        }
    }
}

/// Memory pool for fast allocation
pub struct MemoryPool {
    /// List of chunks
    chunks: RefCell<Vec<Chunk>>,

    /// Size of each chunk
    chunk_size: usize,

    /// Total bytes allocated
    total_allocated: AtomicUsize,

    /// Total bytes used
    total_used: AtomicUsize,

    /// Number of allocations
    allocation_count: AtomicUsize,
}

impl MemoryPool {
    /// Create a new memory pool with default chunk size
    pub fn new() -> Self {
        Self::with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    /// Create a new memory pool with custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self {
            chunks: RefCell::new(Vec::new()),
            chunk_size,
            total_allocated: AtomicUsize::new(0),
            total_used: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
        }
    }

    /// Allocate memory from the pool
    pub fn allocate(&self, size: usize) -> Result<NonNull<u8>, PoolError> {
        self.allocate_aligned(size, MIN_ALIGN)
    }

    /// Allocate aligned memory from the pool
    pub fn allocate_aligned(&self, size: usize, align: usize) -> Result<NonNull<u8>, PoolError> {
        if size == 0 {
            return Err(PoolError::ZeroSizeAllocation);
        }

        if size > self.chunk_size {
            return Err(PoolError::AllocationTooLarge(size, self.chunk_size));
        }

        let mut chunks = self.chunks.borrow_mut();

        // Try to allocate from existing chunks
        for chunk in chunks.iter_mut().rev() {
            if let Some(ptr) = chunk.allocate(size, align) {
                self.total_used.fetch_add(size, Ordering::Relaxed);
                self.allocation_count.fetch_add(1, Ordering::Relaxed);
                return Ok(ptr);
            }
        }

        // Need a new chunk
        let mut new_chunk = Chunk::new(self.chunk_size)?;
        let ptr = new_chunk
            .allocate(size, align)
            .ok_or(PoolError::AllocationFailed)?;

        self.total_allocated
            .fetch_add(self.chunk_size, Ordering::Relaxed);
        self.total_used.fetch_add(size, Ordering::Relaxed);
        self.allocation_count.fetch_add(1, Ordering::Relaxed);

        chunks.push(new_chunk);

        Ok(ptr)
    }

    /// Reset the pool (reuse all chunks)
    pub fn reset(&self) {
        let mut chunks = self.chunks.borrow_mut();
        for chunk in chunks.iter_mut() {
            chunk.reset();
        }
        self.total_used.store(0, Ordering::Relaxed);
        self.allocation_count.store(0, Ordering::Relaxed);
    }

    /// Clear the pool (deallocate all chunks)
    pub fn clear(&self) {
        let mut chunks = self.chunks.borrow_mut();
        chunks.clear();
        self.total_allocated.store(0, Ordering::Relaxed);
        self.total_used.store(0, Ordering::Relaxed);
        self.allocation_count.store(0, Ordering::Relaxed);
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let chunks = self.chunks.borrow();

        PoolStats {
            chunk_count: chunks.len(),
            chunk_size: self.chunk_size,
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            total_used: self.total_used.load(Ordering::Relaxed),
            allocation_count: self.allocation_count.load(Ordering::Relaxed),
            fragmentation: self.calculate_fragmentation(&chunks),
        }
    }

    /// Calculate fragmentation percentage
    fn calculate_fragmentation(&self, chunks: &[Chunk]) -> f64 {
        if chunks.is_empty() {
            return 0.0;
        }

        let total_allocated = self.total_allocated.load(Ordering::Relaxed);
        let total_used = self.total_used.load(Ordering::Relaxed);

        if total_allocated == 0 {
            return 0.0;
        }

        let wasted = total_allocated - total_used;
        (wasted as f64 / total_allocated as f64) * 100.0
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of chunks
    pub chunk_count: usize,

    /// Size of each chunk
    pub chunk_size: usize,

    /// Total bytes allocated
    pub total_allocated: usize,

    /// Total bytes used
    pub total_used: usize,

    /// Number of allocations
    pub allocation_count: usize,

    /// Fragmentation percentage
    pub fragmentation: f64,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Memory Pool Statistics:")?;
        writeln!(f, "  Chunks:           {}", self.chunk_count)?;
        writeln!(f, "  Chunk size:       {} bytes", self.chunk_size)?;
        writeln!(f, "  Total allocated:  {} bytes", self.total_allocated)?;
        writeln!(f, "  Total used:       {} bytes", self.total_used)?;
        writeln!(f, "  Allocations:      {}", self.allocation_count)?;
        writeln!(f, "  Fragmentation:    {:.2}%", self.fragmentation)?;
        writeln!(
            f,
            "  Efficiency:       {:.2}%",
            if self.total_allocated > 0 {
                (self.total_used as f64 / self.total_allocated as f64) * 100.0
            } else {
                0.0
            }
        )?;
        Ok(())
    }
}

/// Memory pool errors
#[derive(Debug, Clone)]
pub enum PoolError {
    /// Allocation failed
    AllocationFailed,

    /// Invalid layout
    InvalidLayout,

    /// Zero size allocation
    ZeroSizeAllocation,

    /// Allocation too large for chunk
    AllocationTooLarge(usize, usize),
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::AllocationFailed => write!(f, "Memory allocation failed"),
            PoolError::InvalidLayout => write!(f, "Invalid memory layout"),
            PoolError::ZeroSizeAllocation => write!(f, "Cannot allocate zero bytes"),
            PoolError::AllocationTooLarge(size, chunk_size) => {
                write!(
                    f,
                    "Allocation of {} bytes exceeds chunk size of {} bytes",
                    size, chunk_size
                )
            }
        }
    }
}

impl std::error::Error for PoolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = MemoryPool::new();
        let stats = pool.stats();
        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.total_allocated, 0);
        assert_eq!(stats.total_used, 0);
    }

    #[test]
    fn test_simple_allocation() {
        let pool = MemoryPool::new();
        let _ptr = pool.allocate(100).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.chunk_count, 1);
        assert_eq!(stats.total_used, 100);
        assert_eq!(stats.allocation_count, 1);
    }

    #[test]
    fn test_multiple_allocations() {
        let pool = MemoryPool::new();

        for _ in 0..10 {
            pool.allocate(100).unwrap();
        }

        let stats = pool.stats();
        assert_eq!(stats.chunk_count, 1);
        assert_eq!(stats.total_used, 1000);
        assert_eq!(stats.allocation_count, 10);
    }

    #[test]
    fn test_chunk_overflow() {
        let pool = MemoryPool::with_chunk_size(1024);

        // Allocate more than one chunk
        pool.allocate(600).unwrap();
        pool.allocate(600).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.chunk_count, 2);
        assert_eq!(stats.total_used, 1200);
    }

    #[test]
    fn test_reset() {
        let pool = MemoryPool::new();

        pool.allocate(100).unwrap();
        pool.allocate(200).unwrap();

        let stats_before = pool.stats();
        assert_eq!(stats_before.total_used, 300);

        pool.reset();

        let stats_after = pool.stats();
        assert_eq!(stats_after.chunk_count, 1); // Chunks still exist
        assert_eq!(stats_after.total_used, 0); // But usage is reset
        assert_eq!(stats_after.allocation_count, 0);
    }

    #[test]
    fn test_clear() {
        let pool = MemoryPool::new();

        pool.allocate(100).unwrap();
        pool.allocate(200).unwrap();

        pool.clear();

        let stats = pool.stats();
        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.total_allocated, 0);
        assert_eq!(stats.total_used, 0);
    }

    #[test]
    fn test_zero_size_allocation() {
        let pool = MemoryPool::new();
        let result = pool.allocate(0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PoolError::ZeroSizeAllocation));
    }

    #[test]
    fn test_allocation_too_large() {
        let pool = MemoryPool::with_chunk_size(1024);
        let result = pool.allocate(2048);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PoolError::AllocationTooLarge(_, _)
        ));
    }

    #[test]
    fn test_aligned_allocation() {
        let pool = MemoryPool::new();
        let ptr = pool.allocate_aligned(100, 16).unwrap();

        // Check alignment
        let addr = ptr.as_ptr() as usize;
        assert_eq!(addr % 16, 0);
    }

    #[test]
    fn test_fragmentation() {
        let pool = MemoryPool::with_chunk_size(1024);

        // Allocate small amount
        pool.allocate(100).unwrap();

        let stats = pool.stats();
        // Fragmentation should be high (only 100 bytes used out of 1024)
        assert!(stats.fragmentation > 80.0);
    }

    #[test]
    fn test_stats_display() {
        let pool = MemoryPool::new();
        pool.allocate(100).unwrap();

        let stats = pool.stats();
        let display = format!("{}", stats);
        assert!(display.contains("Memory Pool Statistics"));
        assert!(display.contains("Chunks"));
    }
}
