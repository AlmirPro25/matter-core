# Sprint 4 — Data Model: List, Map, Struct

## Contexto

Sprint 3.5 está fechado com:
- ✅ Funções com parâmetros e return
- ✅ Escopo hierárquico (global + local)
- ✅ Loops (while, loop, break, continue)
- ✅ Bytecode persistível em disco (MBC1)
- ✅ Comando `matter inspect` com visualização detalhada
- ✅ Equivalência garantida: source == bytecode
- ✅ Semântica clara de `let` vs `set`

**A base está pronta para dados compostos.**

## Objetivo

Implementar tipos de dados compostos essenciais para aplicações reais:
- **List**: Coleções ordenadas e dinâmicas
- **Map**: Dicionários chave-valor
- **Struct**: Estruturas de dados nomeadas

## Motivação

Aplicações reativas precisam de:
```matter
# Estado de uma TODO app
let todos = []
let filters = { "active": true, "completed": false }
let user = { name: "Alice", role: "admin" }

# Manipulação
todos.push({ id: 1, text: "Learn Matter", done: false })
let active_todos = todos.filter(fn(t) { !t.done })
```

## Escopo do Sprint

### 1. List (Prioridade Alta)

**Sintaxe**:
```matter
let empty = []
let numbers = [1, 2, 3, 4, 5]
let mixed = [1, "hello", true]
```

**Operações**:
```matter
# Acesso
let first = numbers[0]
let last = numbers[4]

# Mutação
set numbers[0] = 10
numbers.push(6)
numbers.pop()

# Métodos
let length = numbers.len()
let has_three = numbers.contains(3)
```

**Bytecode**:
- `LoadList(size)` - Cria lista vazia ou com elementos da stack
- `LoadIndex` - Acessa elemento por índice
- `StoreIndex` - Atualiza elemento por índice
- `ListPush` - Adiciona elemento ao final
- `ListPop` - Remove último elemento
- `ListLen` - Retorna tamanho

### 2. Map (Prioridade Alta)

**Sintaxe**:
```matter
let empty = {}
let person = {
    name: "Alice",
    age: 30,
    active: true
}
```

**Operações**:
```matter
# Acesso
let name = person["name"]
let age = person.age  # syntax sugar

# Mutação
set person["age"] = 31
set person.age = 31  # syntax sugar

# Métodos
let has_name = person.has("name")
let keys = person.keys()
let values = person.values()
```

**Bytecode**:
- `LoadMap(size)` - Cria map com pares chave-valor da stack
- `LoadField(key)` - Acessa campo por chave
- `StoreField(key)` - Atualiza campo por chave
- `MapHas` - Verifica se chave existe
- `MapKeys` - Retorna lista de chaves
- `MapValues` - Retorna lista de valores

### 3. Struct (Prioridade Média)

**Sintaxe**:
```matter
struct Todo {
    id: int,
    text: string,
    done: bool
}

let todo = Todo {
    id: 1,
    text: "Learn Matter",
    done: false
}
```

**Operações**:
```matter
# Acesso
let text = todo.text

# Mutação
set todo.done = true

# Pattern matching (futuro)
match todo {
    Todo { done: true, .. } => print "Completed!",
    _ => print "Pending"
}
```

**Bytecode**:
- `LoadStruct(type_id, field_count)` - Cria struct
- `LoadStructField(field_name)` - Acessa campo
- `StoreStructField(field_name)` - Atualiza campo

### 4. Serialização MBC1

**Extensão do formato**:
```
Section: Type Definitions
  - Struct definitions
  - Type metadata

Constants Pool:
  - List literals
  - Map literals
  - Struct literals
```

## Implementação

### Ordem de Execução

**Regra**: Implementar do mais simples ao mais complexo, validando equivalência a cada fase.

### Fase 1: Lists (Prioridade Máxima)

**Por quê primeiro**: Estrutura mais simples, base para Map e Struct.

**Sintaxe**:
```matter
let items = []
items.push(10)
items.push(20)
print items[0]  # 10
print items.len()  # 2
```

**AST**:
```rust
pub enum Expression {
    List(Vec<Expression>),
    Index { target: Box<Expression>, index: Box<Expression> },
}
```

**Bytecode**:
```rust
pub enum Instruction {
    NewList,           // Cria lista vazia
    ListPush,          // Pop value, pop list, push list (com value)
    ListPop,           // Pop list, push value
    LoadIndex,         // Pop index, pop list, push value
    StoreIndex,        // Pop value, pop index, pop list
    ListLen,           // Pop list, push length
}

pub enum Constant {
    List(Vec<ConstantId>),  // Para literais: [1, 2, 3]
}
```

**VM**:
```rust
pub enum Value {
    List(Vec<Value>),
}
```

**Testes**:
```matter
# examples/test_list.matter
let nums = [1, 2, 3]
print nums[0]  # 1
print nums.len()  # 3

nums.push(4)
print nums.len()  # 4

let last = nums.pop()
print last  # 4
```

**Validação**: `source == bytecode` para lists

---

### Fase 2: Maps (Prioridade Alta)

**Por quê segundo**: Essencial para estado de aplicações, mais complexo que List.

**Sintaxe**:
```matter
let state = {}
state["app"] = "pizzaria"
state["count"] = 42
print state["app"]  # pizzaria
```

**AST**:
```rust
pub enum Expression {
    Map(Vec<(String, Expression)>),
    Field { target: Box<Expression>, field: String },
}
```

**Bytecode**:
```rust
pub enum Instruction {
    NewMap,            // Cria map vazio
    MapGet,            // Pop key, pop map, push value
    MapSet,            // Pop value, pop key, pop map
    MapHas,            // Pop key, pop map, push bool
    MapKeys,           // Pop map, push list of keys
    MapValues,         // Pop map, push list of values
}

pub enum Constant {
    Map(Vec<(String, ConstantId)>),  // Para literais: {a: 1, b: 2}
}
```

**VM**:
```rust
pub enum Value {
    Map(HashMap<String, Value>),
}
```

**Testes**:
```matter
# examples/test_map.matter
let person = {
    name: "Alice",
    age: 30
}

print person["name"]  # Alice
print person.age  # 30 (syntax sugar)

set person.age = 31
print person.age  # 31

print person.has("name")  # true
```

**Validação**: `source == bytecode` para maps

---

### Fase 3: Structs (Prioridade Média)

**Por quê terceiro**: Mais complexo, requer type system básico.

**Sintaxe**:
```matter
struct User {
    name: string,
    age: int
}

let u = User {
    name: "Ana",
    age: 20
}

print u.name  # Ana
set u.age = 21
```

**AST**:
```rust
pub enum Statement {
    StructDef {
        name: String,
        fields: Vec<(String, TypeAnnotation)>,
    },
}

pub enum Expression {
    StructLiteral {
        type_name: String,
        fields: Vec<(String, Expression)>,
    },
}
```

**Bytecode**:
```rust
pub enum Instruction {
    NewStruct(String, usize),  // type_name, field_count
    GetField(String),          // Pop struct, push field value
    SetField(String),          // Pop value, pop struct
}

// Nova seção no MBC1
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}
```

**VM**:
```rust
pub enum Value {
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
}
```

**Testes**:
```matter
# examples/test_struct.matter
struct Point {
    x: int,
    y: int
}

let p = Point { x: 10, y: 20 }
print p.x  # 10
print p.y  # 20

set p.x = 15
print p.x  # 15
```

**Validação**: `source == bytecode` para structs

---

### Fase 4: Serialização MBC1

**Extensão do formato**:
```
Magic: "MBC1"
Version: u16
Flags: u16
Total Instructions: u32

Section: Type Definitions (novo)
  - Struct definitions
  - Type metadata

Section: Constants
  - Int, Bool, String, Unit
  - List (novo)
  - Map (novo)

Section: Functions
Section: Events
Section: Main
```

**Validação**: Round-trip completo para todos os tipos

---

### Regra Importante: Dados Puros

**Structs são dados, não classes**:

✅ **Permitido**:
- Campos nomeados
- Acesso por ponto (u.name)
- Mutação via set
- Pattern matching (futuro)

❌ **NÃO permitido** (por enquanto):
- Métodos embutidos
- Herança
- Polimorfismo
- Construtores customizados
- Traits/interfaces

**Por quê**: Mantém a linguagem limpa e focada. Métodos virão depois via traits (Sprint 8+).

**Exemplo do que NÃO fazer**:
```matter
# ❌ NÃO implementar isso agora
struct User {
    name: string
    
    fn greet() {  # ❌ Método embutido
        print "Hello " + self.name
    }
}
```

**Exemplo do que fazer**:
```matter
# ✅ Dados puros
struct User {
    name: string,
    age: int
}

# ✅ Funções separadas
fn greet(user) {
    print "Hello " + user.name
}
```

## Critérios de Sucesso

1. ✅ Parser reconhece sintaxe de list, map, struct
2. ✅ Bytecode compila corretamente
3. ✅ VM executa operações básicas
4. ✅ Serialização MBC1 preserva tipos compostos
5. ✅ `matter inspect` mostra tipos compostos
6. ✅ Equivalência: source == bytecode para tipos compostos
7. ✅ Testes passam: test_list, test_map, test_struct

## O Que Muda Depois de Sprint 4

### Antes: Linguagem de Fluxo
```matter
let x = 10
if x > 5 {
    print "maior"
}
while x > 0 {
    set x = x - 1
}
```

**Limitação**: Só consegue expressar controle de fluxo e estado primitivo.

### Depois: Linguagem de Modelagem de Aplicações
```matter
# Carrinho de compras
let cart = {
    items: [],
    total: 0
}

struct Item {
    id: int,
    name: string,
    price: int
}

cart.items.push(Item {
    id: 1,
    name: "Pizza",
    price: 30
})

# Usuário
struct User {
    name: string,
    role: string,
    active: bool
}

let users = []
users.push(User { name: "Ana", role: "admin", active: true })

# Sessão
let session = {
    user: users[0],
    cart: cart,
    timestamp: 1234567890
}

# Config
let config = {
    app: "pizzaria",
    theme: "dark",
    features: ["delivery", "pickup"]
}

# Estado de app
let app_state = {
    users: users,
    sessions: [session],
    config: config
}

# Árvore de UI
struct Component {
    type: string,
    props: map,
    children: list
}

# Memória de agente
struct Memory {
    context: string,
    facts: list,
    timestamp: int
}
```

**Capacidade**: Pode representar estado real de aplicações complexas.

### A Diferença É Enorme

**Antes** (Sprint 3.5):
- ✅ Execução correta
- ✅ Controle de fluxo
- ✅ Funções e recursão
- ❌ Modelagem de domínio limitada

**Depois** (Sprint 4):
- ✅ Execução correta
- ✅ Controle de fluxo
- ✅ Funções e recursão
- ✅ **Modelagem de domínio completa**

### Aplicações Possíveis

Com tipos compostos, Matter pode implementar:

1. **E-commerce**
   - Carrinho, produtos, usuários, pedidos

2. **Dashboard**
   - Widgets, dados, configuração, estado

3. **Chat/Agent**
   - Mensagens, contexto, memória, histórico

4. **CMS**
   - Posts, autores, categorias, tags

5. **Game State**
   - Jogadores, inventário, mundo, eventos

6. **API Client**
   - Requests, responses, cache, config

### Base Madura

Após Sprint 4, o núcleo está completo:
- ✅ Runtime integrity (Sprint 3.5)
- ✅ Data modeling (Sprint 4)
- 🚀 Pronto para pattern matching (Sprint 5)
- 🚀 Pronto para módulos (Sprint 6)
- 🚀 Pronto para standard library (v0.3)

**A base agora está madura o suficiente para crescer sem reescrever o núcleo.**
