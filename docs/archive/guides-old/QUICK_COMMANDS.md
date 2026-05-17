# Quick Commands - Copy & Paste

**Sprint 25 Validation - Comandos Prontos**

---

## 🚀 Instalação LLVM 17

### 1. Download
```
https://github.com/llvm/llvm-project/releases/download/llvmorg-17.0.6/LLVM-17.0.6-win64.exe
```

### 2. Configurar (PowerShell como Admin)
```powershell
setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM" /M
```

### 3. Verificar
```powershell
llvm-config --version
echo $env:LLVM_SYS_170_PREFIX
```

---

## 🔧 Validação Automática

### Script de Validação
```powershell
cd "f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
.\validate_sprint25.ps1
```

### Validação Full Hardened (workspace)
```powershell
cd "f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
.\scripts\validate-full-workspace.ps1
```

### Validação Full com LLVM obrigatório
```powershell
.\scripts\validate-full-workspace.ps1 -RequireLLVM
```

### Validação Full com resumo JSON
```powershell
.\scripts\validate-full-workspace.ps1 -JsonSummary
```

### Preflight de Ambiente (rápido)
```powershell
.\scripts\preflight-env.ps1
```

### Validação Full com Preflight (fail-fast)
```powershell
.\scripts\validate-full-workspace.ps1 -RunPreflight -JsonSummary
```

### Build Base no F: (evitar pressão no C:)
```powershell
$env:MATTER_BUILD_BASE="F:\matter_core_build"
.\scripts\validate-full-workspace.ps1 -RunPreflight -JsonSummary
```

---

## 🧪 Testes Manuais

### Testar Exemplos com Bytecode
```bash
cargo run -p matter-cli -- run examples/sprint25_simple.matter
cargo run -p matter-cli -- run examples/sprint25_test.matter
cargo run -p matter-cli -- run examples/sprint25_loops.matter
cargo run -p matter-cli -- run examples/sprint25_benchmark.matter
cargo run -p matter-cli -- run examples/sprint25_break_continue.matter
```

### Testar Exemplos com Native
```bash
cargo run -p matter-cli --features llvm -- run-native examples/sprint25_simple.matter
cargo run -p matter-cli --features llvm -- run-native examples/sprint25_test.matter
cargo run -p matter-cli --features llvm -- run-native examples/sprint25_loops.matter
cargo run -p matter-cli --features llvm -- run-native examples/sprint25_benchmark.matter
cargo run -p matter-cli --features llvm -- run-native examples/sprint25_break_continue.matter
```

---

## ⚡ Testar Otimizações

### Compilar com Diferentes Níveis
```bash
# Debug (-O0)
cargo run -p matter-cli --features llvm -- compile-native examples/sprint25_benchmark.matter -o bench_o0 -O0

# Basic (-O1)
cargo run -p matter-cli --features llvm -- compile-native examples/sprint25_benchmark.matter -o bench_o1 -O1

# Balanced (-O2)
cargo run -p matter-cli --features llvm -- compile-native examples/sprint25_benchmark.matter -o bench_o2 -O2

# Maximum (-O3)
cargo run -p matter-cli --features llvm -- compile-native examples/sprint25_benchmark.matter -o bench_o3 -O3
```

### Executar e Comparar
```bash
time ./bench_o0.exe
time ./bench_o1.exe
time ./bench_o2.exe
time ./bench_o3.exe
```

---

## 📊 Benchmarks

### Benchmark Simples
```bash
cargo run -p matter-cli --features llvm -- benchmark examples/sprint25_simple.matter --iterations 10
```

### Benchmark Loops
```bash
cargo run -p matter-cli --features llvm -- benchmark examples/sprint25_loops.matter --iterations 10
```

### Benchmark Intensivo
```bash
cargo run -p matter-cli --features llvm -- benchmark examples/sprint25_benchmark.matter --iterations 10
```

### Benchmark Break/Continue
```bash
cargo run -p matter-cli --features llvm -- benchmark examples/sprint25_break_continue.matter --iterations 10
```

---

## 🔍 Verificações

### Build
```bash
cargo build -p matter-llvm
cargo build -p matter-cli --features llvm
cargo build --workspace
```

### Testes
```bash
cargo test -p matter-llvm
cargo test -p matter-cli
cargo test --workspace
```

### Formato e Check
```bash
cargo fmt
cargo check --workspace
cargo clippy --workspace
```

---

## 📝 Ver LLVM IR

### Mostrar IR Gerado
```bash
cargo run -p matter-cli --features llvm -- show-ir examples/sprint25_simple.matter
cargo run -p matter-cli --features llvm -- show-ir examples/sprint25_test.matter
```

---

## 🎯 Comandos Úteis

### Limpar Build
```bash
cargo clean
```

### Build Release
```bash
cargo build --release -p matter-llvm
cargo build --release -p matter-cli --features llvm
```

### Rodar Exemplo Específico
```bash
# Bytecode
cargo run -p matter-cli -- run examples/[ARQUIVO].matter

# Native
cargo run -p matter-cli --features llvm -- run-native examples/[ARQUIVO].matter

# Native com otimização
cargo run -p matter-cli --features llvm -- run-native examples/[ARQUIVO].matter -O3
```

---

## 🐛 Troubleshooting

### Verificar LLVM
```bash
llvm-config --version
llvm-config --prefix
llvm-config --libdir
```

### Verificar Variáveis de Ambiente
```powershell
echo $env:LLVM_SYS_170_PREFIX
echo $env:PATH
```

### Rebuild Completo
```bash
cargo clean
cargo build -p matter-llvm
cargo test -p matter-llvm
```

---

## 📚 Documentação

### Abrir Documentos
```bash
# Windows
start INSTALL_LLVM_QUICK.md
start VALIDATION_CHECKLIST.md
start OPTIMIZATION_QUICK_GUIDE.md
start examples/SPRINT25_README.md

# Linux/Mac
xdg-open INSTALL_LLVM_QUICK.md
```

---

## 🎉 Após Validação

### Atualizar Status
```bash
# Editar arquivos:
# - CURRENT_STATUS.md → 100%
# - README.md → Sprint 25 Complete
# - PROGRESS.md → Sprint 25: 100%
```

### Criar Relatório Final
```bash
# Criar: SPRINT_25_VALIDATION_RESULTS.md
# Documentar:
# - Speedups obtidos
# - Testes que passaram
# - Issues encontrados
# - Conclusões
```

---

## 🚀 Comandos Rápidos (One-Liners)

### Validação Completa
```bash
cd "f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE" && .\validate_sprint25.ps1
```

### Testar Tudo com Native
```bash
cargo run -p matter-cli --features llvm -- run-native examples/sprint25_simple.matter && cargo run -p matter-cli --features llvm -- run-native examples/sprint25_test.matter && cargo run -p matter-cli --features llvm -- run-native examples/sprint25_break_continue.matter
```

### Benchmark Tudo
```bash
cargo run -p matter-cli --features llvm -- benchmark examples/sprint25_simple.matter --iterations 10 && cargo run -p matter-cli --features llvm -- benchmark examples/sprint25_benchmark.matter --iterations 10
```

---

## 📊 Template de Resultados

### Copiar e Preencher
```
=== SPRINT 25 VALIDATION RESULTS ===

Data: _______________
LLVM Version: _______________

BUILD STATUS:
- matter-llvm build: ✅ / ❌
- matter-cli build: ✅ / ❌
- workspace build: ✅ / ❌

TEST STATUS:
- matter-llvm tests: ___/___
- workspace tests: ___/101
- All tests passing: ✅ / ❌

EXAMPLES:
- sprint25_simple: ✅ / ❌ (output: ___)
- sprint25_test: ✅ / ❌ (output: ___)
- sprint25_loops: ✅ / ❌ (output: ___)
- sprint25_benchmark: ✅ / ❌ (output: ___)
- sprint25_break_continue: ✅ / ❌ (output: ___)

PERFORMANCE:
- sprint25_simple: ___x speedup
- sprint25_test: ___x speedup
- sprint25_loops: ___x speedup
- sprint25_benchmark: ___x speedup
- sprint25_break_continue: ___x speedup

OPTIMIZATION LEVELS:
- O0 vs O3: ___x faster

ISSUES FOUND:
- ___________________
- ___________________

CONCLUSION:
Sprint 25: ✅ COMPLETE / ⏳ PENDING
```

---

**SEM MEDIOCRIDADE - Copy, paste, validate!** 🚀

---

*Quick Commands Reference*  
*Date: 10 de Maio de 2026*  
*Sprint: 25*  
*Purpose: Fast validation*
