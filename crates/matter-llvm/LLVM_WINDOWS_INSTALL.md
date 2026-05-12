# LLVM 17 Installation Guide for Windows

**Date:** 10 de Maio de 2026  
**Target:** Windows 10/11  
**LLVM Version:** 17.0.6  

---

## 🎯 Quick Install (Recommended)

### Method 1: Official Pre-built Binary (Easiest)

1. **Download LLVM 17.0.6 for Windows:**
   - Go to: https://github.com/llvm/llvm-project/releases/tag/llvmorg-17.0.6
   - Download: `LLVM-17.0.6-win64.exe` (approximately 350 MB)

2. **Run the Installer:**
   ```cmd
   LLVM-17.0.6-win64.exe
   ```
   - ✅ Check "Add LLVM to the system PATH for all users"
   - ✅ Install to default location: `C:\Program Files\LLVM`

3. **Set Environment Variable:**
   ```cmd
   setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
   ```

4. **Restart Terminal** (important!)

5. **Verify Installation:**
   ```cmd
   llvm-config --version
   ```
   Expected output: `17.0.6`

---

## 🔧 Alternative Methods

### Method 2: Chocolatey

```cmd
choco install llvm --version=17.0.6
setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
```

### Method 3: Scoop

```cmd
scoop install llvm@17.0.6
setx LLVM_SYS_170_PREFIX "%USERPROFILE%\scoop\apps\llvm\17.0.6"
```

---

## ✅ Verification Steps

### Step 1: Check LLVM Version
```cmd
llvm-config --version
```
**Expected:** `17.0.6`

### Step 2: Check Environment Variable
```cmd
echo %LLVM_SYS_170_PREFIX%
```
**Expected:** `C:\Program Files\LLVM`

### Step 3: Check PATH
```cmd
where llvm-config
```
**Expected:** `C:\Program Files\LLVM\bin\llvm-config.exe`

### Step 4: Test Compilation
```cmd
cd "f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
cargo build -p matter-llvm
```
**Expected:** Build succeeds without errors

---

## 🚨 Troubleshooting

### Problem 1: "llvm-config not found"

**Solution:**
```cmd
# Add to PATH manually
setx PATH "%PATH%;C:\Program Files\LLVM\bin"
```

### Problem 2: "LLVM_SYS_170_PREFIX not set"

**Solution:**
```cmd
setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
```

### Problem 3: "Could not find LLVM"

**Solution:**
1. Check installation directory exists:
   ```cmd
   dir "C:\Program Files\LLVM"
   ```
2. If not found, reinstall LLVM
3. Make sure to restart terminal after setting environment variables

### Problem 4: "Wrong LLVM version"

**Solution:**
```cmd
# Uninstall current version
# Install LLVM 17.0.6 specifically
# Verify version:
llvm-config --version
```

---

## 📋 Post-Installation Checklist

- [ ] LLVM 17.0.6 installed
- [ ] `llvm-config --version` returns `17.0.6`
- [ ] `LLVM_SYS_170_PREFIX` environment variable set
- [ ] LLVM bin directory in PATH
- [ ] Terminal restarted
- [ ] `cargo build -p matter-llvm` succeeds
- [ ] `cargo test -p matter-llvm` passes

---

## 🔗 Resources

- **LLVM Releases:** https://github.com/llvm/llvm-project/releases
- **LLVM Documentation:** https://llvm.org/docs/
- **inkwell (Rust LLVM bindings):** https://github.com/TheDan64/inkwell

---

## 🚀 Next Steps

After successful installation:

1. **Build Matter LLVM Backend:**
   ```cmd
   cargo build -p matter-llvm
   ```

2. **Run Tests:**
   ```cmd
   cargo test -p matter-llvm
   ```

3. **Test Full Workspace:**
   ```cmd
   cargo test --workspace
   ```

4. **Continue Sprint 25 Implementation:**
   - Implement real function calls
   - Implement CLI commands
   - Create test programs
   - Validate end-to-end

---

*LLVM 17 Windows Installation Guide*  
*For Matter Core v0.15-dev*  
*Sprint 25: LLVM Backend*
