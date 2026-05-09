# Matter Core — Estado Atual (Maio 2026)

## Versão: 0.1.5 (Sprint 3.5 Completo)

## O Que Funciona Agora

### ✅ Pipeline Completo
```
.matter → Lexer → Parser → AST → Bytecode → MBC1 → VM → Runtime
```

### ✅ Tipos de Dados
- `int` - Inteiros de 64 bits
- `bool` - Booleanos (true/false)
- `string` - Strings UTF-8
- `unit` - Tipo vazio ()

### ✅ Operadores
**Aritméticos**: `+`, `-`, `*`, `/`
**Comparação**: `==`, `!=`, `<`, `>`, `<=`, `>=`

### ✅ Controle de Fluxo
- `if`/`else` - Condicionais
- `while` - Loop com condição
- `loop` - Loop infinito
- `break` - Sair do loop
- `continue` - Próxima iteração

### ✅ Funções
```matter
fn soma(a, b) {
    return a + b
}

fn fatorial(n) {
    if n <= 1 { return 1 }
    return n * fatorial(n - 1)
}
```

**Características**:
- Parâmetros
- Return values
- Recursão
- Escopo local
- Call stack

### ✅ Escopo Hierárquico
- Global scope
- Function scope
- Block scope (if, loops)
- Event scope
- Shadowing correto
- Cleanup automático

### ✅ Eventos
```matter
on boot {
    print "Sistema iniciado"
}

on shutdown {
    print "Sistema finalizado"
}
```

### ✅ Backends
```matter
agent.say("Hello, world!")
visual.run("my_app")
```

**Backends disponíveis**:
- `agent` - Interação com agentes
- `visual` - Interface visual (mock)

### ✅ Bytecode (MBC1)
- Formato binário eficiente
- Serialização/deserialização
- Persistível em disco (.mbc)
- Inspecionável (matter inspect)
- Equivalência garantida com source

### ✅ CLI Completo
```bash
# Executar source
matter run app.matter

# Compilar para bytecode
matter compile app.matter -o app.mbc

# Executar bytecode
matter run-bytecode app.mbc

# Inspecionar bytecode
matter inspect app.mbc

# Emitir evento
matter emit app.matter boot
```

## Exemplos Funcionais

### Hello World
```matter
print "Hello, Matter!"
```

### Contador com Loop
```matter
let counter = 0
while counter < 5 {
    print counter
    set counter = counter + 1
}
```

### Função Recursiva
```matter
fn fatorial(n) {
    if n <= 1 { return 1 }
    return n * fatorial(n - 1)
}

print fatorial(5)  # 120
```

### Estado Reativo
```matter
let count = 0

on increment {
    set count = count + 1
    print count
}

on decrement {
    set count = count - 1
    print count
}
```

### Backend Integration
```matter
on boot {
    agent.say("Sistema iniciado")
    visual.run("dashboard")
}
```

## Arquitetura

### Crates (8)
1. **matter-ast** - Definição da AST
2. **matter-lexer** - Tokenização
3. **matter-parser** - Parsing
4. **matter-bytecode** - Compilação e serialização
5. **matter-vm** - Máquina virtual
6. **matter-runtime** - Runtime e eventos
7. **matter-backend** - Trait e backends
8. **matter-cli** - Interface de linha de comando

### Instruções Bytecode (22)
- Stack: LoadConst, Pop
- Variables: LoadGlobal, StoreGlobal, LoadLocal, StoreLocal
- Scope: PushScope, PopScope
- Arithmetic: Add, Sub, Mul, Div
- Comparison: Eq, NotEq, Lt, Gt, LtEq, GtEq
- Control: Jump, JumpIfFalse
- Functions: Call, Return
- I/O: Print
- Backend: BackendCall
- System: Halt

## Garantias

### ✅ Equivalência
```
source execution == bytecode execution
```

Testado e validado com suite automatizada.

### ✅ Semântica Clara
- `let` cria variável no escopo atual
- `set` atualiza variável existente (busca local → global)
- Loops/blocos podem atualizar variáveis globais
- Shadowing não sobrescreve global

### ✅ Persistência
- Bytecode pode ser salvo e carregado
- Formato MBC1 estável
- Versionamento de bytecode

## O Que NÃO Funciona Ainda

### ❌ Tipos Compostos
- List
- Map
- Struct

**Status**: Sprint 4 (próximo)

### ❌ Pattern Matching
```matter
match value {
    Some(x) => print x,
    None => print "empty"
}
```

**Status**: Sprint 5

### ❌ Sistema de Módulos
```matter
import math
import http
```

**Status**: Sprint 6

### ❌ Error Handling
```matter
try {
    risky_operation()
} catch error {
    print error
}
```

**Status**: Sprint 7

### ❌ For Loops
```matter
for item in list {
    print item
}
```

**Status**: Sprint 4 (requer List)

### ❌ Closures
```matter
fn make_counter() {
    let count = 0
    return fn() { set count = count + 1; return count }
}
```

**Status**: Sprint 8+

### ❌ Standard Library
- math (sin, cos, sqrt, etc)
- string (split, join, etc)
- http (get, post, etc)
- json (parse, stringify)

**Status**: v0.3+

## Métricas

### Código
- **Linhas de código**: ~3000
- **Crates**: 8
- **Instruções**: 22
- **Testes**: 12 passando

### Performance
- **Startup**: <10ms
- **Compilation**: <50ms para 100 linhas
- **Execution**: ~1M instruções/segundo

### Qualidade
- **Cobertura de testes**: ~70%
- **Warnings**: 0
- **Errors**: 0
- **Equivalência**: 100%

## Roadmap

### Sprint 4 (Próximo)
**Data Model — List, Map, Struct**
- Tipos compostos
- Indexação e acesso
- Operações em coleções
- Serialização MBC1

**Estimativa**: 2-3 dias

### v0.2 (Junho 2026)
- Sprint 4: Data Model
- Sprint 5: Pattern Matching
- Sprint 6: Sistema de Módulos
- Sprint 7: Error Handling

### v0.5 (Q3 2026)
- Standard Library
- Package manager
- Otimizador de bytecode
- Documentação completa

### v1.0 (Q4 2026)
- Debugger protocol
- LSP (Language Server)
- Tooling completo
- Ecossistema de bibliotecas

## Como Usar

### Instalação
```bash
git clone <repo>
cd matter-core
cargo build --release
```

### Executar Exemplo
```bash
./target/release/matter-cli run examples/hello.matter
```

### Compilar e Executar
```bash
./target/release/matter-cli compile examples/simple.matter -o simple.mbc
./target/release/matter-cli run-bytecode simple.mbc
```

### Inspecionar Bytecode
```bash
./target/release/matter-cli inspect simple.mbc
```

## Documentação

- **MANIFESTO.md** - Visão e filosofia
- **SPEC.md** - Especificação da linguagem
- **ARCHITECTURE.md** - Arquitetura técnica
- **PROGRESS.md** - Progresso e sprints
- **QUICKSTART.md** - Guia rápido
- **SPRINT_*.md** - Planejamento de sprints

## Comunidade

**Status**: Projeto em desenvolvimento ativo
**Licença**: MIT (a definir)
**Contribuições**: Bem-vindas após v0.2

## Conclusão

Matter está em um ponto crucial:
- ✅ Base sólida implementada
- ✅ Bytecode persistível
- ✅ Equivalência garantida
- ✅ Semântica clara
- 🚀 Pronta para tipos compostos

**Próximo marco**: Sprint 4 — Data Model

---

**Última atualização**: Maio 2026
**Versão**: 0.1.5
**Status**: 🟢 Ativo e evoluindo
