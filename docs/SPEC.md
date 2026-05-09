# Matter Language Specification v0.1

## Tipos de Dados

### Primitivos

- `int` - Inteiro de 64 bits
- `bool` - Booleano (true/false)
- `string` - String UTF-8
- `unit` - Tipo vazio ()

### Futuros

- `float` - Ponto flutuante
- `list` - Lista dinâmica
- `map` - Mapa chave-valor
- `struct` - Estrutura de dados

## Sintaxe

### Declaração de Variáveis

```matter
let name = value
```

**Semântica**: `let` cria uma nova variável no escopo atual (global ou local).

### Mutação de Estado

```matter
set name = new_value
```

**Semântica**: `set` atualiza uma variável existente seguindo resolução de escopo:
1. Busca no escopo local (se existir)
2. Busca no escopo global
3. Erro se variável não existe

**Importante**: `set` dentro de loops, blocos e event handlers atualiza a variável global, não cria uma nova local. Isso é essencial para estado reativo e persistente.

### Semântica de Escopo

Matter tem dois tipos de escopo:

1. **Escopo Global**: Variáveis declaradas no nível superior
2. **Escopo Local**: Variáveis dentro de funções, loops e blocos

**Regras de Resolução**:
- `let x = value` → cria `x` no escopo atual
- `set x = value` → atualiza `x` existente (busca local → global)
- Loops e blocos criam escopo local para `let`, mas `set` sempre atualiza global
- Funções têm escopo local isolado

**Exemplo**:
```matter
let counter = 0

while counter < 5 {
    print counter
    set counter = counter + 1  # atualiza global, não cria local
}

# counter agora é 5
```

### Funções

```matter
fn function_name(param1, param2) {
    # corpo da função
    return value
}
```

### Eventos

```matter
on event_name {
    # código executado quando evento é disparado
}
```

Eventos nativos:
- `boot` - Inicialização do programa
- `shutdown` - Finalização do programa

### Condicionais

```matter
if condition {
    # then branch
}

if condition {
    # then branch
} else {
    # else branch
}
```

### Expressões

#### Aritméticas
- `+` - Adição
- `-` - Subtração
- `*` - Multiplicação
- `/` - Divisão

#### Comparação
- `==` - Igualdade
- `!=` - Diferença
- `<` - Menor que
- `>` - Maior que
- `<=` - Menor ou igual
- `>=` - Maior ou igual

### Backend Calls

```matter
backend.method(args)
```

Exemplo:
```matter
agent.say("hello")
visual.run("app")
```

### Comentários

```matter
# Comentário de linha
```

## Bytecode MBC1

### Formato

```
Magic: "MBC1" (4 bytes)
Constants Pool
Functions Table
Event Handlers Table
Main Instructions
```

### Instruções

- `LoadConst(id)` - Carrega constante na stack
- `LoadGlobal(name)` - Carrega variável global
- `StoreGlobal(name)` - Armazena no escopo global (sempre)
- `LoadLocal(name)` - Carrega variável local
- `StoreLocal(name)` - Armazena no escopo local
- `PushScope` - Entra em novo escopo
- `PopScope` - Sai do escopo atual
- `Add, Sub, Mul, Div` - Operações aritméticas
- `Eq, NotEq, Lt, Gt, LtEq, GtEq` - Comparações
- `Jump(target)` - Salto incondicional
- `JumpIfFalse(target)` - Salto condicional
- `Call(argc)` - Chamada de função
- `Return` - Retorno de função
- `Print` - Imprime valor
- `BackendCall{backend, method, argc}` - Chamada de backend
- `Pop` - Remove topo da stack
- `Halt` - Para execução

**Nota sobre StoreGlobal**: Esta instrução **sempre** armazena no escopo global, independente do scope stack. Isso garante que variáveis globais possam ser atualizadas de dentro de loops, blocos e event handlers.

## Backends

Backends são contratos que implementam a trait `Backend`:

```rust
pub trait Backend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String>;
}
```

### Backends Padrão

- `agent` - Interação com agentes
  - `say(message)` - Envia mensagem
  
- `visual` - Interface visual
  - `run(app_name)` - Executa aplicação visual

## Execução

### Pipeline

1. **Lexer** - Tokenização do código fonte
2. **Parser** - Construção da AST
3. **Bytecode Builder** - Compilação para MBC
4. **VM** - Execução do bytecode
5. **Runtime** - Gerenciamento de eventos e estado

### Modelo de Execução

- Stack-based VM
- Escopo hierárquico (global + local)
- Variáveis globais mutáveis via `set`
- Funções como valores
- Eventos disparáveis pelo host
- Bytecode persistível em disco (.mbc)
- Equivalência garantida: source execution == bytecode execution

## Limitações Atuais (v0.1)

- ~~Sem variáveis locais (apenas globais)~~ ✅ Implementado
- Sem closures
- Sem tipos compostos (struct, list, map)
- Sem pattern matching
- Sem módulos/imports
- Sem tratamento de erros
- ~~Sem serialização de bytecode~~ ✅ Implementado (MBC1)

## Roadmap

### v0.2
- Variáveis locais e call frames
- Tipos compostos básicos (list, map)
- Serialização de bytecode

### v0.3
- Structs e pattern matching
- Sistema de módulos
- Tratamento de erros

### v1.0
- Sistema de tipos completo
- Otimizador de bytecode
- Persistência de estado
- Package system
