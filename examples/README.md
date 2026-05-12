# Matter Core - Examples

Esta pasta contém exemplos práticos que demonstram os recursos do Matter Core.

## 📚 Exemplos Disponíveis

### Básicos

1. **hello.matter** - Hello World simples
   ```bash
   matter run examples/hello.matter
   ```

2. **functions.matter** - Definição e uso de funções
   ```bash
   matter run examples/functions.matter
   ```

3. **showcase.matter** - Tour completo da linguagem
   ```bash
   matter run examples/showcase.matter
   ```

### Intermediários

4. **calculator.matter** - Calculadora com funções matemáticas
   ```bash
   matter run examples/calculator.matter
   ```

5. **fibonacci.matter** - Sequência de Fibonacci (recursivo e iterativo)
   ```bash
   matter run examples/fibonacci.matter
   ```

6. **data_processing.matter** - Manipulação de listas e estatísticas
   ```bash
   matter run examples/data_processing.matter
   ```

### Avançados

7. **event_driven_app.matter** - Aplicação orientada a eventos
   ```bash
   matter run examples/event_driven_app.matter
   matter emit examples/event_driven_app.matter tap
   ```

8. **backend_integration.matter** - Demonstração de todos os 10 backends
   ```bash
   matter run examples/backend_integration.matter
   ```

9. **todo_app.matter** - Aplicação Todo completa com estado
   ```bash
   matter run examples/todo_app.matter
   matter emit examples/todo_app.matter add_task
   ```

### Visual (PVM/PXL)

10. **visual_basic.matter** - Comandos visuais básicos
    ```bash
    matter run examples/visual_basic.matter
    ```

11. **visual_event.matter** - Visual com eventos
    ```bash
    matter run examples/visual_event.matter
    ```

12. **visual_advanced.matter** - Propriedades visuais avançadas
    ```bash
    matter run examples/visual_advanced.matter
    ```

13. **visual_load.matter** - Carregamento de PVMBC
    ```bash
    matter run examples/visual_load.matter
    ```

### Standard Library

14. **stdlib_demo.matter** - Demonstração da biblioteca padrão
    ```bash
    matter run examples/stdlib_demo.matter
    ```

15. **json_api_demo.matter** - Uso de JSON API
    ```bash
    matter run examples/json_api_demo.matter
    ```

## 🎯 Casos de Uso

### Aprendizado

Use os exemplos básicos para aprender a sintaxe:
```bash
matter run examples/hello.matter
matter run examples/functions.matter
matter run examples/showcase.matter
```

### Experimentação

Use o REPL para experimentar interativamente:
```bash
matter repl
[1]> let x = 10
[2]> print x
10
[3]> :quit
```

### Desenvolvimento

Use os exemplos avançados como templates:
```bash
# Copie um exemplo
cp examples/todo_app.matter my_app.matter

# Modifique e execute
matter run my_app.matter
```

## 🚀 Dicas

### Executar com Eventos

Muitos exemplos suportam eventos:
```bash
# Executar normalmente
matter run examples/event_driven_app.matter

# Emitir evento específico
matter emit examples/event_driven_app.matter tap
matter emit examples/event_driven_app.matter shutdown
```

### Compilar para Bytecode

Compile exemplos para distribuição:
```bash
# Compilar
matter compile examples/calculator.matter -o calculator.mbc

# Executar bytecode
matter run-bytecode calculator.mbc

# Inspecionar
matter inspect calculator.mbc
```

### Usar no REPL

Carregue funções de exemplos no REPL:
```bash
matter repl

# No REPL, defina funções do fibonacci.matter
[1]> fn fib(n) { if n <= 1 { return n } return fib(n-1) + fib(n-2) }
[2]> print fib(10)
55
```

## 📖 Documentação

Para mais informações:
- `matter help` - Ajuda geral
- `matter help run` - Ajuda específica
- `matter backends` - Lista de backends
- `matter examples` - Lista de exemplos

## 🎓 Próximos Passos

1. Execute todos os exemplos básicos
2. Experimente no REPL
3. Modifique os exemplos
4. Crie seus próprios programas
5. Compartilhe com a comunidade!

---

**Matter Core v0.2.0** - Runtime-Oriented Language System
