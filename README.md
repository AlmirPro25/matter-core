# Matter Core

**Um runtime-oriented language system com eventos nativos e backends desacoplados.**

Matter não é apenas uma linguagem — é um sistema completo de **linguagem + runtime + eventos + backends**.

## 🎯 O que torna Matter diferente?

### Eventos como Primitiva da Linguagem
```matter
on boot {
    print "Sistema iniciado"
}

on tap {
    agent.say("Olá!")
}
```
Eventos não são biblioteca — são parte do DNA da linguagem.

### Backends Desacoplados
```matter
agent.say("Processando...")
visual.run("animation")
db.save({ user: "Alice" })
```
Backends são interfaces plugáveis para o mundo externo.

### Bytecode Persistente (próximo marco)
```bash
matter compile app.matter -o app.mbc
matter run-bytecode app.mbc
```
Distribuição de aplicações sem source code.

## 📊 Status do Projeto

✅ **v0.1.5 - Sistema Estável** (Maio 2026)
- Pipeline completo: Source → Lexer → Parser → AST → Bytecode → VM → Runtime
- Funções com recursão
- Hierarquia de escopo completa
- Loops (while, loop, for, break, continue)
- Data Model completo (List, Map, Struct)
- Sistema de eventos
- Backends mock
- **38 testes passando (100%)** ✅
- **22 testes de integração end-to-end** ✅
- **Sistema de erros estruturado** ✅

🔄 **v0.2 - Próximo Marco** (Junho 2026)
- Sprint 6: Standard Library (math, string)
- Sprint 7: REPL Interativo
- Sprint 8: Error Integration
- Sprint 9: Performance Optimization

## 🚀 Quick Start

### Instalação

**Método Rápido (Windows):**

```powershell
# 1. Abra PowerShell como Administrador
# 2. Navegue até a pasta do projeto
cd "caminho\para\matter-core"

# 3. Execute o instalador
.\install.ps1

# 4. Feche e abra um novo terminal
# 5. Teste
matter --help
```

**Método Manual:**

```bash
# Clonar o repositório
git clone <repo-url>
cd matter-core

# Compilar em modo release
cargo build --release

# Executável estará em: target/release/matter-cli.exe
```

### Após Instalação

```bash
# Executar arquivo Matter
matter run meu_programa.matter

# Compilar para bytecode
matter compile programa.matter -o programa.mbc

# Executar bytecode
matter run-bytecode programa.mbc

# Ver exemplos
cd "C:\Program Files\Matter\examples"
matter run hello.matter
```

## 📝 Sintaxe Matter

### Variáveis e Estado

```matter
let x = 10
set x = x + 1
print x
```

### Condicionais

```matter
if x > 5 {
    print "maior que 5"
}

if x == 10 {
    print "exatamente 10"
} else {
    print "não é 10"
}
```

### Funções (em desenvolvimento)

```matter
fn soma(a, b) {
    return a + b
}

let resultado = soma(10, 20)
```

### Eventos

```matter
on boot {
    print "Sistema iniciado"
}

on shutdown {
    print "Desligando..."
}
```

### Backend Calls

```matter
agent.say("Olá do Matter!")
visual.run("pizzaria")
```

## 🏗️ Arquitetura

```
Source Code (.matter)
    ↓
Lexer (tokenização)
    ↓
Parser (construção AST)
    ↓
AST (representação estrutural)
    ↓
Bytecode Builder (compilação MBC)
    ↓
MBC Binary (bytecode Matter)
    ↓
Matter VM (execução stack-based)
    ↓
Runtime (eventos, estado)
    ↓
Backends (visual, agent, etc)
```

## 📦 Estrutura do Projeto

```
matter-core/
├── crates/
│   ├── matter-lexer/      # Análise léxica (tokens)
│   ├── matter-parser/     # Análise sintática (AST)
│   ├── matter-ast/        # Definições da AST
│   ├── matter-bytecode/   # Compilador para MBC
│   ├── matter-vm/         # Máquina virtual stack-based
│   ├── matter-runtime/    # Sistema de eventos e estado
│   ├── matter-backend/    # Contratos de backend
│   └── matter-cli/        # Interface de linha de comando
├── examples/              # Exemplos de código Matter
├── docs/                  # Documentação completa
└── Cargo.toml            # Workspace Rust
```

## ✅ Funcionalidades Implementadas

### v0.1.5 - Sistema Estável ✅
**Conquistas:**
- ✅ Pipeline completo (9 crates modulares)
- ✅ Funções com recursão e call frames
- ✅ Hierarquia de escopo (Global → Event → Function → Block)
- ✅ Loops (while, loop, for, break, continue)
- ✅ Data Model completo (List, Map, Struct)
- ✅ Operadores completos (+, -, *, /, ==, !=, <, >, <=, >=)
- ✅ Sistema de eventos nativos
- ✅ Backends desacoplados
- ✅ Bytecode MBC1 (persistível em disco)
- ✅ CLI funcional (20+ comandos)
- ✅ Sistema de erros estruturado
- ✅ 38 testes unitários passando (100%)
- ✅ 22 testes de integração end-to-end
- ✅ Validação semântica robusta

### Tipos de Dados
- ✅ `int` - Inteiros de 64 bits
- ✅ `bool` - Booleanos (true/false)
- ✅ `string` - Strings UTF-8
- ✅ `unit` - Tipo vazio ()
- ✅ `list` - Listas dinâmicas
- ✅ `map` - Mapas chave-valor
- ✅ `struct` - Estruturas de dados

### Controle de Fluxo
- ✅ `if`/`else` - Condicionais
- ✅ `while` - Loop com condição
- ✅ `loop` - Loop infinito
- ✅ `for` - Iteração em coleções
- ✅ `break` - Sair do loop
- ✅ `continue` - Próxima iteração
- ✅ `fn` - Definição de funções
- ✅ `return` - Retorno de função

### Sistema de Eventos
- ✅ `on <event>` - Event handlers
- ✅ Event dispatch via CLI
- ✅ Event scope isolado

### Backends
- ✅ Trait `Backend` genérica
- ✅ `AgentBackend` (mock)
- ✅ `VisualBackend` (mock)
- ✅ `TraceBackend` (debug)
- ✅ Backend calls: `backend.method(args)`

## 🔜 Próximos Passos

### Sprint 3.5: MBC1 Persistence 🔥 (CRÍTICO)
**Objetivo:** Bytecode em disco

- [ ] Serialização de Bytecode → arquivo .mbc
- [ ] Desserialização de arquivo .mbc → Bytecode
- [ ] `matter compile app.matter -o app.mbc`
- [ ] `matter run-bytecode app.mbc`
- [ ] `matter inspect app.mbc`

**Por quê é crítico:** Separa "protótipo" de "linguagem real". Permite distribuição de aplicações.

### Sprint 4: Data Model
- [ ] List type
- [ ] Map type
- [ ] Struct type
- [ ] For loops com iteração

### Sprint 5: Error System
- [ ] MatterError type estruturado
- [ ] Stack traces
- [ ] Line/column tracking

### Sprint 6: REPL
- [ ] `matter repl` command
- [ ] Interactive shell
- [ ] History e autocomplete

Ver `PROGRESS.md` e `STRATEGIC_VISION.md` para roadmap completo.

## 🎯 Princípios Fundamentais

### 1. Runtime-Oriented Language System
Matter não é apenas uma linguagem — é linguagem + runtime + eventos + backends integrados.

### 2. Eventos como Primitiva
Eventos são parte do DNA da linguagem, não biblioteca externa.

### 3. Backends Desacoplados
Interfaces plugáveis para diferentes domínios (IA, UI, dados, etc).

### 4. Bytecode Persistente
MBC1 permite distribuição de aplicações sem source code.

### 5. Simplicidade Pragmática
Escolhas práticas sobre pureza teórica (ex: Reference Counting > Ownership).

### 6. Estado Mutável Nativo
Estado é cidadão de primeira classe, não efeito colateral.

### 7. Modularidade por Design
Arquitetura permite crescimento sem reescrever tudo.

### 8. IA-Friendly
Sintaxe otimizada para geração por IA/LLM.

## 🌍 Comparação com Outras Linguagens

**vs Python:** Eventos nativos, bytecode próprio, backends desacoplados  
**vs JavaScript:** Sintaxe mais limpa, eventos como primitiva  
**vs Rust:** Mais simples, sem borrow checker, prototipagem rápida  
**vs Erlang/Elixir:** Sintaxe mais familiar, backends flexíveis, VM própria

**Posicionamento:** Aplicações reativas, prototipagem rápida, integração com IA/LLM, sistemas orientados a eventos.

## 📚 Documentação

- [MANIFESTO.md](docs/MANIFESTO.md) - Princípios e filosofia
- [SPEC.md](docs/SPEC.md) - Especificação completa da linguagem
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - Arquitetura técnica detalhada
- [STRATEGIC_VISION.md](STRATEGIC_VISION.md) - Visão estratégica e roadmap
- [PROGRESS.md](PROGRESS.md) - Progresso e sprints
- [SPRINT_3.5.md](SPRINT_3.5.md) - MBC1 Persistence (próximo marco)

## 🧪 Executar Testes

```bash
cargo test
```

## 🔧 Desenvolvimento

```bash
# Compilar em modo debug
cargo build

# Executar com cargo
cargo run --bin matter-cli -- run examples/simple.matter

# Verificar warnings
cargo clippy

# Formatar código
cargo fmt
```

## 📊 Estatísticas do Projeto

- **9 crates** organizados em workspace
- **30+ instruções** de bytecode
- **18 exemplos** funcionais
- **38 testes** passando (100%)
- **22 testes de integração** end-to-end
- **Sprints completos:** 5 (Funções, Scopes, Loops, Data Model, Error System)
- **Arquitetura limpa** com separação rígida
- **Zero dependências externas** (apenas std)
- **Cobertura de testes:** ~75%

## 🎨 Exemplos Completos

### Recursão
```matter
fn fatorial(n) {
    if n <= 1 { return 1 }
    return n * fatorial(n - 1)
}

print fatorial(5)  # 120
```

### Loops
```matter
let i = 0
while i < 5 {
    print i
    set i = i + 1
}

loop {
    if i >= 10 { break }
    set i = i + 1
}
```

### Shadowing
```matter
let x = 10
if true {
    let x = 20
    print x  # 20
}
print x  # 10
```

Ver pasta `examples/` para mais exemplos funcionais.

## 🤝 Contribuindo

Este é um projeto em desenvolvimento ativo. Contribuições são bem-vindas!

## 📄 Licença

MIT

---

**Matter não é apenas uma linguagem. É um runtime-oriented language system.**

Ver `STRATEGIC_VISION.md` para entender o que isso significa.
