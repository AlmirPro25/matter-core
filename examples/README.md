# Matter Core - Examples

Esta pasta contém exemplos práticos que demonstram os recursos do Matter Core.

## 📚 Exemplos Disponíveis

### Básicos

0. **agent_policy_demo.matter** - Demo principal: regra auditavel para agente
   ```bash
   matter run examples/agent_policy_demo.matter
   matter reflect-json examples/agent_policy_demo.matter
   matter reflexive-guard-json examples/agent_policy_demo.matter
   ```

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

### FFI / Bridges

16. **rust_ffi_plugin/** - Plugin Rust `cdylib` usando ABI JSON validada
    ```bash
    cargo build --manifest-path examples/rust_ffi_plugin/Cargo.toml
    cargo test --manifest-path examples/rust_ffi_plugin/Cargo.toml
    matter rust-ffi-validate-args-json @examples/rust_ffi_plugin/args_add.json
    matter rust-ffi-call-json F:/Users/almir/Desktop/matter_target/debug/matter_rust_ffi_plugin.dll add @examples/rust_ffi_plugin/args_add.json
    ```

    Este exemplo cobre chamada dinamica, valores tipados, erro formal e liberacao opcional de memoria via `matter_free_string`.
    Em Windows com `.cargo/config.toml` do workspace, o artefato pode sair em `F:/Users/almir/Desktop/matter_target/debug/`.

17. **go_native_plugin/** - Plugin Go `c-shared` para o bridge `cgo-native`
    ```bash
    go build -buildmode=c-shared -o F:/Users/almir/Desktop/matter_target/debug/matter_go_native_plugin.dll examples/go_native_plugin/plugin.go
    cargo test -p matter-bridge-go-native --features cgo-native
    powershell -ExecutionPolicy Bypass -File ./scripts/native-ffi-smoke.ps1
    ```

    Este exemplo cobre chamada nativa por `libloading`, argumentos JSON tipados, retorno int/string e liberacao de memoria via `matter_free_string`.

18. **node_native_host/** - Host Node.js para o addon N-API nativo
    ```bash
    cargo build -p matter-bridge-nodejs-native
    node examples/node_native_host/smoke.js F:/Users/almir/Desktop/matter_target/debug/matter_bridge_nodejs_native.node
    powershell -ExecutionPolicy Bypass -File ./scripts/native-ffi-smoke.ps1
    ```

    Este exemplo cobre carregamento real do addon por Node.js, exports N-API e chamada JSON tipada retornando int 42.

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
