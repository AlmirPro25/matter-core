# Runtime Integrity — O Que Foi Conquistado

## Definição

**Runtime Integrity** é a garantia de que:

```
o que você escreve = o que compila = o que persiste = o que executa
```

Essa igualdade é uma das coisas mais difíceis em uma linguagem nova.

## Por Que É Difícil

### Problema 1: Semântica Ambígua
Muitas linguagens novas falham porque:
- Sintaxe é clara, mas semântica é vaga
- Comportamento muda entre interpretação e compilação
- Escopo e mutação têm regras inconsistentes

**Matter resolveu**: Semântica operacional clara (let/set/StoreGlobal)

### Problema 2: Bytecode Divergente
Compiladores podem gerar bytecode que:
- Executa diferente do source
- Perde informação de escopo
- Tem bugs de otimização prematura

**Matter resolveu**: Equivalência validada (source == bytecode)

### Problema 3: Persistência Quebrada
Serialização pode:
- Corromper estruturas de dados
- Perder metadados importantes
- Falhar em round-trip

**Matter resolveu**: MBC1 com round-trip completo

## O Que Matter Consolidou

### 1. Semântica Operacional Clara

**Antes** (ambíguo):
```matter
let x = 10
if true {
    set x = 20  # Cria local ou atualiza global? 🤔
}
```

**Agora** (claro):
```matter
let x = 10      # Cria global
if true {
    set x = 20  # Atualiza global ✅
}

let y = 5
if true {
    let y = 10  # Cria local (shadowing) ✅
    print y     # 10
}
print y         # 5
```

**Regras**:
- `let` = introdução de binding no escopo atual
- `set` = mutação de binding existente (busca local → global)
- `StoreGlobal` = mutação explícita do estado global

### 2. Bytecode Persistente (.mbc)

**Antes**: Bytecode só existia em memória
**Agora**: Bytecode é um artefato executável

```bash
# Compilar
matter compile app.matter -o app.mbc

# Distribuir
cp app.mbc /usr/local/bin/

# Executar
matter run-bytecode app.mbc
```

**Benefícios**:
- Distribuição sem source
- Caching de compilação
- Base para otimização
- Package system futuro

### 3. Toolchain Mínima Real

**Antes**: Só tinha `matter run`
**Agora**: Ecossistema completo

```bash
matter compile      # Compilador
matter run          # Interpretador
matter run-bytecode # Executor de bytecode
matter inspect      # Inspetor de bytecode
```

Isso já é o começo de um ecossistema.

### 4. Equivalência Validada

**Antes**: Confiança baseada em esperança
**Agora**: Confiança baseada em testes

```powershell
# test_bytecode_equivalence.ps1
foreach ($file in $testFiles) {
    $sourceOutput = matter run $file
    $bytecodeOutput = matter run-bytecode (compile $file)
    
    assert $sourceOutput == $bytecodeOutput
}
```

**Resultados**:
- ✅ test_loops.matter
- ✅ test_functions.matter
- ✅ test_recursion.matter
- ✅ simple.matter

**Equivalência: 100%**

## O Que Isso Significa

### Para Desenvolvedores

**Confiança**:
- O que você vê é o que você executa
- Sem surpresas entre dev e prod
- Debugging previsível

**Produtividade**:
- Compile uma vez, execute em qualquer lugar
- Cache de compilação
- Distribuição simplificada

### Para a Linguagem

**Maturidade**:
- Semântica estável
- Formato de bytecode versionado
- Base para evolução

**Credibilidade**:
- Não é mais um protótipo
- É uma linguagem real
- Pode ser usada em produção (após v0.2)

### Para o Ecossistema

**Fundação**:
- Package manager pode confiar no bytecode
- Otimizador pode transformar sem quebrar
- Debugger pode mapear bytecode → source

**Crescimento**:
- Standard library pode ser distribuída como .mbc
- Plugins podem ser bytecode
- Módulos podem ser pré-compilados

## Comparação com Outras Linguagens

### Python
- ✅ Bytecode (.pyc)
- ❌ Equivalência não garantida (otimizações podem mudar comportamento)
- ❌ Bytecode não é estável entre versões

### JavaScript
- ❌ Sem bytecode padrão (cada engine tem o seu)
- ❌ Equivalência depende do engine
- ✅ Source é o artefato de distribuição

### Rust
- ✅ Bytecode (LLVM IR)
- ✅ Equivalência garantida
- ✅ Otimizações preservam semântica
- ⚠️ Complexidade alta

### Matter
- ✅ Bytecode (MBC1)
- ✅ Equivalência garantida e testada
- ✅ Formato estável e versionado
- ✅ Simplicidade mantida

## O Que Vem Depois

Com runtime integrity estabelecida, o próximo gargalo é **modelagem**.

### Sprint 4: Data Model
Sem dados compostos, apps reais ficam limitados.

**Objetivo**: Transformar Matter de "linguagem de fluxo" em "linguagem de modelagem de aplicações".

### Sprint 5+: Evolução
Com base sólida, pode crescer sem reescrever o núcleo:
- Pattern matching
- Sistema de módulos
- Error handling
- Standard library
- Otimizador

## Lições Aprendidas

### 1. Bug Pedagógico
O bug de loop infinito foi **essencial**. Ele forçou clareza semântica.

**Antes**: Semântica vaga
**Depois**: Regras explícitas

### 2. Equivalência Primeiro
Validar equivalência desde cedo evita problemas futuros.

**Regra**: Cada nova feature deve passar no teste de equivalência.

### 3. Tooling Importa
`matter inspect` não é luxo, é necessidade.

**Razão**: Debugging de bytecode é impossível sem visualização.

### 4. Documentação Viva
SPEC.md deve refletir a realidade, não a intenção.

**Prática**: Atualizar SPEC após cada mudança semântica.

## Conclusão

**Runtime Integrity** não é um feature, é uma propriedade fundamental.

Matter conquistou isso em Sprint 3.5, e isso separa:
- Protótipo de linguagem real
- Experimento de ferramenta de produção
- Hobby de projeto sério

**A base está sólida. Agora é hora de construir sobre ela.**

---

**Conquistado em**: Sprint 3.5 (Maio 2026)
**Próximo marco**: Sprint 4 — Data Model
**Status**: ✅ Consolidado
