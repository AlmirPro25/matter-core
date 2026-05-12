# Matter Core - Bytecode Compilado

Esta pasta contém arquivos de bytecode compilado (.mbc) para testes e demonstração.

---

## 📦 Arquivos de Bytecode

### Testes Básicos
- `simple.mbc` - Programa simples compilado
- `hello.mbc` - Hello World compilado

### Testes de Features
- `test_functions.mbc` - Testes de funções
- `test_recursion.mbc` - Testes de recursão
- `test_loops.mbc` - Testes de loops
- `test_for.mbc` - Testes de for loops
- `test_lists.mbc` - Testes de listas
- `test_maps.mbc` - Testes de maps
- `test_structs.mbc` - Testes de structs

### Testes de Escopo
- `test_shadow.mbc` - Testes de shadowing
- `test_shadow2.mbc` - Testes de shadowing avançado

### Outros
- `loops.mbc` - Loops compilados
- `recursion.mbc` - Recursão compilada

---

## 🔧 Uso

### Compilar Source para Bytecode

```bash
matter compile programa.matter -o programa.mbc
```

### Executar Bytecode

```bash
matter run-bytecode programa.mbc
```

### Inspecionar Bytecode

```bash
matter inspect programa.mbc
```

---

## 📊 Formato MBC1

O formato MBC1 (Matter ByteCode version 1) é o formato binário usado pelo Matter Core.

### Estrutura

```
MBC1 File:
├── Header (magic number, version)
├── Constants Pool
├── Functions Table
├── Event Handlers
└── Instructions
```

### Vantagens

- **Portável:** Executável em qualquer plataforma
- **Compacto:** Menor que source code
- **Rápido:** Não precisa parsing
- **Distribuível:** Pode distribuir sem source

---

## 🧪 Testes

### Equivalência

Todos os arquivos .mbc devem produzir o mesmo resultado que o source:

```bash
# Executar source
matter run programa.matter

# Compilar e executar bytecode
matter compile programa.matter -o programa.mbc
matter run-bytecode programa.mbc

# Resultados devem ser idênticos
```

### Validação

```bash
# Testar equivalência
.\scripts\test_bytecode_equivalence.ps1
```

---

## 📝 Notas

- Arquivos .mbc são binários, não edite manualmente
- Sempre recompile após modificar o source
- Use `matter inspect` para debug
- Bytecode é versionado (MBC1, MBC2, etc)

---

**Última atualização:** 9 de Maio de 2026  
**Formato:** MBC1
