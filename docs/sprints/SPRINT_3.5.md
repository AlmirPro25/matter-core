# Sprint 3.5 - MBC1 Persistence

## Objetivo Estratégico
**Transformar bytecode de artefato em memória para artefato em disco.**

Este é o marco que separa "protótipo funcional" de "linguagem real".

## O que é MBC1?
**Matter Bytecode v1** - formato binário serializado do bytecode Matter.

### Estrutura do Formato
```
MBC1 File Format
================
[Magic: 4 bytes]     "MBC1"
[Version: 2 bytes]   0x00 0x01
[Sections]
  - Constants Pool
  - Functions Table
  - Event Handlers Table
  - Main Instructions
  - Metadata
```

## Implementação

### 1. Serialização (Bytecode → Arquivo)

#### 1.1 Trait Serialize
```rust
pub trait Serialize {
    fn serialize(&self, writer: &mut impl Write) -> std::io::Result<()>;
}
```

#### 1.2 Implementar para Bytecode
```rust
impl Serialize for Bytecode {
    fn serialize(&self, writer: &mut impl Write) -> std::io::Result<()> {
        // Magic number
        writer.write_all(&self.magic)?;
        
        // Version
        writer.write_all(&[0x00, 0x01])?;
        
        // Constants section
        self.serialize_constants(writer)?;
        
        // Functions section
        self.serialize_functions(writer)?;
        
        // Event handlers section
        self.serialize_event_handlers(writer)?;
        
        // Main instructions
        self.serialize_instructions(&self.main_instructions, writer)?;
        
        Ok(())
    }
}
```

#### 1.3 Serializar Constantes
```rust
fn serialize_constants(&self, writer: &mut impl Write) -> std::io::Result<()> {
    // Count
    writer.write_all(&(self.constants.len() as u32).to_le_bytes())?;
    
    // Each constant
    for constant in &self.constants {
        match constant {
            Constant::Int(n) => {
                writer.write_all(&[0x01])?; // type tag
                writer.write_all(&n.to_le_bytes())?;
            }
            Constant::Bool(b) => {
                writer.write_all(&[0x02])?;
                writer.write_all(&[*b as u8])?;
            }
            Constant::String(s) => {
                writer.write_all(&[0x03])?;
                let bytes = s.as_bytes();
                writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
                writer.write_all(bytes)?;
            }
            Constant::Unit => {
                writer.write_all(&[0x04])?;
            }
        }
    }
    
    Ok(())
}
```

#### 1.4 Serializar Instruções
```rust
fn serialize_instructions(&self, instructions: &[Instruction], writer: &mut impl Write) -> std::io::Result<()> {
    writer.write_all(&(instructions.len() as u32).to_le_bytes())?;
    
    for instr in instructions {
        match instr {
            Instruction::LoadConst(id) => {
                writer.write_all(&[0x01])?;
                writer.write_all(&(*id as u32).to_le_bytes())?;
            }
            Instruction::Add => writer.write_all(&[0x10])?,
            Instruction::Sub => writer.write_all(&[0x11])?,
            Instruction::Mul => writer.write_all(&[0x12])?,
            Instruction::Div => writer.write_all(&[0x13])?,
            Instruction::Print => writer.write_all(&[0x20])?,
            Instruction::Halt => writer.write_all(&[0xFF])?,
            // ... outros opcodes
        }
    }
    
    Ok(())
}
```

### 2. Desserialização (Arquivo → Bytecode)

#### 2.1 Trait Deserialize
```rust
pub trait Deserialize: Sized {
    fn deserialize(reader: &mut impl Read) -> std::io::Result<Self>;
}
```

#### 2.2 Implementar para Bytecode
```rust
impl Deserialize for Bytecode {
    fn deserialize(reader: &mut impl Read) -> std::io::Result<Self> {
        // Verificar magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"MBC1" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid MBC1 magic number"
            ));
        }
        
        // Verificar versão
        let mut version = [0u8; 2];
        reader.read_exact(&mut version)?;
        if version != [0x00, 0x01] {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported MBC1 version"
            ));
        }
        
        let constants = Self::deserialize_constants(reader)?;
        let functions = Self::deserialize_functions(reader)?;
        let event_handlers = Self::deserialize_event_handlers(reader)?;
        let main_instructions = Self::deserialize_instructions(reader)?;
        
        Ok(Bytecode {
            magic,
            constants,
            functions,
            event_handlers,
            main_instructions,
        })
    }
}
```

### 3. CLI Commands

#### 3.1 Compile to Bytecode
```bash
matter compile app.matter -o app.mbc
```

Implementação:
```rust
fn compile_command(input: &str, output: &str) -> Result<()> {
    let source = fs::read_to_string(input)?;
    let bytecode = compile_to_bytecode(&source)?;
    
    let mut file = File::create(output)?;
    bytecode.serialize(&mut file)?;
    
    println!("✓ Compiled {} → {}", input, output);
    Ok(())
}
```

#### 3.2 Run Bytecode
```bash
matter run-bytecode app.mbc
```

Implementação:
```rust
fn run_bytecode_command(input: &str) -> Result<()> {
    let mut file = File::open(input)?;
    let bytecode = Bytecode::deserialize(&mut file)?;
    
    let mut vm = VM::new(bytecode);
    vm.run()?;
    
    Ok(())
}
```

#### 3.3 Inspect Bytecode
```bash
matter inspect app.mbc
```

Output:
```
MBC1 Bytecode File
==================
Version: 1.0
Constants: 5
Functions: 2
Event Handlers: 1
Main Instructions: 42

Constants Pool:
  0: Int(10)
  1: String("Hello")
  2: Bool(true)
  ...

Functions:
  soma(2 params, 8 instructions)
  fatorial(1 param, 15 instructions)

Event Handlers:
  on boot (12 instructions)

Main:
  0000: LoadConst 0
  0001: StoreGlobal "x"
  0002: LoadGlobal "x"
  ...
```

### 4. Testes

#### Teste 1: Round-trip
```rust
#[test]
fn test_serialize_deserialize() {
    let original = create_test_bytecode();
    
    let mut buffer = Vec::new();
    original.serialize(&mut buffer).unwrap();
    
    let mut cursor = Cursor::new(buffer);
    let deserialized = Bytecode::deserialize(&mut cursor).unwrap();
    
    assert_eq!(original, deserialized);
}
```

#### Teste 2: File I/O
```rust
#[test]
fn test_file_roundtrip() {
    let bytecode = create_test_bytecode();
    
    bytecode.save_to_file("test.mbc").unwrap();
    let loaded = Bytecode::load_from_file("test.mbc").unwrap();
    
    assert_eq!(bytecode, loaded);
    
    fs::remove_file("test.mbc").unwrap();
}
```

#### Teste 3: Executar de arquivo
```rust
#[test]
fn test_run_from_file() {
    let source = r#"
        let x = 10
        print x
    "#;
    
    compile_and_save(source, "test.mbc").unwrap();
    
    let output = run_bytecode_file("test.mbc").unwrap();
    assert_eq!(output, "10\n");
    
    fs::remove_file("test.mbc").unwrap();
}
```

## Estrutura de Arquivos

```
crates/matter-bytecode/src/
├── lib.rs              # Bytecode structure
├── serialize.rs        # Serialization logic
├── deserialize.rs      # Deserialization logic
└── opcodes.rs          # Opcode definitions
```

## Benefícios

### 1. Distribuição
```bash
# Compilar uma vez
matter compile app.matter -o app.mbc

# Distribuir apenas o .mbc
# Executar em qualquer máquina com Matter VM
matter run-bytecode app.mbc
```

### 2. Performance
- Pular parsing/compilation
- Carregar bytecode direto na VM
- Startup mais rápido

### 3. Proteção de Código
- Source code não precisa ser distribuído
- Bytecode é menos legível
- Possibilidade de ofuscação futura

### 4. Caching
```bash
# Compilar automaticamente se .matter mais novo que .mbc
matter run app.matter  # compila se necessário
```

## Critérios de Sucesso

✅ **Serialização funcional**
- Bytecode → arquivo .mbc sem perda de informação

✅ **Desserialização funcional**
- Arquivo .mbc → Bytecode válido

✅ **Execução de bytecode**
- `matter run-bytecode app.mbc` funciona

✅ **Round-trip perfeito**
- Bytecode → arquivo → Bytecode = idêntico

✅ **CLI completo**
- `compile`, `run-bytecode`, `inspect` funcionando

## Próximos Passos (Sprint 4)

Após MBC1 em disco:
1. **Data Model** - List, Map
2. **Struct** - Tipos compostos customizados
3. **Error System** - MatterError estruturado
4. **REPL** - Interactive shell

---

**Status:** 🔴 Planejado
**Prioridade:** 🔥 CRÍTICA
**Estimativa:** 2-3 dias

Este sprint é o **marco de maturidade** do projeto.
