//! JIT compiler for converting bytecode to native code

use crate::{
    cache::{CodeCache, NativeFunction},
    JitError,
};
use matter_bytecode::{Bytecode, Instruction};
use matter_native::{runtime::ExecutableMemory, NativeCompiler};
use std::collections::VecDeque;
use std::time::Instant;

/// JIT compiler for converting bytecode to native code
pub struct JitCompiler {
    /// Code cache for storing compiled functions
    pub code_cache: CodeCache,

    /// Queue of functions waiting to be compiled
    compilation_queue: VecDeque<CompilationRequest>,

    /// Total compilation time
    total_compilation_time: std::time::Duration,

    /// Number of functions compiled
    functions_compiled: u64,

    /// Native compiler for code generation
    native_compiler: NativeCompiler,

    /// Enable verbose logging
    verbose: bool,
}

/// Request to compile a function
#[derive(Debug, Clone)]
struct CompilationRequest {
    name: String,
    bytecode: Vec<Instruction>,
    priority: u32,
}

impl JitCompiler {
    /// Create a new JIT compiler with default cache size
    pub fn new() -> Self {
        Self::with_cache_size(100 * 1024 * 1024) // 100MB default
    }

    /// Create a new JIT compiler with specified cache size
    pub fn with_cache_size(cache_size: usize) -> Self {
        Self {
            code_cache: CodeCache::with_capacity(cache_size),
            compilation_queue: VecDeque::new(),
            total_compilation_time: std::time::Duration::ZERO,
            functions_compiled: 0,
            native_compiler: NativeCompiler::new(),
            verbose: false,
        }
    }

    /// Enable or disable verbose logging
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /// Queue a function for JIT compilation
    pub fn queue_compilation(&mut self, name: String, bytecode: Vec<Instruction>, priority: u32) {
        let request = CompilationRequest {
            name,
            bytecode,
            priority,
        };

        // Insert based on priority (higher priority first)
        let pos = self
            .compilation_queue
            .iter()
            .position(|r| r.priority < priority)
            .unwrap_or(self.compilation_queue.len());

        self.compilation_queue.insert(pos, request);
    }

    /// Compile the next function in the queue
    pub fn compile_next(&mut self) -> Result<Option<String>, JitError> {
        if let Some(request) = self.compilation_queue.pop_front() {
            self.compile_function(&request.name, &request.bytecode)?;
            Ok(Some(request.name))
        } else {
            Ok(None)
        }
    }

    /// Compile all queued functions
    pub fn compile_all(&mut self) -> Result<usize, JitError> {
        let mut count = 0;

        while let Some(request) = self.compilation_queue.pop_front() {
            self.compile_function(&request.name, &request.bytecode)?;
            count += 1;
        }

        Ok(count)
    }

    /// Compile a function immediately
    pub fn compile_function(
        &mut self,
        name: &str,
        bytecode: &[Instruction],
    ) -> Result<(), JitError> {
        if bytecode.is_empty() {
            return Err(JitError::CompilationFailed(
                "Cannot JIT compile empty bytecode".to_string(),
            ));
        }

        let start = Instant::now();

        if self.verbose {
            println!("[JIT] Compiling function: {}", name);
        }

        // Create a temporary Bytecode object for the native compiler
        // In a real JIT, we'd have a more efficient way to pass the IR
        let mut wrapper_bytecode = Bytecode::new();
        wrapper_bytecode.main_instructions = bytecode.to_vec();

        // Step 1: Generate machine code using MNC
        let machine_code = self
            .native_compiler
            .compile(&wrapper_bytecode)
            .map_err(JitError::CompilationFailed)?;

        let size = machine_code.len();

        // Step 2: Allocate executable memory and copy code
        let memory = ExecutableMemory::new(&machine_code).map_err(JitError::CompilationFailed)?;

        // Step 3: Create native function wrapper
        let native_func = NativeFunction::new(name.to_string(), size, memory);

        // Step 4: Insert into cache
        self.code_cache
            .insert(name.to_string(), native_func)
            .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

        let compilation_time = start.elapsed();
        self.total_compilation_time += compilation_time;
        self.functions_compiled += 1;

        if self.verbose {
            println!(
                "[JIT] Compiled {} ({} bytes) in {:?}",
                name, size, compilation_time
            );
        }

        Ok(())
    }

    /// Get compilation statistics
    pub fn stats(&self) -> JitCompilerStats {
        JitCompilerStats {
            functions_compiled: self.functions_compiled,
            total_compilation_time: self.total_compilation_time,
            avg_compilation_time: if self.functions_compiled > 0 {
                self.total_compilation_time / self.functions_compiled as u32
            } else {
                std::time::Duration::ZERO
            },
            queued_functions: self.compilation_queue.len(),
            cache_stats: self.code_cache.stats(),
        }
    }

    /// Clear compilation queue
    pub fn clear_queue(&mut self) {
        self.compilation_queue.clear();
    }

    /// Get number of queued compilations
    pub fn queue_len(&self) -> usize {
        self.compilation_queue.len()
    }

    /// Check if a function is compiled
    pub fn is_compiled(&self, name: &str) -> bool {
        self.code_cache.contains(name)
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// JIT compiler statistics
#[derive(Debug, Clone)]
pub struct JitCompilerStats {
    pub functions_compiled: u64,
    pub total_compilation_time: std::time::Duration,
    pub avg_compilation_time: std::time::Duration,
    pub queued_functions: usize,
    pub cache_stats: crate::cache::CacheStats,
}

impl std::fmt::Display for JitCompilerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "JIT Compiler Statistics:")?;
        writeln!(f, "  Functions Compiled: {}", self.functions_compiled)?;
        writeln!(
            f,
            "  Total Compilation Time: {:?}",
            self.total_compilation_time
        )?;
        writeln!(f, "  Avg Compilation Time: {:?}", self.avg_compilation_time)?;
        writeln!(f, "  Queued Functions: {}", self.queued_functions)?;
        writeln!(f, "\n{}", self.cache_stats)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matter_bytecode::Instruction;

    #[test]
    fn test_jit_compiler_creation() {
        let compiler = JitCompiler::new();
        assert_eq!(compiler.functions_compiled, 0);
        assert_eq!(compiler.queue_len(), 0);
    }

    #[test]
    fn test_jit_compiler_queue() {
        let mut compiler = JitCompiler::new();

        let bytecode = vec![Instruction::Return];

        compiler.queue_compilation("func1".to_string(), bytecode.clone(), 1);
        compiler.queue_compilation("func2".to_string(), bytecode.clone(), 2);
        compiler.queue_compilation("func3".to_string(), bytecode, 1);

        assert_eq!(compiler.queue_len(), 3);

        // Higher priority should be compiled first
        let next = compiler.compile_next().unwrap();
        assert_eq!(next, Some("func2".to_string()));
    }

    #[test]
    fn test_jit_compiler_compile_function() {
        let mut compiler = JitCompiler::new();

        let bytecode = vec![Instruction::Return];

        let result = compiler.compile_function("test_func", &bytecode);
        assert!(result.is_ok());
        assert_eq!(compiler.functions_compiled, 1);
        assert!(compiler.is_compiled("test_func"));
    }

    #[test]
    fn test_jit_compiler_compile_all() {
        let mut compiler = JitCompiler::new();

        let bytecode = vec![Instruction::Return];

        compiler.queue_compilation("func1".to_string(), bytecode.clone(), 1);
        compiler.queue_compilation("func2".to_string(), bytecode.clone(), 1);
        compiler.queue_compilation("func3".to_string(), bytecode, 1);

        let count = compiler.compile_all().unwrap();
        assert_eq!(count, 3);
        assert_eq!(compiler.functions_compiled, 3);
        assert_eq!(compiler.queue_len(), 0);
    }

    #[test]
    fn test_jit_compiler_empty_bytecode() {
        let mut compiler = JitCompiler::new();

        let result = compiler.compile_function("empty", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_jit_compiler_stats() {
        let mut compiler = JitCompiler::new();

        let bytecode = vec![Instruction::Return];
        compiler.compile_function("func", &bytecode).unwrap();

        let stats = compiler.stats();
        assert_eq!(stats.functions_compiled, 1);
        assert!(stats.total_compilation_time > std::time::Duration::ZERO);
    }

    #[test]
    fn test_jit_compiler_clear_queue() {
        let mut compiler = JitCompiler::new();

        let bytecode = vec![Instruction::Return];
        compiler.queue_compilation("func".to_string(), bytecode, 1);

        assert_eq!(compiler.queue_len(), 1);

        compiler.clear_queue();

        assert_eq!(compiler.queue_len(), 0);
    }
}
