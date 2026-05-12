# 🎮 JOGOS EM MATTER CORE 🎮

Bem-vindo à coleção de jogos desenvolvidos na linguagem **Matter Core**!

## 🐍 Jogo da Cobrinha (`jogo_cobrinha.matter`)

Um jogo clássico da cobrinha implementado em Matter!

### Como Jogar:
```bash
matter run jogo_cobrinha.matter
```

### Características:
- 🎯 Tabuleiro 20x10
- 🐍 Cobra que se move automaticamente
- 🍎 Sistema de comida e pontuação
- 💥 Detecção de colisão com paredes
- 📊 Sistema de status em tempo real
- 🎬 15 frames de animação

### Mecânica:
- A cobra se move automaticamente para a direita
- Quando come a comida, ganha +10 pontos
- Ao bater na parede, volta para o início
- Modo demonstração (automático)

### Pontuação:
- 🍎 Cada comida = +10 pontos
- 🏆 Objetivo: Comer o máximo de comidas possível!

---

## 🎲 Jogo de Adivinhação (`jogo_adivinhacao.matter`)

Adivinhe o número secreto entre 1 e 100!

### Como Jogar:
```bash
matter run jogo_adivinhacao.matter
```

### Características:
- 🎯 Número secreto entre 1 e 100
- 🤔 7 tentativas para adivinhar
- 📈 Dicas: "Muito alto" ou "Muito baixo"
- 🏆 Sistema de classificação por desempenho
- 🎬 Modo demonstração com busca binária

### Mecânica:
- O jogo escolhe um número secreto
- Você tem 7 tentativas
- A cada tentativa, recebe uma dica
- Tente adivinhar com o menor número de tentativas!

### Sistema de Classificação:
- ⭐⭐⭐ **INCRÍVEL!** - 1 a 3 tentativas
- ⭐⭐ **MUITO BOM!** - 4 a 5 tentativas
- ⭐ **BOM!** - 6 a 7 tentativas
- 😢 **Não acertou** - Mais de 7 tentativas

---

## 🚀 Tecnologia

Todos os jogos foram desenvolvidos usando:

- **Linguagem:** Matter Core v0.1.5
- **Runtime:** Matter VM (stack-based)
- **Bytecode:** MBC1 (Matter Bytecode v1)
- **Paradigma:** Runtime-oriented language system

### Recursos da Linguagem Utilizados:

✅ **Variáveis e Estado Mutável**
```matter
let pontos = 0
set pontos = pontos + 10
```

✅ **Funções com Parâmetros**
```matter
fn verificar_tentativa(tentativa, secreto, numero) {
    return resultado
}
```

✅ **Condicionais (if/else)**
```matter
if tentativa == secreto {
    print "Acertou!"
} else {
    print "Errou!"
}
```

✅ **Loops (while)**
```matter
while frame < 15 {
    set frame = frame + 1
}
```

✅ **Operadores Aritméticos**
```matter
set x = x + 1
set pontos = pontos * 2
```

✅ **Operadores de Comparação**
```matter
if x == y { }
if x < y { }
if x >= y { }
```

---

## 📊 Estatísticas dos Jogos

### Jogo da Cobrinha:
- **Linhas de código:** ~100
- **Funções:** 2 (desenhar_tabuleiro, mostrar_status)
- **Variáveis:** 8
- **Loops:** 3 (while aninhados)
- **Frames:** 15

### Jogo de Adivinhação:
- **Linhas de código:** ~120
- **Funções:** 1 (verificar_tentativa)
- **Variáveis:** 10
- **Condicionais:** 15+
- **Tentativas:** 7

---

## 🎯 Próximos Jogos Planejados

### 🏓 Pong
- Jogo clássico de ping-pong
- 2 jogadores (simulado)
- Sistema de física simples
- Placar em tempo real

### 🧩 Tetris
- Blocos caindo
- Rotação de peças
- Sistema de linhas completas
- Níveis de dificuldade

### 🎰 Jogo da Memória
- Cartas viradas
- Sistema de pares
- Contador de tentativas
- Timer

### 🎯 Jogo de Tiro ao Alvo
- Mira móvel
- Sistema de pontuação
- Dificuldade progressiva
- High score

---

## 💡 Como Criar Seu Próprio Jogo

### 1. Estrutura Básica:
```matter
# Título e apresentação
print "=== MEU JOGO ==="

# Variáveis de estado
let pontos = 0
let vidas = 3

# Loop principal do jogo
let rodada = 0
while rodada < 10 {
    # Lógica do jogo aqui
    set rodada = rodada + 1
}

# Resultado final
print "Fim do jogo!"
print pontos
```

### 2. Adicione Funções:
```matter
fn calcular_pontos(acertos, bonus) {
    let total = acertos * 10
    set total = total + bonus
    return total
}
```

### 3. Use Condicionais:
```matter
if pontos > 100 {
    print "Você ganhou!"
} else {
    print "Tente novamente!"
}
```

### 4. Crie Loops Interessantes:
```matter
let nivel = 1
while nivel <= 5 {
    print "Nível: "
    print nivel
    # Lógica do nível
    set nivel = nivel + 1
}
```

---

## 🏆 Desafios

### Desafio 1: Melhore a Cobrinha
- Adicione movimento em 4 direções
- Implemente crescimento da cobra
- Adicione obstáculos no tabuleiro

### Desafio 2: Melhore a Adivinhação
- Adicione diferentes níveis de dificuldade
- Implemente um sistema de ranking
- Adicione um timer

### Desafio 3: Crie Seu Próprio Jogo
- Use os exemplos como base
- Combine diferentes mecânicas
- Compartilhe com a comunidade!

---

## 📚 Recursos de Aprendizado

### Documentação Matter:
- `docs/MANIFESTO.md` - Filosofia da linguagem
- `docs/SPEC.md` - Especificação completa
- `docs/ARCHITECTURE.md` - Arquitetura técnica
- `README.md` - Guia geral

### Exemplos:
- `examples/` - 18+ exemplos funcionais
- `examples/functions.matter` - Funções
- `examples/test_loops.matter` - Loops
- `examples/showcase.matter` - Showcase completo

---

## 🤝 Contribua

Criou um jogo legal em Matter? Compartilhe!

1. Adicione seu jogo na pasta raiz
2. Documente as regras
3. Teste com `matter run seu_jogo.matter`
4. Compartilhe com a comunidade!

---

## 🎉 Divirta-se!

Matter Core não é apenas uma linguagem de programação - é uma plataforma para criar experiências interativas!

**Desenvolvido com ❤️ usando Matter Core v0.1.5**

---

## 📞 Suporte

Problemas ao executar os jogos?

1. Verifique se Matter está instalado: `matter --version`
2. Teste com exemplos básicos: `matter run examples/hello.matter`
3. Veja a documentação: `COMO_INSTALAR.txt`
4. Execute os testes: `cargo test`

**Matter Core - Runtime-Oriented Language System** 🚀
