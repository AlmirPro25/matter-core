# Visual Backend / PVM Integration

## Visão Geral

O **Visual Backend** é a integração oficial do sistema visual PVM/PXL com o Matter Core. Esta integração mantém a arquitetura desacoplada: **Matter Core permanece uma linguagem geral**, e o visual é apenas um **backend/plugin/target**.

## Arquitetura

```
Matter Core (linguagem geral)
    ↓
.matter → lexer → parser → AST → semantic check → MBC1 → VM/runtime
    ↓
Backends (plugáveis)
    ├── agent (IA/LLM)
    ├── visual (PVM/PXL) ← NOVO
    ├── store (persistência)
    └── net (rede)
```

### Princípio Fundamental

**Matter NÃO depende diretamente do PVM.**

- Matter controla a **intenção geral** (lógica, eventos, estado)
- PXL/PVM **materializa visualmente** (surfaces, regions, SmartPixels)
- O backend visual começa como **trace/mock** e depois pode ser conectado ao **PVM real**

## API Visual

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
Cria uma região visual (forma simples).

```matter
visual.region("checkout", 100, 200, 300, 80)
```

#### `visual.region(name, config)`
Cria uma região visual com configuração completa (futuro).

```matter
visual.region("checkout", {
    x: 100,
    y: 200,
    w: 300,
    h: 80,
    semantic: "action_button",
    behavior: "pulse",
    energy: 1
})
```

#### `visual.pulse(target)`
Anima uma região com efeito pulse.

```matter
visual.pulse("checkout")
```

#### `visual.set(target, key, value)`
Define uma propriedade de uma região.

```matter
visual.set("checkout", "energy", 80)
visual.set("checkout", "material", "glass")
```

## Estrutura do Crate

```
crates/matter-visual/
├── Cargo.toml
└── src/
    └── lib.rs
        ├── VisualRuntime trait (contrato para PVM)
        ├── VisualRegionSpec (especificação de região)
        ├── VisualError (erros do sistema visual)
        ├── TraceVisualBackend (implementação trace/mock)
        └── PvmVisualBackend (placeholder para PVM real)
```

## Implementação Atual: TraceVisualBackend

A implementação atual é um **backend trace/mock** que imprime comandos visuais:

```
[VISUAL] surface main 1080x1920
[VISUAL] region checkout x=100 y=200 w=300 h=80
[VISUAL] pulse checkout
[VISUAL] run pizzaria
```

### Por que começar com trace?

1. **Contrato primeiro**: Define a API antes da implementação
2. **Testes imediatos**: Permite testar a integração sem PVM
3. **Desenvolvimento paralelo**: Matter e PVM podem evoluir independentemente
4. **Validação de design**: Garante que a API faz sentido antes de implementar

## Integração Futura: PvmVisualBackend

Quando o PVM estiver pronto, criaremos `PvmVisualBackend`:

```rust
pub struct PvmVisualBackend {
    pvm_runtime: PvmRuntime,
    pvmbc_loader: PvmbcLoader,
    surfaces: HashMap<String, Surface>,
    regions: HashMap<String, Region>,
}

impl VisualRuntime for PvmVisualBackend {
    fn run_app(&mut self, name: &str) -> Result<(), VisualError> {
        // Integração real com PVM
        self.pvm_runtime.execute_app(name)
    }
    
    fn create_surface(&mut self, name: &str, width: i64, height: i64) -> Result<(), VisualError> {
        // Criar surface real no PVM
        let surface = self.pvm_runtime.create_surface(width, height);
        self.surfaces.insert(name.to_string(), surface);
        Ok(())
    }
    
    // ... outras implementações
}
```

## Validação Semântica

O semantic checker valida aridade mínima:

- `visual.run(name)` → 1 argumento string
- `visual.load(path)` → 1 argumento string
- `visual.pulse(target)` → 1 argumento string
- `visual.surface(name, width, height)` → 3 argumentos
- `visual.region(name, x, y, w, h)` → 5 argumentos (forma simples)
- `visual.region(name, config)` → 2 argumentos (forma com map, futuro)
- `visual.set(target, key, value)` → 3 argumentos

## Exemplos

### Exemplo Básico

```matter
# examples/visual_basic.matter
visual.surface("main", 1080, 1920)
visual.region("checkout", 100, 200, 300, 80)
visual.pulse("checkout")
visual.run("pizzaria")
```

### Exemplo com Eventos

```matter
# examples/visual_event.matter
on boot {
    visual.run("pizzaria")
    visual.surface("main", 1080, 1920)
    visual.region("button", 100, 100, 200, 50)
}

on tap {
    visual.pulse("button")
}
```

### Exemplo Avançado

```matter
# examples/visual_advanced.matter
on boot {
    visual.surface("main", 1080, 1920)
    
    # Criar região de checkout
    visual.region("checkout", 100, 200, 300, 80)
    
    # Configurar propriedades
    visual.set("checkout", "energy", 100)
    visual.set("checkout", "material", "glass")
    
    # Criar região de menu
    visual.region("menu", 50, 50, 400, 60)
    visual.set("menu", "behavior", "slide")
}

on tap {
    visual.pulse("checkout")
    visual.set("checkout", "energy", 80)
}
```

## JSON Bridge

O comando `run-json` retorna comandos visuais estruturados:

```json
{
  "visual_commands": [
    {
      "op": "surface",
      "name": "main",
      "width": 1080,
      "height": 1920
    },
    {
      "op": "region",
      "name": "checkout",
      "x": 100,
      "y": 200,
      "w": 300,
      "h": 80
    },
    {
      "op": "pulse",
      "target": "checkout"
    },
    {
      "op": "run",
      "app": "pizzaria"
    }
  ]
}
```

## Testes

### Testes Unitários

```bash
cargo test --package matter-visual
```

Testes incluídos:
- ✅ `test_trace_visual_run`
- ✅ `test_trace_visual_surface`
- ✅ `test_trace_visual_region_simple`
- ✅ `test_trace_visual_pulse`
- ✅ `test_trace_visual_set`
- ✅ `test_visual_region_with_map`

### Testes de Integração

```bash
cargo test
```

Todos os 22 testes de integração passam, incluindo:
- Equivalência bytecode (compile → run-bytecode)
- Exemplos funcionais
- Validação semântica

### Testar Exemplos

```bash
matter run examples/visual_basic.matter
matter run examples/visual_event.matter
matter run examples/visual_advanced.matter
```

## Roadmap

### v0.2 - Contrato Visual ✅
- [x] Criar crate `matter-visual`
- [x] Definir trait `VisualRuntime`
- [x] Implementar `TraceVisualBackend`
- [x] API `visual.*` completa
- [x] Exemplos funcionais
- [x] Testes passando

### v0.3 - Integração PVM
- [ ] Implementar `PvmVisualBackend`
- [ ] Conectar com PVM real
- [ ] Carregar PVMBC
- [ ] SmartPixels e matéria visual
- [ ] Eventos visuais bidirecionais

### v0.4 - PXL Compiler
- [ ] Compilador PXL → PVMBC
- [ ] Validação de PXL
- [ ] Otimizações visuais
- [ ] Debug visual

## Princípios de Design

1. **Desacoplamento**: Matter não depende do PVM
2. **Contrato primeiro**: API definida antes da implementação
3. **Testabilidade**: Mock permite testes sem PVM
4. **Evolução independente**: Matter e PVM podem crescer separadamente
5. **Integração futura**: Placeholder para PVM real já existe

## Conclusão

O Visual Backend integra PVM/PXL ao Matter Core mantendo a arquitetura limpa e desacoplada. Matter continua sendo uma linguagem geral, e o visual é apenas um target/backend plugável.

**Próximo passo**: Implementar `PvmVisualBackend` quando o PVM estiver pronto.

