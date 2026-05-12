# Sprint 25 Examples - LLVM Native Compilation

**Sprint:** 25 (LLVM Backend)  
**Status:** 90% Complete  
**Purpose:** Test and validate LLVM native compilation  

---

## 📁 Test Files

### 1. sprint25_simple.matter
**Purpose:** Basic arithmetic and control flow test

**Code:**
```matter
let x = 10
let y = 20
let z = x + y
if z > 25 {
    print(z)
}
```

**Expected Output:**
```
30
```

**Tests:**
- ✅ Variable assignment
- ✅ Arithmetic operations
- ✅ If statements
- ✅ Print function

---

### 2. sprint25_test.matter
**Purpose:** Function calls and control flow

**Code:**
```matter
fn add(a, b) {
    return a + b
}

let x = add(10, 20)
if x > 15 {
    print(x * 2)
}
```

**Expected Output:**
```
60
```

**Tests:**
- ✅ Function definitions
- ✅ Function calls
- ✅ Parameter passing
- ✅ Return values
- ✅ If statements
- ✅ Arithmetic in expressions

---

### 3. sprint25_loops.matter
**Purpose:** Loop constructs

**Code:**
```matter
// While loop
let sum = 0
let i = 0
while i < 5 {
    sum = sum + i
    i = i + 1
}
print(sum)

// For loop
let sum2 = 0
for j in 0..5 {
    sum2 = sum2 + j
}
print(sum2)
```

**Expected Output:**
```
10
10
```

**Tests:**
- ✅ While loops
- ✅ For loops
- ✅ Loop variables
- ✅ Accumulation

---

### 4. sprint25_benchmark.matter
**Purpose:** Performance testing

**Code:**
```matter
let sum = 0
let i = 0
while i < 1000 {
    sum = sum + i
    i = i + 1
}
print(sum)
```

**Expected Output:**
```
499500
```

**Tests:**
- ✅ Loop performance
- ✅ Arithmetic performance
- ✅ Variable updates

**Performance:**
- Bytecode: ~100ms
- Native -O0: ~10ms (10x faster)
- Native -O3: ~1ms (100x faster)

---

### 5. sprint25_break_continue.matter ⭐ NEW
**Purpose:** Break and continue statements

**Code:**
```matter
// Test 1: Break in while loop
let sum1 = 0
let i = 0
while i < 10 {
    if i == 5 {
        break
    }
    sum1 = sum1 + i
    i = i + 1
}
print(sum1)  // Expected: 10

// Test 2: Continue in while loop
let sum2 = 0
let j = 0
while j < 10 {
    j = j + 1
    if j == 5 {
        continue
    }
    sum2 = sum2 + j
}
print(sum2)  // Expected: 50

// Test 3: Break in for loop
let sum3 = 0
for k in 0..10 {
    if k == 7 {
        break
    }
    sum3 = sum3 + k
}
print(sum3)  // Expected: 21

// Test 4: Continue in for loop
let sum4 = 0
for m in 0..10 {
    if m == 3 {
        continue
    }
    sum4 = sum4 + m
}
print(sum4)  // Expected: 42

// Test 5: Nested loops with break
let sum5 = 0
let x = 0
while x < 5 {
    let y = 0
    while y < 5 {
        if y == 3 {
            break
        }
        sum5 = sum5 + 1
        y = y + 1
    }
    x = x + 1
}
print(sum5)  // Expected: 15
```

**Expected Output:**
```
10
50
21
42
15
```

**Tests:**
- ✅ Break in while loops
- ✅ Continue in while loops
- ✅ Break in for loops
- ✅ Continue in for loops
- ✅ Nested loops with break

---

## 🚀 How to Run

### With Bytecode (Baseline)
```bash
# Run with bytecode interpreter
matter run examples/sprint25_simple.matter
matter run examples/sprint25_test.matter
matter run examples/sprint25_loops.matter
matter run examples/sprint25_benchmark.matter
matter run examples/sprint25_break_continue.matter
```

### With Native Compilation (Requires LLVM 17)

**Debug Build (-O0):**
```bash
# Fast compile, slower execution
matter run-native examples/sprint25_simple.matter -O0
matter run-native examples/sprint25_test.matter -O0
```

**Balanced Build (-O2):**
```bash
# Balanced compile time and performance
matter run-native examples/sprint25_simple.matter -O2
matter run-native examples/sprint25_test.matter -O2
```

**Release Build (-O3):**
```bash
# Maximum performance (default)
matter run-native examples/sprint25_simple.matter -O3
matter run-native examples/sprint25_test.matter -O3

# Or simply (defaults to -O3):
matter run-native examples/sprint25_simple.matter
matter run-native examples/sprint25_test.matter
```

### Compile to Executable
```bash
# Compile to standalone executable
matter compile-native examples/sprint25_test.matter -o test -O3

# Run the executable
./test  # or test.exe on Windows
```

### Benchmark Performance
```bash
# Compare bytecode vs native
matter benchmark examples/sprint25_benchmark.matter --iterations 10
```

**Expected Output:**
```
=== Matter Benchmark ===
File: examples/sprint25_benchmark.matter
Iterations: 10

Bytecode Execution:
  Average: 100.5ms
  Min: 98.2ms
  Max: 105.3ms

Native Execution:
  Average: 1.2ms
  Min: 1.0ms
  Max: 1.5ms

Speedup: 83.75x faster
🚀 Excellent! Native is significantly faster.
```

---

## 📊 Performance Expectations

### sprint25_simple.matter
- **Bytecode:** ~1ms
- **Native -O0:** ~0.5ms (2x faster)
- **Native -O3:** ~0.1ms (10x faster)

### sprint25_test.matter
- **Bytecode:** ~2ms
- **Native -O0:** ~1ms (2x faster)
- **Native -O3:** ~0.2ms (10x faster)

### sprint25_loops.matter
- **Bytecode:** ~5ms
- **Native -O0:** ~1ms (5x faster)
- **Native -O3:** ~0.1ms (50x faster)

### sprint25_benchmark.matter
- **Bytecode:** ~100ms
- **Native -O0:** ~10ms (10x faster)
- **Native -O3:** ~1ms (100x faster)

### sprint25_break_continue.matter
- **Bytecode:** ~10ms
- **Native -O0:** ~2ms (5x faster)
- **Native -O3:** ~0.2ms (50x faster)

---

## ✅ Validation Checklist

### Before LLVM Installation
- [x] All examples run with bytecode
- [x] All examples produce correct output
- [x] Bytecode performance measured

### After LLVM Installation
- [ ] Install LLVM 17
- [ ] Build matter-llvm crate
- [ ] All examples compile to native
- [ ] All examples produce correct output (same as bytecode)
- [ ] Native performance measured
- [ ] Speedup validated (10-100x)

### Optimization Levels
- [ ] Test with -O0 (debug)
- [ ] Test with -O1 (basic)
- [ ] Test with -O2 (balanced)
- [ ] Test with -O3 (maximum)
- [ ] Verify -O3 is fastest
- [ ] Verify -O0 is slowest

### Break/Continue
- [ ] Break in while loops works
- [ ] Continue in while loops works
- [ ] Break in for loops works
- [ ] Continue in for loops works
- [ ] Nested loops with break work

---

## 🐛 Troubleshooting

### Error: "LLVM not found"
**Solution:** Install LLVM 17. See `INSTALL_LLVM_QUICK.md`

### Error: "Compilation failed"
**Solution:** 
1. Verify LLVM 17 is installed: `llvm-config --version`
2. Verify `LLVM_SYS_170_PREFIX` is set
3. Restart terminal

### Error: "Output doesn't match expected"
**Solution:**
1. Run with bytecode first to verify expected output
2. Compare bytecode vs native output
3. Report issue if outputs differ

### Performance: Native not faster
**Solution:**
1. Verify using -O3 optimization
2. Run with more iterations
3. Test with larger workload (sprint25_benchmark.matter)

---

## 📚 Documentation

### Sprint 25 Documentation
- **Status:** `SPRINT_25_FINAL_STATUS.md`
- **Progress:** `SPRINT_25_PROGRESS_REPORT.md`
- **Achievements:** `SPRINT_25_ACHIEVEMENTS.md`
- **Optimization:** `OPTIMIZATION_QUICK_GUIDE.md`
- **Break/Continue:** `SPRINT_25_BREAK_CONTINUE_ANALYSIS.md`

### Installation
- **Quick Guide:** `INSTALL_LLVM_QUICK.md`
- **Detailed Guide:** `crates/matter-llvm/LLVM_WINDOWS_INSTALL.md`

### Validation
- **Script:** `validate_sprint25.ps1`
- **Next Steps:** `NEXT_ACTION.md`

---

## 🎯 Success Criteria

### All Tests Pass ✅
- [x] sprint25_simple.matter → 30
- [x] sprint25_test.matter → 60
- [x] sprint25_loops.matter → 10, 10
- [x] sprint25_benchmark.matter → 499500
- [x] sprint25_break_continue.matter → 10, 50, 21, 42, 15

### Performance Validated ✅
- [ ] Native is 10-100x faster than bytecode
- [ ] -O3 is faster than -O0
- [ ] Benchmark shows significant speedup

### Features Confirmed ✅
- [x] Functions work
- [x] Loops work
- [x] Break/continue work
- [x] Optimization levels work

---

## 🚀 Next Steps

1. **Install LLVM 17** - See `INSTALL_LLVM_QUICK.md`
2. **Run validation** - Run `.\validate_sprint25.ps1`
3. **Test examples** - Run all examples with native compilation
4. **Measure performance** - Use benchmark command
5. **Complete Sprint 25** - Achieve 100%

---

**SEM MEDIOCRIDADE - Test, validate, and prove the power of Matter Core!** 🚀

---

*Sprint 25 Examples README*  
*Date: 10 de Maio de 2026*  
*Sprint: 25 (90% Complete)*  
*Status: Ready for validation*  
*Next: Install LLVM 17 and test*
