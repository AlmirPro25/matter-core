# Sprint 2 - Scope Hierarchy

## Objetivo
Implementar hierarquia completa de escopo com shadowing correto.

## Hierarquia
```
Block Scope (if, while, nested blocks)
    ↓
Function Scope (parâmetros e locals)
    ↓
Event Scope (dentro de on)
    ↓
Global Scope (top-level)
```

## Regras de Shadowing
- Variável local pode shadowing global sem sobrescrever
- Ao sair do bloco, variável local é destruída
- Global permanece intacta

## Exemplo Esperado
```matter
let total = 10

fn test() {
    let total = 20  # shadowing
    print total     # 20
}

test()
print total         # 10 (global intacta)
```

## Implementação

### 1. Scope Stack
```rust
struct ScopeFrame {
    scope_type: ScopeType,
    variables: HashMap<String, Value>,
}

enum ScopeType {
    Global,
    Event,
    Function,
    Block,
}
```

### 2. Lookup Hierárquico
- Busca do escopo mais interno para o mais externo
- Primeiro match retorna
- Se não encontrar, erro

### 3. Block Scope
- `if` cria novo scope
- `while` cria novo scope
- Blocos aninhados criam scopes aninhados

### 4. Cleanup
- Ao sair do bloco, pop do scope stack
- Variáveis do bloco são destruídas

## Testes

### Teste 1: Shadowing Básico
```matter
let x = 10
fn test() {
    let x = 20
    print x  # 20
}
test()
print x  # 10
```

### Teste 2: Nested Blocks
```matter
let x = 1
if true {
    let x = 2
    print x  # 2
    if true {
        let x = 3
        print x  # 3
    }
    print x  # 2
}
print x  # 1
```

### Teste 3: Function + Block
```matter
let x = 1
fn test() {
    let x = 2
    if true {
        let x = 3
        print x  # 3
    }
    print x  # 2
}
test()
print x  # 1
```

### Teste 4: Event Scope
```matter
let x = 1
on boot {
    let x = 2
    print x  # 2
}
print x  # 1
```

### Teste 5: Global Fallback
```matter
let x = 10
fn test() {
    print x  # 10 (fallback para global)
}
test()
```

## Status
✅ **COMPLETO**

Hierarquia de escopo implementada com sucesso:
- ✅ Block Scope (if, nested blocks)
- ✅ Function Scope (parâmetros e locals)
- ✅ Event Scope (dentro de on)
- ✅ Global Scope (top-level)
- ✅ Shadowing correto
- ✅ Cleanup automático ao sair do bloco
- ✅ Lookup hierárquico (Block → Function → Event → Global)

## Testes Passando

### Shadowing Básico ✅
```matter
let x = 10
fn test() { let x = 20; print x }  # 20
test()
print x  # 10
```

### Nested Blocks ✅
```matter
let y = 1
if true {
    let y = 2; print y  # 2
    if true { let y = 3; print y }  # 3
    print y  # 2
}
print y  # 1
```

### Function + Block ✅
```matter
let z = 1
fn test() {
    let z = 2
    if true { let z = 3; print z }  # 3
    print z  # 2
}
test()
print z  # 1
```

### Global Fallback ✅
```matter
let global = 100
fn test() { print global }  # 100
test()
```

## Próximo Sprint
Sprint 3: Loops (while, for, loop, break, continue)
