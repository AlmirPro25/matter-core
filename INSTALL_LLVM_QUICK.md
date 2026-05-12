# LLVM 17 - Guia de Instalação Rápida

**Objetivo:** Instalar LLVM 17 para validar Sprint 25 (LLVM Backend)

**Tempo Estimado:** 30 minutos

---

## 🎯 Por Que Instalar LLVM 17?

**LLVM 17 é necessário para:**
- ✅ Buildar o crate `matter-llvm`
- ✅ Rodar testes do LLVM backend
- ✅ Validar implementações
- ✅ Medir performance (10-100x speedup)
- ✅ Testar otimizações (-O0 a -O3)
- ✅ Completar Sprint 25 (90% → 100%)

**Sem LLVM 17:**
- ❌ Não pode buildar `matter-llvm`
- ❌ Não pode validar código
- ❌ Não pode medir performance
- ❌ Sprint 25 fica bloqueado em 90%

---

## 📥 Passo 1: Download

### Windows

**Link Direto:**
https://github.com/llvm/llvm-project/releases/download/llvmorg-17.0.6/LLVM-17.0.6-win64.exe

**Ou via GitHub Releases:**
1. Ir para: https://github.com/llvm/llvm-project/releases/tag/llvmorg-17.0.6
2. Procurar: `LLVM-17.0.6-win64.exe`
3. Download (~400 MB)

### Linux (Ubuntu/Debian)

```bash
wget https://github.com/llvm/llvm-project/releases/download/llvmorg-17.0.6/clang+llvm-17.0.6-x86_64-linux-gnu-ubuntu-22.04.tar.xz
tar xf clang+llvm-17.0.6-x86_64-linux-gnu-ubuntu-22.04.tar.xz
sudo mv clang+llvm-17.0.6-x86_64-linux-gnu-ubuntu-22.04 /usr/local/llvm-17
```

### macOS

```bash
brew install llvm@17
```

---

## 🔧 Passo 2: Instalação

### Windows

1. **Executar o instalador:** `LLVM-17.0.6-win64.exe`

2. **Durante a instalação:**
   - ✅ **IMPORTANTE:** Marcar "Add LLVM to the system PATH for all users"
   - ✅ Aceitar o diretório padrão: `C:\Program Files\LLVM`
   - ✅ Completar a instalação

3. **Aguardar:** ~5 minutos

### Linux

```bash
# Adicionar ao PATH
echo 'export PATH="/usr/local/llvm-17/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### macOS

```bash
# Adicionar ao PATH
echo 'export PATH="/opt/homebrew/opt/llvm@17/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

---

## ⚙️ Passo 3: Configuração

### Windows

**Abrir PowerShell como Administrador:**

```powershell
# Definir variável de ambiente
setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM" /M

# Ou se não tiver permissão de admin:
setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"
```

### Linux/macOS

```bash
# Adicionar ao .bashrc ou .zshrc
echo 'export LLVM_SYS_170_PREFIX="/usr/local/llvm-17"' >> ~/.bashrc
source ~/.bashrc

# Ou para macOS com Homebrew:
echo 'export LLVM_SYS_170_PREFIX="/opt/homebrew/opt/llvm@17"' >> ~/.zshrc
source ~/.zshrc
```

---

## ✅ Passo 4: Verificação

### Fechar e Reabrir o Terminal

**IMPORTANTE:** Fechar todos os terminais e abrir um novo para carregar as variáveis de ambiente.

### Verificar Instalação

```bash
# Verificar versão do LLVM
llvm-config --version
```

**Saída Esperada:**
```
17.0.6
```

### Verificar Variável de Ambiente

**Windows:**
```powershell
echo $env:LLVM_SYS_170_PREFIX
```

**Linux/macOS:**
```bash
echo $LLVM_SYS_170_PREFIX
```

**Saída Esperada:**
```
C:\Program Files\LLVM  (Windows)
/usr/local/llvm-17     (Linux)
/opt/homebrew/opt/llvm@17  (macOS)
```

---

## 🚀 Passo 5: Validação Matter Core

### Navegar para o Projeto

```bash
cd "f:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
```

### Rodar Script de Validação

**Windows:**
```powershell
.\validate_sprint25.ps1
```

**Linux/macOS:**
```bash
chmod +x validate_sprint25.sh
./validate_sprint25.sh
```

### O Que o Script Faz

1. ✅ Verifica instalação do LLVM
2. ✅ Formata código (`cargo fmt`)
3. ✅ Verifica workspace (`cargo check --workspace`)
4. ✅ Builda `matter-llvm` (`cargo build -p matter-llvm`)
5. ✅ Roda testes (`cargo test -p matter-llvm`)
6. ✅ Testa exemplos
7. ✅ Roda benchmarks
8. ✅ Mostra resumo

### Tempo Estimado

- **Primeira build:** 5-10 minutos
- **Testes:** 2-3 minutos
- **Total:** ~15 minutos

---

## 🎯 Passo 6: Testar Novas Features

### Testar Níveis de Otimização

```bash
# Debug build (sem otimização)
cargo run -p matter-cli --features llvm -- compile-native examples/sprint25_benchmark.matter -o bench_debug -O0

# Release build (máxima otimização)
cargo run -p matter-cli --features llvm -- compile-native examples/sprint25_benchmark.matter -o bench_release -O3

# Comparar performance
time ./bench_debug.exe
time ./bench_release.exe
```

### Testar Break/Continue

```bash
# Rodar com bytecode
cargo run -p matter-cli -- run examples/sprint25_break_continue.matter

# Rodar com native
cargo run -p matter-cli --features llvm -- run-native examples/sprint25_break_continue.matter

# Comparar outputs (devem ser idênticos)
```

### Testar Benchmark

```bash
# Comparar bytecode vs native
cargo run -p matter-cli --features llvm -- benchmark examples/sprint25_benchmark.matter --iterations 10
```

**Saída Esperada:**
```
=== Matter Benchmark ===
Bytecode: 1.234ms (avg)
Native:   0.012ms (avg)
Speedup:  102.83x faster
🚀 Excellent! Native is significantly faster.
```

---

## ❌ Troubleshooting

### Erro: "llvm-config not found"

**Solução:**
1. Verificar se LLVM está no PATH
2. Reiniciar terminal
3. Verificar instalação: `llvm-config --version`

### Erro: "LLVM_SYS_170_PREFIX not set"

**Solução:**
```bash
# Windows
setx LLVM_SYS_170_PREFIX "C:\Program Files\LLVM"

# Linux/macOS
export LLVM_SYS_170_PREFIX="/usr/local/llvm-17"
```

### Erro: "could not find native static library `LLVM-17`"

**Solução:**
1. Verificar que LLVM 17 está instalado (não 16 ou 18)
2. Verificar `LLVM_SYS_170_PREFIX` aponta para diretório correto
3. Reiniciar terminal

### Erro: Build muito lento

**Normal:** Primeira build do LLVM pode levar 5-10 minutos.

**Solução:** Aguardar. Builds subsequentes serão mais rápidas.

### Erro: Testes falhando

**Solução:**
1. Verificar que todos os exemplos existem
2. Rodar testes individuais: `cargo test -p matter-llvm test_name`
3. Verificar logs de erro
4. Reportar issue se persistir

---

## 📚 Documentação Adicional

### Guia Completo
- **Detalhado:** `crates/matter-llvm/LLVM_WINDOWS_INSTALL.md`
- **Status:** `CURRENT_STATUS.md`
- **Progresso:** `SPRINT_25_PROGRESS_REPORT.md`

### Guias de Uso
- **Otimização:** `OPTIMIZATION_QUICK_GUIDE.md`
- **Break/Continue:** `SPRINT_25_BREAK_CONTINUE_ANALYSIS.md`

---

## ✅ Checklist de Instalação

- [ ] Download LLVM 17.0.6
- [ ] Instalar com "Add to PATH"
- [ ] Configurar `LLVM_SYS_170_PREFIX`
- [ ] Reiniciar terminal
- [ ] Verificar `llvm-config --version` → 17.0.6
- [ ] Verificar `$LLVM_SYS_170_PREFIX` está definido
- [ ] Rodar `.\validate_sprint25.ps1`
- [ ] Todos os testes passam ✅
- [ ] Testar otimizações
- [ ] Testar break/continue
- [ ] Testar benchmark
- [ ] Sprint 25: 90% → 100% ✅

---

## 🎉 Sucesso!

**Quando todos os testes passarem:**

✅ LLVM 17 instalado e funcionando
✅ Matter LLVM backend validado
✅ Otimizações funcionando
✅ Break/continue confirmado
✅ Performance medida (10-100x speedup)
✅ Sprint 25: 100% COMPLETO

**Próximo passo:**
- Documentar resultados finais
- Iniciar Sprint 26 (JIT Compilation)
- Continuar para v1.0

---

**SEM MEDIOCRIDADE - Instale LLVM 17 e valide o poder do Matter Core!** 🚀

---

*LLVM 17 Quick Install Guide*  
*Date: 10 de Maio de 2026*  
*Version: v0.15.0-dev*  
*Sprint: 25 (90% → 100%)*  
*Next: Validate and complete*
