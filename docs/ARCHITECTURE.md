# Matter Core Architecture

## Visão Geral

Matter Core é uma linguagem de programação com runtime próprio, bytecode nativo (MBC) e arquitetura orientada a eventos.

## Estrutura de Crates

```
matter-core/
├── crates/
│   ├── matter-lexer/      # Análise léxica
│   ├── matter-parser/     # Análise sintática
│   ├── matter-ast/        # Árvore de sintaxe abstrata
│   ├── matter-bytecode/   # Geração de bytecode MBC
│   ├── matter-vm/         # Máquina virtual
│   ├── matter-runtime/    # Sistema de eventos e estado
│   ├── matter-backend/    # Contratos de backend
│   └── matter-cli/        # Interface de linha de comando
```

## Pipeline de Compilação e Execução

```
┌─────────────┐
│ Source Code │
│  (.matter)  │
└──────┬──────┘
       │
       v
┌─────────────┐
│   Lexer     │  Tokenização
│             │  (matter-lexer)
└──────┬──────┘
       │
       v
┌─────────────┐
│   Parser    │  Construção da AST
│             │  (matter-parser)
└──────┬──────┘
       │
       v
┌─────────────┐
│     AST     │  Representação estrutural
│             │  (matter-ast)
└──────┬──────┘
       │
       v
┌─────────────┐
│  Bytecode   │  Compilação para MBC
│   Builder   │  (matter-bytecode)
└──────┬──────┘
       │
       v
┌─────────────┐
│ MBC Binary  │  Bytecode Matter
│             │
└──────┬──────┘
       │
       v
┌─────────────┐
│  Matter VM  │  Execução stack-based
│             │  (matter-vm)
└──────┬──────┘
       │
       v
┌─────────────┐
│   Runtime   │  Eventos, estado, scheduler
│             │  (matter-runtime)
└──────┬──────┘
       │
       v
┌─────────────┐
│  Backends   │  visual, agent, terminal, etc
│             │  (matter-backend)
└─────────────┘
```

## Componentes Detalhados

### 1. Lexer (matter-lexer)

**Responsabilidade**: Converter código fonte em tokens.

**Input**: String de código Matter
**Output**: Vec<Token>

**Tokens suportados**:
- Keywords: `let`, `set`, `fn`, `return`, `if`, `else`, `on`, `print`
- Literals: Int, String, Bool
- Operators: `+`, `-`, `*`, `/`, `==`, `!=`, `<`, `>`, `<=`, `>=`
- Delimiters: `(`, `)`, `{`, `}`, `,`, `.`

### 2. Parser (matter-parser)

**Responsabilidade**: Construir AST a partir de tokens.

**Input**: Vec<Token>
**Output**: Program (AST)

**Estratégia**: Recursive descent parser

**Precedência de operadores**:
1. Primary (literals, identifiers, parens)
2. Call (function calls, backend calls)
3. Multiplicative (`*`, `/`)
4. Additive (`+`, `-`)
5. Comparison (`==`, `!=`, `<`, `>`, `<=`, `>=`)

### 3. AST (matter-ast)

**Responsabilidade**: Representar estrutura sintática do programa.

**Estruturas principais**:
- `Program` - Raiz da AST
- `Statement` - Let, Set, Print, FunctionDef, OnEvent, If, Return
- `Expression` - Int, Bool, String, Identifier, Binary, Call, BackendCall
- `BinaryOp` - Add, Sub, Mul, Div, Eq, NotEq, Lt, Gt, LtEq, GtEq

### 4. Bytecode (matter-bytecode)

**Responsabilidade**: Compilar AST para bytecode MBC.

**Formato MBC1**:
```
Magic: "MBC1" (4 bytes)
Constants: Vec<Constant>
Functions: HashMap<String, Function>
EventHandlers: HashMap<String, EventHandler>
MainInstructions: Vec<Instruction>
```

**Instruções**:
- Stack: LoadConst, LoadGlobal, StoreGlobal, Pop
- Arithmetic: Add, Sub, Mul, Div
- Comparison: Eq, NotEq, Lt, Gt, LtEq, GtEq
- Control: Jump, JumpIfFalse, Call, Return
- Built-in: Print
- Backend: BackendCall
- Special: Halt

### 5. VM (matter-vm)

**Responsabilidade**: Executar bytecode MBC.

**Arquitetura**: Stack-based VM

**Componentes**:
- Stack: Vec<Value>
- Globals: HashMap<String, Value>
- Bytecode: Bytecode
- Backends: HashMap<String, Box<dyn Backend>>

**Valores**:
- Int(i64)
- Bool(bool)
- String(String)
- Unit
- Function(String)

**Execução**:
1. Instruction pointer (ip)
2. Fetch instruction
3. Execute instruction
4. Update stack/globals
5. Advance ip
6. Repeat until Halt

### 6. Runtime (matter-runtime)

**Responsabilidade**: Gerenciar eventos, estado e backends.

**Funcionalidades**:
- Registrar backends
- Executar programa principal
- Disparar eventos
- Gerenciar estado global

### 7. Backend (matter-backend)

**Responsabilidade**: Definir contratos para backends externos.

**Trait**:
```rust
pub trait Backend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String>;
}
```

**Backends implementados**:
- `AgentBackend` - Mock para agentes
- `VisualBackend` - Mock para interface visual
- `TraceBackend` - Debug/trace genérico

### 8. CLI (matter-cli)

**Responsabilidade**: Interface de linha de comando.

**Comandos**:
- `run <file>` - Executar arquivo Matter
- `emit <file> <event>` - Disparar evento
- `compile <file> -o <output>` - Compilar para bytecode
- `run-bytecode <file>` - Executar bytecode (futuro)

## Separação de Responsabilidades

### Princípios Arquiteturais

1. **Parser separado da VM**
   - Parser não conhece VM
   - VM não conhece sintaxe
   - AST é a interface

2. **Bytecode separado do backend**
   - Bytecode não conhece backends
   - Backends não conhecem bytecode
   - VM faz a ponte

3. **Runtime separado do OS**
   - Runtime não faz syscalls diretas
   - Backends encapsulam OS
   - Portabilidade garantida

4. **Nenhum acoplamento ao PVM**
   - Matter Core é standalone
   - PVM será apenas um backend futuro
   - Visual backend é mock

## Fluxo de Dados

### Compilação

```
Source → Tokens → AST → Bytecode
```

### Execução

```
Bytecode → VM → Runtime → Backends → Output
```

### Eventos

```
Host → Runtime.emit_event() → VM.execute(handler) → Backends
```

## Segurança

### Limites

- Stack size limitado (implícito)
- String size limitado (futuro)
- Instruction count limitado (futuro)
- Memory limitado (futuro)

### Validação

- Bytecode magic number check
- Instruction validation
- Type checking em runtime
- Backend error handling

## Performance

### Otimizações Atuais

- Constant pooling (reuso de constantes)
- Direct threaded dispatch (match em instruções)

### Otimizações Futuras

- Bytecode optimization pass
- JIT compilation
- Inline caching
- Register-based VM (alternativa)

## Extensibilidade

### Adicionar novo backend

1. Implementar trait `Backend`
2. Registrar no Runtime
3. Usar via `backend.method()`

### Adicionar nova instrução

1. Adicionar em `Instruction` enum
2. Implementar em `BytecodeBuilder`
3. Implementar em `Vm::execute()`

### Adicionar novo tipo

1. Adicionar em `Value` enum
2. Adicionar em `Constant` enum
3. Implementar operações na VM

## Testing

### Estratégia

- Unit tests em cada crate
- Integration tests no CLI
- Example-based testing

### Coverage

- Lexer: tokenização básica
- Parser: construção de AST
- Bytecode: compilação
- VM: execução de instruções

## Roadmap Técnico

### v0.2
- Call frames (variáveis locais)
- Bytecode serialization
- Error handling system

### v0.3
- Type system
- Module system
- Optimizer pass

### v1.0
- JIT compilation
- Debugger protocol
- Package manager
