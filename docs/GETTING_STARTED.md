# Getting Started with Matter Core

Bem-vindo ao Matter Core! Este guia vai te levar do zero ao primeiro programa em minutos.

## 📦 Instalação

### Windows (Recomendado)

```powershell
# 1. Clone o repositório
git clone <repo-url>
cd matter-core

# 2. Compile em release
cargo build --release

# 3. Adicione ao PATH (opcional)
# O executável estará em: target/release/matter-cli.exe
```

### Verificar Instalação

```bash
matter-cli version
# Deve mostrar: Matter CLI v0.4.0
```

## 🚀 Seu Primeiro Programa

### Hello World

Crie um arquivo `hello.matter`:

```matter
print "Hello, Matter!"
```

Execute:

```bash
matter-cli run hello.matter
```

Saída:
```
Hello, Matter!
```

## 📚 Conceitos Básicos

### 1. Variáveis

```matter
# Declarar variável
let name = "Alice"
let age = 30
let active = true

# Imprimir
print name
print age
```

### 2. Operações Matemáticas

```matter
let a = 10
let b = 20

print a + b  # 30
print a * b  # 200
print b / a  # 2
print b - a  # 10
```

### 3. Funções

```matter
# Definir função
fn greet(name) {
    return "Hello, " + name + "!"
}

# Usar função
let message = greet("World")
print message
```

### 4. Condicionais

```matter
let age = 18

if age >= 18 {
    print "Adult"
} else {
    print "Minor"
}
```

### 5. Loops

```matter
# While loop
let i = 0
while i < 5 {
    print i
    set i = i + 1
}

# For loop
let numbers = [1, 2, 3, 4, 5]
for num in numbers {
    print num
}
```

### 6. Listas

```matter
# Criar lista
let fruits = ["apple", "banana", "orange"]

# Acessar elemento
print fruits[0]  # apple

# Adicionar elemento
fruits.push("grape")

# Tamanho
print fruits.len()  # 4
```

### 7. Maps

```matter
# Criar map
let user = {
    "name": "Alice",
    "age": 30,
    "city": "São Paulo"
}

# Acessar valor
print user["name"]  # Alice

# Verificar chave
if user.has("age") {
    print "Age exists"
}
```

### 8. Structs

```matter
# Definir struct
struct Person {
    name: string,
    age: int
}

# Criar instância
let person = Person {
    name: "Bob",
    age: 25
}

# Acessar campo
print person.name  # Bob
```

## 🎯 Exemplos Práticos

### Calculadora

```matter
fn add(a, b) { return a + b }
fn subtract(a, b) { return a - b }
fn multiply(a, b) { return a * b }
fn divide(a, b) { return a / b }

print "Calculator:"
print "10 + 5 = " + add(10, 5)
print "10 - 5 = " + subtract(10, 5)
print "10 * 5 = " + multiply(10, 5)
print "10 / 5 = " + divide(10, 5)
```

### Contador

```matter
let counter = 0

fn increment() {
    set counter = counter + 1
    print "Counter: " + counter
}

fn reset() {
    set counter = 0
    print "Counter reset"
}

increment()
increment()
increment()
reset()
```

### Lista de Tarefas

```matter
let tasks = []

fn add_task(task) {
    tasks.push(task)
    print "Task added: " + task
}

fn list_tasks() {
    print "=== Tasks ==="
    for task in tasks {
        print "- " + task
    }
}

add_task("Learn Matter")
add_task("Build app")
add_task("Deploy")
list_tasks()
```

## 🔧 Backends

Matter Core tem backends integrados para funcionalidades extras:

### Store (Persistência)

```matter
# Salvar dados
store.set("username", "alice")

# Recuperar dados
let name = store.get("username")
print name

# Verificar existência
if store.has("username") {
    print "User exists"
}
```

### Math (Matemática)

```matter
print math.abs(0 - 10)      # 10
print math.max(5, 10)       # 10
print math.min(5, 10)       # 5
print math.pow(2, 3)        # 8
print math.sqrt(16)         # 4
```

### String (Strings)

```matter
let text = "hello world"

print string.upper(text)    # HELLO WORLD
print string.lower(text)    # hello world
print string.len(text)      # 11
```

### Time (Tempo)

```matter
let now = time.now()
print "Timestamp: " + now

# Sleep (delay)
time.sleep(1000)  # 1 segundo
```

### Random (Aleatório)

```matter
print random.int()          # Número aleatório
print random.bool()         # true ou false

let options = ["A", "B", "C"]
print random.choice(options)  # Escolha aleatória
```

### JSON

```matter
let data = {"name": "Alice", "age": 30}

# Serializar
let json_str = json.stringify(data)
print json_str

# Deserializar
let parsed = json.parse(json_str)
print parsed["name"]
```

## 🎮 Eventos

Matter Core tem eventos nativos:

```matter
# Executar ao iniciar
on boot {
    print "App started!"
    store.set("start_time", time.now())
}

# Executar ao finalizar
on shutdown {
    print "App closing..."
}

# Evento customizado
on tick {
    print "Tick!"
}

# Disparar evento
spawn tick
```

## 📦 Compilação

### Compilar para Bytecode

```bash
# Compilar
matter-cli compile app.matter -o app.mbc

# Executar bytecode
matter-cli run-bytecode app.mbc

# Inspecionar bytecode
matter-cli inspect app.mbc
```

### Com Otimização

```bash
# Compilar otimizado (futuro)
matter-cli compile app.matter -o app.mbc --optimize
```

## 🐛 Debug

### Verificar Sintaxe

```bash
matter-cli check app.matter
```

### Ver Tokens

```bash
matter-cli tokens-json app.matter
```

### REPL Interativo

```bash
matter-cli repl

[1]> let x = 10
[2]> print x
10
[3]> :quit
```

## 📖 Próximos Passos

1. **Explore os Exemplos**
   ```bash
   cd examples
   matter-cli run hello.matter
   ```

2. **Veja as Aplicações**
   ```bash
   cd examples/apps
   matter-cli run counter_app.matter
   ```

3. **Leia a Documentação**
   - `docs/SPEC.md` - Especificação completa
   - `docs/ARCHITECTURE.md` - Arquitetura
   - `examples/apps/README.md` - Guia de apps

4. **Crie Seu Projeto**
   ```bash
   # Criar novo projeto (futuro)
   matter-cli pkg new my-app
   ```

## 💡 Dicas

- Use `print` para debug
- Teste no REPL antes de criar arquivo
- Veja exemplos para aprender padrões
- Leia mensagens de erro com atenção
- Comece simples e vá evoluindo

## 🆘 Ajuda

### Comandos Úteis

```bash
matter-cli help              # Ajuda geral
matter-cli help run          # Ajuda específica
matter-cli version           # Versão
matter-cli backends          # Lista backends
matter-cli examples          # Lista exemplos
```

### Problemas Comuns

**Erro de sintaxe:**
- Verifique parênteses, chaves e aspas
- Use `matter-cli check` para validar

**Variável não definida:**
- Use `let` antes de usar variável
- Verifique escopo (global vs local)

**Função não encontrada:**
- Verifique nome da função
- Defina função antes de usar

## 🎓 Recursos

- **Exemplos:** `examples/`
- **Apps:** `examples/apps/`
- **Docs:** `docs/`
- **Testes:** `tests/`

## 🚀 Pronto!

Você agora sabe o básico de Matter Core. Comece a construir algo incrível!

```matter
print "Happy coding with Matter! 🎉"
```

---

**Matter Core v0.4.0** - Runtime-Oriented Language System
