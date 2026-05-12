# Matter Polyglot Examples

Exemplos de uso do sistema Matter Polyglot - importando e usando bibliotecas de **5 linguagens diferentes**!

## 🌍 **LINGUAGENS SUPORTADAS**

1. ✅ **Python** - Machine Learning, Data Science
2. ✅ **Node.js** - Web Servers, APIs
3. ✅ **Rust** - Performance, Processamento Paralelo
4. ✅ **Go** - Concorrência, Microservices
5. ✅ **Java** - Enterprise, Business Logic

**Total: 3.6M+ packages acessíveis!** 🚀

---

## 🐍 **Python Examples**

### NumPy Integration
```bash
matter run examples/polyglot/python_numpy.matter
```

Demonstra:
- Criação de arrays NumPy
- Operações estatísticas (mean, std, sum)
- Operações matemáticas (square, sqrt)
- Multiplicação de matrizes
- Transposição

### Pandas Integration
```bash
matter run examples/polyglot/python_pandas.matter
```

Demonstra:
- Criação de DataFrames
- Estatísticas descritivas
- Filtragem de dados
- Análise de dados de vendas
- Cálculos agregados

### Machine Learning (scikit-learn)
```bash
matter run examples/polyglot/python_ml.matter
```

Demonstra:
- Regressão Linear
- Regressão Logística
- Cross-validation
- Predições
- Métricas de performance

---

## 📦 **Node.js Examples**

### Express Web Server
```bash
matter run examples/polyglot/nodejs_express.matter
```

Demonstra:
- Criação de servidor Express
- Rotas GET/POST
- JSON responses
- Middleware
- API REST

### Axios HTTP Client
```bash
matter run examples/polyglot/nodejs_axios.matter
```

Demonstra:
- Requisições HTTP
- GET/POST requests
- Headers customizados
- Error handling
- Async/await

---

## 🦀 **Rust Examples**

### Rayon Parallel Processing
```bash
matter run examples/polyglot/rust_rayon.matter
```

Demonstra:
- Processamento paralelo
- Par_iter
- Map/filter/reduce paralelo
- Performance gains

---

## 🐹 **Go Examples**

### Gin Web Framework
```bash
matter run examples/polyglot/go_gin.matter
```

Demonstra:
- Servidor web com Gin
- Rotas REST
- JSON responses
- Parâmetros de rota
- Middleware

### Go Concurrency
```bash
matter run examples/polyglot/go_concurrency.matter
```

Demonstra:
- Goroutines
- Channels
- Worker pools
- WaitGroups
- Concorrência nativa

---

## ☕ **Java Examples**

### Spring Boot REST API
```bash
matter run examples/polyglot/java_spring.matter
```

Demonstra:
- Spring Boot application
- REST controllers
- GET/POST endpoints
- Dependency injection
- Auto-configuration

### Java Collections and Streams
```bash
matter run examples/polyglot/java_collections.matter
```

Demonstra:
- ArrayList e HashMap
- Stream API
- Filter/map/reduce
- Sorting
- Lambda expressions

---

## 🌍 **Hybrid Examples**

### Python + Node.js ML API
```bash
matter run examples/polyglot/hybrid_ml_api.matter
```

Demonstra:
- Python para treinar modelo ML
- Node.js para servir API
- Integração perfeita
- Código híbrido em um arquivo

### Ultimate Hybrid System (5 Linguagens!)
```bash
matter run examples/polyglot/ultimate_hybrid.matter
```

Demonstra:
- **Python** - Machine Learning e Data Science
- **Rust** - Processamento paralelo
- **Java** - Business logic
- **Go** - Microservices
- **Node.js** - Main API
- **TUDO EM UM ARQUIVO!** 🚀

---

## 📦 **Instalação de Dependências**

### Python
```bash
pip install numpy pandas scikit-learn
```

### Node.js
```bash
npm install express axios
```

### Rust
```bash
cargo install rayon serde
```

### Go
```bash
go get github.com/gin-gonic/gin
```

### Java
```bash
# Spring Boot via Maven/Gradle
# Ou use Spring Initializr
```

Ou use o gerenciador do Matter:
```bash
matter install
```

---

## 🎯 **Como Funciona**

### 1. Import Syntax

```matter
# Python
import "numpy" from python as np
import "pandas" from python as pd

# Node.js
import "express" from nodejs
import "axios" from nodejs

# Rust
import "rayon" from rust

# Go
import "github.com/gin-gonic/gin" from go

# Java
import "org.springframework.boot.SpringApplication" from java
```

### 2. Uso Direto

```matter
# Python: NumPy
let arr = np.array([1, 2, 3])
let mean = np.mean(arr)

# Node.js: Express
let app = express()
app.listen(3000)

# Rust: Rayon
let result = rayon.par_iter(data).map(|x| x * 2).collect()

# Go: Gin
let router = gin.Default()
router.Run(":8080")

# Java: Collections
let list = ArrayList.new()
list.add("item")
```

### 3. Conversão Automática de Tipos

Matter converte automaticamente entre tipos:

| Matter | Python | JavaScript | Rust | Go | Java |
|--------|--------|------------|------|----|----- |
| int | int | number | i64 | int | int |
| float | float | number | f64 | float64 | double |
| string | str | string | String | string | String |
| bool | bool | boolean | bool | bool | boolean |
| list | list | Array | Vec<T> | []T | ArrayList |
| map | dict | Object | HashMap | map | HashMap |

---

## 📚 **Documentação**

Ver `SPRINT_39_POLYGLOT_PLAN.md` para detalhes técnicos completos.

Ver `MATTER_POLYGLOT_FINAL.md` para visão geral do sistema.

Ver `MATTER_FOR_AI_AGENTS.md` para uso por IA.

---

## 🎉 **Conclusão**

**Matter Polyglot permite:**
- ✅ Usar a melhor linguagem para cada tarefa
- ✅ Acesso a 3.6M+ packages
- ✅ Código híbrido em um arquivo
- ✅ Zero overhead (FFI direto)
- ✅ Conversão automática de tipos

**Nenhuma outra linguagem tem isso!** 🏆
