# Sprint 25: Next Steps - Real Completion

**Date:** 10 de Maio de 2026  
**Current Status:** 65% Complete (Honest Assessment)  
**Target:** 100% Complete with Validation  

---

## 🎯 Mission

**Complete Sprint 25 with working code, not just documentation.**

**Success = This program works:**
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

---

## 🚨 CRITICAL: Install LLVM 17 First

**Nothing else works without LLVM 17 installed.**

### Installation Steps:

1. **Download LLVM 17.0.6:**
   - URL: https://github.com/llvm/llvm-project/releases/tag/llvmorg-17.0.6
   - File: `LLVM-17.0.6-win64.exe`

2. **Run Installer:**
   - ✅ Check "Add LLVM to system PATH"
   - Install to: `C:\Program Files\LLVM`

3. **Set Environment Variable:**
   ```cmd
   setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
   ```

4. **Restart Terminal**

5. **Verify:**
   ```cmd
   llvm-config --version
   ```
   Should output: `17.0.6`

**See:** `crates/matter-llvm/LLVM_WINDOWS_INSTALL.md` for detailed guide

---

## 📋 Execution Order

### Phase 1: Validation (CRITICAL)

**Before writing any new code, validate what exists:**

```bash
# After installing LLVM 17:
cargo fmt
cargo check --workspace
cargo build -p matter-llvm
cargo test -p matter-llvm
```

**Expected:**
- Build succeeds ✅
- 14 tests pass ✅

**If this fails, fix build errors first.**

---

### Phase 2: Implement Real Functions (HIGH PRIORITY)

**Current Problem:**
- Function calls are stubs (return 0)
- Function definitions don't exist
- Parameters don't work
- Return values don't work

**What to Implement:**

#### 2.1: Function Definitions
**File:** `crates/matter-llvm/src/lib.rs`

Add method to compile function definitions:
```rust
fn compile_function_definition(
    &mut self,
    name: &str,
    param_count: usize,
    body_instructions: &[Instruction],
) -> Result<(), String> {
    // Create LLVM function type
    let param_types = vec![self.i64_type().into(); param_count];
    let fn_type = self.i64_type().fn_type(&param_types, false);
    
    // Add function to module
    let function = self.module.add_function(name, fn_type, None);
    
    // Create entry basic block
    let entry_bb = self.context.append_basic_block(function, "entry");
    self.builder.position_at_end(entry_bb);
    
    // Map parameters to local variables
    for (i, param) in function.get_param_iter().enumerate() {
        let param_name = format!("param_{}", i);
        let alloca = self.builder.build_alloca(self.i64_type(), &param_name)?;
        self.builder.build_store(alloca, param)?;
        self.variables.insert(param_name, alloca);
    }
    
    // Compile function body
    // ... compile instructions ...
    
    // Store function for later calls
    self.functions.insert(name.to_string(), function);
    
    Ok(())
}
```

#### 2.2: Real Function Calls
**File:** `crates/matter-llvm/src/lib.rs`

Replace stub in `compile_call()`:
```rust
fn compile_call(&mut self, arg_count: usize) -> Result<(), String> {
    // Pop function name/identifier
    let func_name = /* get function name from stack or constant */;
    
    // Pop arguments
    let mut args = Vec::new();
    for _ in 0..arg_count {
        args.push(self.stack.pop()?);
    }
    args.reverse();
    
    // Lookup function
    let function = self.functions.get(func_name)
        .ok_or_else(|| format!("Function '{}' not found", func_name))?;
    
    // Build call
    let call_result = self.builder.build_call(
        *function,
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

#### 2.3: Return Values
**File:** `crates/matter-llvm/src/lib.rs`

Replace stub in `compile_return()`:
```rust
fn compile_return(&mut self) -> Result<(), String> {
    let return_value = self.stack.pop()?;
    self.builder.build_return(Some(&return_value))?;
    Ok(())
}
```

**Test:**
```bash
cargo test test_function_call
cargo test test_return_value
```

---

### Phase 3: Implement CLI Commands (HIGH PRIORITY)

**Current Problem:**
- CLI commands documented but not coded
- No `show-ir` command
- No `compile-native` command
- No `run-native` command

**What to Implement:**

#### 3.1: show-ir Command
**File:** `crates/matter-cli/src/main.rs`

Add to match statement (around line 150):
```rust
"show-ir" => {
    if args.len() < 3 {
        eprintln!("Usage: matter show-ir <file.matter>");
        process::exit(1);
    }
    show_ir(&args[2]);
}
```

Add function:
```rust
fn show_ir(file: &str) {
    // Read source
    let source = fs::read_to_string(file)
        .unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        });
    
    // Parse
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    
    // Compile to bytecode
    let mut builder = BytecodeBuilder::new();
    builder.compile(&program).unwrap();
    let bytecode = builder.build();
    
    // Generate LLVM IR
    let ir = matter_llvm::get_llvm_ir(&bytecode).unwrap();
    
    // Print IR
    println!("{}", ir);
}
```

#### 3.2: compile-native Command
**File:** `crates/matter-cli/src/main.rs`

Add to match statement:
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
```

Add function:
```rust
fn compile_native(file: &str, output: &str) {
    // Read and parse
    let source = fs::read_to_string(file).unwrap();
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    
    // Compile to bytecode
    let mut builder = BytecodeBuilder::new();
    builder.compile(&program).unwrap();
    let bytecode = builder.build();
    
    // Compile to native
    matter_llvm::compile_to_native(&bytecode, output).unwrap();
    
    println!("Compiled to: {}", output);
}
```

#### 3.3: run-native Command
**File:** `crates/matter-cli/src/main.rs`

Add to match statement:
```rust
"run-native" => {
    if args.len() < 3 {
        eprintln!("Usage: matter run-native <file.matter>");
        process::exit(1);
    }
    run_native(&args[2]);
}
```

Add function:
```rust
fn run_native(file: &str) {
    // Compile to temporary executable
    let temp_output = ".matter_temp_output";
    compile_native(file, temp_output);
    
    // Run executable
    let output = std::process::Command::new(format!("./{}.exe", temp_output))
        .output()
        .expect("Failed to run");
    
    // Print output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    
    // Clean up
    std::fs::remove_file(format!("{}.exe", temp_output)).ok();
}
```

**Test:**
```bash
matter show-ir examples/hello.matter
matter compile-native examples/hello.matter -o hello
matter run-native examples/hello.matter
```

---

### Phase 4: Create Test Program (VALIDATION)

**File:** `examples/sprint25_test.matter`

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

**Test:**
```bash
matter run-native examples/sprint25_test.matter
```

---

### Phase 5: Final Validation (CRITICAL)

**Run all validation checks:**

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
./test.exe
matter run-native examples/sprint25_test.matter
```

**All must succeed before declaring complete.**

---

## 🎯 Definition of Done

**Sprint 25 is complete when:**

- [x] Phase 1: LLVM IR Generation (100%) ✅
- [ ] LLVM 17 installed and verified ❌
- [ ] `cargo build -p matter-llvm` succeeds ❌
- [ ] `cargo test -p matter-llvm` passes ❌
- [ ] Function definitions implemented ❌
- [ ] Real function calls implemented ❌
- [ ] Parameter passing works ❌
- [ ] Return values work ❌
- [ ] CLI `show-ir` command works ❌
- [ ] CLI `compile-native` command works ❌
- [ ] CLI `run-native` command works ❌
- [ ] Test program compiles ❌
- [ ] Test program runs ❌
- [ ] Output is correct (60) ❌

**Current:** 1/14 ✅  
**Target:** 14/14 ✅

---

## 📅 Realistic Timeline

**Day 1:** Install LLVM, validate Phase 1  
**Day 2-3:** Implement functions (definitions, calls, parameters, returns)  
**Day 4:** Implement CLI commands  
**Day 5:** Create test program, validate end-to-end  

**Total:** 5 days of real work

---

## 🚨 Rules

1. **Install LLVM first** - Nothing works without it
2. **Validate before continuing** - Run tests after each change
3. **No celebration until validation passes** - Tests must pass
4. **Be honest about status** - Partial is partial, not complete
5. **No Sprint 26 until Sprint 25 is done** - Finish what we started

---

## 💡 Priority Order

1. 🔥 **CRITICAL:** Install LLVM 17
2. 🔥 **CRITICAL:** Validate Phase 1 builds and tests
3. 🔥 **HIGH:** Implement function definitions
4. 🔥 **HIGH:** Implement real function calls
5. 🔥 **HIGH:** Implement CLI commands
6. 🔥 **CRITICAL:** Create and run test program
7. 🔥 **CRITICAL:** Validate everything works

---

## 📝 Files to Modify

1. `crates/matter-llvm/src/lib.rs` - Add function implementation
2. `crates/matter-cli/src/main.rs` - Add CLI commands
3. `examples/sprint25_test.matter` - Create test program

---

## ✅ Success Metric

**When this works, Sprint 25 is complete:**

```bash
matter run-native examples/sprint25_test.matter
# Output: 60
```

**Not before.**

---

*Sprint 25 Next Steps*  
*Date: 10 de Maio de 2026*  
*Status: 65% → 100%*  
*ETA: 5 days with proper validation*  
*SEM MEDIOCRIDADE - Real work only*
