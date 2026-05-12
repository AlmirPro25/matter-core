//! Hot path detection for identifying code worth JIT compiling

use crate::profiler::Profiler;
use std::collections::HashSet;

/// Hot path detector for identifying frequently executed code
#[derive(Debug, Clone)]
pub struct HotPathDetector {
    /// Reference to profiler for statistics
    profiler: Profiler,

    /// Set of functions identified as hot
    hot_functions: HashSet<String>,

    /// Set of loops identified as hot
    hot_loops: HashSet<usize>,

    /// Minimum call count to consider a function hot
    function_threshold: u64,

    /// Minimum iteration count to consider a loop hot
    loop_threshold: u64,
}

impl HotPathDetector {
    /// Create a new hot path detector with default thresholds
    pub fn new(profiler: Profiler) -> Self {
        Self {
            profiler,
            hot_functions: HashSet::new(),
            hot_loops: HashSet::new(),
            function_threshold: 1000,
            loop_threshold: 10000,
        }
    }

    /// Create a new hot path detector with custom thresholds
    pub fn with_thresholds(
        profiler: Profiler,
        function_threshold: u64,
        loop_threshold: u64,
    ) -> Self {
        Self {
            profiler,
            hot_functions: HashSet::new(),
            hot_loops: HashSet::new(),
            function_threshold,
            loop_threshold,
        }
    }

    /// Update hot path detection based on current profiler statistics
    pub fn update(&mut self) {
        // Update hot functions
        self.hot_functions.clear();
        for (name, &count) in &self.profiler.function_calls {
            if count >= self.function_threshold {
                self.hot_functions.insert(name.clone());
            }
        }

        // Update hot loops
        self.hot_loops.clear();
        for (&loop_id, &count) in &self.profiler.loop_iterations {
            if count >= self.loop_threshold {
                self.hot_loops.insert(loop_id);
            }
        }
    }

    /// Check if a function is hot
    pub fn is_hot_function(&self, function_name: &str) -> bool {
        self.hot_functions.contains(function_name)
    }

    /// Check if a loop is hot
    pub fn is_hot_loop(&self, loop_id: usize) -> bool {
        self.hot_loops.contains(&loop_id)
    }

    /// Get all hot functions
    pub fn hot_functions(&self) -> &HashSet<String> {
        &self.hot_functions
    }

    /// Get all hot loops
    pub fn hot_loops(&self) -> &HashSet<usize> {
        &self.hot_loops
    }

    /// Get number of hot functions
    pub fn hot_function_count(&self) -> usize {
        self.hot_functions.len()
    }

    /// Get number of hot loops
    pub fn hot_loop_count(&self) -> usize {
        self.hot_loops.len()
    }

    /// Get functions that are approaching hot threshold
    pub fn warming_functions(&self, threshold_percentage: f64) -> Vec<(String, u64, f64)> {
        let min_calls = (self.function_threshold as f64 * threshold_percentage) as u64;

        self.profiler
            .function_calls
            .iter()
            .filter(|(name, &count)| {
                count >= min_calls
                    && count < self.function_threshold
                    && !self.hot_functions.contains(*name)
            })
            .map(|(name, &count)| {
                let percentage = (count as f64 / self.function_threshold as f64) * 100.0;
                (name.clone(), count, percentage)
            })
            .collect()
    }

    /// Get the profiler reference
    pub fn profiler(&self) -> &Profiler {
        &self.profiler
    }

    /// Get mutable profiler reference
    pub fn profiler_mut(&mut self) -> &mut Profiler {
        &mut self.profiler
    }

    /// Reset hot path detection
    pub fn reset(&mut self) {
        self.hot_functions.clear();
        self.hot_loops.clear();
        self.profiler.reset();
    }

    /// Get hot path statistics
    pub fn stats(&self) -> HotPathStats {
        HotPathStats {
            hot_functions: self.hot_function_count(),
            hot_loops: self.hot_loop_count(),
            function_threshold: self.function_threshold,
            loop_threshold: self.loop_threshold,
            total_functions: self.profiler.function_calls.len(),
            total_loops: self.profiler.loop_iterations.len(),
        }
    }
}

/// Hot path detection statistics
#[derive(Debug, Clone)]
pub struct HotPathStats {
    pub hot_functions: usize,
    pub hot_loops: usize,
    pub function_threshold: u64,
    pub loop_threshold: u64,
    pub total_functions: usize,
    pub total_loops: usize,
}

impl std::fmt::Display for HotPathStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Hot Path Detection Statistics:")?;
        writeln!(
            f,
            "  Hot Functions: {} / {}",
            self.hot_functions, self.total_functions
        )?;
        writeln!(f, "  Hot Loops: {} / {}", self.hot_loops, self.total_loops)?;
        writeln!(f, "  Function Threshold: {}", self.function_threshold)?;
        writeln!(f, "  Loop Threshold: {}", self.loop_threshold)?;

        if self.total_functions > 0 {
            let hot_percentage = (self.hot_functions as f64 / self.total_functions as f64) * 100.0;
            writeln!(f, "  Hot Function Percentage: {:.2}%", hot_percentage)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_path_detector_creation() {
        let profiler = Profiler::new();
        let detector = HotPathDetector::new(profiler);

        assert_eq!(detector.hot_function_count(), 0);
        assert_eq!(detector.hot_loop_count(), 0);
    }

    #[test]
    fn test_hot_path_detector_update() {
        let mut profiler = Profiler::with_threshold(100);

        // Simulate hot function
        for _ in 0..1500 {
            profiler.record_call("hot_func");
        }

        // Simulate cold function
        for _ in 0..50 {
            profiler.record_call("cold_func");
        }

        let mut detector = HotPathDetector::with_thresholds(profiler, 1000, 10000);
        detector.update();

        assert!(detector.is_hot_function("hot_func"));
        assert!(!detector.is_hot_function("cold_func"));
        assert_eq!(detector.hot_function_count(), 1);
    }

    #[test]
    fn test_hot_path_detector_loops() {
        let mut profiler = Profiler::new();

        // Simulate hot loop
        for _ in 0..15000 {
            profiler.record_loop_iteration(0);
        }

        // Simulate cold loop
        for _ in 0..5000 {
            profiler.record_loop_iteration(1);
        }

        let mut detector = HotPathDetector::with_thresholds(profiler, 1000, 10000);
        detector.update();

        assert!(detector.is_hot_loop(0));
        assert!(!detector.is_hot_loop(1));
        assert_eq!(detector.hot_loop_count(), 1);
    }

    #[test]
    fn test_warming_functions() {
        let mut profiler = Profiler::new();

        // Function at 80% of threshold
        for _ in 0..800 {
            profiler.record_call("warming_func");
        }

        // Function at 50% of threshold
        for _ in 0..500 {
            profiler.record_call("cool_func");
        }

        let mut detector = HotPathDetector::with_thresholds(profiler, 1000, 10000);
        detector.update();

        let warming = detector.warming_functions(0.7);
        assert_eq!(warming.len(), 1);
        assert_eq!(warming[0].0, "warming_func");
        assert_eq!(warming[0].1, 800);
        assert!((warming[0].2 - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_hot_path_stats() {
        let mut profiler = Profiler::new();

        for _ in 0..1500 {
            profiler.record_call("func1");
        }
        for _ in 0..500 {
            profiler.record_call("func2");
        }

        let mut detector = HotPathDetector::with_thresholds(profiler, 1000, 10000);
        detector.update();

        let stats = detector.stats();
        assert_eq!(stats.hot_functions, 1);
        assert_eq!(stats.total_functions, 2);
        assert_eq!(stats.function_threshold, 1000);
    }

    #[test]
    fn test_hot_path_reset() {
        let mut profiler = Profiler::new();

        for _ in 0..1500 {
            profiler.record_call("func");
        }

        let mut detector = HotPathDetector::with_thresholds(profiler, 1000, 10000);
        detector.update();

        assert_eq!(detector.hot_function_count(), 1);

        detector.reset();

        assert_eq!(detector.hot_function_count(), 0);
        assert_eq!(detector.profiler().total_calls(), 0);
    }
}
