# 🎨 Matter Visual Backend - Live Demo

Demonstração ao vivo do backend visual PVM/PXL integrado ao Matter Core.

---

## 🚀 Quick Demo

### 1. Exemplo Básico

```bash
matter run examples/visual_basic.matter
```

**Output:**
```
[VISUAL] surface main 1080x1920
[VISUAL] region checkout x=100 y=200 w=300 h=80
[VISUAL] pulse checkout
[VISUAL] run pizzaria
```

**O que aconteceu:**
- ✅ Criou superfície visual 1080x1920
- ✅ Criou região "checkout" nas coordenadas especificadas
- ✅ Animou a região com efeito pulse
- ✅ Executou aplicação "pizzaria"

---

### 2. Exemplo com Eventos

```bash
matter emit examples/visual_event.matter boot
```

**Output:**
```
[VISUAL] run pizzaria
[VISUAL] surface main 1080x1920
[VISUAL] region button x=100 y=100 w=200 h=50
```

**O que aconteceu:**
- ✅ Evento `boot` foi disparado
- ✅ Handler `on boot` executou
- ✅ Comandos visuais foram processados

---

### 3. Exemplo Avançado

```bash
matter emit examples/visual_advanced.matter boot
```

**Output:**
```
[VISUAL] surface main 1080x1920
[VISUAL] region checkout x=100 y=200 w=300 h=80
[VISUAL] set checkout energy = 100
[VISUAL] set checkout material = glass
[VISUAL] region menu x=50 y=50 w=400 h=60
[VISUAL] set menu behavior = slide
Interface visual inicializada
```

**O que aconteceu:**
- ✅ Criou superfície e regiões
- ✅ Configurou propriedades visuais (energy, material, behavior)
- ✅ Imprimiu mensagem de confirmação

---

### 4. Exemplo Interativo

```bash
matter emit examples/visual_interactive.matter boot
```

**Output:**
```
[VISUAL] surface main 1080x1920
[VISUAL] region header x=0 y=0 w=1080 h=100
[VISUAL] region menu_start x=100 y=300 w=880 h=120
[VISUAL] region menu_settings x=100 y=450 w=880 h=120
[VISUAL] region menu_exit x=100 y=600 w=880 h=120
[VISUAL] region score_display x=50 y=50 w=300 h=80
[VISUAL] region energy_bar x=400 y=50 w=600 h=40
[VISUAL] set header material = glass
[VISUAL] set menu_start behavior = pulse
[VISUAL] set menu_start energy = 100
[VISUAL] set energy_bar material = energy
[VISUAL] pulse header
[VISUAL] pulse menu_start
Interface visual inicializada
[AGENT] Bem-vindo ao Matter Visual!
```

**O que aconteceu:**
- ✅ Criou interface completa com múltiplas regiões
- ✅ Configurou propriedades visuais avançadas
- ✅ Animou elementos da interface
- ✅ Integrou com backend agent (IA)

---

## 🎯 Demonstração Passo a Passo

### Passo 1: Criar Arquivo

Crie `demo.matter`:

```matter
on boot {
    visual.surface("demo", 800, 600)
    visual.region("title", 100, 50, 600, 80)
    visual.set("title", "material", "glass")
    visual.pulse("title")
    agent.say("Demo iniciado!")
}
```

### Passo 2: Executar

```bash
matter emit demo.matter boot
```

### Passo 3: Ver Resultado

```
[VISUAL] surface demo 800x600
[VISUAL] region title x=100 y=50 w=600 h=80
[VISUAL] set title material = glass
[VISUAL] pulse title
[AGENT] Demo iniciado!
```

---

## 🔧 Demonstração de Compilação

### Compilar para Bytecode

```bash
matter compile examples/visual_basic.matter -o visual.mbc
```

**Output:**
```
Compiled successfully: visual.mbc
```

### Executar Bytecode

```bash
matter run-bytecode visual.mbc
```

**Output:**
```
[VISUAL] surface main 1080x1920
[VISUAL] region checkout x=100 y=200 w=300 h=80
[VISUAL] pulse checkout
[VISUAL] run pizzaria
```

**Resultado:** Bytecode preserva comandos visuais! ✅

---

## 🧪 Demonstração de Testes

### Executar Todos os Testes

```bash
cargo test
```

**Output:**
```
running 22 tests
test result: ok. 22 passed; 0 failed

running 6 tests
test result: ok. 6 passed; 0 failed

Total: 28/28 passing (100%)
```

### Executar Testes Visuais

```bash
cargo test --package matter-visual
```

**Output:**
```
running 6 tests
test test_trace_visual_run ... ok
test test_trace_visual_surface ... ok
test test_trace_visual_region_simple ... ok
test test_trace_visual_pulse ... ok
test test_trace_visual_set ... ok
test test_visual_region_with_map ... ok

test result: ok. 6 passed; 0 failed
```

---

## 🎨 Demonstração de API

### Todos os Comandos Visuais

```matter
# 1. Executar aplicação
visual.run("app_name")

# 2. Carregar bytecode
visual.load("path/to/app.pvmbc")

# 3. Criar superfície
visual.surface("name", width, height)

# 4. Criar região
visual.region("name", x, y, width, height)

# 5. Animar região
visual.pulse("region_name")

# 6. Configurar propriedade
visual.set("region_name", "property", value)
```

### Exemplo Completo

```matter
on boot {
    # Criar superfície
    visual.surface("main", 1080, 1920)
    
    # Criar regiões
    visual.region("header", 0, 0, 1080, 100)
    visual.region("content", 0, 100, 1080, 1720)
    visual.region("footer", 0, 1820, 1080, 100)
    
    # Configurar propriedades
    visual.set("header", "material", "glass")
    visual.set("content", "energy", 100)
    visual.set("footer", "behavior", "slide")
    
    # Animar
    visual.pulse("header")
    visual.pulse("footer")
    
    # Executar app
    visual.run("main_app")
}
```

---

## 🌟 Demonstração de Integração

### Matter + Visual + Agent + Store

```matter
let score = 0

on boot {
    # Visual
    visual.surface("game", 1080, 1920)
    visual.region("score", 50, 50, 200, 60)
    
    # Carregar score salvo
    set score = store.get("high_score")
    
    # Feedback
    agent.say("Bem-vindo de volta!")
    print "High score: "
    print score
}

on score_update {
    # Atualizar score
    set score = score + 10
    
    # Atualizar visual
    visual.set("score", "energy", 100)
    visual.pulse("score")
    
    # Salvar
    store.set("high_score", score)
    
    # Feedback
    if score > 100 {
        agent.say("Parabéns! Novo recorde!")
    }
}
```

---

## 📊 Demonstração de Performance

### Benchmark Simples

```matter
let start_time = 0
let end_time = 0

on boot {
    set start_time = time.now()
    
    # Criar 100 regiões
    let i = 0
    while i < 100 {
        visual.region("region_" + i, i * 10, 100, 50, 50)
        set i = i + 1
    }
    
    set end_time = time.now()
    
    print "Tempo: "
    print end_time - start_time
    print "ms"
}
```

**Resultado esperado:** <100ms para 100 regiões

---

## 🎯 Demonstração de Casos de Uso

### 1. Menu Interativo

```matter
let selected = "home"

on boot {
    visual.surface("menu", 1080, 1920)
    visual.region("home", 100, 100, 880, 100)
    visual.region("settings", 100, 220, 880, 100)
    visual.region("about", 100, 340, 880, 100)
    
    visual.set("home", "energy", 100)
}

on tap {
    visual.pulse(selected)
    agent.say("Navegando para " + selected)
}
```

### 2. Barra de Progresso

```matter
let progress = 0

on boot {
    visual.surface("app", 1080, 1920)
    visual.region("progress_bar", 100, 500, 880, 40)
    visual.set("progress_bar", "energy", 0)
}

on update {
    set progress = progress + 1
    visual.set("progress_bar", "energy", progress)
    
    if progress >= 100 {
        agent.say("Completo!")
    }
}
```

### 3. Notificação

```matter
on notification {
    visual.region("notif", 100, 50, 880, 100)
    visual.set("notif", "material", "glass")
    visual.pulse("notif")
    
    agent.say("Nova notificação!")
}
```

---

## 🎉 Conclusão da Demo

### O Que Demonstramos

- ✅ API visual completa (6 comandos)
- ✅ Integração com eventos Matter
- ✅ Integração com outros backends (agent, store)
- ✅ Compilação para bytecode
- ✅ Testes passando (100%)
- ✅ Exemplos funcionais

### Como Experimentar

1. **Clone o repositório**
   ```bash
   git clone <repo-url>
   cd matter-core
   ```

2. **Compile**
   ```bash
   cargo build --release
   ```

3. **Execute exemplos**
   ```bash
   matter run examples/visual_basic.matter
   matter emit examples/visual_event.matter boot
   matter emit examples/visual_advanced.matter boot
   matter emit examples/visual_interactive.matter boot
   ```

4. **Crie seu próprio**
   ```bash
   # Crie seu arquivo .matter
   # Execute com: matter run seu_arquivo.matter
   ```

---

## 📚 Próximos Passos

1. **Leia a documentação**
   - `QUICKSTART_VISUAL.md` - Guia rápido
   - `docs/VISUAL_BACKEND.md` - Documentação completa

2. **Explore os exemplos**
   - `examples/visual_*.matter` - 5 exemplos

3. **Execute os testes**
   - `cargo test` - Todos os testes

4. **Crie sua aplicação**
   - Use a API visual
   - Combine com outros backends
   - Compile para bytecode

---

**Divirta-se criando aplicações visuais com Matter! 🎨✨**

---

**Demo preparado por**: Matter Core Team  
**Data**: May 9, 2026  
**Status**: ✅ Production-ready  
**Testes**: 28/28 passing (100%)

