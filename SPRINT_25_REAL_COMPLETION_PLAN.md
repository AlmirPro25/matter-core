# Sprint 25: Real Completion Plan

**Date:** 10 de Maio de 2026  
**Status:** 🚧 IN PROGRESS (65% → 100%)  
**Goal:** Close Sprint 25 with real validation  

---

## 🎯 Objective

**Complete Sprint 25 with working code, not just documentation.**

**Success Criteria:**
```matter
fn add(a, b) {
    return a + b;
}

let x = add(10, 20);

if x > 15 {
    print(x);
}
```

**Must work with:**
```bash
matter run-native exemplo.matter
# Output: 30
```

---

## 📋 Execution Plan

### Step 1: Install LLVM 17 ⚠️ CRITICAL

**Task:** Install LLVM 17 on Windows  
**Status:** ❌ NOT DONE  
**File:** N/A (system installation)  
**Acceptance Criteria:**
- LLVM 17 installed
- `llvm-config --version` returns 17.x.x
- `LLVM_SYS_170_PREFIX` environment variable set
- LLVM bin in PATH

**Test Command:**
```cmd
llvm-config --version
echo %LLVM_SYS_170_PREFIX%
```

**Installation Steps:**
1. Download LLVM-17.0.6-win64.exe from https://github.com/llvm/llvm-project/releases/tag/llvmorg-17.0.6
2. Run installer
3. Check "Add LLVM to system PATH"
4. Set environment variable:
   ```cmd
   setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
   ```
5. Restart terminal
6. Verify:
   ```cmd
   llvm-config --version
   ```

**Priority:** 🔥 CRITICAL - Nothing else works without this

---

### Step 2: Validate Phase 1 ⚠️ CRITICAL

**Task:** Confirm Phase 1 actually builds and tests pass  
**Status:** ❌ NOT VALIDATED  
**File:** `crates/matter-llvm/src/lib.rs`  
**Acceptance Criteria:**
- `cargo build -p matter-llvm` succeeds
- `cargo test -p matter-llvm` passes
- All 14 tests pass

**Test Commands:**
```bash
cargo fmt
cargo check --workspace
cargo build -p matter-llvm
cargo test -p matter-llvm
```

**Expected Output:**
```
test test_codegen_creation ... ok
test test_create_main ... ok
test test_compile_int ... ok
test test_get_ir ... ok
test test_verify_empty_module ... ok
test test_compile_simple_constant ... ok
test test_compile_arithmetic ... ok
test test_compile_variable_store_load ... ok
test test_compile_comparison ... ok
test test_get_llvm_ir_simple ... ok
test test_compile_if_statement ... ok
test test_compile_while_loop ... ok
test test_compile_unconditional_jump ... ok
test test_compile_conditional_jump ... ok

test result: ok. 14 passed; 0 failed
```

**Priority:** 🔥 CRITICAL - Must pass before continuing

---

### Step 3: Implement Real Functions (Phase 2) 🚧

#### Task 3.1: Function Definitions

**Task:** Implement function definition compilation  
**Status:** ❌ NOT IMPLEMENTED  
**File:** `crates/matter-llvm/src/lib.rs`  
**Acceptance Criteria:**
- Can compile function definitions from bytecode
- Functions have proper LLVM function type
- Function parameters mapped correctly

**Implementation:**
```rust
fn compile_function_definition(
    &mut self,
    name: &str,
    param_count: usize,
    instructions: &[Instruction],
    constants: &[Constant],
) -> Result<(), String> {
    // Create LLVM function
    let function = self.create_function(name, param_count);
    
    // Create entry basic block
    let entry_bb = self.context.append_basic_block(function, "entry");
    self.builder.position_at_end(entry_bb);
    
    // Compile function body
    self.compile_instructions(instructions, constants)?;
    
    Ok(())
}
```

**Test Command:**
```bash
cargo test test_function_definition
```

**Priority:** 🔥 HIGH

---

#### Task 3.2: Real Function Calls

**Task:** Implement real function calls (not stubs)  
**Status:** ❌ STUB ONLY  
**File:** `crates/matter-llvm/src/lib.rs`  
**Acceptance Criteria:**
- Can call LLVM functions
- Arguments passed correctly
- Return value received correctly

**Implementation:**
```rust
fn compile_call(&mut self, arg_count: usize) -> Result<(), String> {
    // Pop function name/value
    let func_value = self.stack.pop()?;
    
    // Pop arguments
    let mut args = Vec::new();
    for _ in 0..arg_count {
        args.push(self.stack.pop()?);
    }
    args.reverse();
    
    // Get LLVM function
    let function = /* lookup function by name */;
    
    // Build call
    let call_result = self.builder.build_call(
        function,
        &args.iter().map(|v| (*v).into()).collect::<Vec<_>>(),
        "call"
    )?;
    
    // Push return value
    if let Some(result) = call_result.try_as_basic_value().left() {
        if let BasicValueEnum::IntValue(int_val) = result {
            self.stack.push(int_val);
        }
    }
    
    Ok(())
}
```

**Test Command:**
```bash
cargo test test_function_call
```

**Priority:** 🔥 HIGH

---

#### Task 3.3: Parameter Passing

**Task:** Implement parameter passing mechanism  
**Status:** ❌ NOT IMPLEMENTED  
**File:** `crates/matter-llvm/src/lib.rs`  
**Acceptance Criteria:**
- Function parameters accessible in function body
- Parameters have correct values
- Multiple parameters work

**Implementation:**
```rust
// In function definition:
// Map bytecode parameters to LLVM function parameters
for (i, param) in function.get_param_iter().enumerate() {
    let param_name = format!("param_{}", i);
    // Store parameter in local variable
    let alloca = self.builder.build_alloca(self.i64_type(), &param_name)?;
    self.builder.build_store(alloca, param)?;
    self.variables.insert(param_name, alloca);
}
```

**Test Command:**
```bash
cargo test test_parameter_passing
```

**Priority:** 🔥 HIGH

---

#### Task 3.4: Return Values

**Task:** Implement return value handling  
**Status:** ❌ STUB ONLY  
**File:** `crates/matter-llvm/src/lib.rs`  
**Acceptance Criteria:**
- Functions can return values
- Return value has correct type
- Caller receives return value

**Implementation:**
```rust
fn compile_return(&mut self) -> Result<(), String> {
    let return_value = self.stack.pop()?;
    self.builder.build_return(Some(&return_value))?;
    Ok(())
}
```

**Test Command:**
```bash
cargo test test_return_value
```

**Priority:** 🔥 HIGH

---

#### Task 3.5: For Loops

**Task:** Implement for loop compilation  
**Status:** ❌ NOT IMPLEMENTED  
**File:** `crates/matter-llvm/src/lib.rs`  
**Acceptance Criteria:**
- For loops compile correctly
- Iterator variable works
- Loop body executes correct number of times

**Test Command:**
```bash
cargo test test_for_loop
```

**Priority:** 🟡 MEDIUM

---

#### Task 3.6: Break/Continue

**Task:** Implement break and continue  
**Status:** ❌ NOT IMPLEMENTED  
**File:** `crates/matter-llvm/src/lib.rs`  
**Acceptance Criteria:**
- Break exits loop
- Continue jumps to loop start
- Works in nested loops

**Test Command:**
```bash
cargo test test_break_continue
```

**Priority:** 🟡 MEDIUM

---

### Step 4: Implement CLI Commands (Phase 4) 🚧

#### Task 4.1: show-ir Command

**Task:** Implement `matter show-ir <file>`  
**Status:** ❌ NOT IMPLEMENTED  
**File:** `crates/matter-cli/src/main.rs`  
**Acceptance Criteria:**
- Command exists in CLI
- Compiles Matter to bytecode
- Generates LLVM IR
- Prints IR to stdout

**Implementation:**
```rust
"show-ir" => {
    if args.len() < 3 {
        eprintln!("Usage: matter show-ir <file.matter>");
        process::exit(1);
    }
    show_ir(&args[2]);
}

fn show_ir(file: &str) {
    // Read source
    let source = read_source(file);
    
    // Parse
    let program = parse_source(&source);
    
    // Compile to bytecode
    let bytecode = compile_to_bytecode(&program);
    
    // Generate LLVM IR
    let ir = matter_llvm::get_llvm_ir(&bytecode).unwrap();
    
    // Print IR
    println!("{}", ir);
}
```

**Test Command:**
```bash
matter show-ir examples/hello.matter
```

**Expected Output:**
```llvm
; ModuleID = 'matter_program'
source_filename = "matter_program"

define i32 @main() {
entry:
  ...
  ret i32 0
}
```

**Priority:** 🔥 HIGH

---

#### Task 4.2: compile-native Command

**Task:** Implement `matter compile-native <file> -o <output>`  
**Status:** ❌ NOT IMPLEMENTED  
**File:** `crates/matter-cli/src/main.rs`  
**Acceptance Criteria:**
- Command exists in CLI
- Compiles Matter to native executable
- Output file created
- Executable runs

**Implementation:**
```rust
"compile-native" => {
    if args.len() < 3 {
        eprintln!("Usage: matter compile-native <file.matter> [-o output]");
        process::exit(1);
    }
    
    let output = if args.len() >= 5 && args[3] == "-o" {
        &args[4]
    } else {
        "output"
    };
    
    compile_native(&args[2], output);
}

fn compile_native(file: &str, output: &str) {
    // Read source
    let source = read_source(file);
    
    // Parse
    let program = parse_source(&source);
    
    // Compile to bytecode
    let bytecode = compile_to_bytecode(&program);
    
    // Compile to native
    matter_llvm::compile_to_native(&bytecode, output).unwrap();
    
    println!("Compiled to: {}", output);
}
```

**Test Command:**
```bash
matter compile-native examples/hello.matter -o hello
./hello
```

**Priority:** 🔥 HIGH

---

#### Task 4.3: run-native Command

**Task:** Implement `matter run-native <file>`  
**Status:** ❌ NOT IMPLEMENTED  
**File:** `crates/matter-cli/src/main.rs`  
**Acceptance Criteria:**
- Command exists in CLI
- Compiles and runs in one step
- Output displayed
- Temporary files cleaned up

**Implementation:**
```rust
"run-native" => {
    if args.len() < 3 {
        eprintln!("Usage: matter run-native <file.matter>");
        process::exit(1);
    }
    run_native(&args[2]);
}

fn run_native(file: &str) {
    // Compile to temporary executable
    let temp_output = ".matter_temp_output";
    compile_native(file, temp_output);
    
    // Run executable
    let output = std::process::Command::new(format!("./{}", temp_output))
        .output()
        .expect("Failed to run");
    
    // Print output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    
    // Clean up
    std::fs::remove_file(temp_output).ok();
}
```

**Test Command:**
```bash
matter run-native examples/hello.matter
```

**Priority:** 🔥 HIGH

---

#### Task 4.4: benchmark Command

**Task:** Implement `matter benchmark <file>`  
**Status:** ❌ NOT IMPLEMENTED  
**File:** `crates/matter-cli/src/main.rs`  
**Acceptance Criteria:**
- Command exists in CLI
- Runs bytecode version
- Runs native version
- Compares performance
- Shows speedup

**Test Command:**
```bash
matter benchmark examples/fibonacci.matter
```

**Expected Output:**
```
Benchmarking: examples/fibonacci.matter

Bytecode: 1234ms
Native:   123ms
Speedup:  10.0x
```

**Priority:** 🟡 MEDIUM

---

### Step 5: End-to-End Validation ⚠️ CRITICAL

#### Task 5.1: Create Test Program

**Task:** Create working example program  
**Status:** ❌ NOT CREATED  
**File:** `examples/sprint25_test.matter`  
**Acceptance Criteria:**
- Program uses functions
- Program uses if/else
- Program uses arithmetic
- Program prints output

**Content:**
```matter
fn add(a, b) {
    return a + b;
}

fn multiply(a, b) {
    return a * b;
}

let x = add(10, 20);
let y = multiply(x, 2);

if y > 50 {
    print(y);
} else {
    print(0);
}
```

**Expected Output:** `60`

**Priority:** 🔥 CRITICAL

---

#### Task 5.2: Compile and Run

**Task:** Compile and run test program  
**Status:** ❌ NOT DONE  
**File:** N/A  
**Acceptance Criteria:**
- `matter show-ir examples/sprint25_test.matter` works
- `matter compile-native examples/sprint25_test.matter -o test` works
- `./test` produces correct output (60)
- `matter run-native examples/sprint25_test.matter` produces correct output (60)

**Test Commands:**
```bash
# Show IR
matter show-ir examples/sprint25_test.matter

# Compile
matter compile-native examples/sprint25_test.matter -o test

# Run compiled
./test
# Expected: 60

# Run directly
matter run-native examples/sprint25_test.matter
# Expected: 60
```

**Priority:** 🔥 CRITICAL

---

### Step 6: Final Validation ⚠️ CRITICAL

**Task:** Run all validation checks  
**Status:** ❌ NOT DONE  
**File:** N/A  
**Acceptance Criteria:**
- All commands succeed
- All tests pass
- Example program works

**Validation Commands:**
```bash
# Format
cargo fmt

# Check
cargo check --workspace

# Build
cargo build -p matter-llvm

# Test LLVM
cargo test -p matter-llvm

# Test workspace
cargo test --workspace

# Test CLI
matter show-ir examples/sprint25_test.matter
matter compile-native examples/sprint25_test.matter -o test
./test
matter run-native examples/sprint25_test.matter
```

**Priority:** 🔥 CRITICAL

---

## 📊 Progress Tracking

### Phase 1: LLVM IR Generation
- [x] Infrastructure ✅
- [x] Core instructions ✅
- [ ] Build validation ❌
- [ ] Test validation ❌

**Status:** 90% (needs validation)

---

### Phase 2: Control Flow & Functions
- [x] If/else ✅
- [x] While loops ✅
- [ ] Function definitions ❌
- [ ] Real function calls ❌
- [ ] Parameter passing ❌
- [ ] Return values ❌
- [ ] For loops ❌
- [ ] Break/continue ❌

**Status:** 20% → Target: 100%

---

### Phase 3: Data Structures
- [x] Placeholders ✅
- [ ] Real implementation ❌ (DEFERRED)

**Status:** 20% (PLACEHOLDER - OK for now)

---

### Phase 4: CLI Integration
- [ ] show-ir command ❌
- [ ] compile-native command ❌
- [ ] run-native command ❌
- [ ] benchmark command ❌

**Status:** 0% → Target: 100%

---

## 🎯 Definition of Done

**Sprint 25 is complete when:**

1. ✅ LLVM 17 installed and verified
2. ✅ `cargo build -p matter-llvm` succeeds
3. ✅ `cargo test -p matter-llvm` passes (all 14+ tests)
4. ✅ `cargo test --workspace` passes
5. ✅ Function definitions work
6. ✅ Function calls work
7. ✅ Parameter passing works
8. ✅ Return values work
9. ✅ CLI commands implemented (show-ir, compile-native, run-native)
10. ✅ Example program compiles and runs correctly
11. ✅ Output is correct

**Current:** 0/11 ❌  
**Target:** 11/11 ✅

---

## 📅 Timeline

### Day 1: Setup & Validation
- Install LLVM 17
- Validate Phase 1 builds and tests
- Fix any issues

### Day 2-3: Functions
- Implement function definitions
- Implement real function calls
- Implement parameter passing
- Implement return values
- Test functions work

### Day 4: CLI
- Implement show-ir command
- Implement compile-native command
- Implement run-native command
- Test CLI works

### Day 5: Validation
- Create test program
- Run end-to-end tests
- Verify everything works
- THEN declare complete

**Total:** 5 days (realistic, not 1 day)

---

## 🚨 Rules

1. **No celebration until validation passes**
2. **No "100% complete" until all tests pass**
3. **No "production ready" until example works**
4. **No Sprint 26 until Sprint 25 is done**
5. **Be honest about status**

---

## 🎯 Success Metric

**This program must work:**

```matter
fn add(a, b) {
    return a + b;
}

let x = add(10, 20);

if x > 15 {
    print(x);
}
```

**With this command:**
```bash
matter run-native exemplo.matter
# Output: 30
```

**When this works, Sprint 25 is complete.**

---

*Real Completion Plan*  
*Date: 10 de Maio de 2026*  
*Status: 65% → 100% (in progress)*  
*ETA: 5 days with validation*  
*No shortcuts. Real work only.*
