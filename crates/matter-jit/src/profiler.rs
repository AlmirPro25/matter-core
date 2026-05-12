//! Runtime profiler for collecting execution statistics

use std::cmp::Reverse;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Runtime profiler for collecting execution statistics
#[derive(Debug, Clone)]
pub struct Profiler {
    /// Number of times each function has been called
    pub function_calls: HashMap<String, u64>,

    /// Number of iterations for each loop
    pub loop_iterations: HashMap<usize, u64>,

    /// Cumulative execution time for each function
    pub execution_times: HashMap<String, Duration>,

    /// Threshold for considering a function "hot"
    pub hot_threshold: u64,

    /// Start times for functions currently executing
    function_start_times: HashMap<String, Instant>,

    /// Total profiling overhead
    overhead: Duration,
}

impl Profiler {
    /// Create a new profiler with default hot threshold (1000 calls)
    pub fn new() -> Self {
        Self::with_threshold(1000)
    }

    /// Create a new profiler with custom hot threshold
    pub fn with_threshold(hot_threshold: u64) -> Self {
        Self {
            function_calls: HashMap::new(),
            loop_iterations: HashMap::new(),
            execution_times: HashMap::new(),
            hot_threshold,
            function_start_times: HashMap::new(),
            overhead: Duration::ZERO,
        }
    }

    /// Record a function call
    pub fn record_call(&mut self, function_name: &str) {
        let start = Instant::now();

        self.record_call_count(function_name);
        self.function_start_times
            .insert(function_name.to_string(), Instant::now());

        self.overhead += start.elapsed();
    }

    /// Record only the call count, without timing bookkeeping.
    pub fn record_call_count(&mut self, function_name: &str) -> u64 {
        if let Some(count) = self.function_calls.get_mut(function_name) {
            *count += 1;
            *count
        } else {
            self.function_calls.insert(function_name.to_string(), 1);
            1
        }
    }

    /// Record function return (to measure execution time)
    pub fn record_return(&mut self, function_name: &str) {
        let start = Instant::now();

        if let Some(start_time) = self.function_start_times.remove(function_name) {
            let execution_time = start_time.elapsed();
            *self
                .execution_times
                .entry(function_name.to_string())
                .or_insert(Duration::ZERO) += execution_time;
        }

        self.overhead += start.elapsed();
    }

    /// Record loop iteration
    pub fn record_loop_iteration(&mut self, loop_id: usize) {
        let start = Instant::now();

        *self.loop_iterations.entry(loop_id).or_insert(0) += 1;

        self.overhead += start.elapsed();
    }

    /// Check if a function is "hot" (called frequently)
    pub fn is_hot_function(&self, function_name: &str) -> bool {
        self.function_calls
            .get(function_name)
            .map(|&count| count >= self.hot_threshold)
            .unwrap_or(false)
    }

    /// Check if a loop is "hot" (many iterations)
    pub fn is_hot_loop(&self, loop_id: usize) -> bool {
        self.loop_iterations
            .get(&loop_id)
            .map(|&count| count >= self.hot_threshold * 10)
            .unwrap_or(false)
    }

    /// Get the top N hottest functions by call count
    pub fn top_functions_by_calls(&self, n: usize) -> Vec<(String, u64)> {
        let mut functions: Vec<_> = self
            .function_calls
            .iter()
            .map(|(name, &count)| (name.clone(), count))
            .collect();

        functions.sort_by_key(|b| Reverse(b.1));
        functions.truncate(n);
        functions
    }

    /// Get the top N hottest functions by execution time
    pub fn top_functions_by_time(&self, n: usize) -> Vec<(String, Duration)> {
        let mut functions: Vec<_> = self
            .execution_times
            .iter()
            .map(|(name, &time)| (name.clone(), time))
            .collect();

        functions.sort_by_key(|b| Reverse(b.1));
        functions.truncate(n);
        functions
    }

    /// Get total number of function calls
    pub fn total_calls(&self) -> u64 {
        self.function_calls.values().sum()
    }

    /// Get total execution time
    pub fn total_execution_time(&self) -> Duration {
        self.execution_times.values().sum()
    }

    /// Get profiling overhead
    pub fn overhead(&self) -> Duration {
        self.overhead
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.function_calls.clear();
        self.loop_iterations.clear();
        self.execution_times.clear();
        self.function_start_times.clear();
        self.overhead = Duration::ZERO;
    }

    /// Get statistics summary
    pub fn summary(&self) -> ProfilerSummary {
        ProfilerSummary {
            total_calls: self.total_calls(),
            total_execution_time: self.total_execution_time(),
            overhead: self.overhead,
            hot_functions: self
                .function_calls
                .iter()
                .filter(|(_, &count)| count >= self.hot_threshold)
                .count(),
            hot_loops: self
                .loop_iterations
                .iter()
                .filter(|(_, &count)| count >= self.hot_threshold * 10)
                .count(),
        }
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of profiler statistics
#[derive(Debug, Clone)]
pub struct ProfilerSummary {
    pub total_calls: u64,
    pub total_execution_time: Duration,
    pub overhead: Duration,
    pub hot_functions: usize,
    pub hot_loops: usize,
}

impl std::fmt::Display for ProfilerSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Profiler Summary:")?;
        writeln!(f, "  Total Function Calls: {}", self.total_calls)?;
        writeln!(f, "  Total Execution Time: {:?}", self.total_execution_time)?;
        writeln!(f, "  Profiling Overhead: {:?}", self.overhead)?;
        writeln!(f, "  Hot Functions: {}", self.hot_functions)?;
        writeln!(f, "  Hot Loops: {}", self.hot_loops)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_record_call() {
        let mut profiler = Profiler::new();

        profiler.record_call("test_function");
        profiler.record_call("test_function");
        profiler.record_call("other_function");

        assert_eq!(profiler.function_calls.get("test_function"), Some(&2));
        assert_eq!(profiler.function_calls.get("other_function"), Some(&1));
    }

    #[test]
    fn test_profiler_is_hot_function() {
        let mut profiler = Profiler::with_threshold(5);

        for _ in 0..10 {
            profiler.record_call("hot_function");
        }

        for _ in 0..3 {
            profiler.record_call("cold_function");
        }

        assert!(profiler.is_hot_function("hot_function"));
        assert!(!profiler.is_hot_function("cold_function"));
    }

    #[test]
    fn test_profiler_execution_time() {
        let mut profiler = Profiler::new();

        profiler.record_call("test_function");
        thread::sleep(Duration::from_millis(10));
        profiler.record_return("test_function");

        let time = profiler.execution_times.get("test_function").unwrap();
        assert!(time.as_millis() >= 10);
    }

    #[test]
    fn test_profiler_top_functions() {
        let mut profiler = Profiler::new();

        for _ in 0..100 {
            profiler.record_call("func1");
        }
        for _ in 0..50 {
            profiler.record_call("func2");
        }
        for _ in 0..25 {
            profiler.record_call("func3");
        }

        let top = profiler.top_functions_by_calls(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "func1");
        assert_eq!(top[0].1, 100);
        assert_eq!(top[1].0, "func2");
        assert_eq!(top[1].1, 50);
    }

    #[test]
    fn test_profiler_reset() {
        let mut profiler = Profiler::new();

        profiler.record_call("test");
        profiler.record_loop_iteration(0);

        assert_eq!(profiler.total_calls(), 1);

        profiler.reset();

        assert_eq!(profiler.total_calls(), 0);
        assert_eq!(profiler.function_calls.len(), 0);
        assert_eq!(profiler.loop_iterations.len(), 0);
    }

    #[test]
    fn test_profiler_summary() {
        let mut profiler = Profiler::with_threshold(2);

        for _ in 0..5 {
            profiler.record_call("hot_func");
        }
        profiler.record_call("cold_func");

        let summary = profiler.summary();
        assert_eq!(summary.total_calls, 6);
        assert_eq!(summary.hot_functions, 1);
    }
}
