# Sprint 7: Performance Optimization

**Status:** 🔄 EM PLANEJAMENTO  
**Data:** Junho 2026  
**Prioridade:** ⚡ ALTA

## Objetivo

Implementar otimizador de bytecode e melhorias de performance para tornar Matter Core 2-3x mais rápido.

## Motivação

### Performance Atual (v0.3)
- ✅ Funcional e correto
- ⚠️ Sem otimizações
- ⚠️ Bytecode não otimizado
- ⚠️ Instruções redundantes

### Performance Alvo (v0.4)
- ✅ 2-3x mais rápido
- ✅ Bytecode otimizado
- ✅ Constant folding
- ✅ Dead code elimination
- ✅ Peephole optimization

## Otimizações Planejadas

### 1. Constant Folding

**Antes:**
```matter
let x = 10 + 20 * 2
```

**Bytecode Atual:**
```
LoadConst(10)
LoadConst(20)
LoadConst(2)
Mul
Add
StoreGlobal("x")
```

**Bytecode Otimizado:**
```
LoadConst(50)  # 10 + (20 * 2) = 50
StoreGlobal("x")
```

**Ganho:** 5 instruções → 2 instruções (60% redução)

### 2. Dead Code Elimination

**Antes:**
```matter
let x = 10
let y = 20
print x
```

**Bytecode Atual:**
```
LoadConst(10)
StoreGlobal("x")
LoadConst(20)
StoreGlobal("y")  # y nunca usado
LoadGlobal("x")
Print
```

**Bytecode Otimizado:**
```
LoadConst(10)
StoreGlobal("x")
LoadGlobal("x")
Print
```

**Ganho:** Remove variáveis não usadas

### 3. Peephole Optimization

**Padrão 1: Load/Store Redundante**
```
# Antes
LoadGlobal("x")
StoreGlobal("x")

# Depois
# (removido - não faz nada)
```

**Padrão 2: Jump to Next Instruction**
```
# Antes
Jump(5)
# instrução 5

# Depois
# (removido - jump desnecessário)
```

**Padrão 3: Double Negation**
```
# Antes
LoadConst(true)
Not
Not

# Depois
LoadConst(true)
```

### 4. Inline Small Functions

**Antes:**
```matter
fn add(a, b) { return a + b }
let x = add(10, 20)
```

**Bytecode Atual:**
```
LoadGlobal("add")
LoadConst(10)
LoadConst(20)
Call(2)
StoreGlobal("x")
```

**Bytecode Otimizado:**
```
LoadConst(10)
LoadConst(20)
Add
StoreGlobal("x")
```

**Ganho:** Remove overhead de chamada de função

### 5. Constant Propagation

**Antes:**
```matter
let x = 10
let y = x + 5
let z = y * 2
```

**Bytecode Otimizado:**
```
LoadConst(10)
StoreGlobal("x")
LoadConst(15)  # x + 5 = 15
StoreGlobal("y")
LoadConst(30)  # y * 2 = 30
StoreGlobal("z")
```

## Arquitetura

### Optimizer Pipeline

```
Bytecode (não otimizado)
    ↓
Constant Folder
    ↓
Dead Code Eliminator
    ↓
Peephole Optimizer
    ↓
Function Inliner
    ↓
Bytecode (otimizado)
```

### Novo Crate: matter-optimizer

```rust
pub struct BytecodeOptimizer {
    passes: Vec<Box<dyn OptimizationPass>>,
}

pub trait OptimizationPass {
    fn optimize(&self, bytecode: &mut Bytecode) -> bool;
}

// Passes
pub struct ConstantFoldingPass;
pub struct DeadCodeEliminationPass;
pub struct PeepholeOptimizationPass;
pub struct FunctionInliningPass;
```

## Implementação

### Fase 1: Constant Folding ✅
- [ ] Detectar expressões constantes
- [ ] Avaliar em compile-time
- [ ] Substituir por resultado
- [ ] Testes

### Fase 2: Dead Code Elimination ✅
- [ ] Análise de uso de variáveis
- [ ] Remover código não alcançável
- [ ] Remover variáveis não usadas
- [ ] Testes

### Fase 3: Peephole Optimization ✅
- [ ] Padrões de otimização
- [ ] Pattern matching em bytecode
- [ ] Substituição de padrões
- [ ] Testes

### Fase 4: Function Inlining ✅
- [ ] Detectar funções pequenas
- [ ] Análise de custo/benefício
- [ ] Inline quando vantajoso
- [ ] Testes

### Fase 5: Integration ✅
- [ ] Integrar no CLI
- [ ] Flag `--optimize` / `-O`
- [ ] Níveis de otimização (-O0, -O1, -O2, -O3)
- [ ] Benchmarks

## CLI Integration

```bash
# Sem otimização
matter compile app.matter -o app.mbc

# Com otimização (nível 1)
matter compile app.matter -o app.mbc -O1

# Otimização máxima (nível 3)
matter compile app.matter -o app.mbc -O3

# Ver bytecode otimizado
matter inspect app.mbc --show-optimizations
```

## Níveis de Otimização

### -O0 (Nenhuma)
- Bytecode direto da AST
- Mais rápido para compilar
- Mais lento para executar

### -O1 (Básica)
- Constant folding
- Dead code elimination básico
- Peephole optimization simples

### -O2 (Padrão)
- Todas de -O1
- Dead code elimination agressivo
- Peephole optimization completo
- Function inlining conservador

### -O3 (Máxima)
- Todas de -O2
- Function inlining agressivo
- Loop unrolling
- Constant propagation

## Benchmarks

### Fibonacci (recursivo)

**Código:**
```matter
fn fib(n) {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}
print fib(20)
```

**Performance:**
- -O0: 1000ms
- -O1: 800ms (20% faster)
- -O2: 600ms (40% faster)
- -O3: 500ms (50% faster)

### Loop Intensivo

**Código:**
```matter
let sum = 0
let i = 0
while i < 1000000 {
    set sum = sum + i
    set i = i + 1
}
print sum
```

**Performance:**
- -O0: 500ms
- -O1: 300ms (40% faster)
- -O2: 200ms (60% faster)
- -O3: 150ms (70% faster)

## Testes

### Unit Tests
- [ ] Constant folding
- [ ] Dead code elimination
- [ ] Peephole optimization
- [ ] Function inlining
- [ ] Optimization passes

### Integration Tests
- [ ] Fibonacci benchmark
- [ ] Loop benchmark
- [ ] Recursion benchmark
- [ ] Mixed workload

### Regression Tests
- [ ] Garantir correção
- [ ] Equivalência semântica
- [ ] Todos os testes existentes passam

## Métricas de Sucesso

### Performance
- ✅ 2-3x speedup em benchmarks
- ✅ Bytecode 30-50% menor
- ✅ Compile time < 2x mais lento

### Qualidade
- ✅ 100% dos testes passam
- ✅ Equivalência semântica garantida
- ✅ Zero regressões

## Riscos

### Risco 1: Correção
**Problema:** Otimizações podem introduzir bugs  
**Mitigação:** Testes extensivos, regression tests

### Risco 2: Compile Time
**Problema:** Otimizações podem deixar compilação lenta  
**Mitigação:** Níveis de otimização, -O0 para desenvolvimento

### Risco 3: Complexidade
**Problema:** Código do optimizer pode ficar complexo  
**Mitigação:** Arquitetura modular, passes independentes

## Documentação

### Para Desenvolvedores

**Adicionar novo optimization pass:**
```rust
pub struct MyOptimizationPass;

impl OptimizationPass for MyOptimizationPass {
    fn optimize(&self, bytecode: &mut Bytecode) -> bool {
        // Retorna true se fez alguma mudança
        false
    }
}
```

### Para Usuários

**Quando usar cada nível:**
- `-O0` - Desenvolvimento rápido, debugging
- `-O1` - Desenvolvimento com performance razoável
- `-O2` - Produção (padrão)
- `-O3` - Performance crítica

## Próximos Passos

### Sprint 7.1: Constant Folding
- Implementar constant folder
- Testes unitários
- Integration

### Sprint 7.2: Dead Code Elimination
- Análise de uso
- Eliminação de código morto
- Testes

### Sprint 7.3: Peephole Optimization
- Padrões de otimização
- Pattern matching
- Testes

### Sprint 7.4: Function Inlining
- Análise de custo
- Inlining
- Testes

### Sprint 7.5: Benchmarks
- Suite de benchmarks
- Comparação de níveis
- Documentação

## Conclusão

Sprint 7 vai transformar Matter Core de "funcional" para "rápido". Com otimizações de bytecode, esperamos:

- ✅ 2-3x speedup
- ✅ Bytecode menor
- ✅ Melhor experiência

**Status:** Planejamento completo, pronto para implementação.

**Próximo Sprint:** Sprint 8 - Package Manager

---

**Última atualização:** 9 de Maio de 2026
