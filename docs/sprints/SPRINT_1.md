# Sprint 1 - Funções Robustas

## Objetivo
Fazer `fn soma(a, b) { return a + b }` funcionar corretamente com:
- Stack frames
- Local scope
- Argumentos posicionais
- Return value
- Call stack
- Recursion-safe structure

## Problema Atual
A VM atual usa apenas globals. Não há:
- Stack frames para chamadas de função
- Local variables
- Argument binding
- Proper call stack

## Implementação

### 1. Call Frame Structure
```rust
struct CallFrame {
    function_name: String,
    locals: HashMap<String, Value>,
    return_address: usize,
}
```

### 2. VM com Call Stack
```rust
struct Vm {
    stack: Vec<Value>,           // value stack
    call_stack: Vec<CallFrame>,  // call frames
    globals: HashMap<String, Value>,
    bytecode: Bytecode,
    backends: HashMap<String, Box<dyn Backend>>,
}
```

### 3. Instruções Adicionais
- `LoadLocal(name)` - Carrega variável local
- `StoreLocal(name)` - Armazena variável local
- `SetupFrame(param_count)` - Cria novo frame
- `CleanupFrame` - Remove frame

### 4. Compilação de Funções
- Parâmetros viram locals no frame
- Variáveis dentro da função são locals
- Return limpa o frame e coloca valor na stack

## Testes
```matter
fn soma(a, b) {
    return a + b
}

let resultado = soma(10, 20)
print resultado  # deve imprimir 30

fn fatorial(n) {
    if n <= 1 {
        return 1
    }
    return n * fatorial(n - 1)
}

print fatorial(5)  # deve imprimir 120
```

## Status
✅ **COMPLETO**

Funções agora funcionam corretamente com:
- ✅ Stack frames
- ✅ Local scope
- ✅ Argumentos posicionais
- ✅ Return value
- ✅ Call stack
- ✅ Recursion-safe structure

## Testes Passando
```matter
fn soma(a, b) { return a + b }
soma(10, 20)  # 30 ✅

fn fatorial(n) {
    if n <= 1 { return 1 }
    return n * fatorial(n - 1)
}
fatorial(5)  # 120 ✅
```

## Próximo Sprint
Sprint 2: Corrigir escopo (local/function/event/global hierarchy)
