//! Matter Core Benchmark Suite
//!
//! Provides comprehensive performance benchmarking for Matter Core.

use std::time::{Duration, Instant};

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration: Duration,
    pub iterations: u64,
    pub ops_per_sec: f64,
    pub memory_bytes: usize,
}

impl BenchmarkResult {
    pub fn new(name: String, duration: Duration, iterations: u64, memory_bytes: usize) -> Self {
        let ops_per_sec = if duration.as_secs_f64() > 0.0 {
            iterations as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        Self {
            name,
            duration,
            iterations,
            ops_per_sec,
            memory_bytes,
        }
    }

    pub fn format_duration(&self) -> String {
        let ms = self.duration.as_millis();
        if ms < 1000 {
            format!("{}ms", ms)
        } else {
            format!("{:.2}s", self.duration.as_secs_f64())
        }
    }

    pub fn format_ops_per_sec(&self) -> String {
        if self.ops_per_sec >= 1_000_000.0 {
            format!("{:.2}M ops/sec", self.ops_per_sec / 1_000_000.0)
        } else if self.ops_per_sec >= 1_000.0 {
            format!("{:.2}K ops/sec", self.ops_per_sec / 1_000.0)
        } else {
            format!("{:.0} ops/sec", self.ops_per_sec)
        }
    }

    pub fn format_memory(&self) -> String {
        if self.memory_bytes >= 1_000_000 {
            format!("{:.2}MB", self.memory_bytes as f64 / 1_000_000.0)
        } else if self.memory_bytes >= 1_000 {
            format!("{:.2}KB", self.memory_bytes as f64 / 1_000.0)
        } else {
            format!("{}B", self.memory_bytes)
        }
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    results: Vec<BenchmarkResult>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Run a benchmark
    pub fn bench<F>(&mut self, name: &str, iterations: u64, f: F)
    where
        F: Fn(),
    {
        // Warm up
        for _ in 0..10 {
            f();
        }

        // Measure
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        let duration = start.elapsed();

        // Estimate memory (simplified)
        let memory_bytes = std::mem::size_of_val(&f) * iterations as usize;

        let result = BenchmarkResult::new(name.to_string(), duration, iterations, memory_bytes);

        println!(
            "✓ {} - {} ({}, {})",
            result.name,
            result.format_duration(),
            result.format_ops_per_sec(),
            result.format_memory()
        );

        self.results.push(result);
    }

    /// Get all results
    pub fn results(&self) -> &[BenchmarkResult] {
        &self.results
    }

    /// Print summary
    pub fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║              Matter Core Benchmark Summary                 ║");
        println!("╠════════════════════════════════════════════════════════════╣");

        for result in &self.results {
            println!(
                "║ {:30} │ {:10} │ {:15} ║",
                truncate(&result.name, 30),
                result.format_duration(),
                result.format_ops_per_sec()
            );
        }

        println!("╚════════════════════════════════════════════════════════════╝");
    }

    /// Export results as JSON
    pub fn export_json(&self) -> String {
        let mut json = String::from("[\n");
        for (i, result) in self.results.iter().enumerate() {
            json.push_str(&format!(
                "  {{\n    \"name\": \"{}\",\n    \"duration_ms\": {},\n    \"ops_per_sec\": {:.2},\n    \"memory_bytes\": {}\n  }}",
                result.name,
                result.duration.as_millis(),
                result.ops_per_sec,
                result.memory_bytes
            ));
            if i < self.results.len() - 1 {
                json.push_str(",\n");
            } else {
                json.push('\n');
            }
        }
        json.push_str("]\n");
        json
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:width$}", s, width = max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result() {
        let result =
            BenchmarkResult::new("test".to_string(), Duration::from_millis(100), 1000, 1024);

        assert_eq!(result.name, "test");
        assert_eq!(result.iterations, 1000);
        assert!(result.ops_per_sec > 0.0);
    }

    #[test]
    fn test_benchmark_runner() {
        let mut runner = BenchmarkRunner::new();

        runner.bench("simple", 100, || {
            let _ = 1 + 1;
        });

        assert_eq!(runner.results().len(), 1);
        assert_eq!(runner.results()[0].name, "simple");
    }

    #[test]
    fn test_format_duration() {
        let result =
            BenchmarkResult::new("test".to_string(), Duration::from_millis(500), 1000, 1024);

        assert_eq!(result.format_duration(), "500ms");
    }

    #[test]
    fn test_format_ops_per_sec() {
        let result = BenchmarkResult::new(
            "test".to_string(),
            Duration::from_millis(100),
            1_000_000,
            1024,
        );

        assert!(result.format_ops_per_sec().contains("M ops/sec"));
    }

    #[test]
    fn test_export_json() {
        let mut runner = BenchmarkRunner::new();
        runner.bench("test", 100, || {
            let _ = 1 + 1;
        });

        let json = runner.export_json();
        assert!(json.contains("\"name\": \"test\""));
        assert!(json.contains("\"duration_ms\""));
        assert!(json.contains("\"ops_per_sec\""));
    }
}
