//! Matter JIT - Just-In-Time Compilation Infrastructure
//!
//! This crate provides JIT compilation capabilities for Matter Core,
//! enabling dynamic optimization of hot code paths during runtime.

pub mod cache;
pub mod compiler;
pub mod hot_path;
pub mod profiler;

pub use cache::CodeCache;
pub use compiler::JitCompiler;
pub use hot_path::HotPathDetector;
pub use profiler::Profiler;

use std::fmt;

/// JIT compilation error types
#[derive(Debug, Clone)]
pub enum JitError {
    CompilationFailed(String),
    CacheFull,
    InvalidBytecode(String),
    LLVMError(String),
    DecompilationFailed(String),
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JitError::CompilationFailed(msg) => write!(f, "JIT compilation failed: {}", msg),
            JitError::CacheFull => write!(f, "JIT code cache is full"),
            JitError::InvalidBytecode(msg) => write!(f, "Invalid bytecode: {}", msg),
            JitError::LLVMError(msg) => write!(f, "LLVM error: {}", msg),
            JitError::DecompilationFailed(msg) => write!(f, "Decompilation failed: {}", msg),
        }
    }
}

impl std::error::Error for JitError {}

/// JIT compilation statistics
#[derive(Debug, Clone, Default)]
pub struct JitStats {
    pub functions_compiled: u64,
    pub compilation_time_ms: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub total_code_size: usize,
    pub hot_functions: usize,
}

impl JitStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.cache_hits as f64) / (total as f64) * 100.0
        }
    }

    pub fn avg_compilation_time(&self) -> f64 {
        if self.functions_compiled == 0 {
            0.0
        } else {
            (self.compilation_time_ms as f64) / (self.functions_compiled as f64)
        }
    }
}

impl fmt::Display for JitStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "JIT Compilation Statistics:")?;
        writeln!(f, "  Functions Compiled: {}", self.functions_compiled)?;
        writeln!(
            f,
            "  Total Compilation Time: {}ms",
            self.compilation_time_ms
        )?;
        writeln!(
            f,
            "  Avg Compilation Time: {:.2}ms",
            self.avg_compilation_time()
        )?;
        writeln!(f, "  Cache Hits: {}", self.cache_hits)?;
        writeln!(f, "  Cache Misses: {}", self.cache_misses)?;
        writeln!(f, "  Cache Hit Rate: {:.2}%", self.hit_rate())?;
        writeln!(f, "  Cache Evictions: {}", self.cache_evictions)?;
        writeln!(f, "  Total Code Size: {} bytes", self.total_code_size)?;
        writeln!(f, "  Hot Functions: {}", self.hot_functions)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_stats_hit_rate() {
        let mut stats = JitStats::new();
        stats.cache_hits = 80;
        stats.cache_misses = 20;
        assert_eq!(stats.hit_rate(), 80.0);
    }

    #[test]
    fn test_jit_stats_avg_compilation_time() {
        let mut stats = JitStats::new();
        stats.functions_compiled = 10;
        stats.compilation_time_ms = 500;
        assert_eq!(stats.avg_compilation_time(), 50.0);
    }

    #[test]
    fn test_jit_error_display() {
        let error = JitError::CompilationFailed("test error".to_string());
        assert_eq!(error.to_string(), "JIT compilation failed: test error");
    }
}
