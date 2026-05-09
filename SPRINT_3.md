# Sprint 3 - Loops

## Objetivo
Implementar estruturas de repetição completas.

## Estruturas

### 1. While Loop
```matter
while condition {
    # body
}
```

### 2. Loop Infinito
```matter
loop {
    # body
    if condition {
        break
    }
}
```

### 3. For Loop (futuro - requer ranges)
```matter
for i in 0..10 {
    # body
}
```

## Controle de Fluxo

### Break
Sai do loop mais interno.

### Continue
Pula para próxima iteração.

## Implementação

### 1. AST
```rust
enum Statement {
    While { condition: Expression, body: Vec<Statement> },
    Loop { body: Vec<Statement> },
    Break,
    Continue,
}
```

### 2. Bytecode
```rust
enum Instruction {
    // Loop control
    Loop(usize),           // jump back to start
    BreakLoop(usize),      // jump to end
    ContinueLoop(usize),   // jump to condition check
}
```

### 3. Loop Context
- Stack de loop contexts para break/continue
- Cada loop registra seus jump targets

## Testes

### Teste 1: While Básico
```matter
let i = 0
while i < 5 {
    print i
    set i = i + 1
}
# Output: 0 1 2 3 4
```

### Teste 2: Loop com Break
```matter
let i = 0
loop {
    print i
    set i = i + 1
    if i >= 5 {
        break
    }
}
# Output: 0 1 2 3 4
```

### Teste 3: Continue
```matter
let i = 0
while i < 5 {
    set i = i + 1
    if i == 3 {
        continue
    }
    print i
}
# Output: 1 2 4 5
```

### Teste 4: Nested Loops
```matter
let i = 0
while i < 3 {
    let j = 0
    while j < 3 {
        print i * 10 + j
        set j = j + 1
    }
    set i = i + 1
}
# Output: 0 1 2 10 11 12 20 21 22
```

### Teste 5: Break em Nested
```matter
let i = 0
while i < 5 {
    if i == 3 {
        break
    }
    print i
    set i = i + 1
}
# Output: 0 1 2
```

## Status
🔴 Em desenvolvimento
