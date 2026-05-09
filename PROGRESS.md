# Matter Core - Progress Tracker

## Marco 1: Protótipo Funcional ✅
**Status:** COMPLETO
**Data:** Maio 2026

### Conquistas
- ✅ Pipeline completo (Source → Lexer → Parser → AST → Bytecode → VM → Runtime → Backends)
- ✅ 8 crates modulares
- ✅ CLI funcional (run, emit, compile)
- ✅ Eventos nativos
- ✅ Backends desacoplados
- ✅ Testes unitários passando

---

## Sprint 1: Funções Robustas ✅
**Status:** COMPLETO
**Data:** Maio 2026

### Objetivo
Fazer funções funcionarem corretamente com parâmetros, retorno e recursão.

### Implementado
- ✅ Call frames com locals
- ✅ Stack de chamadas
- ✅ Binding de argumentos
- ✅ Local scope dentro de funções
- ✅ Return values
- ✅ Recursão funcional

### Testes
```matter
fn soma(a, b) { return a + b }
soma(10, 20)  # 30 ✅

fn fatorial(n) {
    if n <= 1 { return 1 }
    return n * fatorial(n - 1)
}
fatorial(5)  # 120 ✅
```

### Mudanças Técnicas
1. **CallFrame structure** - Adicionado para gerenciar locals
2. **call_stack** - Stack de frames para recursão
3. **LoadLocal/StoreLocal** - Instruções para variáveis locais
4. **compile_function_statement** - Compilação específica para corpo de função
5. **compile_function_expression** - Resolução de escopo em funções

---

## Sprint 2: Hierarquia de Escopo ✅
**Status:** COMPLETO
**Data:** Maio 2026

### Objetivo
Implementar hierarquia completa de escopo com shadowing correto.

### Implementado
- ✅ ScopeFrame structure com tipo de escopo
- ✅ Scope stack para hierarquia
- ✅ Block scope (if, nested blocks)
- ✅ Function scope
- ✅ Event scope
- ✅ Global scope
- ✅ Shadowing sem sobrescrever global
- ✅ Cleanup automático ao sair do bloco
- ✅ Lookup hierárquico (Block → Function → Event → Global)

### Testes
```matter
let x = 10
fn test() { let x = 20; print x }  # 20 ✅
test()
print x  # 10 ✅

let y = 1
if true {
    let y = 2; print y  # 2 ✅
    if true { let y = 3; print y }  # 3 ✅
    print y  # 2 ✅
}
print y  # 1 ✅
```

### Mudanças Técnicas
1. **ScopeFrame** - Frame de escopo com variables HashMap
2. **ScopeType enum** - Global, Event, Function, Block
3. **scope_stack** - Stack de scopes para hierarquia
4. **PushScope/PopScope** - Instruções para gerenciar blocos
5. **Lookup hierárquico** - Busca do mais interno para o mais externo
6. **Cleanup automático** - Pop de scope destrói variáveis

---

## Sprint 3: Loops ✅
**Status:** COMPLETO
**Data:** Maio 2026

### Objetivo
Implementar estruturas de repetição completas.

### Implementado
- ✅ While loop com condition check
- ✅ Loop infinito
- ✅ Break statement
- ✅ Continue statement
- ✅ Loop context stack para nested loops
- ✅ Jump patching para break/continue

### Mudanças Técnicas
1. **LoopContext** - Stack de contextos para break/continue
2. **Jump patching** - Resolver jumps após compilar loop body
3. **PushScope/PopScope** - Scopes automáticos em loops

---

## Sprint 3.5: MBC1 Persistence ✅
**Status:** COMPLETO
**Data:** Maio 2026
**Prioridade:** 🔥 CRÍTICA

### Objetivo
**Transformar bytecode de artefato em memória para artefato em disco.**

Este é o marco que separa "protótipo funcional" de "linguagem real".

### Implementado
- ✅ Serialização de Bytecode (formato MBC1)
- ✅ Desserialização de Bytecode
- ✅ `matter compile` command
- ✅ `matter run-bytecode` command
- ✅ `matter inspect` command (com visualização detalhada)
- ✅ Testes de round-trip
- ✅ Teste de equivalência (source == bytecode)
- ✅ **Bug fix crítico**: Semântica de `StoreGlobal`

### Entregáveis
```bash
matter compile app.matter -o app.mbc
matter run-bytecode app.mbc
matter inspect app.mbc  # Mostra bytecode formatado
```

### Aprendizado Chave
O bug de loop infinito revelou a diferença entre `StoreLocal` e `StoreGlobal`, clarificando a semântica de escopo:
- `let` = cria variável no escopo atual
- `set` = atualiza variável existente (busca local → global)
- `StoreGlobal` = **sempre** armazena no global (essencial para loops/eventos)

### Validação
- ✅ Todos os testes passam: loops, functions, recursion, simple
- ✅ Equivalência garantida: source execution == bytecode execution
- ✅ Bytecode persistível e inspecionável

### Por quê foi crítico
- ✅ Permite distribuição de aplicações
- ✅ Habilita caching e otimização
- ✅ Base para package system futuro
- ✅ Separa "protótipo" de "linguagem real"

---

## Sprint 4: Data Model �
**Status:** EM PLANEJAMENTO
**Prioridade:** ALTA

### Objetivo
Implementar tipos compostos (List, Map, Struct).

### Tarefas
- [ ] Implementar List type (sintaxe, parser, bytecode, VM)
- [ ] Implementar Map type
- [ ] Implementar Struct type
- [ ] Operações em coleções (push, pop, len, etc)
- [ ] Indexação e acesso ([], .)
- [ ] Serialização MBC1 para tipos compostos
- [ ] Testes de equivalência

### Por quê é importante
- Permite modelar estado real de aplicações
- Habilita estruturas de dados complexas
- Prepara terreno para pattern matching
- Completa o conjunto mínimo de tipos para v0.2

**Ver:** `SPRINT_4.md` para detalhes completos

---

## Sprint 5: Error System 🔴
**Status:** PLANEJADO

### Objetivo
Implementar sistema de erros estruturado.

### Tarefas
- [ ] Criar MatterError type
- [ ] Stack traces
- [ ] Line/column tracking
- [ ] Error propagation
- [ ] Try/catch (futuro)

---

## Sprint 6: REPL 🔴
**Status:** PLANEJADO
**Prioridade:** ALTA

### Objetivo
Criar shell interativo para Matter.

### Tarefas
- [ ] Implementar `matter repl` command
- [ ] Multi-line input
- [ ] History
- [ ] Autocomplete básico
- [ ] Pretty printing

---

## Sprint 7: Sistema de Módulos 🔴
**Status:** PLANEJADO

### Objetivo
Permitir importação de módulos.

### Tarefas
- [ ] Implementar `import`
- [ ] Resolver paths de módulos
- [ ] Namespace management
- [ ] Módulos padrão (math, etc)

---

## Decisão Arquitetural: Modelo de Memória

### Opções Avaliadas
1. **Garbage Collection** (estilo Python/JS)
2. **Ownership** (estilo Rust) - ❌ Muito complexo
3. **Reference Counting** (estilo Swift) - ✅ Recomendado

### Decisão: Reference Counting + Cycle Detection
**Justificativa:**
- Pragmático para linguagem de alto nível
- Simples de implementar
- Determinístico
- Permite evolução futura
- Usado com sucesso em Swift e Python

**Implementação:** Sprint 8+

---

## Métricas Atuais

### Código
- **Crates:** 8
- **Linhas de código:** ~2500
- **Instruções bytecode:** 22
- **Testes:** 8 passando

### Funcionalidades
- **Tipos:** int, bool, string, unit
- **Operadores:** +, -, *, /, ==, !=, <, >, <=, >=
- **Controle:** if/else, funções, recursão
- **Eventos:** on boot, on shutdown, etc
- **Backends:** agent, visual (mock)

### Exemplos Funcionais
- ✅ hello.matter
- ✅ simple.matter
- ✅ showcase.matter
- ✅ backend.matter
- ✅ events.matter
- ✅ state.matter
- ✅ test_functions.matter
- ✅ test_recursion.matter

---

## Próximos Marcos

### Marco 2: Linguagem Completa (v0.2)
**Target:** Junho 2026
- [ ] MBC1 Persistence (Sprint 3.5) 🔥
- [ ] Data Model - List, Map, Struct (Sprint 4)
- [ ] Error System estruturado (Sprint 5)
- [ ] REPL interativo (Sprint 6)
- [ ] Sistema de módulos básico (Sprint 7)

### Marco 3: Ecossistema (v0.5)
**Target:** Q3 2026
- [ ] Standard Library (math, string, http, json)
- [ ] Package manager básico
- [ ] Documentação completa
- [ ] Otimizador de bytecode

### Marco 4: Produção (v1.0)
**Target:** Q4 2026
- [ ] Debugger protocol
- [ ] LSP (Language Server)
- [ ] Tooling completo (formatter, linter)
- [ ] Performance benchmarks
- [ ] Ecossistema de bibliotecas

---

**Última atualização:** Maio 2026
**Status geral:** 🟢 No caminho certo

## Visão Estratégica

Matter não é "mais uma linguagem".

É um **runtime-oriented language system** com:
- ✅ Eventos nativos no DNA da linguagem
- ✅ Backends desacoplados
- ✅ VM própria com bytecode MBC1
- 🔄 Persistência de bytecode (próximo marco)

**Diferencial:** Comportamento reativo como primitiva, não como biblioteca.

Mais próximo de Erlang/Elixir, mas com VM própria e foco em backends flexíveis.

**Próximo passo crítico:** Sprint 3.5 - MBC1 Persistence

Ver `STRATEGIC_VISION.md` para análise completa.
