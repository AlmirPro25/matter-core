//! Memory statistics and monitoring

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global memory statistics
static TOTAL_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static TOTAL_DEALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_USAGE: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Record an allocation
pub fn record_allocation(size: usize) {
    TOTAL_ALLOCATED.fetch_add(size, Ordering::Relaxed);
    ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);

    // Update peak usage
    let current = current_usage();
    let mut peak = PEAK_USAGE.load(Ordering::Relaxed);
    while current > peak {
        match PEAK_USAGE.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => break,
            Err(x) => peak = x,
        }
    }
}

/// Record a deallocation
pub fn record_deallocation(size: usize) {
    TOTAL_DEALLOCATED.fetch_add(size, Ordering::Relaxed);
    DEALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Get current memory usage
pub fn current_usage() -> usize {
    TOTAL_ALLOCATED.load(Ordering::Relaxed) - TOTAL_DEALLOCATED.load(Ordering::Relaxed)
}

/// Get peak memory usage
pub fn peak_usage() -> usize {
    PEAK_USAGE.load(Ordering::Relaxed)
}

/// Reset all statistics
pub fn reset_stats() {
    TOTAL_ALLOCATED.store(0, Ordering::Relaxed);
    TOTAL_DEALLOCATED.store(0, Ordering::Relaxed);
    PEAK_USAGE.store(0, Ordering::Relaxed);
    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    DEALLOCATION_COUNT.store(0, Ordering::Relaxed);
}

/// Memory statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub total_deallocated: usize,
    pub current_usage: usize,
    pub peak_usage: usize,
    pub allocation_count: usize,
    pub deallocation_count: usize,
}

impl MemoryStats {
    /// Get current memory statistics
    pub fn current() -> Self {
        Self {
            total_allocated: TOTAL_ALLOCATED.load(Ordering::Relaxed),
            total_deallocated: TOTAL_DEALLOCATED.load(Ordering::Relaxed),
            current_usage: current_usage(),
            peak_usage: peak_usage(),
            allocation_count: ALLOCATION_COUNT.load(Ordering::Relaxed),
            deallocation_count: DEALLOCATION_COUNT.load(Ordering::Relaxed),
        }
    }

    /// Check for potential memory leaks
    pub fn has_leak(&self) -> bool {
        // If current usage is more than 50% of total allocated, might be a leak
        self.current_usage > self.total_allocated / 2
    }

    /// Get average allocation size
    pub fn avg_allocation_size(&self) -> f64 {
        if self.allocation_count == 0 {
            0.0
        } else {
            self.total_allocated as f64 / self.allocation_count as f64
        }
    }

    /// Get average deallocation size
    pub fn avg_deallocation_size(&self) -> f64 {
        if self.deallocation_count == 0 {
            0.0
        } else {
            self.total_deallocated as f64 / self.deallocation_count as f64
        }
    }

    /// Get memory efficiency (deallocated / allocated)
    pub fn efficiency(&self) -> f64 {
        if self.total_allocated == 0 {
            0.0
        } else {
            (self.total_deallocated as f64 / self.total_allocated as f64) * 100.0
        }
    }
}

impl fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Memory Statistics:")?;
        writeln!(
            f,
            "  Total Allocated: {} bytes ({} allocations)",
            self.total_allocated, self.allocation_count
        )?;
        writeln!(
            f,
            "  Total Deallocated: {} bytes ({} deallocations)",
            self.total_deallocated, self.deallocation_count
        )?;
        writeln!(f, "  Current Usage: {} bytes", self.current_usage)?;
        writeln!(f, "  Peak Usage: {} bytes", self.peak_usage)?;
        writeln!(
            f,
            "  Avg Allocation Size: {:.2} bytes",
            self.avg_allocation_size()
        )?;
        writeln!(
            f,
            "  Avg Deallocation Size: {:.2} bytes",
            self.avg_deallocation_size()
        )?;
        writeln!(f, "  Memory Efficiency: {:.2}%", self.efficiency())?;

        if self.has_leak() {
            writeln!(f, "  ⚠️  WARNING: Potential memory leak detected!")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_allocation() {
        reset_stats();

        record_allocation(100);
        record_allocation(200);

        assert_eq!(TOTAL_ALLOCATED.load(Ordering::Relaxed), 300);
        assert_eq!(ALLOCATION_COUNT.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_record_deallocation() {
        reset_stats();

        record_allocation(100);
        record_deallocation(50);

        assert_eq!(current_usage(), 50);
        assert_eq!(DEALLOCATION_COUNT.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_peak_usage() {
        reset_stats();

        record_allocation(100);
        assert_eq!(peak_usage(), 100);

        record_allocation(200);
        assert_eq!(peak_usage(), 300);

        record_deallocation(150);
        assert_eq!(peak_usage(), 300); // Peak doesn't decrease
        assert_eq!(current_usage(), 150);
    }

    #[test]
    fn test_memory_stats() {
        reset_stats();

        record_allocation(1000);
        record_allocation(2000);
        record_deallocation(500);

        let stats = MemoryStats::current();

        assert_eq!(stats.total_allocated, 3000);
        assert_eq!(stats.total_deallocated, 500);
        assert_eq!(stats.current_usage, 2500);
        assert_eq!(stats.allocation_count, 2);
        assert_eq!(stats.deallocation_count, 1);
    }

    #[test]
    fn test_avg_allocation_size() {
        reset_stats();

        record_allocation(100);
        record_allocation(200);
        record_allocation(300);

        let stats = MemoryStats::current();
        assert_eq!(stats.avg_allocation_size(), 200.0);
    }

    #[test]
    fn test_efficiency() {
        reset_stats();

        record_allocation(1000);
        record_deallocation(800);

        let stats = MemoryStats::current();
        assert_eq!(stats.efficiency(), 80.0);
    }

    #[test]
    fn test_leak_detection() {
        reset_stats();

        record_allocation(1000);
        record_deallocation(100);

        let stats = MemoryStats::current();
        assert!(stats.has_leak()); // 900/1000 > 50%
    }

    #[test]
    fn test_no_leak() {
        reset_stats();

        record_allocation(1000);
        record_deallocation(600);

        let stats = MemoryStats::current();
        assert!(!stats.has_leak()); // 400/1000 < 50%
    }

    #[test]
    fn test_reset_stats() {
        record_allocation(1000);
        record_deallocation(500);

        reset_stats();

        assert_eq!(current_usage(), 0);
        assert_eq!(peak_usage(), 0);
        assert_eq!(ALLOCATION_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(DEALLOCATION_COUNT.load(Ordering::Relaxed), 0);
    }
}
