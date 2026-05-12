# PVM Integration Guide - Phase 2

Guia completo para implementar a integração real do PVM com Matter Core.

---

## 📋 Pré-requisitos

### Fase 1 Completa ✅

- ✅ Crate `matter-visual` criado
- ✅ Trait `VisualRuntime` definida
- ✅ `TraceVisualBackend` funcionando
- ✅ API visual estável
- ✅ Testes passando (100%)
- ✅ Documentação completa

### PVM Pronto

- [ ] PVM runtime compilável
- [ ] PVMBC loader funcional
- [ ] SmartPixels implementados
- [ ] Surface/Region API disponível
- [ ] Event system do PVM

---

## 🎯 Objetivo da Fase 2

**Substituir `TraceVisualBackend` por `PvmVisualBackend` real**, conectando Matter Core ao PVM runtime.

---

## 🏗️ Arquitetura da Integração

### Camadas

```
Matter Core
    ↓
matter-visual (trait VisualRuntime)
    ↓
PvmVisualBackend (implementação real)
    ↓
PVM Runtime (SmartPixels, PVMBC, etc)
```

### Princípios

1. **Matter não depende do PVM diretamente**
   - Dependência via trait `VisualRuntime`
   - PVM é uma implementação plugável

2. **Contrato já definido**
   - API visual estável
   - Não quebrar código existente

3. **Coexistência**
   - `TraceVisualBackend` continua disponível (debug)
   - `PvmVisualBackend` é opcional (feature flag)

---

## 📝 Passo a Passo

### Passo 1: Adicionar PVM como Dependência

#### 1.1. Atualizar `crates/matter-visual/Cargo.toml`

```toml
[package]
name = "matter-visual"
version.workspace = true
edition.workspace = true

[dependencies]
matter-backend = { path = "../matter-backend" }

# PVM integration (optional)
pvm-runtime = { path = "../../pvm/pvm-runtime", optional = true }
pvm-bytecode = { path = "../../pvm/pvm-bytecode", optional = true }

[features]
default = []
pvm = ["pvm-runtime", "pvm-bytecode"]
```

#### 1.2. Justificativa

- Feature flag `pvm` permite compilar sem PVM
- Mantém `TraceVisualBackend` como padrão
- PVM é opt-in, não obrigatório

---

### Passo 2: Implementar `PvmVisualBackend`

#### 2.1. Estrutura Básica

```rust
// crates/matter-visual/src/lib.rs

#[cfg(feature = "pvm")]
use pvm_runtime::{PvmRuntime, Surface, Region};
#[cfg(feature = "pvm")]
use pvm_bytecode::PvmbcLoader;

#[cfg(feature = "pvm")]
pub struct PvmVisualBackend {
    runtime: PvmRuntime,
    loader: PvmbcLoader,
    surfaces: HashMap<String, Surface>,
    regions: HashMap<String, Region>,
}

#[cfg(feature = "pvm")]
impl PvmVisualBackend {
    pub fn new() -> Result<Self, VisualError> {
        let runtime = PvmRuntime::new()
            .map_err(|e| VisualError::RuntimeError(e.to_string()))?;
        let loader = PvmbcLoader::new();
        
        Ok(Self {
            runtime,
            loader,
            surfaces: HashMap::new(),
            regions: HashMap::new(),
        })
    }
}
```

#### 2.2. Implementar `VisualRuntime`

```rust
#[cfg(feature = "pvm")]
impl VisualRuntime for PvmVisualBackend {
    fn run_app(&mut self, name: &str) -> Result<(), VisualError> {
        self.runtime.execute_app(name)
            .map_err(|e| VisualError::RuntimeError(e.to_string()))
    }

    fn load_pvmbc(&mut self, path: &str) -> Result<(), VisualError> {
        let bytecode = self.loader.load_from_file(path)
            .map_err(|e| VisualError::RuntimeError(e.to_string()))?;
        
        self.runtime.load_bytecode(bytecode)
            .map_err(|e| VisualError::RuntimeError(e.to_string()))
    }

    fn create_surface(&mut self, name: &str, width: i64, height: i64) -> Result<(), VisualError> {
        let surface = self.runtime.create_surface(width as u32, height as u32)
            .map_err(|e| VisualError::RuntimeError(e.to_string()))?;
        
        self.surfaces.insert(name.to_string(), surface);
        Ok(())
    }

    fn create_region(&mut self, region: VisualRegionSpec) -> Result<(), VisualError> {
        let pvm_region = self.runtime.create_region(
            region.x as i32,
            region.y as i32,
            region.w as u32,
            region.h as u32,
        ).map_err(|e| VisualError::RuntimeError(e.to_string()))?;
        
        // Configurar propriedades opcionais
        if let Some(semantic) = region.semantic {
            pvm_region.set_semantic(&semantic);
        }
        if let Some(behavior) = region.behavior {
            pvm_region.set_behavior(&behavior);
        }
        if let Some(material) = region.material {
            pvm_region.set_material(&material);
        }
        if let Some(energy) = region.energy {
            pvm_region.set_energy(energy);
        }
        
        self.regions.insert(region.name.clone(), pvm_region);
        Ok(())
    }

    fn pulse(&mut self, target: &str) -> Result<(), VisualError> {
        let region = self.regions.get_mut(target)
            .ok_or_else(|| VisualError::InvalidArgument(format!("Region not found: {}", target)))?;
        
        region.pulse()
            .map_err(|e| VisualError::RuntimeError(e.to_string()))
    }

    fn set_property(&mut self, target: &str, key: &str, value: Value) -> Result<(), VisualError> {
        let region = self.regions.get_mut(target)
            .ok_or_else(|| VisualError::InvalidArgument(format!("Region not found: {}", target)))?;
        
        match key {
            "energy" => {
                let energy = value.as_int()
                    .map_err(|_| VisualError::InvalidArgument("energy must be int".to_string()))?;
                region.set_energy(energy as f64);
            }
            "material" => {
                let material = value.as_string()
                    .map_err(|_| VisualError::InvalidArgument("material must be string".to_string()))?;
                region.set_material(&material);
            }
            "behavior" => {
                let behavior = value.as_string()
                    .map_err(|_| VisualError::InvalidArgument("behavior must be string".to_string()))?;
                region.set_behavior(&behavior);
            }
            _ => return Err(VisualError::InvalidArgument(format!("Unknown property: {}", key))),
        }
        
        Ok(())
    }
}
```

#### 2.3. Implementar `Backend` trait

```rust
#[cfg(feature = "pvm")]
impl Backend for PvmVisualBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        // Reutilizar a lógica do TraceVisualBackend
        // mas chamar self (PvmVisualBackend) em vez de TraceVisualBackend
        match method {
            "run" => {
                if args.len() != 1 {
                    return Err(format!("visual.run expects 1 argument, got {}", args.len()));
                }
                let name = args[0].as_string()
                    .map_err(|_| "visual.run expects string argument".to_string())?;
                self.run_app(&name)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            // ... resto dos métodos igual ao TraceVisualBackend
            _ => Err(format!("Unknown visual method: {}", method)),
        }
    }
}
```

---

### Passo 3: Atualizar Runtime

#### 3.1. Adicionar Feature Flag

```toml
# crates/matter-runtime/Cargo.toml

[features]
default = []
pvm = ["matter-visual/pvm"]
```

#### 3.2. Usar PvmVisualBackend Condicionalmente

```rust
// crates/matter-runtime/src/lib.rs

#[cfg(feature = "pvm")]
use matter_visual::PvmVisualBackend;
#[cfg(not(feature = "pvm"))]
use matter_visual::TraceVisualBackend;

pub fn new(bytecode: Bytecode) -> Self {
    let mut vm = Vm::new(bytecode);
    
    // Register backends
    vm.register_backend("agent".to_string(), Box::new(AgentBackend::new()));
    
    // Visual backend: PVM se disponível, senão Trace
    #[cfg(feature = "pvm")]
    {
        match PvmVisualBackend::new() {
            Ok(backend) => vm.register_backend("visual".to_string(), Box::new(backend)),
            Err(e) => {
                eprintln!("Warning: Failed to initialize PVM backend: {}", e);
                eprintln!("Falling back to TraceVisualBackend");
                vm.register_backend("visual".to_string(), Box::new(TraceVisualBackend::new()));
            }
        }
    }
    #[cfg(not(feature = "pvm"))]
    {
        vm.register_backend("visual".to_string(), Box::new(TraceVisualBackend::new()));
    }
    
    vm.register_backend("store".to_string(), Box::new(StoreBackend::new()));
    vm.register_backend("net".to_string(), Box::new(NetBackend::new()));
    
    Self { vm }
}
```

---

### Passo 4: Compilar e Testar

#### 4.1. Compilar sem PVM (padrão)

```bash
cargo build
```

Usa `TraceVisualBackend` (comportamento atual).

#### 4.2. Compilar com PVM

```bash
cargo build --features pvm
```

Usa `PvmVisualBackend` (integração real).

#### 4.3. Testar

```bash
# Sem PVM (trace)
cargo test

# Com PVM (real)
cargo test --features pvm
```

---

### Passo 5: Eventos Bidirecionais

#### 5.1. PVM → Matter

Quando o PVM detecta eventos (tap, swipe, etc), deve notificar Matter:

```rust
impl PvmVisualBackend {
    pub fn poll_events(&mut self) -> Vec<String> {
        self.runtime.poll_events()
            .into_iter()
            .map(|e| e.name)
            .collect()
    }
}
```

#### 5.2. Integrar no Runtime

```rust
// matter-runtime
pub fn update(&mut self) -> Result<(), String> {
    #[cfg(feature = "pvm")]
    {
        if let Some(visual) = self.get_visual_backend() {
            let events = visual.poll_events();
            for event in events {
                self.emit_event(&event)?;
            }
        }
    }
    Ok(())
}
```

---

## 🧪 Testes

### Testes de Integração

```rust
#[cfg(feature = "pvm")]
#[test]
fn test_pvm_integration() {
    let source = r#"
        visual.surface("main", 1080, 1920)
        visual.region("button", 100, 100, 200, 50)
        visual.pulse("button")
    "#;

    let mut parser = Parser::from_source(source);
    let program = parser.parse().unwrap();
    let builder = BytecodeBuilder::new();
    let bytecode = builder.build_checked(&program).unwrap();
    
    let mut runtime = Runtime::new(bytecode);
    assert!(runtime.run().is_ok());
    
    // Verificar que PVM realmente renderizou
    // (depende da API do PVM)
}
```

---

## 📊 Validação

### Checklist de Integração

- [ ] PVM compila como dependência
- [ ] `PvmVisualBackend` implementa `VisualRuntime`
- [ ] `PvmVisualBackend` implementa `Backend`
- [ ] Feature flag `pvm` funciona
- [ ] Fallback para `TraceVisualBackend` funciona
- [ ] Todos os comandos visuais funcionam com PVM
- [ ] Eventos bidirecionais funcionam
- [ ] Testes passam com e sem PVM
- [ ] Documentação atualizada

---

## 🚨 Possíveis Problemas

### 1. Incompatibilidade de API

**Problema**: API do PVM não corresponde exatamente ao contrato `VisualRuntime`.

**Solução**: Criar camada de adaptação dentro de `PvmVisualBackend`.

### 2. Gerenciamento de Memória

**Problema**: PVM usa ownership diferente de Matter.

**Solução**: Usar `Rc<RefCell<>>` ou similar para compartilhar referências.

### 3. Threading

**Problema**: PVM pode exigir thread separada para renderização.

**Solução**: Usar channels para comunicação entre threads.

### 4. Performance

**Problema**: Overhead de conversão entre tipos Matter e PVM.

**Solução**: Otimizar conversões, usar zero-copy quando possível.

---

## 📚 Documentação a Atualizar

Após integração real:

1. **`docs/VISUAL_BACKEND.md`**
   - Adicionar seção "PVM Integration"
   - Documentar feature flag
   - Exemplos com PVM real

2. **`README.md`**
   - Atualizar seção de instalação
   - Mencionar feature `pvm`

3. **`QUICKSTART_VISUAL.md`**
   - Adicionar instruções para PVM
   - Diferenças entre Trace e PVM

4. **Novo: `PVM_INTEGRATION.md`**
   - Detalhes técnicos da integração
   - Troubleshooting
   - Performance tuning

---

## 🎯 Critérios de Sucesso

### Funcional

- [ ] Comandos visuais renderizam no PVM
- [ ] PVMBC carrega e executa
- [ ] SmartPixels aparecem na tela
- [ ] Eventos PVM → Matter funcionam
- [ ] Performance aceitável (>30 FPS)

### Técnico

- [ ] Código limpo e bem documentado
- [ ] Testes passando (100%)
- [ ] Zero memory leaks
- [ ] Fallback funciona
- [ ] Feature flag estável

### Qualidade

- [ ] Documentação completa
- [ ] Exemplos funcionais
- [ ] Error handling robusto
- [ ] Logging adequado

---

## 🚀 Próximos Passos Após Integração

### Fase 3: Otimizações

1. **Batch de Comandos**
   - Agrupar múltiplos comandos visuais
   - Reduzir overhead de chamadas

2. **Cache de Regiões**
   - Evitar recriação desnecessária
   - Reutilizar objetos PVM

3. **Async/Await**
   - Operações visuais assíncronas
   - Não bloquear runtime Matter

### Fase 4: Features Avançadas

1. **Animações Complexas**
   - Timelines
   - Easing functions
   - Keyframes

2. **Shaders Customizados**
   - Material system
   - Visual effects

3. **Debug Visual**
   - Inspector de regiões
   - Performance profiler
   - Visual debugger

---

## 💡 Dicas

### 1. Comece Simples

Implemente primeiro apenas `create_surface` e `create_region`. Valide que funciona antes de adicionar mais.

### 2. Use Logging

```rust
#[cfg(feature = "pvm")]
impl PvmVisualBackend {
    fn log(&self, msg: &str) {
        if std::env::var("MATTER_VISUAL_DEBUG").is_ok() {
            eprintln!("[PVM] {}", msg);
        }
    }
}
```

### 3. Testes Incrementais

Adicione um teste para cada método implementado. Não espere implementar tudo para testar.

### 4. Documentação Inline

Documente decisões de design no código:

```rust
// Usamos Rc<RefCell<>> aqui porque o PVM precisa
// compartilhar referências entre Surface e Region
```

---

## 🎉 Conclusão

Este guia fornece um caminho claro para integrar o PVM real ao Matter Core, mantendo a arquitetura limpa e desacoplada estabelecida na Fase 1.

**A base está sólida. Agora é hora de conectar ao PVM real!**

---

**Autor**: Matter Core Team  
**Data**: Maio 2026  
**Status**: Guia para Fase 2  
**Pré-requisito**: Fase 1 completa ✅

