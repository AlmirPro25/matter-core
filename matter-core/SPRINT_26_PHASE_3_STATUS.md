# 🔄 SPRINT 26 - FASE 3: CONTROLE DE FLUXO

**Data:** 10 de Maio de 2026  
**Status:** 🔄 EM PROGRESSO (20%)  
**Prioridade:** 🔥 ALTA

---

## 🎯 OBJETIVO

Implementar controle de fluxo completo no compilador nativo Matter (MNC), tornando-o **Turing-complete**.

---

## 📊 PROGRESSO ATUAL

### ✅ Já Implementado (Fase 1 e 2)

1. **Jump Básico**
   - `compile_jump` - JMP rel32
   - Patch de offsets relativos
   - Jumps forward e backward

2. **Jump Condicional**
   - `compile_jump_if_false` - JE rel32
   - Test de condição
   - Patch de offsets

3. **Comparações**
   - `compile_eq` - Igualdade
   - `compile_not_eq` - Diferença
   - `compile_lt` - Menor que
   - `compile_gt` - Maior que
   - `compile_lt_eq` - Menor ou igual
   - `compile_gt_eq` - Maior ou igual

### 🔄 Em Progresso

4. **If/Else Statements**
   - ✅ Estrutura básica funciona via jumps
   - 🔄 Otimização de código gerado
   - 🔄 Nested if statements
   - 🔄 If-else chains

5. **While Loops**
   - ✅ Estrutura básica funciona via jumps
   - 🔄 Otimização de backward jumps
   - 🔄 Loop invariant code motion
   - 🔄 Nested loops

### ⏳ Planejado

6. **For Loops**
   - Iteração sobre ranges
   - Iteração sobre coleções
   - Loop unrolling

7. **Break/Continue**
   - Break statement
   - Continue statement
   - Loop context management

8. **Advanced Control Flow**
   - Switch/match statements
   - Pattern matching
   - Guard clauses

---

## 🔧 IMPLEMENTAÇÃO TÉCNICA

### Estruturas de Controle em x86-64

#### If Statement

**Matter Code:**
```matter
if condition {
    // then block
}
// after
```

**x86-64 Assembly:**
```assembly
    ; Evaluate condition
    pop rax
    test rax, rax
    je .after           ; Jump if false
    
    ; Then block
    ; ...
    
.after:
    ; Continue
```

#### If-Else Statement

**Matter Code:**
```matter
if condition {
    // then block
} else {
    // else block
}
// after
```

**x86-64 Assembly:**
```assembly
    ; Evaluate condition
    pop rax
    test rax, rax
    je .else            ; Jump if false
    
    ; Then block
    ; ...
    jmp .after          ; Skip else
    
.else:
    ; Else block
    ; ...
    
.after:
    ; Continue
```

#### While Loop

**Matter Code:**
```matter
while condition {
    // body
}
// after
```

**x86-64 Assembly:**
```assembly
.loop_start:
    ; Evaluate condition
    pop rax
    test rax, rax
    je .loop_end        ; Jump if false
    
    ; Body
    ; ...
    
    jmp .loop_start     ; Backward jump
    
.loop_end:
    ; Continue
```

#### For Loop (via Range)

**Matter Code:**
```matter
for i in range(n) {
    // body
}
```

**Desugared to While:**
```matter
let i = 0
while i < n {
    // body
    set i = i + 1
}
```

---

## 📋 TAREFAS PENDENTES

### Tarefa 3.1: Otimizar Geração de If/Else ⏳

**Objetivo:** Melhorar código gerado para if/else

**Implementação:**
```rust
fn compile_if_else(
    &mut self,
    condition: &Instruction,
    then_block: &[Instruction],
    else_block: Option<&[Instruction]>,
    constants: &[Constant],
) -> Result<(), String> {
    // Compile condition
    self.compile_instruction(condition, constants)?;
    
    // Pop condition result
    self.emit_pop(Register::RAX);
    self.emit_test_reg(Register::RAX);
    
    if let Some(else_instrs) = else_block {
        // If-else case
        let else_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder
        
        // Then block
        for instr in then_block {
            self.compile_instruction(instr, constants)?;
        }
        
        let after_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder
        
        // Patch else jump
        let else_start = self.code.len();
        self.patch_jump_at(else_jump_pos, else_start)?;
        
        // Else block
        for instr in else_instrs {
            self.compile_instruction(instr, constants)?;
        }
        
        // Patch after jump
        let after_start = self.code.len();
        self.patch_jump_at(after_jump_pos, after_start)?;
    } else {
        // If only case
        let after_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder
        
        // Then block
        for instr in then_block {
            self.compile_instruction(instr, constants)?;
        }
        
        // Patch after jump
        let after_start = self.code.len();
        self.patch_jump_at(after_jump_pos, after_start)?;
    }
    
    Ok(())
}
```

### Tarefa 3.2: Otimizar Geração de Loops ⏳

**Objetivo:** Melhorar código gerado para loops

**Implementação:**
```rust
fn compile_while_loop(
    &mut self,
    condition: &Instruction,
    body: &[Instruction],
    constants: &[Constant],
) -> Result<(), String> {
    // Loop start
    let loop_start = self.code.len();
    
    // Compile condition
    self.compile_instruction(condition, constants)?;
    
    // Pop condition result
    self.emit_pop(Register::RAX);
    self.emit_test_reg(Register::RAX);
    
    // Jump to end if false
    let end_jump_pos = self.code.len();
    self.emit_je(0); // Placeholder
    
    // Body
    for instr in body {
        self.compile_instruction(instr, constants)?;
    }
    
    // Jump back to start
    let back_offset = (loop_start as i32) - (self.code.len() as i32) - 5;
    self.emit_jmp(back_offset);
    
    // Patch end jump
    let loop_end = self.code.len();
    self.patch_jump_at(end_jump_pos, loop_end)?;
    
    Ok(())
}
```

### Tarefa 3.3: Implementar Break/Continue 🆕

**Objetivo:** Suporte a break e continue em loops

**Estrutura:**
```rust
struct LoopContext {
    start_label: usize,
    end_label: usize,
    break_jumps: Vec<usize>,
    continue_jumps: Vec<usize>,
}

impl X86CodeGen {
    fn push_loop_context(&mut self, start: usize) {
        self.loop_stack.push(LoopContext {
            start_label: start,
            end_label: 0, // Will be set later
            break_jumps: Vec::new(),
            continue_jumps: Vec::new(),
        });
    }
    
    fn pop_loop_context(&mut self) -> Option<LoopContext> {
        self.loop_stack.pop()
    }
    
    fn compile_break(&mut self) -> Result<(), String> {
        let ctx = self.loop_stack.last_mut()
            .ok_or("Break outside loop")?;
        
        let jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder
        ctx.break_jumps.push(jump_pos);
        
        Ok(())
    }
    
    fn compile_continue(&mut self) -> Result<(), String> {
        let ctx = self.loop_stack.last()
            .ok_or("Continue outside loop")?;
        
        let back_offset = (ctx.start_label as i32) - (self.code.len() as i32) - 5;
        self.emit_jmp(back_offset);
        
        Ok(())
    }
}
```

### Tarefa 3.4: Implementar For Loops 🆕

**Objetivo:** Suporte a for loops sobre ranges e coleções

**Desugaring:**
```rust
// for i in range(n) { body }
// Becomes:
// let __iter = 0
// while __iter < n {
//     let i = __iter
//     body
//     set __iter = __iter + 1
// }
```

### Tarefa 3.5: Criar Testes de Integração 🆕

**Arquivo:** `crates/matter-native/tests/control_flow.rs`

```rust
#[test]
fn test_if_statement() {
    // Test simple if
}

#[test]
fn test_if_else_statement() {
    // Test if-else
}

#[test]
fn test_nested_if() {
    // Test nested if statements
}

#[test]
fn test_while_loop() {
    // Test while loop
}

#[test]
fn test_nested_loops() {
    // Test nested loops
}

#[test]
fn test_break_statement() {
    // Test break
}

#[test]
fn test_continue_statement() {
    // Test continue
}

#[test]
fn test_for_loop() {
    // Test for loop
}
```

---

## 🧪 VALIDAÇÃO

### Testes Existentes

1. ✅ **test_jump_if_false_and_jump_offsets_are_patched**
   - Valida patch de jumps condicionais
   - Testa if-else básico

2. ✅ **test_loop_contains_backward_jump**
   - Valida backward jumps em loops
   - Testa while loop básico

3. ✅ **test_fuzz_cfg_jump_patch_stability**
   - Fuzzing de controle de fluxo
   - 120 programas gerados aleatoriamente

### Testes Necessários

4. ⏳ **test_nested_if_statements**
   - If dentro de if
   - Múltiplos níveis

5. ⏳ **test_if_else_chains**
   - If-else-if-else
   - Switch-like patterns

6. ⏳ **test_nested_loops**
   - Loop dentro de loop
   - Múltiplos níveis

7. ⏳ **test_break_in_loop**
   - Break simples
   - Break em nested loops

8. ⏳ **test_continue_in_loop**
   - Continue simples
   - Continue em nested loops

9. ⏳ **test_for_loop_range**
   - For sobre range
   - For com diferentes limites

10. ⏳ **test_complex_control_flow**
    - Combinação de if/else e loops
    - Funções com controle de fluxo complexo

---

## 📊 MÉTRICAS

### Código Atual

- **Linhas implementadas:** ~1500 (Fase 1 e 2)
- **Métodos de controle de fluxo:** 3 (jump, jump_if_false, comparisons)
- **Testes passando:** 15 unitários + 10 integração

### Meta da Fase 3

- **Linhas a adicionar:** ~300
- **Métodos novos:** 5 (if_else, while, for, break, continue)
- **Testes a adicionar:** 10 novos

### Performance Esperada

**If Statement:**
- Bytecode VM: ~50 ns
- Native: ~2 ns
- **Speedup: 25x**

**While Loop (10 iterações):**
- Bytecode VM: ~500 ns
- Native: ~20 ns
- **Speedup: 25x**

**Nested Loops (10x10):**
- Bytecode VM: ~5 µs
- Native: ~200 ns
- **Speedup: 25x**

---

## 🎯 PRÓXIMOS PASSOS

### Esta Semana

1. **Otimizar If/Else** (2 dias)
   - Melhorar geração de código
   - Reduzir instruções desnecessárias
   - Testes

2. **Otimizar Loops** (2 dias)
   - Melhorar backward jumps
   - Loop invariant code motion
   - Testes

3. **Implementar Break/Continue** (1 dia)
   - Loop context stack
   - Patch de jumps
   - Testes

4. **Implementar For Loops** (1 dia)
   - Desugaring para while
   - Range support
   - Testes

5. **Validação Completa** (1 dia)
   - Executar todos os testes
   - Benchmarks
   - Documentação

---

## 🎉 MARCO: TURING-COMPLETE

Ao completar a Fase 3, o compilador nativo Matter será **Turing-complete**!

**Capacidades:**
- ✅ Variáveis e estado
- ✅ Funções e recursão
- ✅ Condicionais (if/else)
- ✅ Loops (while, for)
- ✅ Break/Continue
- ✅ Comparações
- ✅ Aritmética

**Isso significa:**
- Pode computar qualquer função computável
- Pode implementar qualquer algoritmo
- Pode resolver qualquer problema computacional

---

## 💡 OTIMIZAÇÕES FUTURAS (Fase 5)

### Loop Optimizations

1. **Loop Unrolling**
   ```assembly
   ; Original
   .loop:
       ; body
       dec rcx
       jnz .loop
   
   ; Unrolled (4x)
   .loop:
       ; body 1
       ; body 2
       ; body 3
       ; body 4
       sub rcx, 4
       jnz .loop
   ```

2. **Loop Invariant Code Motion**
   ```assembly
   ; Original
   .loop:
       mov rax, [rbp-8]    ; Invariant
       add rax, rbx        ; Variant
       ; ...
       jmp .loop
   
   ; Optimized
   mov rax, [rbp-8]        ; Moved outside
   .loop:
       add rax, rbx
       ; ...
       jmp .loop
   ```

3. **Strength Reduction**
   ```assembly
   ; Original
   .loop:
       imul rax, rcx, 4    ; Expensive
       ; ...
       jmp .loop
   
   ; Optimized
   .loop:
       lea rax, [rcx*4]    ; Cheaper
       ; ...
       jmp .loop
   ```

---

## 📚 REFERÊNCIAS

### Control Flow em x86-64

1. **Conditional Jumps**
   - JE/JZ - Jump if equal/zero
   - JNE/JNZ - Jump if not equal/not zero
   - JL/JNGE - Jump if less
   - JG/JNLE - Jump if greater
   - JLE/JNG - Jump if less or equal
   - JGE/JNL - Jump if greater or equal

2. **Unconditional Jumps**
   - JMP - Unconditional jump
   - CALL - Call function
   - RET - Return from function

3. **Loop Instructions**
   - LOOP - Decrement RCX and jump if not zero
   - LOOPE/LOOPZ - Loop while equal/zero
   - LOOPNE/LOOPNZ - Loop while not equal/not zero

### Documentação

1. **Intel Manual**
   - Volume 1: Basic Architecture
   - Volume 2: Instruction Set Reference

2. **AMD64 ABI**
   - Calling conventions
   - Stack alignment
   - Register usage

---

## ✅ CHECKLIST DE PROGRESSO

### Fase 3: Controle de Fluxo

- [x] Jump básico implementado
- [x] JumpIfFalse implementado
- [x] Comparações implementadas
- [ ] If/Else otimizado
- [ ] While loops otimizado
- [ ] For loops implementado
- [ ] Break implementado
- [ ] Continue implementado
- [ ] Testes de integração criados
- [ ] Exemplo completo validado
- [ ] Documentação atualizada
- [ ] Benchmarks executados

---

**SEM MEDIOCRIDADE - FASE 3 EM PROGRESSO!** 🚀

---

*Sprint 26 - Fase 3: Controle de Fluxo*  
*Data: 10 de Maio de 2026*  
*Status: 🔄 EM PROGRESSO (20%)*  
*Próximo: Otimizar If/Else e Loops*  
*Progresso Sprint 26: 60%*  
*Progresso Matter Core: 95%*

