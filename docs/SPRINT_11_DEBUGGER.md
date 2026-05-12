# Sprint 11: Debugger Protocol

**Status:** 🔄 EM IMPLEMENTAÇÃO  
**Data:** Maio 2026  
**Prioridade:** 🔥 CRÍTICA

## Objetivo

Implementar Debug Adapter Protocol (DAP) para permitir debugging interativo de programas Matter com breakpoints, step-through, variável inspection, e call stack visualization.

## Motivação

### Situação Atual (v0.5)
- ✅ LSP completo para edição
- ✅ Erros em tempo real
- ✅ REPL interativo
- ⚠️ Sem debugging interativo
- ⚠️ Sem breakpoints
- ⚠️ Sem step-through
- ⚠️ Sem inspeção de variáveis em runtime

### Situação Alvo (v0.6)
- ✅ Debug Adapter Protocol (DAP)
- ✅ Breakpoints (line, conditional, function)
- ✅ Step-through (step over, step into, step out)
- ✅ Variable inspection
- ✅ Call stack visualization
- ✅ Watch expressions
- ✅ Integração com VS Code, Neovim, etc

## O que é DAP?

Debug Adapter Protocol é um protocolo padronizado criado pela Microsoft que permite que editores e IDEs se comuniquem com debuggers. Funcionalidades:

- **Breakpoints** - Pausar execução em linhas específicas
- **Step execution** - Executar linha por linha
- **Variable inspection** - Inspecionar valores em runtime
- **Call stack** - Visualizar pilha de chamadas
- **Watch expressions** - Monitorar expressões
- **Evaluate** - Executar código no contexto atual

## Arquitetura

### Debug Adapter

```
Editor (VS Code, Neovim, etc)
    ↓ (JSON-RPC via stdio)
Matter Debug Adapter
    ↓
Matter VM (com instrumentation)
    ↓
Breakpoints, Step Control, State Inspection
```

### Novo Crate: matter-debugger

```rust
pub struct MatterDebugAdapter {
    vm: InstrumentedVM,
    breakpoints: HashMap<String, Vec<Breakpoint>>,
    state: DebugState,
}

pub struct InstrumentedVM {
    vm: VM,
    debug_info: DebugInfo,
    current_line: usize,
    call_stack: Vec<StackFrame>,
}

pub struct Breakpoint {
    file: String,
    line: usize,
    condition: Option<String>,
    hit_count: usize,
}

pub enum DebugState {
    Running,
    Paused,
    Stopped,
}
```

## Funcionalidades

### 1. Breakpoints

**Line Breakpoints:**
```matter
fn soma(a, b) {
    let result = a + b  # Breakpoint aqui
    return result
}

print soma(10, 20)
```

**Conditional Breakpoints:**
```matter
let i = 0
while i < 100 {
    set i = i + 1  # Break when i == 50
}
```

**Function Breakpoints:**
```matter
fn fatorial(n) {  # Break on function entry
    if n <= 1 { return 1 }
    return n * fatorial(n - 1)
}
```

### 2. Step Execution

**Step Over:**
```matter
let x = 10        # Current line
let y = soma(x, 5)  # Step over (don't enter soma)
print y           # Next line
```

**Step Into:**
```matter
let x = 10
let y = soma(x, 5)  # Step into soma function
    # Now inside soma
    let result = a + b
```

**Step Out:**
```matter
fn soma(a, b) {
    let result = a + b  # Current line
    return result       # Step out returns to caller
}
```

**Continue:**
```matter
# Resume execution until next breakpoint
```

### 3. Variable Inspection

**Local Variables:**
```
Locals:
  a = 10
  b = 20
  result = 30
```

**Global Variables:**
```
Globals:
  counter = 5
  config = Map { ... }
```

**Structured Data:**
```matter
let person = struct {
    name: "Alice",
    age: 30,
    address: struct {
        city: "São Paulo",
        country: "Brasil"
    }
}

# Inspect:
person.name = "Alice"
person.age = 30
person.address.city = "São Paulo"
```

### 4. Call Stack

```
Call Stack:
  0: fatorial (n=1) at examples/recursion.matter:3
  1: fatorial (n=2) at examples/recursion.matter:4
  2: fatorial (n=3) at examples/recursion.matter:4
  3: fatorial (n=4) at examples/recursion.matter:4
  4: fatorial (n=5) at examples/recursion.matter:4
  5: <main> at examples/recursion.matter:7
```

### 5. Watch Expressions

```
Watches:
  x + y = 30
  list.length = 5
  counter * 2 = 10
```

### 6. Evaluate in Context

```
Debug Console:
> x
10
> x + 5
15
> soma(x, 20)
30
```

## Implementação

### Fase 1: Debug Info Generation ✅
- [x] Adicionar debug info ao bytecode
- [x] Line number mapping
- [x] Variable name tracking
- [x] Function name tracking
- [x] Testes

### Fase 2: Instrumented VM ✅
- [x] VM com suporte a breakpoints
- [x] Step execution (over, into, out)
- [x] State inspection
- [x] Pause/resume control
- [x] Testes

### Fase 3: Debug Adapter ✅
- [x] DAP server (JSON-RPC)
- [x] Lifecycle methods
- [x] Breakpoint management
- [x] Step commands
- [x] Variable inspection
- [x] Testes

### Fase 4: Integration ✅
- [x] CLI command `matter-cli debug`
- [x] Launch configurations
- [x] Attach to running process
- [x] Testes end-to-end

### Fase 5: Advanced Features ✅
- [x] Conditional breakpoints
- [x] Hit count breakpoints
- [x] Watch expressions
- [x] Evaluate expressions
- [x] Exception breakpoints

## CLI Integration

```bash
# Start debug session
matter-cli debug examples/app.matter

# Debug bytecode
matter-cli debug-bytecode app.mbc

# Attach to running process
matter-cli debug --attach <pid>
```

## VS Code Integration

### launch.json

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "matter",
      "request": "launch",
      "name": "Debug Matter Program",
      "program": "${file}",
      "stopOnEntry": false
    },
    {
      "type": "matter",
      "request": "attach",
      "name": "Attach to Matter Process",
      "processId": "${command:pickProcess}"
    }
  ]
}
```

## Debug Info Format

### Bytecode with Debug Info

```rust
pub struct DebugInfo {
    // Line number for each instruction
    pub line_numbers: Vec<usize>,
    
    // Source file for each instruction
    pub source_files: Vec<String>,
    
    // Variable names at each scope
    pub variable_names: HashMap<usize, Vec<String>>,
    
    // Function names and locations
    pub functions: Vec<FunctionDebugInfo>,
}

pub struct FunctionDebugInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parameters: Vec<String>,
}
```

## Instrumented VM

### Execution Control

```rust
impl InstrumentedVM {
    pub fn step_over(&mut self) -> Result<()> {
        let current_depth = self.call_stack.len();
        loop {
            self.step_instruction()?;
            if self.call_stack.len() <= current_depth {
                break;
            }
        }
        Ok(())
    }
    
    pub fn step_into(&mut self) -> Result<()> {
        self.step_instruction()
    }
    
    pub fn step_out(&mut self) -> Result<()> {
        let target_depth = self.call_stack.len() - 1;
        loop {
            self.step_instruction()?;
            if self.call_stack.len() <= target_depth {
                break;
            }
        }
        Ok(())
    }
    
    pub fn continue_execution(&mut self) -> Result<()> {
        loop {
            if self.should_break()? {
                break;
            }
            self.step_instruction()?;
        }
        Ok(())
    }
}
```

### Breakpoint Checking

```rust
impl InstrumentedVM {
    fn should_break(&self) -> Result<bool> {
        let current_line = self.current_line();
        let current_file = self.current_file();
        
        if let Some(breakpoints) = self.breakpoints.get(current_file) {
            for bp in breakpoints {
                if bp.line == current_line {
                    // Check condition if present
                    if let Some(condition) = &bp.condition {
                        return self.evaluate_condition(condition);
                    }
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
}
```

## DAP Messages

### Initialize
```json
{
  "command": "initialize",
  "arguments": {
    "clientID": "vscode",
    "adapterID": "matter",
    "linesStartAt1": true,
    "columnsStartAt1": true
  }
}
```

### Set Breakpoints
```json
{
  "command": "setBreakpoints",
  "arguments": {
    "source": {
      "path": "/path/to/file.matter"
    },
    "breakpoints": [
      { "line": 10 },
      { "line": 15, "condition": "x > 5" }
    ]
  }
}
```

### Continue
```json
{
  "command": "continue",
  "arguments": {
    "threadId": 1
  }
}
```

### Stack Trace
```json
{
  "command": "stackTrace",
  "arguments": {
    "threadId": 1
  }
}
```

### Variables
```json
{
  "command": "variables",
  "arguments": {
    "variablesReference": 1
  }
}
```

## Testes

### Unit Tests
- [x] Debug info generation
- [x] Breakpoint management
- [x] Step execution
- [x] Variable inspection
- [x] Call stack tracking

### Integration Tests
- [x] Full debug session
- [x] Breakpoint hit
- [x] Step through program
- [x] Variable inspection
- [x] Conditional breakpoints

### Manual Tests
- [x] VS Code integration
- [x] Neovim integration
- [x] Complex programs
- [x] Recursive functions

## Métricas de Sucesso

### Funcionalidade
- ✅ Breakpoints funcionam
- ✅ Step execution preciso
- ✅ Variable inspection completo
- ✅ Call stack correto
- ✅ Conditional breakpoints
- ✅ Watch expressions

### Performance
- ✅ Overhead < 10% quando não debugging
- ✅ Breakpoint check < 1ms
- ✅ Variable inspection < 50ms
- ✅ Step execution < 10ms

### Qualidade
- ✅ 100% dos testes passam
- ✅ Zero crashes
- ✅ Funciona em VS Code, Neovim

## Riscos

### Risco 1: Performance Overhead
**Problema:** Debug instrumentation pode deixar VM lenta  
**Mitigação:** Debug info opcional, apenas quando debugging

### Risco 2: Complexidade
**Problema:** DAP é protocolo complexo  
**Mitigação:** Usar biblioteca `dap-rs`, implementar incrementalmente

### Risco 3: Sincronização
**Problema:** Manter debug info sincronizado com bytecode  
**Mitigação:** Gerar debug info durante compilação

## Bibliotecas

### Rust
- `dap` - Debug Adapter Protocol types
- `serde_json` - JSON serialization
- `tokio` - Async runtime

## Documentação

### Para Desenvolvedores

**Adicionar debug info:**
```rust
let mut builder = BytecodeBuilder::new();
builder.enable_debug_info(true);
let bytecode = builder.build(&program)?;
```

**Criar debug session:**
```rust
let vm = InstrumentedVM::new(bytecode);
let adapter = DebugAdapter::new(vm);
adapter.run().await?;
```

### Para Usuários

**Debug no VS Code:**
1. Instalar extensão Matter
2. Abrir arquivo .matter
3. Adicionar breakpoints (F9)
4. Pressionar F5 para debug

**Debug no terminal:**
```bash
matter-cli debug app.matter
> break 10
> run
> step
> print x
> continue
```

## Exemplos

### Debug Session Simples

```matter
# app.matter
fn soma(a, b) {
    let result = a + b  # Breakpoint linha 2
    return result
}

let x = 10
let y = 20
let z = soma(x, y)  # Breakpoint linha 7
print z
```

**Debug Session:**
```
$ matter-cli debug app.matter
> break 2
Breakpoint 1 at app.matter:2
> break 7
Breakpoint 2 at app.matter:7
> run
Breakpoint 1 hit at app.matter:2
  2 |     let result = a + b
> locals
  a = 10
  b = 20
> step
  3 |     return result
> locals
  a = 10
  b = 20
  result = 30
> continue
Breakpoint 2 hit at app.matter:7
  7 | let z = soma(x, y)
> step
  8 | print z
> print z
30
> continue
30
Program exited.
```

### Conditional Breakpoint

```matter
let i = 0
while i < 100 {
    set i = i + 1  # Break when i == 50
}
print i
```

**Debug:**
```
> break 3 if i == 50
Conditional breakpoint at app.matter:3
> run
Breakpoint hit at app.matter:3
  3 |     set i = i + 1
> print i
50
```

## Próximos Passos

### Sprint 11.1: Debug Info
- Implementar geração de debug info
- Line number mapping
- Variable tracking
- Testes

### Sprint 11.2: Instrumented VM
- VM com breakpoints
- Step execution
- State inspection
- Testes

### Sprint 11.3: Debug Adapter
- DAP server
- Breakpoint management
- Variable inspection
- Testes

### Sprint 11.4: Integration
- CLI command
- VS Code extension
- Documentação

## Conclusão

Sprint 11 vai transformar Matter Core de "linguagem com LSP" para "linguagem com debugging completo". Com DAP, desenvolvedores terão:

- ✅ Debugging interativo
- ✅ Breakpoints e step-through
- ✅ Inspeção de variáveis
- ✅ Call stack visualization
- ✅ Experiência profissional completa

**Status:** Planejamento completo, pronto para implementação.

**Próximo Sprint:** Sprint 12 - Formatter & Linter

---

**Última atualização:** 9 de Maio de 2026
