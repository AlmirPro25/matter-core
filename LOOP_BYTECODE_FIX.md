# Correção do Bug de Loop Infinito no Bytecode

## Problema

Ao executar `matter-cli run-bytecode loops.mbc`, o programa imprimia zeros infinitamente em vez de executar os loops corretamente.

## Investigação

### 1. Análise do Bytecode

Usando `matter-cli inspect loops.mbc`, identificamos que o bytecode estava sendo gerado corretamente:

```
4: LoadGlobal("i")      # carrega i
5: LoadConst(2)         # carrega 5
6: Lt                   # i < 5
7: JumpIfFalse(17)      # se falso, pula para 17
8: PushScope            # entra em novo escopo
9: LoadGlobal("i")      # carrega i
10: Print               # imprime i
11: LoadGlobal("i")     # carrega i
12: LoadConst(3)        # carrega 1
13: Add                 # i + 1
14: StoreGlobal("i")    # armazena em i
15: PopScope            # sai do escopo
16: Jump(4)             # volta para 4
```

### 2. Jump Targets Corretos

Os jump targets estavam sendo serializados e deserializados corretamente:
- `Jump(4)` na instrução 16 volta corretamente para a instrução 4
- `JumpIfFalse(17)` na instrução 7 pula corretamente para a instrução 17
- Break e continue estavam sendo patcheados corretamente

### 3. Causa Raiz

O problema estava na **VM**, não no bytecode. A instrução `StoreGlobal` tinha uma lógica incorreta:

```rust
// CÓDIGO INCORRETO
Instruction::StoreGlobal(name) => {
    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
    
    // Se estamos em escopo local, armazena local
    // Senão, armazena global
    if self.scope_stack.is_empty() {
        self.globals.insert(name.clone(), value);
    } else {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.variables.insert(name.clone(), value);
        }
    }
}
```

**Problema**: Quando um loop executa `PushScope`, qualquer `StoreGlobal` dentro do loop estava armazenando no escopo local em vez do global. Isso significa que `set i = i + 1` criava uma variável local `i` em vez de atualizar a global `i`, causando o loop infinito.

## Solução

Corrigir a VM para que `StoreGlobal` **sempre** armazene no escopo global:

```rust
// CÓDIGO CORRETO
Instruction::StoreGlobal(name) => {
    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
    
    // StoreGlobal SEMPRE armazena no escopo global
    // Isso é necessário para que variáveis globais possam ser
    // atualizadas de dentro de loops e blocos
    self.globals.insert(name.clone(), value);
}
```

## Validação

### 1. Serialização/Deserialização

✅ Jump targets são serializados corretamente como `u32` little-endian
✅ Break patching funciona corretamente
✅ Continue patching funciona corretamente
✅ Instruction pointer é atualizado corretamente
✅ Stack cleanup por iteração funciona corretamente
✅ Variável de loop é atualizada corretamente

### 2. Comando `matter inspect`

Criado comando melhorado que mostra:
- Instruction index
- Opcode
- Jump targets com comentários
- Constants inline
- Comentários explicativos

Exemplo:
```
│    4: LoadGlobal("i")      ; load i
│    5: LoadConst(2)         ; const[2] = 5
│    6: Lt                   ; pop b, pop a, push a<b
│    7: JumpIfFalse(17)      ; -> 17 if false
│    8: PushScope            ; enter new scope
│    9: LoadGlobal("i")      ; load i
│   10: Print                ; pop and print
│   11: LoadGlobal("i")      ; load i
│   12: LoadConst(3)         ; const[3] = 1
│   13: Add                  ; pop b, pop a, push a+b
│   14: StoreGlobal("i")     ; store i
│   15: PopScope             ; exit scope
│   16: Jump(4)              ; -> 4
```

### 3. Teste de Equivalência

Criado script `test_bytecode_equivalence.ps1` que verifica:
```
compile -> save -> load -> execute == source execution
```

**Resultados**:
- ✅ test_loops.matter: PASSOU
- ✅ test_functions.matter: PASSOU
- ✅ test_recursion.matter: PASSOU
- ✅ simple.matter: PASSOU

## Arquivos Modificados

1. **crates/matter-vm/src/lib.rs**
   - Corrigido `StoreGlobal` para sempre armazenar no escopo global

2. **crates/matter-cli/src/main.rs**
   - Melhorado comando `inspect` com formatação visual e comentários

3. **test_bytecode_equivalence.ps1** (novo)
   - Script de teste automatizado para validar equivalência

## Conclusão

O problema não estava na serialização/deserialização do bytecode, mas sim na semântica de execução da VM. A correção garante que:

1. Variáveis globais podem ser atualizadas de dentro de loops e blocos
2. O bytecode persistido executa exatamente como o código fonte
3. Todos os testes de loop (while, loop, break, continue, nested) funcionam corretamente
