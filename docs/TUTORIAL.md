# Matter Core - Tutorial Completo

Este tutorial vai te ensinar Matter Core do básico ao avançado através de exemplos práticos.

## 📚 Índice

1. [Fundamentos](#fundamentos)
2. [Funções](#funções)
3. [Estruturas de Dados](#estruturas-de-dados)
4. [Controle de Fluxo](#controle-de-fluxo)
5. [Backends](#backends)
6. [Eventos](#eventos)
7. [Projeto Prático](#projeto-prático)

---

## 1. Fundamentos

### Variáveis e Tipos

Matter Core tem tipos dinâmicos:

```matter
# Inteiros
let age = 30
let year = 2026

# Strings
let name = "Alice"
let city = "São Paulo"

# Booleanos
let active = true
let verified = false

# Unit (vazio)
let nothing = ()
```

### Operações

```matter
# Aritméticas
let sum = 10 + 20        # 30
let diff = 20 - 10       # 10
let product = 5 * 4      # 20
let quotient = 20 / 4    # 5

# Comparações
let equal = 10 == 10     # true
let not_equal = 10 != 5  # true
let greater = 10 > 5     # true
let less = 5 < 10        # true

# Strings
let greeting = "Hello, " + "World!"  # "Hello, World!"
```

### Mutação

```matter
let counter = 0

# Mudar valor
set counter = counter + 1
print counter  # 1

set counter = counter * 2
print counter  # 2
```

**Exercício 1:** Crie um programa que calcula a área de um retângulo.

<details>
<summary>Solução</summary>

```matter
let width = 10
let height = 5
let area = width * height
print "Area: " + area
```
</details>

---

## 2. Funções

### Definir Funções

```matter
# Função simples
fn greet(name) {
    return "Hello, " + name + "!"
}

# Usar função
let message = greet("Alice")
print message
```

### Múltiplos Parâmetros

```matter
fn add(a, b) {
    return a + b
}

fn multiply(a, b, c) {
    return a * b * c
}

print add(10, 20)           # 30
print multiply(2, 3, 4)     # 24
```

### Recursão

```matter
fn factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

print factorial(5)  # 120
```

### Funções como Valores

```matter
fn operation(a, b, op) {
    if op == "add" {
        return a + b
    }
    if op == "multiply" {
        return a * b
    }
    return 0
}

print operation(10, 5, "add")       # 15
print operation(10, 5, "multiply")  # 50
```

**Exercício 2:** Crie uma função que calcula o n-ésimo número de Fibonacci.

<details>
<summary>Solução</summary>

```matter
fn fibonacci(n) {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

print fibonacci(10)  # 55
```
</details>

---

## 3. Estruturas de Dados

### Listas

```matter
# Criar lista
let numbers = [1, 2, 3, 4, 5]
let fruits = ["apple", "banana", "orange"]

# Acessar elementos
print numbers[0]  # 1
print fruits[2]   # orange

# Modificar
set numbers[0] = 10
print numbers[0]  # 10

# Métodos
numbers.push(6)
print numbers.len()  # 6

let last = numbers.pop()
print last  # 6
```

### Maps

```matter
# Criar map
let person = {
    "name": "Alice",
    "age": 30,
    "city": "São Paulo"
}

# Acessar
print person["name"]  # Alice

# Modificar
set person["age"] = 31

# Verificar chave
if person.has("email") {
    print "Has email"
} else {
    print "No email"
}

# Listar chaves
let keys = person.keys()
for key in keys {
    print key + ": " + person[key]
}
```

### Structs

```matter
# Definir struct
struct User {
    name: string,
    age: int,
    active: bool
}

# Criar instância
let user = User {
    name: "Bob",
    age: 25,
    active: true
}

# Acessar campos
print user.name   # Bob
print user.age    # 25

# Modificar campo
set user.age = 26
```

**Exercício 3:** Crie uma lista de usuários (structs) e filtre os maiores de 18 anos.

<details>
<summary>Solução</summary>

```matter
struct User { name: string, age: int }

let users = []
users.push(User { name: "Alice", age: 25 })
users.push(User { name: "Bob", age: 17 })
users.push(User { name: "Carol", age: 30 })

let adults = []
for user in users {
    if user.age >= 18 {
        adults.push(user)
    }
}

print "Adults: " + adults.len()
```
</details>

---

## 4. Controle de Fluxo

### If/Else

```matter
let score = 85

if score >= 90 {
    print "A"
} else if score >= 80 {
    print "B"
} else if score >= 70 {
    print "C"
} else {
    print "F"
}
```

### While Loop

```matter
let i = 0
while i < 5 {
    print i
    set i = i + 1
}
```

### For Loop

```matter
let numbers = [1, 2, 3, 4, 5]

for num in numbers {
    print num * 2
}
```

### Loop Infinito

```matter
let counter = 0

loop {
    print counter
    set counter = counter + 1
    
    if counter >= 5 {
        break
    }
}
```

### Break e Continue

```matter
let i = 0
while i < 10 {
    set i = i + 1
    
    if i == 5 {
        continue  # Pula 5
    }
    
    if i == 8 {
        break  # Para em 8
    }
    
    print i
}
```

**Exercício 4:** Crie um programa que encontra números primos até 20.

<details>
<summary>Solução</summary>

```matter
fn is_prime(n) {
    if n <= 1 { return false }
    if n == 2 { return true }
    
    let i = 2
    while i < n {
        if n - (n / i) * i == 0 {
            return false
        }
        set i = i + 1
    }
    return true
}

let i = 2
while i <= 20 {
    if is_prime(i) {
        print i
    }
    set i = i + 1
}
```
</details>

---

## 5. Backends

### Store (Persistência)

```matter
# Salvar
store.set("username", "alice")
store.set("score", 100)

# Recuperar
let name = store.get("username")
print name

# Verificar
if store.has("score") {
    let score = store.get("score")
    print "Score: " + score
}

# Deletar
store.delete("score")

# Listar chaves
let keys = store.list()
print "Keys: " + keys.len()
```

### Math

```matter
print math.abs(0 - 10)      # 10
print math.max(5, 10)       # 10
print math.min(5, 10)       # 5
print math.pow(2, 3)        # 8
print math.sqrt(16)         # 4
print math.mod(10, 3)       # 1
print math.clamp(15, 0, 10) # 10
```

### String

```matter
let text = "Hello World"

print string.upper(text)    # HELLO WORLD
print string.lower(text)    # hello world
print string.len(text)      # 11
print string.trim("  hi  ") # hi

let parts = string.split("a,b,c", ",")
print parts[0]  # a
```

### Time & Random

```matter
# Time
let now = time.now()
print "Timestamp: " + now

# Random
print random.int()
print random.bool()

let choices = ["A", "B", "C"]
print random.choice(choices)
```

### JSON

```matter
let data = {"name": "Alice", "age": 30}

# Stringify
let json = json.stringify(data)
print json

# Parse
let parsed = json.parse(json)
print parsed["name"]
```

**Exercício 5:** Crie um sistema de high scores com persistência.

<details>
<summary>Solução</summary>

```matter
fn save_score(name, score) {
    let key = "score_" + name
    store.set(key, score)
    print "Score saved: " + name + " = " + score
}

fn get_score(name) {
    let key = "score_" + name
    if store.has(key) {
        return store.get(key)
    }
    return 0
}

save_score("alice", 100)
save_score("bob", 150)

print "Alice: " + get_score("alice")
print "Bob: " + get_score("bob")
```
</details>

---

## 6. Eventos

### Event Handlers

```matter
on boot {
    print "App started!"
    store.set("start_time", time.now())
}

on shutdown {
    print "App closing..."
}

on tick {
    print "Tick!"
}
```

### Spawn Events

```matter
on custom_event {
    print "Custom event triggered!"
}

# Disparar evento
spawn custom_event
```

### Eventos com Estado

```matter
let event_count = 0

on increment {
    set event_count = event_count + 1
    print "Event count: " + event_count
}

spawn increment
spawn increment
spawn increment
```

**Exercício 6:** Crie um sistema de notificações com eventos.

<details>
<summary>Solução</summary>

```matter
let notifications = []

on notify {
    let message = "New notification!"
    notifications.push(message)
    print message
    print "Total: " + notifications.len()
}

spawn notify
spawn notify
```
</details>

---

## 7. Projeto Prático: Sistema de Biblioteca

Vamos criar um sistema completo de gerenciamento de biblioteca.

```matter
# Estrutura de livro
struct Book {
    id: int,
    title: string,
    author: string,
    available: bool
}

# Estado
let books = []
let next_id = 1

# Adicionar livro
fn add_book(title, author) {
    let book = Book {
        id: next_id,
        title: title,
        author: author,
        available: true
    }
    
    books.push(book)
    set next_id = next_id + 1
    
    print "Book added: " + title
    return book
}

# Listar livros
fn list_books() {
    print "=== Library Books ==="
    
    for book in books {
        let status = "Available"
        if book.available == false {
            set status = "Borrowed"
        }
        
        print book.id + ". " + book.title + " by " + book.author + " [" + status + "]"
    }
}

# Emprestar livro
fn borrow_book(book_id) {
    let i = 0
    while i < books.len() {
        let book = books[i]
        if book.id == book_id {
            if book.available {
                set book.available = false
                set books[i] = book
                print "Book borrowed: " + book.title
                return ()
            } else {
                print "Book not available"
                return ()
            }
        }
        set i = i + 1
    }
    print "Book not found"
}

# Devolver livro
fn return_book(book_id) {
    let i = 0
    while i < books.len() {
        let book = books[i]
        if book.id == book_id {
            set book.available = true
            set books[i] = book
            print "Book returned: " + book.title
            return ()
        }
        set i = i + 1
    }
    print "Book not found"
}

# Buscar por autor
fn find_by_author(author) {
    print "Books by " + author + ":"
    
    for book in books {
        if book.author == author {
            print "- " + book.title
        }
    }
}

# Usar o sistema
add_book("1984", "George Orwell")
add_book("Brave New World", "Aldous Huxley")
add_book("Animal Farm", "George Orwell")

list_books()

borrow_book(1)
list_books()

return_book(1)
list_books()

find_by_author("George Orwell")
```

---

## 🎓 Conclusão

Você agora domina Matter Core! Próximos passos:

1. **Explore os exemplos** em `examples/`
2. **Veja as aplicações** em `examples/apps/`
3. **Crie seu próprio projeto**
4. **Contribua com a comunidade**

## 📖 Recursos Adicionais

- `docs/SPEC.md` - Especificação completa
- `docs/ARCHITECTURE.md` - Arquitetura do sistema
- `docs/GETTING_STARTED.md` - Guia de início rápido
- `examples/apps/README.md` - Guia de aplicações

---

**Happy coding with Matter Core! 🚀**
