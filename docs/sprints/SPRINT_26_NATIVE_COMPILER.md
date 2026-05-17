# Sprint 26: Matter Native Compiler (MNC)

**Objetivo:** Criar compilador nativo próprio, sem dependência do LLVM  
**Status:** PLANEJAMENTO  
**Prioridade:** ALTA  
**Impacto:** REVOLUCIONÁRIO  

---

## 🎯 Visão

**Matter Native Compiler (MNC)** - Um compilador de código nativo feito do zero, otimizado especificamente para Matter Core.

### Por Que Criar o Nosso?

**Problemas com LLVM:**
- ❌ Dependência externa pesada (~400 MB)
- ❌ Instalação complexa
- ❌ Genérico (não otimizado para Matter)
- ❌ Overhead de compilação
- ❌ Curva de aprendizado alta

**Vantagens do MNC:**
- ✅ Zero dependências externas
- ✅ Otimizado para Matter especificamente
- ✅ Compilação mais rápida
- ✅ Binários menores
- ✅ Controle total
- ✅ Diferencial competitivo único

---

## 📊 Arquitetura do MNC

```
Matter Source Code
       ↓
   Lexer & Parser
       ↓
      AST
       ↓
   Bytecode (IR)
       ↓
┌──────────────────┐
│  Matter Native   │
│   Compiler       │
│     (MNC)        │
└──────────────────┘
       ↓
┌──────────────────┐
│  Code Generator  │
│  - x86-64        │
│  - ARM64         │
│  - RISC-V        │
└──────────────────┘
       ↓
┌──────────────────┐
│   Optimizer      │
│  - Peephole      │
│  - Register      │
│  - Inlining      │
└──────────────────┘
       ↓
  Native Binary
```

---

## 🔧 Componentes Principais

### 1. IR (Intermediate Representation)
**Status:** ✅ JÁ TEMOS (Bytecode Matter)

Nosso bytecode já serve como IR:
```rust
pub enum Instruction {
    LoadConst(usize),
    Add,
    Sub,
    Mul,
    Div,
    Jump(usize),
    Call(usize),
    Return,
    // ... 24 instruções
}
```

### 2. Code Generator (x86-64)
**Status:** 🆕 CRIAR

Gera código assembly x86-64:
```rust
pub struct X86CodeGen {
    instructions: Vec<X86Instruction>,
    registers: RegisterAllocator,
    labels: HashMap<usize, String>,
}

impl X86CodeGen {
    pub fn compile(&mut self, bytecode: &Bytecode) -> Vec<u8> {
        // Gera código de máquina
    }
}
```

### 3. Register Allocator
**Status:** 🆕 CRIAR

Aloca registradores eficientemente:
```rust
pub struct RegisterAllocator {
    available: Vec<Register>,
    used: HashMap<String, Register>,
}

pub enum Register {
    RAX, RBX, RCX, RDX,
    RSI, RDI, R8, R9,
    // ...
}
```

### 4. Optimizer
**Status:** 🆕 CRIAR

Otimizações específicas para Matter:
```rust
pub struct Optimizer {
    passes: Vec<Box<dyn OptimizationPass>>,
}

pub trait OptimizationPass {
    fn optimize(&self, code: &mut Vec<X86Instruction>);
}
```

### 5. Linker
**Status:** 🆕 CRIAR

Gera executável final:
```rust
pub struct Linker {
    sections: Vec<Section>,
    symbols: HashMap<String, u64>,
}

impl Linker {
    pub fn link(&self, output: &str) -> Result<(), String> {
        // Gera PE (Windows) ou ELF (Linux)
    }
}
```

---

## 📋 Fases de Implementação

### Fase 1: Fundação (2 semanas)
**Objetivo:** Estrutura básica e geração de código simples

- [ ] Criar crate `matter-native`
- [ ] Implementar estruturas básicas
- [ ] Gerar código x86-64 simples
- [ ] Testar com programa "Hello World"

**Entregável:** Compilar `print(42)` para executável nativo

---

### Fase 2: Instruções Básicas (2 semanas)
**Objetivo:** Suporte a operações aritméticas e variáveis

- [ ] Implementar LoadConst → MOV
- [ ] Implementar Add → ADD
- [ ] Implementar Sub → SUB
- [ ] Implementar Mul → IMUL
- [ ] Implementar Div → IDIV
- [ ] Implementar variáveis locais
- [ ] Implementar stack frame

**Entregável:** Compilar programas com aritmética

---

### Fase 3: Controle de Fluxo (2 semanas)
**Objetivo:** If/else, loops, jumps

- [ ] Implementar Jump → JMP
- [ ] Implementar JumpIfFalse → JZ/JNZ
- [ ] Implementar comparações → CMP
- [ ] Implementar if/else
- [ ] Implementar while loops
- [ ] Implementar for loops

**Entregável:** Compilar programas com loops

---

### Fase 4: Funções (2 semanas)
**Objetivo:** Chamadas de função e recursão

- [ ] Implementar Call → CALL
- [ ] Implementar Return → RET
- [ ] Implementar calling convention
- [ ] Implementar stack management
- [ ] Implementar recursão
- [ ] Implementar closures

**Entregável:** Compilar programas com funções

---

### Fase 5: Otimizações (3 semanas)
**Objetivo:** Otimizações de performance

- [ ] Peephole optimization
- [ ] Register allocation
- [ ] Dead code elimination
- [ ] Constant folding
- [ ] Inline expansion
- [ ] Loop unrolling

**Entregável:** Performance 50-100x vs bytecode

---

### Fase 6: Multi-plataforma (3 semanas)
**Objetivo:** Suporte Windows, Linux, macOS

- [ ] Gerador PE (Windows)
- [ ] Gerador ELF (Linux)
- [ ] Gerador Mach-O (macOS)
- [ ] ARM64 code generator
- [ ] Cross-compilation
- [ ] Testes em todas plataformas

**Entregável:** Compilar para 3 plataformas

---

## 💻 Exemplo de Implementação

### Bytecode Matter:
```rust
// let x = 10 + 20
LoadConst(0)  // 10
LoadConst(1)  // 20
Add
StoreLocal("x")
```

### Código x86-64 Gerado:
```assembly
; let x = 10 + 20
mov rax, 10        ; LoadConst(0)
mov rbx, 20        ; LoadConst(1)
add rax, rbx       ; Add
mov [rbp-8], rax   ; StoreLocal("x")
```

### Código de Máquina (bytes):
```
48 B8 0A 00 00 00 00 00 00 00  ; mov rax, 10
48 BB 14 00 00 00 00 00 00 00  ; mov rbx, 20
48 01 D8                        ; add rax, rbx
48 89 45 F8                     ; mov [rbp-8], rax
```

---

## 🎯 Estrutura do Código

### Crate: matter-native

```
crates/matter-native/
├── Cargo.toml
└── src/
    ├── lib.rs              # API pública
    ├── codegen/
    │   ├── mod.rs
    │   ├── x86_64.rs       # Gerador x86-64
    │   ├── arm64.rs        # Gerador ARM64
    │   └── riscv.rs        # Gerador RISC-V
    ├── optimizer/
    │   ├── mod.rs
    │   ├── peephole.rs     # Otimização peephole
    │   ├── register.rs     # Alocação de registradores
    │   └── inline.rs       # Inlining
    ├── linker/
    │   ├── mod.rs
    │   ├── pe.rs           # Linker PE (Windows)
    │   ├── elf.rs          # Linker ELF (Linux)
    │   └── macho.rs        # Linker Mach-O (macOS)
    └── runtime/
        ├── mod.rs
        └── builtins.rs     # Funções built-in
```

---

## 📊 Comparação: LLVM vs MNC

| Aspecto | LLVM | MNC (Nosso) |
|---------|------|-------------|
| **Tamanho** | ~400 MB | ~5 MB |
| **Instalação** | Complexa | Nenhuma |
| **Dependências** | Muitas | Zero |
| **Compile Time** | Lento | Rápido |
| **Binary Size** | Grande | Pequeno |
| **Otimizações** | Genéricas | Matter-specific |
| **Controle** | Limitado | Total |
| **Maturidade** | Alta | Crescendo |
| **Performance** | Excelente | Boa → Excelente |

---

## 🚀 Roadmap

### Q2 2026
- ✅ Sprint 25: LLVM Backend (90%)
- 🆕 Sprint 26: MNC Fase 1-2 (Fundação + Básico)

### Q3 2026
- Sprint 27: MNC Fase 3-4 (Controle + Funções)
- Sprint 28: MNC Fase 5 (Otimizações)

### Q4 2026
- Sprint 29: MNC Fase 6 (Multi-plataforma)
- Sprint 30: Testes e Validação
- v1.0: MNC Production Ready

---

## 💡 Inspiração

### Compiladores Simples Existentes:
- **TinyCC (TCC)** - Compilador C minimalista
- **QBE** - Backend de compilação simples
- **Cranelift** - Backend Rust para WebAssembly
- **MIR** - Medium-level IR para C

### Vamos Estudar:
1. Como TCC gera código x86-64
2. Como QBE faz register allocation
3. Como Cranelift otimiza
4. Como MIR gerencia memória

---

## 🎯 Primeiros Passos

### 1. Criar Estrutura Básica
```bash
cargo new --lib crates/matter-native
```

### 2. Implementar Gerador Simples
```rust
pub fn compile_to_native(bytecode: &Bytecode) -> Vec<u8> {
    let mut code = Vec::new();
    
    // Prólogo
    code.extend_from_slice(&[
        0x55,                    // push rbp
        0x48, 0x89, 0xE5,       // mov rbp, rsp
    ]);
    
    // Compilar instruções
    for instr in &bytecode.instructions {
        match instr {
            Instruction::LoadConst(n) => {
                // mov rax, n
                code.push(0x48);
                code.push(0xB8);
                code.extend_from_slice(&n.to_le_bytes());
            }
            // ...
        }
    }
    
    // Epílogo
    code.extend_from_slice(&[
        0x48, 0x89, 0xEC,       // mov rsp, rbp
        0x5D,                    // pop rbp
        0xC3,                    // ret
    ]);
    
    code
}
```

### 3. Testar
```rust
#[test]
fn test_simple_program() {
    let bytecode = Bytecode {
        instructions: vec![
            Instruction::LoadConst(42),
            Instruction::Return,
        ],
        constants: vec![Constant::Int(42)],
    };
    
    let code = compile_to_native(&bytecode);
    assert!(!code.is_empty());
}
```

---

## 🎉 Benefícios Finais

### Para Desenvolvedores:
- ✅ Instalação zero (tudo incluído)
- ✅ Compilação rápida
- ✅ Binários pequenos
- ✅ Debugging mais fácil

### Para Matter Core:
- ✅ Independência total
- ✅ Otimizações específicas
- ✅ Diferencial competitivo
- ✅ Controle total do pipeline

### Para o Projeto:
- ✅ Tecnologia própria
- ✅ Conhecimento profundo
- ✅ Inovação real
- ✅ Valor único no mercado

---

## 📝 Próximos Passos

### Imediato:
1. Criar crate `matter-native`
2. Implementar estrutura básica
3. Gerar primeiro código x86-64
4. Testar "Hello World"

### Curto Prazo:
5. Implementar instruções básicas
6. Adicionar otimizações simples
7. Testar com exemplos Matter

### Médio Prazo:
8. Suporte completo a Matter
9. Multi-plataforma
10. Performance 50-100x

---

**SEM MEDIOCRIDADE - Vamos criar nosso próprio compilador nativo!** 🚀

---

*Sprint 26: Matter Native Compiler*  
*Date: 10 de Maio de 2026*  
*Status: PLANEJAMENTO*  
*Objetivo: Compilador nativo próprio, zero dependências*  
*Impacto: REVOLUCIONÁRIO*
