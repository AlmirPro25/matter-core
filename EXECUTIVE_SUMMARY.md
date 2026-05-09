# Matter Core - Executive Summary

## O que é Matter?

**Matter é um runtime-oriented language system.**

Não é "mais uma linguagem de programação".

É um sistema completo de:
- **Linguagem** (sintaxe, semântica)
- **Runtime** (execução, estado, lifecycle)
- **Eventos** (primitivas nativas, não biblioteca)
- **Backends** (interfaces desacopladas)

## Por que Matter existe?

### Problema
Linguagens tradicionais tratam eventos como biblioteca externa.
Você precisa importar, configurar, gerenciar manualmente.

### Solução Matter
Eventos são **primitivas da linguagem**.

```matter
on boot {
    agent.say("Sistema iniciado")
}

on tap {
    visual.run("animation")
}
```

Isso não é biblioteca. É sintaxe nativa.

## Diferencial Estratégico

### 1. Comportamento Reativo no DNA
Eventos não são add-on. São parte fundamental da linguagem.

### 2. Backends Desacoplados
```matter
agent.say("...")      # IA/LLM
visual.run("...")     # UI/Graphics
db.save(...)          # Persistência
http.get("...")       # Network
```

Backends são interfaces plugáveis. Fácil adicionar novos domínios.

### 3. Bytecode Persistente (próximo marco)
```bash
matter compile app.matter -o app.mbc
matter run-bytecode app.mbc
```

Distribuir aplicações sem source code.
Startup mais rápido (pular parsing).

### 4. VM Própria
Não depende de BEAM (Erlang), JVM (Java), ou V8 (JavaScript).

Stack-based VM com instruções próprias (MBC1).

## Comparação Rápida

| Aspecto | Python | JavaScript | Rust | Matter |
|---------|--------|------------|------|--------|
| Eventos nativos | ❌ | ❌ | ❌ | ✅ |
| Bytecode próprio | ✅ | ✅ | ❌ | ✅ |
| Backends desacoplados | ❌ | ❌ | ❌ | ✅ |
| Simplicidade | ✅ | ✅ | ❌ | ✅ |
| Ecossistema maduro | ✅ | ✅ | ✅ | ⏳ |

## Status Atual (Maio 2026)

### ✅ Completo (v0.1)
- Pipeline completo (Lexer → Parser → AST → Bytecode → VM → Runtime)
- Funções com recursão
- Hierarquia de escopo (Global → Event → Function → Block)
- Loops (while, loop, break, continue)
- Sistema de eventos
- Backends mock
- CLI funcional

### 🔄 Em Progresso
- Sprint 3: Loops (quase completo)
- Sprint 3.5: MBC1 Persistence (planejado)

### ⏳ Próximos 90 Dias
- **Maio:** Sprint 3.5 - MBC1 Persistence 🔥
- **Junho:** Sprint 4 (Data Model), Sprint 5 (Error System), Sprint 6 (REPL)
- **Julho:** Sprint 7 (Módulos), Sprint 8 (Stdlib), Sprint 9 (Otimizações)

## Marco Crítico: Sprint 3.5

### Por que MBC1 Persistence é crítico?

**Separa "protótipo" de "linguagem real".**

Hoje:
```bash
matter run app.matter  # compila e executa
```

Depois do Sprint 3.5:
```bash
matter compile app.matter -o app.mbc  # compilar uma vez
matter run-bytecode app.mbc           # executar N vezes
```

**Benefícios:**
1. **Distribuição** - Enviar apenas .mbc, não source
2. **Performance** - Pular parsing/compilation
3. **Caching** - Compilar apenas quando source mudar
4. **Base para package system** - Futuro `matter install`

## Arquitetura em Camadas

```
┌─────────────────────────────────────┐
│         Matter Source Code          │  .matter files
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│  Lexer → Parser → AST → Bytecode    │  Compilation
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│      MBC1 Bytecode File (.mbc)      │  Artifact ← SPRINT 3.5
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│         Matter VM + Runtime         │  Execution
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│           Backends Layer            │  External interfaces
└─────────────────────────────────────┘
```

## Decisões Arquiteturais

### Modelo de Memória: Reference Counting ✅

**Opções avaliadas:**
1. Garbage Collection (Python/JS)
2. Ownership (Rust) - ❌ Muito complexo
3. Reference Counting (Swift) - ✅ Escolhido

**Justificativa:**
- Pragmático para linguagem de alto nível
- Simples de implementar
- Determinístico
- Usado com sucesso em Swift e Python

**Implementação:** Sprint 8+

## Roadmap de Alto Nível

### Fase 1: Infraestrutura (Q2 2026)
**Objetivo:** Consolidar núcleo

- Sprint 3.5: MBC1 Persistence
- Sprint 4: Data Model (List, Map, Struct)
- Sprint 5: Error System
- Sprint 6: REPL

### Fase 2: Ecossistema (Q3 2026)
**Objetivo:** Tornar Matter utilizável

- Módulos e imports
- Package manager básico
- Standard library (math, string, http, json)
- Documentação completa

### Fase 3: Produção (Q4 2026)
**Objetivo:** Tornar Matter production-ready

- Otimizador de bytecode
- Debugger protocol
- LSP (Language Server)
- Tooling (formatter, linter)
- Performance benchmarks

## Métricas de Sucesso

### Técnicas
- ✅ Pipeline completo funcional
- ✅ Testes unitários passando
- 🔄 Bytecode persistente (Sprint 3.5)
- ⏳ Data model completo
- ⏳ Error system robusto
- ⏳ REPL funcional

### Adoção
- ⏳ 10+ exemplos práticos
- ⏳ Documentação completa
- ⏳ Tutorial interativo
- ⏳ Community feedback

### Performance
- ⏳ Benchmark suite
- ⏳ Otimizações básicas
- ⏳ Profiling tools

## Posicionamento

### Matter é para:
- ✅ Aplicações reativas
- ✅ Prototipagem rápida
- ✅ Integração com IA/LLM
- ✅ Sistemas orientados a eventos
- ✅ Backends customizados

### Matter não é para:
- ❌ Sistemas de baixo nível
- ❌ Performance crítica (ainda)
- ❌ Aplicações legacy
- ❌ Substituir linguagens estabelecidas

## Próxima Ação Imediata

**Sprint 3.5: MBC1 Persistence**

**Entregáveis:**
1. Serialização de Bytecode → arquivo .mbc
2. Desserialização de arquivo .mbc → Bytecode
3. CLI: `matter compile`
4. CLI: `matter run-bytecode`
5. CLI: `matter inspect`
6. Testes de round-trip
7. Documentação do formato

**Estimativa:** 2-3 dias

**Prioridade:** 🔥 CRÍTICA

Este é o marco que transforma Matter de protótipo em linguagem real.

## Conclusão

Matter não é "mais uma linguagem".

É um **runtime-oriented language system** que:
- Trata eventos como primitivas
- Desacopla backends
- Tem VM e bytecode próprios
- Foca em reatividade e simplicidade

A base está sólida.
A arquitetura está correta.
A visão está clara.

**Agora o trabalho é transformar núcleo em ecossistema.**

---

**Versão:** 1.0  
**Data:** Maio 2026  
**Status:** 🟢 Visão consolidada

**Documentos relacionados:**
- `STRATEGIC_VISION.md` - Análise completa
- `PROGRESS.md` - Progresso detalhado
- `SPRINT_3.5.md` - Próximo marco
- `README.md` - Overview técnico
