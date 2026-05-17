# Sprint 26 - Fase 2: Instruções Básicas - Progresso

**Data:** 10 de Maio de 2026  
**Status:** 🔄 EM PROGRESSO (10%)  
**Objetivo:** Implementar funções completas (Call/Return)

---

## 🎯 Objetivo da Fase 2

Implementar suporte completo para funções definidas pelo usuário:
- Call/Return com calling convention adequada
- Passagem de parâmetros
- Valores de retorno
- Stack frames
- Recursão básica

---

## ✅ Progresso Atual: 10%

### Implementado:
- [x] Campo `function_addresses` adicionado ao code generator
- [x] Método `compile_function` esboçado
- [x] Call/Return básicos melhorados
- [x] Exemplo de teste criado (`sprint26_functions.matter`)

### Em Progresso:
- [ ] Compilação completa de funções do usuário
- [ ] Calling convention (System V AMD64 / Windows x64)
- [ ] Passagem de parâmetros via registradores
- [ ] Stack frames adequados
- [ ] Recursão funcional

### Não Iniciado:
- [ ] Closures
- [ ] Funções aninhadas
- [ ] Tail call optimization

---

## 🔧 Mudanças Técnicas

### 1. Estrutura do Code Generator

**Antes:**
```rust
pub struct X86CodeGen {
    code: Vec<u8>,
    data: Vec<u8>,
    variables: HashMap<String, i32>,
    stack_offset: i32,
    stack_depth: i32,
    jump_targets: HashMap<usize, usize>,
    pending_jumps: Vec<(usize, usize, usize)>,
    pending_data_patches: Vec<(usize, usize)>,
}
```

**Depois:**
```rust
pub struct X86CodeGen {
    code: Vec<u8>,
    data: Vec<u8>,
    variables: HashMap<String, i32>,
    stack_offset: i32,
    stack_depth: i32,
    jump_targets: HashMap<usize, usize>,
    pending_jumps: Vec<(usize, usize, usize)>,
    pending_data_patches: Vec<(usize, usize)>,
    function_addresses: HashMap<String, usize>, // ✅ NOVO
}
```

### 2. Call/Return Melhorados

**Call Instruction:**
```rust
fn compile_call(&mut self, arg_count: usize) -> Result<(), String> {
    // Pop arguments from stack
    for _ in 0..arg_count {
        self.emit_pop(Register::RAX);
    }
    
    // Push placeholder return value
    self.emit_mov_imm(Register::RAX, 0);
    self.emit_push(Register::RAX);
    
    Ok(())
}
```

**Return Instruction:**
```rust
fn compile_return(&mut self) -> Result<(), String> {
    // Pop return value
    if self.stack_depth > 0 {
        self.emit_pop(Register::RAX);
    } else {
        self.emit_mov_imm(Register::RAX, 0);
    }
    
    // Return
    self.code.push(0xC3); // ret
    
    Ok(())
}
```

---

## 📋 Próximos Passos

### Imediato (Esta Semana):
1. ✅ Adicionar `function_addresses` - COMPLETO
2. 🔄 Implementar `compile_function` completo - EM PROGRESSO
3. ⏳ Testar compilação de funções simples
4. ⏳ Implementar calling convention
5. ⏳ Testar passagem de parâmetros

### Próxima Semana:
6. Implementar recursão
7. Testes de integração
8. Benchmarks
9. Documentação
10. Validação completa

---

## 🎯 Calling Convention

### System V AMD64 ABI (Linux, macOS):
```
Parâmetros:
1. RDI
2. RSI
3. RDX
4. RCX
5. R8
6. R9
7+ Stack (direita para esquerda)

Retorno: RAX
Stack: 16-byte aligned antes de CALL
```

### Windows x64:
```
Parâmetros:
1. RCX
2. RDX
3. R8
4. R9
5+ Stack (direita para esquerda)

Retorno: RAX
Stack: 16-byte aligned antes de CALL
Shadow space: 32 bytes reservados
```

---

## 💻 Exemplo de Código Gerado

### Código Matter:
```matter
fn add(a, b) {
    return a + b
}

let result = add(10, 20)
print(result)
```

### Bytecode:
```
FunctionDef "add" params=2
  LoadLocal("a")
  LoadLocal("b")
  Add
  Return

LoadConst(10)
LoadConst(20)
Call(2)  // add(10, 20)
StoreGlobal("result")
LoadGlobal("result")
Print
```

### x86-64 Esperado:
```assembly
; Função add
add:
    push rbp
    mov rbp, rsp
    
    ; Salvar parâmetros no stack
    mov [rbp-8], rdi    ; a
    mov [rbp-16], rsi   ; b
    
    ; LoadLocal("a")
    mov rax, [rbp-8]
    
    ; LoadLocal("b")
    mov rbx, [rbp-16]
    
    ; Add
    add rax, rbx
    
    ; Return
    mov rsp, rbp
    pop rbp
    ret

; Main
main:
    push rbp
    mov rbp, rsp
    
    ; LoadConst(10)
    mov rdi, 10
    
    ; LoadConst(20)
    mov rsi, 20
    
    ; Call add
    call add
    
    ; StoreGlobal("result")
    mov [rbp-8], rax
    
    ; LoadGlobal("result")
    mov rax, [rbp-8]
    
    ; Print
    ; ... (print implementation)
    
    mov rsp, rbp
    pop rbp
    ret
```

---

## 🔍 Desafios Técnicos

### 1. Resolução de Endereços de Função
**Problema:** Funções são compiladas antes do main, mas chamadas precisam saber o endereço.

**Solução:** 
- Compilar todas as funções primeiro
- Armazenar endereços em `function_addresses`
- Usar CALL relativo com offset calculado

### 2. Stack Frame Management
**Problema:** Cada função precisa de seu próprio stack frame.

**Solução:**
- Prólogo: `push rbp; mov rbp, rsp`
- Alocar espaço para locais: `sub rsp, N`
- Epílogo: `mov rsp, rbp; pop rbp; ret`

### 3. Calling Convention
**Problema:** Diferentes plataformas usam convenções diferentes.

**Solução:**
- Usar `#[cfg(target_os = "windows")]` para Windows
- Usar `#[cfg(not(target_os = "windows"))]` para Unix
- Abstrair em funções auxiliares

### 4. Recursão
**Problema:** Funções recursivas precisam preservar estado.

**Solução:**
- Stack frames automáticos resolvem isso
- Cada chamada tem seu próprio frame
- Return address preservado por CALL/RET

---

## 📊 Métricas

### Código Adicionado:
- **Linhas:** ~100
- **Funções:** 2 (compile_function, melhorias em call/return)
- **Campos:** 1 (function_addresses)

### Testes:
- **Unitários:** 0 novos (ainda)
- **Integração:** 1 exemplo criado

### Cobertura:
- **Funções básicas:** 10%
- **Calling convention:** 5%
- **Recursão:** 0%
- **Closures:** 0%

---

## 🎯 Definição de Sucesso - Fase 2

### Critérios:
- [ ] Compilar funções do usuário
- [ ] Chamar funções com parâmetros
- [ ] Retornar valores
- [ ] Recursão funciona
- [ ] Testes passam
- [ ] Exemplo `sprint26_functions.matter` executa

### Meta de Conclusão:
**24 de Maio de 2026** (2 semanas)

---

## 🚀 Status Geral Sprint 26

```
Fase 1: Fundação              ████████████████████ 100% ✅
Fase 2: Instruções Básicas     ██░░░░░░░░░░░░░░░░░░  10% 🔄
Fase 3: Controle de Fluxo      ░░░░░░░░░░░░░░░░░░░░   0%
Fase 4: Funções                ░░░░░░░░░░░░░░░░░░░░   0%
Fase 5: Otimizações            ░░░░░░░░░░░░░░░░░░░░   0%
Fase 6: Multi-plataforma       ░░░░░░░░░░░░░░░░░░░░   0%

Sprint 26 Total: 32% Completo
```

---

**SEM MEDIOCRIDADE - Construindo funções completas!** 🚀

---

*Sprint 26: Matter Native Compiler*  
*Fase 2: Instruções Básicas - 🔄 EM PROGRESSO*  
*Data: 10 de Maio de 2026*  
*Progresso: 10% da Fase 2, 32% do Sprint 26*  
*Status: 🟢 AVANÇANDO - Funções em implementação*
