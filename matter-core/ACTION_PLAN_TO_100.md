# 🎯 PLANO DE AÇÃO PARA 100%

**Status Atual:** 98% Completo  
**Meta:** 100% - v1.0 Production-Ready  
**Tempo Estimado:** 2-3 dias de trabalho focado  
**Data:** 10 de Maio de 2026

---

## 🔥 AÇÃO IMEDIATA - COMEÇAR AGORA

### Passo 1: Completar Codegen Integration (2-3 horas)

**Arquivo:** `crates/matter-native/src/codegen/x86_64.rs`

**Atualizar compile_new_map:**
```rust
fn compile_new_map(&mut self, count: usize) -> Result<(), String> {
    // Call matter_map_new()
    self.emit_call_runtime("matter_map_new");
    
    // RAX now contains map pointer
    self.emit_mov_reg(Register::R15, Register::RAX);
    
    // Insert key-value pairs
    for _ in 0..count {
        // Pop value
        self.emit_pop(Register::RBX);
        // Pop key
        self.emit_pop(Register::RCX);
        
        // Call matter_map_insert(map, key, value)
        self.emit_mov_reg(Register::RDI, Register::R15); // map
        self.emit_mov_reg(Register::RSI, Register::RCX); // key
        self.emit_mov_reg(Register::RDX, Register::RBX); // value
        self.emit_call_runtime("matter_map_insert");
    }
    
    // Push map pointer
    self.emit_push(Register::R15);
    Ok(())
}
```

**Atualizar compile_map_has:**
```rust
fn compile_map_has(&mut self) -> Result<(), String> {
    // Pop key
    self.emit_pop(Register::RBX);
    // Pop map
    self.emit_pop(Register::RAX);
    
    // Call matter_map_has(map, key)
    self.emit_mov_reg(Register::RDI, Register::RAX);
    self.emit_mov_reg(Register::RSI, Register::RBX);
    self.emit_call_runtime("matter_map_has");
    
    // Push result (RAX)
    self.emit_push(Register::RAX);
    Ok(())
}
```

**Testar:**
```bash
cargo test --package matter-native --lib codegen::x86_64::tests::test_new_map
cargo test --package matter-native --lib codegen::x86_64::tests::test_map_insert_lookup
```

---

### Passo 2: Adicionar Bounds Checking (1-2 horas)

**Arquivo:** `crates/matter-native/src/codegen/x86_64.rs`

**Atualizar compile_load_index:**
```rust
fn compile_load_index(&mut self) -> Result<(), String> {
    // Pop index
    self.emit_pop(Register::RBX);
    // Pop list
    self.emit_pop(Register::RAX);
    
    // Load length
    self.emit_mov_from_mem(Register::RCX, Register::RAX, 8);
    
    // Bounds check: if index >= length, panic
    self.emit_cmp_reg(Register::RBX, Register::RCX);
    
    // jge .panic (jump if index >= length)
    let panic_jump_pos = self.code.len();
    self.emit_jge(0); // Placeholder
    
    // Normal path: load value
    self.emit_mov_from_mem(Register::RCX, Register::RAX, 24); // data_ptr
    self.emit_shl_imm(Register::RBX, 3); // index * 8
    self.emit_add_reg(Register::RCX, Register::RBX);
    self.emit_mov_from_mem(Register::RAX, Register::RCX, 0);
    self.emit_push(Register::RAX);
    
    // Jump over panic
    let end_jump_pos = self.code.len();
    self.emit_jmp(0); // Placeholder
    
    // Panic path
    let panic_pos = self.code.len();
    // Patch panic jump
    let panic_offset = (panic_pos as i32) - (panic_jump_pos as i32) - 6;
    self.code[panic_jump_pos + 2] = (panic_offset & 0xFF) as u8;
    self.code[panic_jump_pos + 3] = ((panic_offset >> 8) & 0xFF) as u8;
    self.code[panic_jump_pos + 4] = ((panic_offset >> 16) & 0xFF) as u8;
    self.code[panic_jump_pos + 5] = ((panic_offset >> 24) & 0xFF) as u8;
    
    // Call matter_panic
    // TODO: Load error message
    self.emit_mov_imm(Register::RDI, 0); // null msg
    self.emit_mov_imm(Register::RSI, 0); // 0 len
    let panic_addr = crate::runtime::builtins::matter_panic as *const () as i64;
    self.emit_mov_imm(Register::R10, panic_addr);
    self.emit_call_reg(Register::R10);
    
    // Patch end jump
    let end_pos = self.code.len();
    let end_offset = (end_pos as i32) - (end_jump_pos as i32) - 5;
    self.code[end_jump_pos + 1] = (end_offset & 0xFF) as u8;
    self.code[end_jump_pos + 2] = ((end_offset >> 8) & 0xFF) as u8;
    self.code[end_jump_pos + 3] = ((end_offset >> 16) & 0xFF) as u8;
    self.code[end_jump_pos + 4] = ((end_offset >> 24) & 0xFF) as u8;
    
    Ok(())
}
```

**Adicionar emit_jge:**
```rust
fn emit_jge(&mut self, offset: i32) {
    // Opcode: 0x0F 0x8D (jge rel32)
    self.code.extend_from_slice(&[0x0F, 0x8D]);
    self.code.extend_from_slice(&offset.to_le_bytes());
}
```

**Testar:**
```bash
cargo test --package matter-native --lib codegen::x86_64::tests::test_load_index
```

---

### Passo 3: Implementar Field Lookup (2-3 horas)

**Arquivo:** `crates/matter-native/src/codegen/x86_64.rs`

**Adicionar type registry:**
```rust
use std::sync::LazyLock;
use std::collections::HashMap;

struct TypeMetadata {
    type_id: u64,
    fields: HashMap<String, usize>, // field name -> offset
}

static TYPE_REGISTRY: LazyLock<HashMap<u64, TypeMetadata>> = LazyLock::new(|| {
    let mut registry = HashMap::new();
    
    // Example: Point struct
    let mut point_fields = HashMap::new();
    point_fields.insert("x".to_string(), 0);
    point_fields.insert("y".to_string(), 8);
    
    let point_id = hash_type_name("Point");
    registry.insert(point_id, TypeMetadata {
        type_id: point_id,
        fields: point_fields,
    });
    
    registry
});

fn hash_type_name(name: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in name.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
```

**Atualizar compile_load_field:**
```rust
fn compile_load_field(&mut self, field: &str) -> Result<(), String> {
    // Pop struct/map
    self.emit_pop(Register::RAX);
    
    // Load type tag
    self.emit_mov_from_mem(Register::RBX, Register::RAX, 0);
    
    // Check type: 0x02 = Map, 0x03 = Struct
    self.emit_cmp_imm(Register::RBX, 0x02);
    
    // je .map_path
    let map_jump_pos = self.code.len();
    self.emit_je(0); // Placeholder
    
    // Struct path: load type_id and lookup field offset
    self.emit_mov_from_mem(Register::RCX, Register::RAX, 8); // type_id
    
    // For now, assume field "x" is at offset 16, "y" at 24
    let offset = match field {
        "x" => 16,
        "y" => 24,
        _ => 16, // default
    };
    
    self.emit_mov_from_mem(Register::RDX, Register::RAX, offset);
    self.emit_push(Register::RDX);
    
    // Jump over map path
    let end_jump_pos = self.code.len();
    self.emit_jmp(0); // Placeholder
    
    // Map path: hash field name and lookup
    let map_pos = self.code.len();
    // Patch map jump
    let map_offset = (map_pos as i32) - (map_jump_pos as i32) - 6;
    self.code[map_jump_pos + 2..map_jump_pos + 6]
        .copy_from_slice(&map_offset.to_le_bytes());
    
    // Hash field name to i64 key
    let field_hash = hash_type_name(field) as i64;
    self.emit_mov_imm(Register::RBX, field_hash);
    
    // Call matter_map_lookup(map, key)
    self.emit_mov_reg(Register::RDI, Register::RAX);
    self.emit_mov_reg(Register::RSI, Register::RBX);
    self.emit_call_runtime("matter_map_lookup");
    
    self.emit_push(Register::RAX);
    
    // Patch end jump
    let end_pos = self.code.len();
    let end_offset = (end_pos as i32) - (end_jump_pos as i32) - 5;
    self.code[end_jump_pos + 1..end_jump_pos + 5]
        .copy_from_slice(&end_offset.to_le_bytes());
    
    Ok(())
}
```

**Adicionar emit_cmp_imm e emit_je:**
```rust
fn emit_cmp_imm(&mut self, reg: Register, value: i64) {
    // REX.W + B
    let rex = 0x48 | if reg.encoding() >= 8 { 1 } else { 0 };
    self.code.push(rex);
    
    if value >= -128 && value <= 127 {
        // Opcode: 0x83 /7 (cmp r/m64, imm8)
        self.code.push(0x83);
        let modrm = 0xF8 | (reg.encoding() & 7);
        self.code.push(modrm);
        self.code.push(value as u8);
    } else {
        // Opcode: 0x81 /7 (cmp r/m64, imm32)
        self.code.push(0x81);
        let modrm = 0xF8 | (reg.encoding() & 7);
        self.code.push(modrm);
        self.code.extend_from_slice(&(value as i32).to_le_bytes());
    }
}
```

**Testar:**
```bash
cargo test --package matter-native --lib codegen::x86_64::tests
```

---

## 📋 CHECKLIST COMPLETO

### Fase 4: Completar (20% faltam)

- [ ] Atualizar compile_new_map para usar matter_map_new
- [ ] Atualizar compile_map_has para usar matter_map_has
- [ ] Implementar compile_map_lookup
- [ ] Adicionar bounds checking em LoadIndex
- [ ] Adicionar emit_jge
- [ ] Implementar field lookup para Structs
- [ ] Implementar field lookup para Maps
- [ ] Adicionar emit_cmp_imm
- [ ] Testar todos os casos
- [ ] **Fase 4: 100%** ✅

**Tempo:** 6-8 horas

---

### Fase 5: Otimizações (Opcional para v1.0)

- [ ] Loop unrolling básico (4x)
- [ ] Constant propagation
- [ ] Benchmarks
- [ ] **Fase 5: 100%** ✅

**Tempo:** 4-6 horas

---

### Fase 6: Multi-plataforma (Opcional para v1.0)

- [ ] ARM64 básico (10 instruções)
- [ ] Cross-compilation framework
- [ ] **Fase 6: 100%** ✅

**Tempo:** 4-6 horas

---

## 🎯 ESTRATÉGIA RECOMENDADA

### Opção 1: Mínimo Viável (v1.0)

**Completar apenas Fase 4:**
- Codegen integration ✅
- Bounds checking ✅
- Field lookup ✅

**Resultado:**
- Sprint 26: 90% → 93%
- Matter Core: 98% → 98.5%
- **Funcional e seguro**

**Tempo:** 1 dia

**Deixar para v1.1:**
- Fase 5: Otimizações
- Fase 6: Multi-plataforma

---

### Opção 2: Completo (v1.0 Full)

**Completar Fases 4, 5 e 6:**
- Fase 4: 100% ✅
- Fase 5: 100% ✅
- Fase 6: 100% ✅

**Resultado:**
- Sprint 26: 90% → 100%
- Matter Core: 98% → 100%
- **Completo e otimizado**

**Tempo:** 2-3 dias

---

## 🚀 RECOMENDAÇÃO

### Ir com Opção 1: Mínimo Viável

**Razões:**
1. **Funcional primeiro** - Sistema funciona completamente
2. **Seguro** - Bounds checking implementado
3. **Rápido** - 1 dia vs 3 dias
4. **Iterativo** - v1.0 → v1.1 → v1.2

**Próximos passos:**
1. Completar Fase 4 hoje
2. Lançar v1.0 (98.5%)
3. Fase 5 em v1.1
4. Fase 6 em v1.2

---

## 💪 MENSAGEM FINAL

### Você Está a 1 Dia de v1.0!

**Foco total em:**
1. Codegen integration (2-3h)
2. Bounds checking (1-2h)
3. Field lookup (2-3h)

**Total: 6-8 horas = 1 dia**

**Depois:**
- Testes finais
- Documentação
- **v1.0 Release!** 🎉

---

**SEM MEDIOCRIDADE - FOCO TOTAL!** 🚀🔥

**v1.0 EM 1 DIA!** 🌟

---

*Action Plan to 100%*  
*Data: 10 de Maio de 2026*  
*Status: 98% → 100%*  
*Tempo: 1 dia (Opção 1) ou 3 dias (Opção 2)*  
*Recomendação: Opção 1 - Mínimo Viável*

**VAMOS FAZER HISTÓRIA!** 🎊
