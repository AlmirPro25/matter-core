# Sprint 21: Memory Management & Garbage Collection

## Overview
Implement a robust memory management system for Matter Core using reference counting with cycle detection, ensuring efficient memory usage and preventing memory leaks.

## Status: 🚧 IN PROGRESS

## Objectives

### Primary Goals
1. **Reference Counting** - Track object lifetimes
2. **Cycle Detection** - Identify and collect circular references
3. **Memory Pool** - Efficient allocation/deallocation
4. **Weak References** - Break cycles explicitly
5. **Memory Statistics** - Track usage and leaks

### Performance Targets
- **Allocation**: <100ns per object
- **Deallocation**: <50ns per object
- **Cycle Detection**: <10ms per collection
- **Memory Overhead**: <16 bytes per object
- **Throughput**: >1M allocations/sec

## Architecture

### Memory Model
```
┌─────────────────────────────────────────────────┐
│              Matter Core Memory                 │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌──────────────┐    ┌──────────────┐         │
│  │ Memory Pool  │───▶│  Allocator   │         │
│  │ (Arena-based)│    │  (Fast Path) │         │
│  └──────────────┘    └──────────────┘         │
│         │                    │                  │
│         ▼                    ▼                  │
│  ┌──────────────────────────────────┐         │
│  │      Reference Counted           │         │
│  │         Objects                  │         │
│  │  (Strong + Weak References)      │         │
│  └──────────────┬───────────────────┘         │
│                 │                              │
│                 ▼                              │
│  ┌──────────────────────────────────┐         │
│  │     Cycle Detector               │         │
│  │  (Mark & Sweep for Cycles)       │         │
│  └──────────────────────────────────┘         │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Components

#### 1. Reference Counted Object
**Purpose**: Track object lifetime with reference counting

**Structure**:
```rust
pub struct RcObject<T> {
    /// Strong reference count
    strong_count: AtomicUsize,
    
    /// Weak reference count
    weak_count: AtomicUsize,
    
    /// Object data
    data: T,
    
    /// Cycle detection color (White/Gray/Black)
    color: AtomicU8,
    
    /// Next object in allocation list
    next: Option<*mut RcObject<T>>,
}

impl<T> RcObject<T> {
    pub fn new(data: T) -> Rc<T> {
        let obj = RcObject {
            strong_count: AtomicUsize::new(1),
            weak_count: AtomicUsize::new(0),
            data,
            color: AtomicU8::new(WHITE),
            next: None,
        };
        
        Rc { ptr: Box::into_raw(Box::new(obj)) }
    }
    
    pub fn inc_strong(&self) {
        self.strong_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn dec_strong(&self) -> bool {
        let old = self.strong_count.fetch_sub(1, Ordering::Release);
        old == 1 // Returns true if this was the last reference
    }
    
    pub fn strong_count(&self) -> usize {
        self.strong_count.load(Ordering::Relaxed)
    }
}
```

#### 2. Smart Pointers
**Purpose**: Provide safe reference counting API

**Types**:
```rust
/// Strong reference (owns the object)
pub struct Rc<T> {
    ptr: *mut RcObject<T>,
}

impl<T> Rc<T> {
    pub fn new(data: T) -> Self {
        RcObject::new(data)
    }
    
    pub fn downgrade(&self) -> Weak<T> {
        unsafe {
            (*self.ptr).weak_count.fetch_add(1, Ordering::Relaxed);
        }
        Weak { ptr: self.ptr }
    }
    
    pub fn strong_count(&self) -> usize {
        unsafe { (*self.ptr).strong_count() }
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        unsafe {
            (*self.ptr).inc_strong();
        }
        Rc { ptr: self.ptr }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        unsafe {
            if (*self.ptr).dec_strong() {
                // Last strong reference - deallocate
                drop(Box::from_raw(self.ptr));
            }
        }
    }
}

/// Weak reference (doesn't own the object)
pub struct Weak<T> {
    ptr: *mut RcObject<T>,
}

impl<T> Weak<T> {
    pub fn upgrade(&self) -> Option<Rc<T>> {
        unsafe {
            let strong = (*self.ptr).strong_count.load(Ordering::Relaxed);
            if strong == 0 {
                None
            } else {
                (*self.ptr).inc_strong();
                Some(Rc { ptr: self.ptr })
            }
        }
    }
}
```

#### 3. Memory Pool
**Purpose**: Fast allocation with arena-based memory

**Implementation**:
```rust
pub struct MemoryPool {
    /// Current arena
    current_arena: Arena,
    
    /// List of all arenas
    arenas: Vec<Arena>,
    
    /// Total allocated bytes
    total_allocated: AtomicUsize,
    
    /// Total deallocated bytes
    total_deallocated: AtomicUsize,
    
    /// Peak memory usage
    peak_usage: AtomicUsize,
}

struct Arena {
    /// Memory buffer
    buffer: Vec<u8>,
    
    /// Current offset
    offset: usize,
    
    /// Capacity
    capacity: usize,
}

impl MemoryPool {
    pub fn new() -> Self {
        Self::with_capacity(1024 * 1024) // 1MB default
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            current_arena: Arena::new(capacity),
            arenas: Vec::new(),
            total_allocated: AtomicUsize::new(0),
            total_deallocated: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
        }
    }
    
    pub fn allocate<T>(&mut self, data: T) -> *mut T {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();
        
        // Try to allocate from current arena
        if let Some(ptr) = self.current_arena.allocate(size, align) {
            unsafe {
                std::ptr::write(ptr as *mut T, data);
            }
            self.total_allocated.fetch_add(size, Ordering::Relaxed);
            return ptr as *mut T;
        }
        
        // Need new arena
        self.arenas.push(std::mem::replace(
            &mut self.current_arena,
            Arena::new(self.current_arena.capacity * 2)
        ));
        
        self.allocate(data)
    }
    
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            total_deallocated: self.total_deallocated.load(Ordering::Relaxed),
            current_usage: self.current_usage(),
            peak_usage: self.peak_usage.load(Ordering::Relaxed),
            arena_count: self.arenas.len() + 1,
        }
    }
    
    fn current_usage(&self) -> usize {
        self.total_allocated.load(Ordering::Relaxed) - 
        self.total_deallocated.load(Ordering::Relaxed)
    }
}
```

#### 4. Cycle Detector
**Purpose**: Detect and collect circular references

**Algorithm**: Tri-color marking
- **White**: Not visited
- **Gray**: Visited, children not processed
- **Black**: Visited, children processed

**Implementation**:
```rust
pub struct CycleDetector {
    /// Objects to check for cycles
    objects: Vec<*mut dyn RcObject>,
    
    /// Collection threshold
    threshold: usize,
    
    /// Number of collections performed
    collections: u64,
    
    /// Objects collected
    objects_collected: u64,
}

impl CycleDetector {
    pub fn new() -> Self {
        Self::with_threshold(1000)
    }
    
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            objects: Vec::new(),
            threshold,
            collections: 0,
            objects_collected: 0,
        }
    }
    
    pub fn register(&mut self, obj: *mut dyn RcObject) {
        self.objects.push(obj);
        
        if self.objects.len() >= self.threshold {
            self.collect();
        }
    }
    
    pub fn collect(&mut self) {
        let start = std::time::Instant::now();
        
        // Mark phase
        self.mark_roots();
        
        // Sweep phase
        let collected = self.sweep();
        
        self.collections += 1;
        self.objects_collected += collected;
        
        let duration = start.elapsed();
        if duration.as_millis() > 10 {
            eprintln!("[GC] Collection took {:?}", duration);
        }
    }
    
    fn mark_roots(&mut self) {
        // Mark all objects as white
        for obj in &self.objects {
            unsafe {
                (**obj).set_color(WHITE);
            }
        }
        
        // Mark reachable objects
        for obj in &self.objects {
            unsafe {
                if (**obj).strong_count() > 0 {
                    self.mark(*obj);
                }
            }
        }
    }
    
    fn mark(&self, obj: *mut dyn RcObject) {
        unsafe {
            if (*obj).color() != WHITE {
                return;
            }
            
            (*obj).set_color(GRAY);
            
            // Mark children
            for child in (*obj).children() {
                self.mark(child);
            }
            
            (*obj).set_color(BLACK);
        }
    }
    
    fn sweep(&mut self) -> u64 {
        let mut collected = 0;
        
        self.objects.retain(|obj| {
            unsafe {
                if (**obj).color() == WHITE && (**obj).strong_count() == 0 {
                    // Object is unreachable - collect it
                    drop(Box::from_raw(*obj));
                    collected += 1;
                    false
                } else {
                    true
                }
            }
        });
        
        collected
    }
}
```

#### 5. Memory Statistics
**Purpose**: Track memory usage and detect leaks

**Structure**:
```rust
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub total_deallocated: usize,
    pub current_usage: usize,
    pub peak_usage: usize,
    pub arena_count: usize,
    pub object_count: usize,
    pub cycle_collections: u64,
    pub objects_collected: u64,
}

impl MemoryStats {
    pub fn leak_check(&self) -> Option<MemoryLeak> {
        if self.current_usage > self.total_allocated / 2 {
            Some(MemoryLeak {
                leaked_bytes: self.current_usage,
                leaked_objects: self.object_count,
            })
        } else {
            None
        }
    }
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Memory Statistics:")?;
        writeln!(f, "  Total Allocated: {} bytes", self.total_allocated)?;
        writeln!(f, "  Total Deallocated: {} bytes", self.total_deallocated)?;
        writeln!(f, "  Current Usage: {} bytes", self.current_usage)?;
        writeln!(f, "  Peak Usage: {} bytes", self.peak_usage)?;
        writeln!(f, "  Arena Count: {}", self.arena_count)?;
        writeln!(f, "  Object Count: {}", self.object_count)?;
        writeln!(f, "  Cycle Collections: {}", self.cycle_collections)?;
        writeln!(f, "  Objects Collected: {}", self.objects_collected)?;
        Ok(())
    }
}
```

## Implementation Plan

### Phase 1: Reference Counting ✅
**Files to Create**:
- `crates/matter-memory/Cargo.toml`
- `crates/matter-memory/src/lib.rs`
- `crates/matter-memory/src/rc.rs`
- `crates/matter-memory/src/weak.rs`

**Features**:
- Strong references (Rc)
- Weak references (Weak)
- Atomic reference counting
- Thread-safe operations

### Phase 2: Memory Pool
**Files to Create**:
- `crates/matter-memory/src/pool.rs`
- `crates/matter-memory/src/arena.rs`

**Features**:
- Arena-based allocation
- Fast allocation path
- Memory statistics
- Peak usage tracking

### Phase 3: Cycle Detection
**Files to Create**:
- `crates/matter-memory/src/cycle.rs`

**Features**:
- Tri-color marking
- Cycle detection
- Automatic collection
- Configurable threshold

### Phase 4: Integration with VM
**Files to Modify**:
- `crates/matter-backend/src/lib.rs`
- `crates/matter-vm/src/lib.rs`

**Features**:
- Use Rc for all heap objects
- Automatic memory management
- Cycle collection on GC trigger

### Phase 5: CLI Integration
**Files to Modify**:
- `crates/matter-cli/src/main.rs`

**Features**:
- `--gc-stats` flag
- `--gc-threshold` option
- Memory leak detection

## Configuration

### CLI Flags
```bash
# Show GC statistics
matter run app.matter --gc-stats

# Set collection threshold
matter run app.matter --gc-threshold 5000

# Enable verbose GC logging
matter run app.matter --gc-verbose

# Detect memory leaks
matter run app.matter --leak-check
```

### Environment Variables
```bash
MATTER_GC_THRESHOLD=1000
MATTER_GC_VERBOSE=1
MATTER_LEAK_CHECK=1
```

## Performance Expectations

### Allocation Performance
- **Fast Path**: <100ns per allocation
- **Slow Path**: <1μs per allocation (new arena)
- **Throughput**: >1M allocations/sec

### Deallocation Performance
- **Reference Drop**: <50ns
- **Object Destruction**: <100ns

### Cycle Collection
- **Frequency**: Every 1000 allocations (default)
- **Duration**: <10ms per collection
- **Pause Time**: <5ms (amortized)

### Memory Overhead
- **Per Object**: 16 bytes (2x usize + color + next)
- **Per Arena**: 8KB-1MB (configurable)
- **Total**: <5% overhead

## Testing Strategy

### Unit Tests
1. Reference counting correctness
2. Weak reference upgrade/downgrade
3. Memory pool allocation
4. Cycle detection accuracy
5. Memory leak detection

### Integration Tests
1. Complex object graphs
2. Circular references
3. Memory pressure scenarios
4. Concurrent allocation

### Stress Tests
1. Million object allocation
2. Deep object hierarchies
3. Rapid allocation/deallocation
4. Memory leak scenarios

## Success Criteria

### Functional
- ✅ Reference counting works correctly
- ✅ Weak references prevent cycles
- ✅ Cycle detector finds all cycles
- ✅ Memory pool is efficient
- ✅ No memory leaks

### Performance
- ✅ <100ns allocation
- ✅ <50ns deallocation
- ✅ <10ms cycle collection
- ✅ <5% memory overhead

### Quality
- ✅ Comprehensive tests
- ✅ Clear documentation
- ✅ Thread-safe operations
- ✅ Memory leak detection

## Comparison with Other GC Systems

### vs Rust std::rc::Rc
- ✅ Rust: No cycle detection
- ✅ Matter: Automatic cycle collection
- ✅ Both: Reference counting

### vs Python GC
- ✅ Python: Reference counting + generational GC
- ✅ Matter: Reference counting + cycle detection
- ✅ Matter: Lower overhead

### vs Go GC
- ✅ Go: Concurrent mark-sweep
- ✅ Matter: Reference counting (deterministic)
- ✅ Go: Better for high-throughput

### vs Java GC
- ✅ Java: Generational GC, many algorithms
- ✅ Matter: Simpler, more predictable
- ✅ Java: Better for large heaps

## Future Enhancements

### Phase 6: Generational GC
- Young generation (frequent collection)
- Old generation (infrequent collection)
- Promotion policy

### Phase 7: Concurrent GC
- Background cycle collection
- Incremental marking
- Reduced pause times

### Phase 8: Compacting GC
- Memory defragmentation
- Improved cache locality
- Reduced memory footprint

## Conclusion

Sprint 21 implements a robust memory management system for Matter Core using reference counting with cycle detection. This provides:

- **Deterministic**: Objects freed immediately when unreachable
- **Efficient**: <100ns allocation, <50ns deallocation
- **Safe**: Automatic cycle collection prevents leaks
- **Observable**: Rich statistics and leak detection

**Matter Core v0.11.0 will have production-grade memory management!** 🚀
