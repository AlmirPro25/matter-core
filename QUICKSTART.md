# Matter Core - Quick Start Guide

## 🚀 Instalação Rápida

```bash
# 1. Compilar o projeto
cargo build --release

# 2. Testar instalação
.\target\release\matter-cli.exe run examples\showcase.matter
```

## 📝 Seu Primeiro Programa

Crie um arquivo `meu_programa.matter`:

```matter
# Variáveis
let nome = "Mundo"
let numero = 42

# Print
print "Olá"
print nome
print numero

# Aritmética
let resultado = 10 + 20
print resultado

# Condicionais
if resultado > 25 {
    print "Resultado é grande!"
}

# Estado mutável
set resultado = resultado * 2
print resultado

# Backend calls
agent.say("Meu primeiro programa Matter!")
```

Execute:

```bash
.\target\release\matter-cli.exe run meu_programa.matter
```

Tambem da para executar codigo direto, sem criar arquivo. Esse caminho e o mais util para API, ChatGPT, agentes e execucao em nuvem:

```bash
.\target\release\matter-cli.exe eval "print 41 + 1"
.\target\release\matter-cli.exe eval-json "print 41 + 1"
```

Ou enviar Matter Core por stdin usando `-`:

```bash
"print 7 * 6" | .\target\release\matter-cli.exe run -
"print 1 print 2" | .\target\release\matter-cli.exe run-json -
"print 10" | .\target\release\matter-cli.exe check -
"print 10" | .\target\release\matter-cli.exe check-json -
"on boot { print \"ok\" }" | .\target\release\matter-cli.exe emit-json - boot
"print 10" | .\target\release\matter-cli.exe compile - -o programa.mbc
"print 10" | .\target\release\matter-cli.exe compile-json - -o programa.mbc
```

### Descobrir capacidades da API
```bash
.\target\release\matter-cli.exe capabilities-json
```

### Inspecionar pacote
```bash
.\target\release\matter-cli.exe package-json
.\target\release\matter-cli.exe package-json matter.toml
```

### Executar projeto pelo manifesto
```bash
.\target\release\matter-cli.exe project-check-json
.\target\release\matter-cli.exe project-run-json
.\target\release\matter-cli.exe project-imports-json
.\target\release\matter-cli.exe project-compile-json -o target\project.mbc
```

Dentro de comandos de projeto, dependências locais do `matter.toml` também podem ser importadas por alias:

```matter
import "math_tools"
```

## 🎯 Comandos Principais

### Executar programa
```bash
.\target\release\matter-cli.exe run <arquivo.matter|->
```

### Executar codigo direto
```bash
.\target\release\matter-cli.exe eval "print 42"
```

### Executar codigo direto para API
```bash
.\target\release\matter-cli.exe eval-json "print 42"

### Inspecionar tokens para API
```bash
.\target\release\matter-cli.exe tokens-json <arquivo.matter|->
```

### Inspecionar imports para API
```bash
.\target\release\matter-cli.exe imports-json <arquivo.matter|->
```

### Usar a biblioteca padrao
```matter
import "std/math.matter"

print square(6)
print clamp(99, 0, 10)
```

```bash
.\target\release\matter-cli.exe run examples\test_stdlib.matter
```

### Persistir estado
```matter
store.set("counter", 41)
print store.get("counter")
print store.has("counter")
```

```bash
$env:MATTER_STORE_PATH="target\meu_store.json"
.\target\release\matter-cli.exe run examples\test_store.matter
```

### Fazer chamadas HTTP
```matter
print net.status("http://example.com")
print net.ok("http://example.com")
```

```bash
.\target\release\matter-cli.exe run examples\network.matter
```

### Enfileirar eventos
```matter
on boot {
    print "boot"
    spawn tick
}

on tick {
    print "tick"
}

spawn boot
print "main"
```

### Executar para API
```bash
.\target\release\matter-cli.exe run-json <arquivo.matter|->
```

### Validar sem executar
```bash
.\target\release\matter-cli.exe check <arquivo.matter|->
```

### Validar para API
```bash
.\target\release\matter-cli.exe check-json <arquivo.matter|->
```
### Disparar evento
```bash
.\target\release\matter-cli.exe emit <arquivo.matter|-> <nome_evento>
```

### Disparar evento para API
```bash
.\target\release\matter-cli.exe emit-json <arquivo.matter|-> <nome_evento>
```

### Compilar bytecode
```bash
.\target\release\matter-cli.exe compile <arquivo.matter|-> -o output.mbc
```

### Compilar bytecode para API
```bash
.\target\release\matter-cli.exe compile-json <arquivo.matter|-> -o output.mbc
```

### Inspecionar bytecode para API
```bash
.\target\release\matter-cli.exe inspect-json <arquivo.mbc>
```

### Executar bytecode para API
```bash
.\target\release\matter-cli.exe run-bytecode-json <arquivo.mbc>
```

### Disparar evento de bytecode para API
```bash
.\target\release\matter-cli.exe emit-bytecode-json <arquivo.mbc> <nome_evento>
```

### Testar ponte API/JSON
```bash
powershell -ExecutionPolicy Bypass -File .\test_api_bridge.ps1
```

### Validacao completa
```bash
powershell -ExecutionPolicy Bypass -File .\test_all.ps1
```

## 📚 Exemplos Prontos

```bash
# Exemplo básico
.\target\release\matter-cli.exe run examples\simple.matter

# Showcase completo
.\target\release\matter-cli.exe run examples\showcase.matter

# Backend calls
.\target\release\matter-cli.exe run examples\backend.matter

# Eventos
.\target\release\matter-cli.exe emit examples\events.matter boot

# Estado mutável
.\target\release\matter-cli.exe run examples\state.matter
```

## 🔧 Sintaxe Essencial

### Variáveis
```matter
let x = 10          # declaração
set x = x + 1       # mutação
```

### Operadores
```matter
# Aritmética
+ - * /

# Comparação
== != < > <= >=
```

### Controle de Fluxo
```matter
if condicao {
    # código
}

if condicao {
    # então
} else {
    # senão
}
```

### Funções (básico)
```matter
fn nome(param1, param2) {
    return param1 + param2
}
```

### Imports locais
```matter
import "modules/math_tools.matter"

print dobro(21)
```

Imports usam caminhos relativos ao arquivo que declara o import.

### Eventos
```matter
on boot {
    print "Iniciado"
}

on shutdown {
    print "Finalizando"
}
```

### Backend Calls
```matter
agent.say("mensagem")
visual.run("app_name")
```

## 🧪 Testar Tudo

```bash
cargo test
```

## 📖 Documentação Completa

- [README.md](README.md) - Visão geral completa
- [docs/MANIFESTO.md](docs/MANIFESTO.md) - Filosofia e princípios
- [docs/SPEC.md](docs/SPEC.md) - Especificação da linguagem
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - Arquitetura técnica

## ⚡ Dicas

1. **Comentários**: Use `#` para comentários de linha
2. **Strings**: Use aspas duplas `"texto"`
3. **Números**: Apenas inteiros por enquanto
4. **Booleanos**: `true` e `false`
5. **Print**: Use `print` para saída no console

## 🐛 Troubleshooting

### Erro de compilação Rust
```bash
# Instalar/atualizar Rust
rustup update

# Usar toolchain GNU (Windows)
rustup default stable-gnu
```

### Arquivo não encontrado
```bash
# Use caminhos relativos ou absolutos
.\target\release\matter-cli.exe run .\examples\hello.matter
```

## 🎓 Próximos Passos

1. Execute todos os exemplos em `examples/`
2. Crie seus próprios programas Matter
3. Leia a documentação completa
4. Explore o código fonte em `crates/`
5. Contribua com o projeto!

---

**Bem-vindo ao Matter Core!** 🚀
