# Quick Start: Visual Backend

Guia rápido para usar o backend visual PVM/PXL integrado ao Matter Core.

---

## 🚀 Instalação

O backend visual já está integrado ao Matter Core. Nenhuma instalação adicional necessária.

```bash
# Compilar Matter Core
cargo build --release

# Ou usar o instalador (Windows)
.\install.ps1
```

---

## 📝 Seu Primeiro Programa Visual

### 1. Criar arquivo `hello_visual.matter`

```matter
# Criar superfície visual
visual.surface("main", 1080, 1920)

# Criar região
visual.region("hello", 100, 100, 300, 80)

# Animar região
visual.pulse("hello")

# Executar aplicação
visual.run("hello_app")
```

### 2. Executar

```bash
matter run hello_visual.matter
```

### 3. Output esperado

```
[VISUAL] surface main 1080x1920
[VISUAL] region hello x=100 y=100 w=300 h=80
[VISUAL] pulse hello
[VISUAL] run hello_app
```

---

## 🎨 API Visual

### Comandos Disponíveis

#### `visual.run(app_name)`
Executa uma aplicação visual.

```matter
visual.run("pizzaria")
```

#### `visual.load(path)`
Carrega bytecode visual pré-compilado (PVMBC).

```matter
visual.load("apps/pizzaria.pvmbc")
```

#### `visual.surface(name, width, height)`
Cria uma superfície visual.

```matter
visual.surface("main", 1080, 1920)
```

#### `visual.region(name, x, y, w, h)`
Cria uma região visual.

```matter
visual.region("button", 100, 200, 300, 80)
```

#### `visual.pulse(target)`
Anima uma região com efeito pulse.

```matter
visual.pulse("button")
```

#### `visual.set(target, key, value)`
Define uma propriedade de uma região.

```matter
visual.set("button", "energy", 100)
visual.set("button", "material", "glass")
```

---

## 🎯 Exemplos Práticos

### Exemplo 1: Interface Básica

```matter
# Criar superfície
visual.surface("main", 1080, 1920)

# Criar botões
visual.region("btn_start", 100, 100, 200, 60)
visual.region("btn_stop", 100, 180, 200, 60)

# Animar botão de start
visual.pulse("btn_start")
```

### Exemplo 2: Com Eventos

```matter
on boot {
    visual.surface("main", 1080, 1920)
    visual.region("menu", 50, 50, 400, 60)
    print "Interface inicializada"
}

on tap {
    visual.pulse("menu")
    print "Menu ativado"
}
```

### Exemplo 3: Propriedades Visuais

```matter
# Criar região
visual.region("checkout", 100, 200, 300, 80)

# Configurar propriedades
visual.set("checkout", "energy", 100)
visual.set("checkout", "material", "glass")
visual.set("checkout", "behavior", "pulse")

# Animar
visual.pulse("checkout")
```

### Exemplo 4: Carregar PVMBC

```matter
on boot {
    # Carregar aplicação pré-compilada
    visual.load("apps/pizzaria.pvmbc")
    
    # Executar
    visual.run("pizzaria")
}
```

---

## 🧪 Testar Exemplos

O Matter Core vem com 4 exemplos visuais prontos:

```bash
# Exemplo básico
matter run examples/visual_basic.matter

# Exemplo com eventos
matter emit examples/visual_event.matter boot

# Exemplo avançado
matter emit examples/visual_advanced.matter boot

# Exemplo com PVMBC
matter emit examples/visual_load.matter boot
```

---

## 🔧 Compilar para Bytecode

```bash
# Compilar para bytecode
matter compile hello_visual.matter -o hello.mbc

# Executar bytecode
matter run-bytecode hello.mbc
```

O bytecode preserva todos os comandos visuais!

---

## 📚 Documentação Completa

Para mais detalhes, consulte:

- **`docs/VISUAL_BACKEND.md`** - Documentação completa
- **`docs/SPEC.md`** - Especificação da linguagem
- **`README.md`** - Visão geral do projeto

---

## 🎓 Conceitos Importantes

### 1. Backend Desacoplado

O visual é um **backend plugável**. Matter não depende do PVM.

```
Matter Core (linguagem geral)
    ↓
Backends (plugáveis)
    └── visual (PVM/PXL)
```

### 2. Trace vs PVM Real

**Atualmente**: `TraceVisualBackend` (mock/trace)
- Imprime comandos visuais
- Permite testar sem PVM
- Perfeito para desenvolvimento

**Futuro**: `PvmVisualBackend` (PVM real)
- Integração com PVM runtime
- SmartPixels e matéria visual
- Execução PVMBC real

### 3. Contrato Primeiro

A API visual está **completa e estável**. Quando o PVM estiver pronto, basta trocar o backend.

---

## 🐛 Troubleshooting

### Comando visual não funciona

Verifique a sintaxe:
```matter
# ✅ Correto
visual.surface("main", 1080, 1920)

# ❌ Errado (faltam argumentos)
visual.surface("main")
```

### Erro de tipo

Certifique-se de usar os tipos corretos:
```matter
# ✅ Correto
visual.region("button", 100, 200, 300, 80)

# ❌ Errado (strings em vez de números)
visual.region("button", "100", "200", "300", "80")
```

---

## 💡 Dicas

### 1. Use eventos para inicialização

```matter
on boot {
    visual.surface("main", 1080, 1920)
    # ... resto da inicialização
}
```

### 2. Combine com outros backends

```matter
on boot {
    visual.run("app")
    agent.say("Aplicação iniciada")
    store.set("status", "running")
}
```

### 3. Organize regiões

```matter
# Criar todas as regiões primeiro
visual.region("header", 0, 0, 1080, 100)
visual.region("content", 0, 100, 1080, 1720)
visual.region("footer", 0, 1820, 1080, 100)

# Depois configurar propriedades
visual.set("header", "material", "glass")
visual.set("footer", "material", "metal")
```

---

## 🎉 Próximos Passos

1. ✅ Experimente os exemplos em `examples/`
2. ✅ Crie sua própria interface visual
3. ✅ Combine visual com eventos Matter
4. ✅ Explore a documentação completa

---

## 📞 Suporte

- **Documentação**: `docs/VISUAL_BACKEND.md`
- **Exemplos**: `examples/visual_*.matter`
- **Testes**: `cargo test --package matter-visual`

---

**Divirta-se criando interfaces visuais com Matter! 🎨✨**

