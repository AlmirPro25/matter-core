# Sprint 20: JIT Compilation Foundation

## Overview
Implement the foundation for Just-In-Time (JIT) compilation to enable dynamic optimization of hot code paths during runtime.

## Status: 🚧 IN PROGRESS

## Objectives

### Primary Goals
1. **Hot Path Detection** - Identify frequently executed code
2. **Profiling Infrastructure** - Collect runtime statistics
3. **Optimization Triggers** - Decide when to JIT compile
4. **Code Cache** - Store compiled native code
5. **Fallback Mechanism** - Graceful degradation to bytecode

### Performance Targets
- **Startup**: No degradation (JIT is optional)
- **Steady State**: 5-10x improvement on hot paths
- **Memory**: <10MB overhead for JIT cache
- **Compilation Time**: <100ms per function

## Architecture

### JIT Pipeline
```
Bytecode → Profiler → Hot Path Detection → JIT Compiler → Native Code → Cache
                                                ↓
                                          Execution
                                                ↓
                                          Fallback to Bytecode (if needed)
```

### Components

#### 1. Profiler
**Purpose**: Collect execution statistics

**Metrics**:
- Function call count
- Loop iteration count
- Execution time per function
- Branch prediction data

**Implementation**:
```rust
pub struct Profiler {
    function_calls: HashMap<String, u64>,
    loop_iterations: HashMap<usize, u64>,
    execution_times: HashMap<String, Duration>,
    hot_threshold: u64,
}
```

#### 2. Hot Path Detector
**Purpose**: Identify code worth JIT compiling

**Heuristics**:
- Function called >1000 times
- Loop executed >10000 iterations
- Function execution time >10ms cumulative
- Recursive functions with depth >5

**Implementation**:
```rust
pub struct HotPathDetector {
    profiler: Profiler,
    hot_functions: HashSet<String>,
    hot_loops: HashSet<usize>,
}

impl HotPathDetector {
    pub fn is_hot(&self, function: &str) -> bool {
        self.profiler.function_calls.get(function)
            .map(|&count| count > self.profiler.hot_threshold)
            .unwrap_or(false)
    }
}
```

#### 3. JIT Compiler
**Purpose**: Compile bytecode to native code

**Strategy**: Use LLVM backend (already implemented in matter-llvm)

**Workflow**:
1. Extract bytecode for hot function
2. Decompile to AST (or use cached AST)
3. Compile to native code via LLVM
4. Store in code cache
5. Patch VM to call native code

**Implementation**:
```rust
pub struct JitCompiler {
    llvm_backend: LLVMCodegen,
    code_cache: CodeCache,
    compilation_queue: VecDeque<String>,
}

impl JitCompiler {
    pub fn compile_function(&mut self, name: &str, bytecode: &[Instruction]) -> Result<NativeFunction, JitError> {
        // 1. Convert bytecode to AST (if needed)
        let ast = self.decompile_bytecode(bytecode)?;
        
        // 2. Compile to native code
        let native_code = self.llvm_backend.compile_function(&ast)?;
        
        // 3. Cache the result
        self.code_cache.insert(name, native_code.clone());
        
        Ok(native_code)
    }
}
```

#### 4. Code Cache
**Purpose**: Store and manage compiled native code

**Features**:
- LRU eviction policy
- Size limits (configurable)
- Invalidation on code changes
- Persistence across runs (optional)

**Implementation**:
```rust
pub struct CodeCache {
    cache: HashMap<String, NativeFunction>,
    access_order: VecDeque<String>,
    max_size: usize,
    current_size: usize,
}

impl CodeCache {
    pub fn insert(&mut self, name: String, function: NativeFunction) {
        // Evict if necessary
        while self.current_size + function.size() > self.max_size {
            self.evict_lru();
        }
        
        self.cache.insert(name.clone(), function);
        self.access_order.push_back(name);
        self.current_size += function.size();
    }
    
    pub fn get(&mut self, name: &str) -> Option<&NativeFunction> {
        if let Some(func) = self.cache.get(name) {
            // Update access order
            self.access_order.retain(|n| n != name);
            self.access_order.push_back(name.to_string());
            Some(func)
        } else {
            None
        }
    }
    
    fn evict_lru(&mut self) {
        if let Some(name) = self.access_order.pop_front() {
            if let Some(func) = self.cache.remove(&name) {
                self.current_size -= func.size();
            }
        }
    }
}
```

#### 5. Hybrid VM
**Purpose**: Execute both bytecode and native code

**Strategy**:
- Check code cache before executing function
- If native code exists, call it directly
- Otherwise, execute bytecode normally
- Profile execution for future JIT compilation

**Implementation**:
```rust
pub struct HybridVm {
    vm: Vm,
    profiler: Profiler,
    hot_path_detector: HotPathDetector,
    jit_compiler: JitCompiler,
    jit_enabled: bool,
}

impl HybridVm {
    pub fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, VmError> {
        // Profile the call
        self.profiler.record_call(name);
        
        // Check if JIT is enabled and function is in cache
        if self.jit_enabled {
            if let Some(native_func) = self.jit_compiler.code_cache.get(name) {
                return native_func.call(args);
            }
            
            // Check if function is hot and should be JIT compiled
            if self.hot_path_detector.is_hot(name) {
                if let Ok(native_func) = self.jit_compiler.compile_function(name, &self.get_bytecode(name)) {
                    return native_func.call(args);
                }
            }
        }
        
        // Fallback to bytecode execution
        self.vm.call_function(name, args)
    }
}
```

## Implementation Plan

### Phase 1: Profiling Infrastructure ✅
**Files to Create**:
- `crates/matter-jit/Cargo.toml`
- `crates/matter-jit/src/lib.rs`
- `crates/matter-jit/src/profiler.rs`
- `crates/matter-jit/src/hot_path.rs`

**Features**:
- Basic profiling
- Hot path detection
- Statistics collection

### Phase 2: Code Cache
**Files to Create**:
- `crates/matter-jit/src/cache.rs`

**Features**:
- LRU cache implementation
- Size management
- Eviction policy

### Phase 3: JIT Compiler Integration
**Files to Modify**:
- `crates/matter-jit/src/compiler.rs`
- `crates/matter-vm/src/lib.rs` (add JIT hooks)

**Features**:
- Bytecode to AST decompilation
- LLVM backend integration
- Native code generation

### Phase 4: Hybrid VM
**Files to Modify**:
- `crates/matter-runtime/src/lib.rs`

**Features**:
- Dual execution mode
- Automatic JIT triggering
- Fallback mechanism

### Phase 5: CLI Integration
**Files to Modify**:
- `crates/matter-cli/src/main.rs`

**Features**:
- `--jit` flag
- `--jit-threshold` option
- `--jit-cache-size` option
- JIT statistics reporting

## Configuration

### CLI Flags
```bash
# Enable JIT compilation
matter run app.matter --jit

# Set hot threshold
matter run app.matter --jit --jit-threshold 500

# Set cache size (MB)
matter run app.matter --jit --jit-cache-size 50

# Show JIT statistics
matter run app.matter --jit --jit-stats
```

### Environment Variables
```bash
MATTER_JIT_ENABLED=1
MATTER_JIT_THRESHOLD=1000
MATTER_JIT_CACHE_SIZE=100
MATTER_JIT_VERBOSE=1
```

### Configuration File
```toml
# matter.toml
[jit]
enabled = true
threshold = 1000
cache_size = 100  # MB
verbose = false
```

## Performance Expectations

### Benchmark Scenarios

#### 1. Fibonacci (Recursive)
**Without JIT**: 150ms
**With JIT**: 15ms (10x improvement)

#### 2. Loop Intensive
**Without JIT**: 200ms
**With JIT**: 25ms (8x improvement)

#### 3. Function Calls
**Without JIT**: 100ms
**With JIT**: 20ms (5x improvement)

#### 4. Mixed Workload
**Without JIT**: 500ms
**With JIT**: 100ms (5x improvement)

### Memory Overhead
- **Profiler**: ~1MB
- **Code Cache**: 10-100MB (configurable)
- **JIT Compiler**: ~5MB
- **Total**: 16-106MB

### Compilation Time
- **Simple Function**: 10-50ms
- **Complex Function**: 50-200ms
- **Recursive Function**: 20-100ms

## Testing Strategy

### Unit Tests
1. Profiler accuracy
2. Hot path detection
3. Code cache eviction
4. JIT compilation correctness
5. Fallback mechanism

### Integration Tests
1. End-to-end JIT pipeline
2. Performance benchmarks
3. Memory usage
4. Compilation time

### Stress Tests
1. Large code cache
2. Many hot functions
3. Rapid compilation
4. Cache thrashing

## Risks and Mitigations

### Risk 1: Compilation Overhead
**Impact**: JIT compilation takes too long
**Mitigation**: 
- Compile in background thread
- Use compilation queue
- Set reasonable thresholds

### Risk 2: Memory Pressure
**Impact**: Code cache uses too much memory
**Mitigation**:
- Configurable cache size
- LRU eviction
- Monitor memory usage

### Risk 3: Incorrect Compilation
**Impact**: JIT compiled code produces wrong results
**Mitigation**:
- Extensive testing
- Fallback to bytecode on error
- Verification mode (compare results)

### Risk 4: Startup Latency
**Impact**: JIT infrastructure slows down startup
**Mitigation**:
- Lazy initialization
- JIT is optional (disabled by default)
- Minimal overhead when disabled

## Success Criteria

### Functional
- ✅ Profiler collects accurate statistics
- ✅ Hot path detection identifies hot code
- ✅ JIT compiler produces correct native code
- ✅ Code cache manages memory efficiently
- ✅ Fallback mechanism works reliably

### Performance
- ✅ 5-10x improvement on hot paths
- ✅ <100ms compilation time per function
- ✅ <10MB memory overhead (excluding cache)
- ✅ No startup degradation

### Usability
- ✅ Simple CLI flags
- ✅ Reasonable defaults
- ✅ Clear statistics
- ✅ Easy to disable

## Future Enhancements

### Phase 6: Advanced Optimizations
- Inlining
- Loop unrolling
- Constant propagation
- Dead code elimination

### Phase 7: Tiered Compilation
- Tier 1: Bytecode (interpreter)
- Tier 2: Quick JIT (fast compilation, basic optimization)
- Tier 3: Optimizing JIT (slow compilation, aggressive optimization)

### Phase 8: Speculative Optimization
- Type specialization
- Branch prediction
- Inline caching

### Phase 9: Persistent Cache
- Save compiled code to disk
- Load on startup
- Invalidate on source changes

## Comparison with Other JITs

### vs V8 (JavaScript)
- ✅ V8: Mature, highly optimized
- ✅ V8: Tiered compilation
- ✅ Matter: Simpler, easier to understand
- ✅ Matter: Leverages LLVM

### vs PyPy (Python)
- ✅ PyPy: Tracing JIT
- ✅ PyPy: Very fast
- ✅ Matter: Simpler architecture
- ✅ Matter: Method-based JIT

### vs LuaJIT
- ✅ LuaJIT: Extremely fast
- ✅ LuaJIT: Tracing JIT
- ✅ Matter: LLVM-based
- ✅ Matter: Easier to extend

## Documentation

### User Guide
- How to enable JIT
- Configuration options
- Performance tuning
- Troubleshooting

### Developer Guide
- JIT architecture
- Adding optimizations
- Profiler API
- Cache management

### API Reference
- Profiler API
- JIT Compiler API
- Code Cache API
- Hybrid VM API

## Conclusion

Sprint 20 lays the foundation for JIT compilation in Matter Core. By implementing profiling, hot path detection, and a hybrid VM, we enable dynamic optimization of frequently executed code.

**Key Benefits**:
- 5-10x performance improvement on hot paths
- Minimal overhead when disabled
- Graceful fallback to bytecode
- Foundation for future optimizations

**Next Steps**:
1. Implement profiling infrastructure
2. Create code cache
3. Integrate LLVM backend
4. Build hybrid VM
5. Add CLI integration
6. Benchmark and optimize

**Matter Core v0.10.0 will be significantly faster!** 🚀
