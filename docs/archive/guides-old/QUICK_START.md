# Matter Core - Quick Start Guide

**Welcome to Matter Core!** 🚀

This guide will get you up and running in 10 minutes.

---

## 📋 Prerequisites

- **Rust:** 1.70+ ([Install](https://rustup.rs/))
- **Git:** For cloning the repository
- **LLVM 17:** For native compilation (optional but recommended)

---

## 🚀 Installation

### Step 1: Clone Repository

```bash
git clone https://github.com/your-org/matter-core.git
cd matter-core
```

### Step 2: Build Project

```bash
cargo build --release
```

### Step 3: Install CLI (Optional)

```bash
cargo install --path crates/matter-cli
```

Or add to PATH:
```bash
# Windows
$env:PATH += ";$PWD\target\release"

# Linux/Mac
export PATH="$PWD/target/release:$PATH"
```

---

## 🎯 Your First Matter Program

### Create a file: `hello.matter`

```matter
print("Hello, Matter!")

let x = 10
let y = 20
let sum = x + y

print(sum)
```

### Run it:

```bash
matter run hello.matter
```

**Output:**
```
Hello, Matter!
30
```

---

## 🔥 Try More Features

### Functions

```matter
fn add(a, b) {
    return a + b
}

fn multiply(a, b) {
    return a * b
}

let result = multiply(add(5, 3), 2)
print(result)  // Output: 16
```

### Loops

```matter
// While loop
let i = 0
while i < 5 {
    print(i)
    i = i + 1
}

// For loop
let numbers = [1, 2, 3, 4, 5]
for num in numbers {
    print(num)
}
```

### Data Structures

```matter
// Lists
let fruits = ["apple", "banana", "orange"]
print(fruits[0])  // apple

// Maps
let person = {
    name: "Alice",
    age: 30
}
print(person.name)  // Alice

// Structs
struct Point {
    x: int,
    y: int
}

let p = Point { x: 10, y: 20 }
print(p.x)  // 10
```

### Events

```matter
on boot {
    print("System started!")
}

on tap {
    print("Button tapped!")
}
```

---

## ⚡ Native Compilation (10-100x Faster)

### Install LLVM 17

**Windows:**
1. Download: [LLVM 17.0.6](https://github.com/llvm/llvm-project/releases/tag/llvmorg-17.0.6)
2. Run installer (check "Add to PATH")
3. Set environment variable:
   ```cmd
   setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
   ```
4. Restart terminal

**Linux:**
```bash
sudo apt install llvm-17 llvm-17-dev
```

**macOS:**
```bash
brew install llvm@17
```

### Compile to Native

```bash
# Show LLVM IR
matter show-ir hello.matter

# Compile to executable
matter compile-native hello.matter -o hello

# Run native executable
./hello  # or hello.exe on Windows

# Or compile and run in one step
matter run-native hello.matter
```

### Benchmark Performance

```bash
matter benchmark hello.matter --iterations 10
```

**Expected output:**
```
=== Matter Benchmark ===
File: hello.matter
Iterations: 10

✓ Bytecode execution:
  Average: 1.234ms
  Min:     1.100ms
  Max:     1.500ms

✓ Native execution:
  Average: 0.012ms
  Min:     0.010ms
  Max:     0.015ms

=== Results ===
Speedup: 102.83x faster
🚀 Excellent! Native is significantly faster.
```

---

## 📚 CLI Commands

### Basic Commands

```bash
# Run source file
matter run <file.matter>

# Evaluate expression
matter eval "print(10 + 20)"

# Check syntax
matter check <file.matter>

# Compile to bytecode
matter compile <file.matter> -o output.mbc

# Run bytecode
matter run-bytecode output.mbc
```

### LLVM Commands (requires LLVM 17)

```bash
# Show LLVM IR
matter show-ir <file.matter>

# Compile to native
matter compile-native <file.matter> -o output

# Run native
matter run-native <file.matter>

# Benchmark
matter benchmark <file.matter> [--iterations N]
```

### Development Commands

```bash
# Interactive REPL
matter repl

# Format code
matter format <file.matter> --write

# Lint code
matter lint <file.matter>

# Debug
matter debug <file.matter>

# Language Server
matter lsp
```

### Memory Commands

```bash
# Show GC stats
matter gc-stats <file.matter>

# Force GC
matter gc-collect <file.matter>

# Profile memory
matter gc-profile <file.matter>
```

---

## 🧪 Examples

The repository includes 60+ examples:

```bash
# Basic examples
matter run examples/hello.matter
matter run examples/functions.matter
matter run examples/loops.matter

# Data structures
matter run examples/lists.matter
matter run examples/maps.matter
matter run examples/structs.matter

# Advanced
matter run examples/recursion.matter
matter run examples/events.matter
matter run examples/concurrency.matter

# Applications
matter run examples/apps/calculator.matter
matter run examples/apps/todo.matter
```

---

## 🔧 Development

### Run Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p matter-vm
cargo test -p matter-llvm

# With output
cargo test -- --nocapture
```

### Build Documentation

```bash
cargo doc --open
```

### Format Code

```bash
cargo fmt
```

### Lint Code

```bash
cargo clippy
```

---

## 📖 Learn More

### Documentation

- **Manifesto:** `docs/MANIFESTO.md` - Philosophy and vision
- **Spec:** `docs/SPEC.md` - Language specification
- **Roadmap:** `ROADMAP_2026.md` - Project roadmap
- **Examples:** `examples/README.md` - Example programs

### Sprints

- **Sprint 25:** LLVM Backend (current)
- **Sprint 26:** JIT Compilation (next)
- See `docs/` for all sprint documentation

### Architecture

```
Source Code (.matter)
    ↓
Lexer (tokens)
    ↓
Parser (AST)
    ↓
Bytecode Compiler
    ↓
Optimizer (4 passes)
    ↓
┌─────────────┬──────────────┬─────────────┐
│  Bytecode   │   LLVM IR    │  WebAssembly│
│  (VM)       │  (Native)    │  (Browser)  │
└─────────────┴──────────────┴─────────────┘
```

---

## 🐛 Troubleshooting

### "LLVM not found"

Install LLVM 17 and set `LLVM_SYS_170_PREFIX`:
```bash
setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
```

### "Command not found: matter"

Add to PATH or use:
```bash
cargo run -p matter-cli -- <command>
```

### Build Errors

```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Test Failures

```bash
# Run specific test
cargo test <test_name> -- --nocapture

# Show test output
cargo test -- --show-output
```

---

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test --workspace`
5. Format code: `cargo fmt`
6. Submit a pull request

See `CONTRIBUTING.md` for details.

---

## 📊 Project Status

**Current Version:** v0.15.0-dev  
**Current Sprint:** Sprint 25 (80% Complete)  
**Tests:** 101 passing (100%)  
**Crates:** 24 modules  
**Examples:** 60+ programs  

---

## 🎯 Next Steps

1. ✅ Try the examples
2. ✅ Write your first program
3. ✅ Install LLVM for native compilation
4. ✅ Benchmark your code
5. ✅ Read the documentation
6. ✅ Join the community

---

## 📞 Support

- **Issues:** [GitHub Issues](https://github.com/your-org/matter-core/issues)
- **Discussions:** [GitHub Discussions](https://github.com/your-org/matter-core/discussions)
- **Discord:** [Join our server](https://discord.gg/matter-core)

---

## 📄 License

MIT License - See `LICENSE` file for details.

---

**Welcome to Matter Core! Let's build something amazing together.** 🚀

---

*Quick Start Guide*  
*Last Updated: 10 de Maio de 2026*  
*Version: v0.15.0-dev*
