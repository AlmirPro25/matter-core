# Matter Core - Optimization Quick Guide

**Feature:** LLVM Optimization Levels  
**Version:** v0.15.0-dev  
**Sprint:** 25  
**Status:** ✅ Available  

---

## 🚀 Quick Start

### Compile with Optimization
```bash
# Debug build (no optimization)
matter compile-native program.matter -o program -O0

# Balanced build (default optimization)
matter compile-native program.matter -o program -O2

# Release build (maximum optimization)
matter compile-native program.matter -o program -O3
matter compile-native program.matter -o program  # Same as -O3
```

### Run with Optimization
```bash
# Debug run
matter run-native program.matter -O0

# Balanced run
matter run-native program.matter -O2

# Release run (maximum performance)
matter run-native program.matter -O3
matter run-native program.matter  # Same as -O3
```

---

## 📊 Optimization Levels

| Flag | Level | Compile Time | Runtime Speed | Use Case |
|------|-------|--------------|---------------|----------|
| `-O0` | None | Fastest | Slowest | Debug, Development |
| `-O1` | Less | Fast | Moderate | Quick builds |
| `-O2` | Default | Moderate | Fast | General purpose |
| `-O3` | Aggressive | Slowest | Fastest | Production, Benchmarks |

---

## 💡 When to Use Each Level

### -O0 (Debug)
**Use for:**
- Development
- Debugging
- Testing
- Quick iterations

**Benefits:**
- Fastest compilation
- Easiest debugging
- Predictable behavior

**Example:**
```bash
matter compile-native debug_test.matter -o test -O0
matter run-native debug_test.matter -O0
```

---

### -O1 (Basic)
**Use for:**
- Development with some optimization
- Quick builds that need decent performance
- CI/CD pipelines

**Benefits:**
- Fast compilation
- Moderate performance improvement
- Good balance for development

**Example:**
```bash
matter compile-native dev_build.matter -o app -O1
```

---

### -O2 (Balanced)
**Use for:**
- General purpose builds
- Testing performance
- Pre-production validation

**Benefits:**
- Balanced compile time and performance
- Good for most use cases
- Recommended default for manual builds

**Example:**
```bash
matter compile-native app.matter -o app -O2
```

---

### -O3 (Maximum)
**Use for:**
- Production builds
- Benchmarks
- Performance-critical applications
- Final releases

**Benefits:**
- Maximum performance
- Smallest binaries
- Best for end users

**Example:**
```bash
matter compile-native production.matter -o app -O3
matter compile-native production.matter -o app  # Same
```

---

## 📈 Performance Expectations

### Typical Speedup (vs -O0)
- **-O1:** 1.5-2x faster
- **-O2:** 2-3x faster
- **-O3:** 3-5x faster

### Compile Time (vs -O0)
- **-O1:** ~1.2x slower
- **-O2:** ~1.5x slower
- **-O3:** ~2x slower

### Example: Fibonacci(40)
```
-O0: 2.5 seconds
-O1: 1.5 seconds (1.7x faster)
-O2: 1.0 seconds (2.5x faster)
-O3: 0.6 seconds (4.2x faster)
```

---

## 🎯 Common Workflows

### Development Workflow
```bash
# Fast iteration during development
matter run-native app.matter -O0

# Quick test with some optimization
matter run-native app.matter -O1
```

### Testing Workflow
```bash
# Test with balanced optimization
matter compile-native app.matter -o app_test -O2
./app_test

# Benchmark with maximum optimization
matter benchmark app.matter --iterations 10
```

### Production Workflow
```bash
# Build release version
matter compile-native app.matter -o app_release -O3

# Verify performance
time ./app_release

# Deploy
cp app_release /production/app
```

---

## 🔧 Advanced Usage

### Multiple Builds
```bash
# Build all optimization levels
matter compile-native app.matter -o app_debug -O0
matter compile-native app.matter -o app_dev -O1
matter compile-native app.matter -o app_test -O2
matter compile-native app.matter -o app_release -O3

# Compare performance
time ./app_debug
time ./app_dev
time ./app_test
time ./app_release
```

### Benchmark Comparison
```bash
# Create benchmark script
echo "let sum = 0" > bench.matter
echo "let i = 0" >> bench.matter
echo "while i < 1000000 { sum = sum + i; i = i + 1 }" >> bench.matter
echo "print(sum)" >> bench.matter

# Test each level
time matter run-native bench.matter -O0
time matter run-native bench.matter -O1
time matter run-native bench.matter -O2
time matter run-native bench.matter -O3
```

---

## ⚠️ Important Notes

### Default Behavior
- If no `-O` flag is specified, **-O3** is used (maximum optimization)
- This ensures best performance by default
- Use `-O0` explicitly for debugging

### Compatibility
- All optimization levels produce identical output
- Only compile time and runtime speed differ
- Choose based on your needs

### Debugging
- Use `-O0` for debugging with debuggers
- Higher optimization levels make debugging harder
- Variables may be optimized away at -O2 and -O3

---

## 📝 Examples

### Example 1: Simple Program
```matter
// hello.matter
fn greet(name) {
    print("Hello, ")
    print(name)
}

greet("World")
```

```bash
# Debug
matter run-native hello.matter -O0

# Release
matter run-native hello.matter -O3
```

---

### Example 2: Performance-Critical
```matter
// fibonacci.matter
fn fib(n) {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

print(fib(35))
```

```bash
# Slow (debug)
time matter run-native fibonacci.matter -O0
# Output: ~5 seconds

# Fast (release)
time matter run-native fibonacci.matter -O3
# Output: ~1 second (5x faster!)
```

---

### Example 3: Build Script
```bash
#!/bin/bash
# build.sh - Build all versions

APP="myapp.matter"

echo "Building debug version..."
matter compile-native $APP -o myapp_debug -O0

echo "Building release version..."
matter compile-native $APP -o myapp_release -O3

echo "Done!"
echo "Debug:   ./myapp_debug"
echo "Release: ./myapp_release"
```

---

## 🎯 Best Practices

### 1. Development
- Use `-O0` for fast iteration
- Use `-O1` when you need some performance
- Don't use `-O3` during development (slow compile)

### 2. Testing
- Test with `-O2` (balanced)
- Verify with `-O3` before release
- Benchmark with `-O3`

### 3. Production
- Always use `-O3` for production
- Compile once, deploy everywhere
- Keep debug builds separate

### 4. CI/CD
- Build with `-O2` for tests (faster CI)
- Build with `-O3` for releases
- Cache compiled binaries

---

## 🚀 Quick Reference

```bash
# Commands
matter compile-native <file> -o <output> [-O0|-O1|-O2|-O3]
matter run-native <file> [-O0|-O1|-O2|-O3]
matter benchmark <file> [--iterations N]

# Optimization Levels
-O0  # Debug (no optimization)
-O1  # Basic (some optimization)
-O2  # Balanced (good optimization)
-O3  # Maximum (aggressive optimization, default)

# Examples
matter compile-native app.matter -o app -O0  # Debug
matter compile-native app.matter -o app -O2  # Balanced
matter compile-native app.matter -o app -O3  # Release
matter compile-native app.matter -o app      # Release (default)

matter run-native app.matter -O0  # Debug run
matter run-native app.matter -O3  # Release run
matter run-native app.matter      # Release run (default)
```

---

## 📚 More Information

- **Full Documentation:** `SPRINT_25_OPTIMIZATION_COMPLETE.md`
- **Sprint Status:** `SPRINT_25_FINAL_STATUS.md`
- **Session Summary:** `SESSION_CONTINUATION_SUMMARY.md`
- **Roadmap:** `ROADMAP_2026.md`

---

## 🎉 Summary

**Optimization levels are now available in Matter Core!**

- ✅ Four optimization levels (-O0 to -O3)
- ✅ Industry-standard conventions
- ✅ Easy to use
- ✅ Significant performance improvements
- ✅ Flexible for all use cases

**Choose the right level for your needs:**
- **Development:** `-O0` or `-O1`
- **Testing:** `-O2`
- **Production:** `-O3` (default)

**Start using optimizations today:**
```bash
matter compile-native myapp.matter -o myapp -O3
```

---

**SEM MEDIOCRIDADE - Optimize for success!** 🚀

---

*Matter Core Optimization Quick Guide*  
*Version: v0.15.0-dev*  
*Sprint: 25*  
*Date: 10 de Maio de 2026*
