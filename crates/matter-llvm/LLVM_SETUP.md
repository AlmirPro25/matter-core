# LLVM Setup Guide for Matter Core

## Windows Installation

### Option 1: Pre-built Binaries (Recommended)

1. Download LLVM 17.0 from: https://github.com/llvm/llvm-project/releases/tag/llvmorg-17.0.6
2. Download: `LLVM-17.0.6-win64.exe`
3. Run the installer
4. **IMPORTANT:** Check "Add LLVM to system PATH" during installation
5. Set environment variable:
   ```cmd
   setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
   ```
6. Restart your terminal/IDE

### Option 2: Chocolatey

```powershell
choco install llvm --version=17.0.6
setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
```

### Option 3: Build from Source (Advanced)

```powershell
# Install CMake and Visual Studio Build Tools first
git clone --depth 1 --branch llvmorg-17.0.6 https://github.com/llvm/llvm-project.git
cd llvm-project
mkdir build
cd build
cmake -G "Visual Studio 17 2022" -A x64 -DLLVM_ENABLE_PROJECTS="clang" ../llvm
cmake --build . --config Release
setx LLVM_SYS_170_PREFIX "path\to\llvm-project\build\Release"
```

## Verification

After installation, verify LLVM is accessible:

```cmd
llvm-config --version
# Should output: 17.0.6 or similar
```

Test the Matter LLVM backend:

```cmd
cargo test -p matter-llvm
```

## Troubleshooting

### Error: "No suitable version of LLVM was found"

**Solution:**
1. Ensure LLVM 17.0 is installed
2. Set `LLVM_SYS_170_PREFIX` environment variable
3. Restart terminal/IDE
4. Run `cargo clean` and rebuild

### Error: "llvm-config not found"

**Solution:**
1. Add LLVM bin directory to PATH:
   ```cmd
   setx PATH "%PATH%;C:\Program Files\LLVM\bin"
   ```
2. Restart terminal

### Error: "linking with `link.exe` failed"

**Solution:**
1. Install Visual Studio Build Tools
2. Ensure MSVC toolchain is installed:
   ```cmd
   rustup toolchain install stable-msvc
   rustup default stable-msvc
   ```

## Linux Installation

### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install llvm-17 llvm-17-dev clang-17
export LLVM_SYS_170_PREFIX=/usr/lib/llvm-17
```

### Fedora/RHEL

```bash
sudo dnf install llvm17 llvm17-devel clang17
export LLVM_SYS_170_PREFIX=/usr/lib64/llvm17
```

## macOS Installation

### Homebrew

```bash
brew install llvm@17
export LLVM_SYS_170_PREFIX=/opt/homebrew/opt/llvm@17
```

## Environment Variables

Add to your shell profile (`.bashrc`, `.zshrc`, etc.):

```bash
export LLVM_SYS_170_PREFIX=/path/to/llvm-17
export PATH=$LLVM_SYS_170_PREFIX/bin:$PATH
```

## Next Steps

Once LLVM is installed:

1. Build Matter LLVM backend:
   ```cmd
   cargo build -p matter-llvm
   ```

2. Run tests:
   ```cmd
   cargo test -p matter-llvm
   ```

3. Try compiling a Matter program:
   ```cmd
   cargo run -- compile-native examples/hello.matter -o hello
   ```

## Resources

- LLVM Official: https://llvm.org/
- LLVM Releases: https://github.com/llvm/llvm-project/releases
- inkwell Documentation: https://thedan64.github.io/inkwell/
- llvm-sys Documentation: https://docs.rs/llvm-sys/

---

**Note:** LLVM installation is required for Sprint 25. The Matter Core project will continue to work with bytecode interpretation even without LLVM installed.
