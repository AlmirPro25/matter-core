//! Code cache for storing JIT compiled native code

use matter_native::runtime::{ExecutableMemory, NativeRuntime};
use std::cmp::Reverse;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;

/// Native function wrapper for JIT compiled code
#[derive(Clone)]
pub struct NativeFunction {
    /// Function name
    pub name: String,

    /// Compiled code size in bytes
    pub size: usize,

    /// Executable memory containing the native code
    pub memory: Arc<ExecutableMemory>,

    /// Compilation timestamp
    pub compiled_at: std::time::Instant,

    /// Number of times this function has been called
    pub call_count: u64,
}

impl NativeFunction {
    /// Create a new native function
    pub fn new(name: String, size: usize, memory: ExecutableMemory) -> Self {
        Self {
            name,
            size,
            memory: Arc::new(memory),
            compiled_at: std::time::Instant::now(),
            call_count: 0,
        }
    }

    /// Get the size of this function in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Record a call to this function
    pub fn record_call(&mut self) {
        self.call_count += 1;
    }

    /// Get age of this function
    pub fn age(&self) -> std::time::Duration {
        self.compiled_at.elapsed()
    }

    /// Execute the native function (returning i64)
    ///
    /// # Safety
    ///
    /// Caller must ensure the compiled code and runtime pointer satisfy the
    /// expected ABI and memory safety contracts.
    pub unsafe fn execute(&self, runtime: &mut NativeRuntime) -> i64 {
        self.memory.execute_i64(runtime)
    }
}

/// LRU cache for JIT compiled native code
pub struct CodeCache {
    /// Map of function name to native function
    cache: HashMap<String, NativeFunction>,

    /// Access order for LRU eviction (most recent at back)
    access_order: VecDeque<String>,

    /// Maximum cache size in bytes
    max_size: usize,

    /// Current cache size in bytes
    current_size: usize,

    /// Number of cache evictions
    eviction_count: u64,

    /// Number of cache hits
    hit_count: u64,

    /// Number of cache misses
    miss_count: u64,
}

impl CodeCache {
    /// Create a new code cache with default size (100MB)
    pub fn new() -> Self {
        Self::with_capacity(100 * 1024 * 1024)
    }

    /// Create a new code cache with specified capacity in bytes
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            access_order: VecDeque::new(),
            max_size,
            current_size: 0,
            eviction_count: 0,
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Insert a native function into the cache
    pub fn insert(&mut self, name: String, function: NativeFunction) -> Result<(), CacheError> {
        let func_size = function.size();

        // Check if function is too large for cache
        if func_size > self.max_size {
            return Err(CacheError::FunctionTooLarge {
                size: func_size,
                max_size: self.max_size,
            });
        }

        // Evict functions until there's enough space
        while self.current_size + func_size > self.max_size {
            if !self.evict_lru() {
                return Err(CacheError::EvictionFailed);
            }
        }

        // Remove old entry if exists
        if let Some(old_func) = self.cache.remove(&name) {
            self.current_size -= old_func.size();
            self.access_order.retain(|n| n != &name);
        }

        // Insert new function
        self.cache.insert(name.clone(), function);
        self.access_order.push_back(name.clone());
        self.current_size += func_size;

        Ok(())
    }

    /// Get a native function from the cache
    pub fn get(&mut self, name: &str) -> Option<&mut NativeFunction> {
        if self.cache.contains_key(name) {
            // Update access order (move to back)
            self.access_order.retain(|n| n != name);
            self.access_order.push_back(name.to_string());

            self.hit_count += 1;

            // Record call and return function
            if let Some(func) = self.cache.get_mut(name) {
                func.record_call();
                Some(func)
            } else {
                // Defensive fallback: if cache key disappeared unexpectedly,
                // treat as a miss instead of panicking.
                self.miss_count += 1;
                None
            }
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// Get the raw code pointer for a cached function.
    pub fn get_code_ptr(&mut self, name: &str) -> Option<*const u8> {
        self.get(name).map(|func| func.memory.code_ptr())
    }

    /// Check if a function is in the cache
    pub fn contains(&self, name: &str) -> bool {
        self.cache.contains_key(name)
    }

    /// Remove a function from the cache
    pub fn remove(&mut self, name: &str) -> Option<NativeFunction> {
        if let Some(func) = self.cache.remove(name) {
            self.current_size -= func.size();
            self.access_order.retain(|n| n != name);
            Some(func)
        } else {
            None
        }
    }

    /// Evict the least recently used function
    fn evict_lru(&mut self) -> bool {
        if let Some(name) = self.access_order.pop_front() {
            if let Some(func) = self.cache.remove(&name) {
                self.current_size -= func.size();
                self.eviction_count += 1;
                return true;
            }
        }
        false
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
        self.current_size = 0;
    }

    /// Get current cache size in bytes
    pub fn size(&self) -> usize {
        self.current_size
    }

    /// Get maximum cache size in bytes
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Get number of functions in cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get cache utilization percentage
    pub fn utilization(&self) -> f64 {
        (self.current_size as f64 / self.max_size as f64) * 100.0
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            (self.hit_count as f64 / total as f64) * 100.0
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.current_size,
            capacity: self.max_size,
            function_count: self.cache.len(),
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            eviction_count: self.eviction_count,
            utilization: self.utilization(),
            hit_rate: self.hit_rate(),
        }
    }

    /// Get list of cached functions sorted by call count
    pub fn hot_functions(&self, limit: usize) -> Vec<(String, u64)> {
        let mut functions: Vec<_> = self
            .cache
            .iter()
            .map(|(name, func)| (name.clone(), func.call_count))
            .collect();

        functions.sort_by_key(|b| Reverse(b.1));
        functions.truncate(limit);
        functions
    }
}

impl Default for CodeCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub function_count: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub utilization: f64,
    pub hit_rate: f64,
}

impl fmt::Display for CacheStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Code Cache Statistics:")?;
        writeln!(
            f,
            "  Size: {} / {} bytes ({:.2}% utilized)",
            self.size, self.capacity, self.utilization
        )?;
        writeln!(f, "  Functions: {}", self.function_count)?;
        writeln!(f, "  Hits: {}", self.hit_count)?;
        writeln!(f, "  Misses: {}", self.miss_count)?;
        writeln!(f, "  Hit Rate: {:.2}%", self.hit_rate)?;
        writeln!(f, "  Evictions: {}", self.eviction_count)?;
        Ok(())
    }
}

/// Cache error types
#[derive(Debug, Clone)]
pub enum CacheError {
    FunctionTooLarge { size: usize, max_size: usize },
    EvictionFailed,
    CacheFull,
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::FunctionTooLarge { size, max_size } => {
                write!(
                    f,
                    "Function size ({} bytes) exceeds cache capacity ({} bytes)",
                    size, max_size
                )
            }
            CacheError::EvictionFailed => write!(f, "Failed to evict function from cache"),
            CacheError::CacheFull => write!(f, "Cache is full and cannot evict more functions"),
        }
    }
}

impl std::error::Error for CacheError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = CodeCache::with_capacity(1000);

        let memory = ExecutableMemory::new(&[0xC3]).unwrap();
        let func = NativeFunction::new("test".to_string(), 1, memory);
        cache.insert("test".to_string(), func).unwrap();

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.size(), 1);
        assert!(cache.contains("test"));

        let retrieved = cache.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = CodeCache::with_capacity(250);

        // Insert 3 functions of 100 bytes each
        cache
            .insert(
                "func1".to_string(),
                NativeFunction::new(
                    "func1".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();
        cache
            .insert(
                "func2".to_string(),
                NativeFunction::new(
                    "func2".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();
        cache
            .insert(
                "func3".to_string(),
                NativeFunction::new(
                    "func3".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();

        assert_eq!(cache.len(), 2); // func1 should be evicted
        assert!(!cache.contains("func1"));
        assert!(cache.contains("func2"));
        assert!(cache.contains("func3"));
    }

    #[test]
    fn test_cache_access_order() {
        let mut cache = CodeCache::with_capacity(250);

        cache
            .insert(
                "func1".to_string(),
                NativeFunction::new(
                    "func1".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();
        cache
            .insert(
                "func2".to_string(),
                NativeFunction::new(
                    "func2".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();

        // Access func1 to make it more recent
        cache.get("func1");

        // Insert func3, should evict func2 (least recently used)
        cache
            .insert(
                "func3".to_string(),
                NativeFunction::new(
                    "func3".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();

        assert!(cache.contains("func1"));
        assert!(!cache.contains("func2"));
        assert!(cache.contains("func3"));
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = CodeCache::with_capacity(1000);

        cache
            .insert(
                "func".to_string(),
                NativeFunction::new(
                    "func".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();

        // 3 hits
        cache.get("func");
        cache.get("func");
        cache.get("func");

        // 1 miss
        cache.get("nonexistent");

        assert_eq!(cache.hit_rate(), 75.0);
    }

    #[test]
    fn test_cache_function_too_large() {
        let mut cache = CodeCache::with_capacity(100);

        let func = NativeFunction::new(
            "huge".to_string(),
            200,
            ExecutableMemory::new(&[0xC3; 200]).unwrap(),
        );
        let result = cache.insert("huge".to_string(), func);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CacheError::FunctionTooLarge { .. }
        ));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = CodeCache::with_capacity(1000);

        cache
            .insert(
                "func1".to_string(),
                NativeFunction::new(
                    "func1".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();
        cache
            .insert(
                "func2".to_string(),
                NativeFunction::new(
                    "func2".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = CodeCache::with_capacity(1000);

        cache
            .insert(
                "func".to_string(),
                NativeFunction::new(
                    "func".to_string(),
                    100,
                    ExecutableMemory::new(&[0xC3; 100]).unwrap(),
                ),
            )
            .unwrap();
        cache.get("func");
        cache.get("nonexistent");

        let stats = cache.stats();
        assert_eq!(stats.size, 100);
        assert_eq!(stats.capacity, 1000);
        assert_eq!(stats.function_count, 1);
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 1);
        assert_eq!(stats.utilization, 10.0);
        assert_eq!(stats.hit_rate, 50.0);
    }
}
