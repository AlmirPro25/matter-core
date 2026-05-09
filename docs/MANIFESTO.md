# MANIFESTO DA LINGUAGEM MATTER CORE

## Princípio 1 — Linguagem é infraestrutura

Matter não é uma DSL.
Matter é infraestrutura universal de execução.

Ela deve poder construir:
- aplicações
- sistemas
- jogos
- automação
- agentes
- interfaces
- serviços
- runtimes

A linguagem não serve ao visual.
O visual é apenas um backend.

## Princípio 2 — Intenção antes de implementação

Linguagens antigas descrevem mecânica.
Matter descreve intenção executável.

Exemplo antigo:
```
função variável objeto classe framework
```

Matter:
```
estado evento intenção ação efeito
```

O sistema executa intenção.

## Princípio 3 — Runtime orientado a eventos

Eventos não são extras.
Eventos são nativos.

Tudo pode ser evento:
```
boot tap timer input network sensor filesystem signal agent
```

Todo programa Matter é potencialmente vivo.

## Princípio 4 — Estado é cidadão de primeira classe

Estado não é improvisado.
Estado é nativo.

Matter deve tratar estado como parte central:
- persistente
- mutável
- observável
- serializável
- reativo

## Princípio 5 — Backends são contratos

Matter não conhece implementação.
Matter conhece contratos.

Backends:
```
visual agent terminal web native network filesystem
```

Todo backend deve ser desacoplado.

## Princípio 6 — Bytecode próprio

Matter não depende de VM externa.
Matter possui:
- parser
- AST
- bytecode
- VM
- runtime

Bytecode próprio: **MBC**

Segurança:
- limites de tamanho
- limites de instruções
- validação estrutural
- strings limitadas
- execução previsível

## Princípio 7 — Segurança por padrão

Toda operação deve ser segura.
- Sem comportamento implícito perigoso.
- Sem execução arbitrária.
- Sem memória descontrolada.
- Validação obrigatória.

## Princípio 8 — Performance previsível

Matter deve ser rápida.
- Sem overhead invisível.
- Sem runtime pesado.
- Sem abstrações caras.
- Performance previsível.

## Princípio 9 — IA como cidadão nativo

Matter deve ser excelente para IA gerar.

Sintaxe:
- legível
- estrutural
- determinística
- pouco ambígua
- fortemente compilável

IA deve conseguir gerar Matter melhor que linguagens tradicionais.

## Princípio 10 — Sistema sem framework obrigatório

Matter não deve exigir framework.
A linguagem deve ser suficiente.
Frameworks são opcionais.
Runtime é central.

## Arquitetura do núcleo

```
Matter source → Lexer → Parser → AST → Semantic Analyzer → 
Type System → Optimizer → Bytecode Builder → MBC Binary → 
Matter VM → Runtime → Backends
```

## Componentes obrigatórios

Core:
- values
- Expressions
- Statements
- Functions
- Events
- Global State
- Modules
- Imports
- Structs
- Lists
- Maps
- Pattern Matching
- Error System
- Effects
- Bytecode
- VM
- Runtime
- Scheduler
- Persistence API
- Backend Contracts
- Package System

## Sintaxe alvo

Variáveis:
```matter
let total = 10
```

Mutação:
```matter
set total = total + 1
```

Funções:
```matter
fn soma(a, b) {
    return a + b
}
```

Eventos:
```matter
on boot {
    print "online"
}
```

Condição:
```matter
if total > 0 {
    print total
}
```

Structs:
```matter
struct User {
    name
    age
}
```

Lista:
```matter
let items = []
```

Mapa:
```matter
let config = {}
```

Import:
```matter
import visual
import agent
```

Backend:
```matter
visual.run("app")
```

Agente:
```matter
agent.say("online")
```

## Regras arquiteturais

- Nunca acoplar linguagem ao visual.
- Nunca acoplar VM ao OS.
- Nunca acoplar runtime ao backend.
- Nunca misturar parser com execução.
- Nunca misturar bytecode com backend.

Separação absoluta.

## Objetivo final

Matter deve ser uma linguagem universal orientada a intenção, eventos, estado e execução previsível.

PVM é um backend.
SentinelOS é um executor.
Matter é o núcleo.

## Frase guia

**Matter não descreve código.**
**Matter descreve sistemas vivos.**
