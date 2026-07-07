# Matter Core - Estrutura do Projeto

**Versão:** v0.2.1  
**Data:** 9 de Maio de 2026

---

## 📂 Estrutura de Diretórios

```
matter-core/
│
├── 📁 crates/                      # Código fonte (10 crates)
│   ├── matter-ast/                # Definições da AST
│   ├── matter-backend/            # Contratos de backend
│   ├── matter-bytecode/           # Compilador MBC1
│   ├── matter-cli/                # Interface CLI
│   ├── matter-error/              # Sistema de erros
│   ├── matter-lexer/              # Análise léxica
│   ├── matter-parser/             # Análise sintática
│   ├── matter-runtime/            # Sistema de eventos
│   ├── matter-stdlib/             # Biblioteca padrão
│   ├── matter-visual/             # Backend visual
│   └── matter-vm/                 # Máquina virtual
│
├── 📁 docs/                        # Documentação
│   ├── INDEX.md                   # Índice de documentação
│   ├── MANIFESTO.md               # Filosofia e princípios
│   ├── SPEC.md                    # Especificação da linguagem
│   ├── ARCHITECTURE.md            # Arquitetura técnica
│   ├── VISUAL_BACKEND.md          # Backend visual
│   ├── API_CLI_BRIDGE.md          # Bridge API/CLI
│   │
│   ├── 📁 guides/                 # Guias de uso
│   │   ├── INSTALL_GUIDE.md      # Instalação
│   │   ├── QUICKSTART.md         # Início rápido
│   │   └── PVM_INTEGRATION_GUIDE.md
│   │
│   ├── 📁 sprints/                # Documentação de sprints
│   │   ├── SPRINT_1.md           # Funções
│   │   ├── SPRINT_2.md           # Escopo
│   │   ├── SPRINT_3.md           # Loops
│   │   ├── SPRINT_3.5_SUMMARY.md # MBC1
│   │   ├── SPRINT_3.8_SUMMARY.md # CLI
│   │   ├── SPRINT_4_SUMMARY.md   # REPL
│   │   ├── SPRINT_4.1_PERSISTENT_STATE.md
│   │   └── SPRINT_5_SHOWCASE_EXAMPLES.md
│   │
│   ├── 📁 sessions/               # Resumos de sessões
│   │   ├── SESSION_SUMMARY_v0.2.0.md
│   │   ├── SESSION_SUMMARY_v0.2.1.md
│   │   └── EXECUTIVE_SUMMARY.md
│   │
│   └── 📁 releases/               # Notas de release
│       ├── RELEASE_NOTES_v0.1.6.md
│       └── CURRENT_STATE.md
│
├── 📁 examples/                    # Exemplos de código
│   ├── README.md                  # Guia de exemplos
│   ├── hello.matter               # Hello World
│   ├── calculator.matter          # Calculadora
│   ├── fibonacci.matter           # Fibonacci
│   ├── data_processing.matter     # Processamento de dados
│   ├── event_driven_app.matter    # App com eventos
│   ├── backend_integration.matter # Todos os backends
│   ├── todo_app.matter            # App completo
│   └── ... (31 exemplos totais)
│
├── 📁 tests/                       # Testes de integração
│   ├── integration_test.rs        # Testes end-to-end
│   └── visual_backend_test.rs     # Testes visual backend
│
├── 📁 scripts/                     # Scripts de automação
│   ├── README.md                  # Documentação de scripts
│   ├── install.ps1                # Instalação global
│   ├── install-local.ps1          # Instalação local
│   ├── test_all.ps1               # Executar todos os testes
│   ├── test_repl_simple.ps1       # Testar REPL
│   └── test_bytecode_equivalence.ps1
│
├── 📁 bytecode/                    # Bytecode compilado
│   ├── README.md                  # Documentação de bytecode
│   ├── simple.mbc                 # Exemplos compilados
│   ├── test_functions.mbc
│   └── ... (arquivos .mbc)
│
├── 📁 target/                      # Build artifacts (gitignored)
│
├── 📄 README.md                    # Visão geral do projeto
├── 📄 PROGRESS.md                  # Progresso e roadmap
├── 📄 STRATEGIC_VISION.md          # Visão estratégica
├── 📄 PROJECT_STRUCTURE.md         # Este arquivo
├── 📄 Cargo.toml                   # Workspace Rust
├── 📄 Cargo.lock                   # Lock de dependências
├── 📄 .gitignore                   # Arquivos ignorados
└── 📄 matter.toml                  # Configuração Matter
```

---

## 📊 Estatísticas

### Código Fonte
- **10 crates** modulares
- **~4000 linhas** de código Rust
- **30+ instruções** de bytecode
- **Zero dependências** externas

### Documentação
- **50+ documentos** markdown
- **10 sprints** documentados
- **4 sessões** de desenvolvimento
- **3 releases** documentadas

### Exemplos
- **31 exemplos** funcionais
- **6 categorias** (básico, intermediário, avançado, visual, stdlib, jogos)
- **100% cobertura** de backends

### Testes
- **28 testes** de integração
- **100% passando**
- **~85% cobertura** de código

---

## 🎯 Navegação por Objetivo

### Quero Aprender Matter

1. [README.md](README.md) - Visão geral
2. [docs/guides/QUICKSTART.md](docs/guides/QUICKSTART.md) - Início rápido
3. [docs/SPEC.md](docs/SPEC.md) - Especificação
4. [examples/README.md](examples/README.md) - Exemplos práticos

### Quero Usar Matter

1. [docs/guides/INSTALL_GUIDE.md](docs/guides/INSTALL_GUIDE.md) - Instalação
2. [examples/](examples/) - Exemplos prontos
3. [docs/VISUAL_BACKEND.md](docs/VISUAL_BACKEND.md) - Backend visual

### Quero Contribuir

1. [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - Arquitetura
2. [PROGRESS.md](PROGRESS.md) - Roadmap
3. [docs/sprints/](docs/sprints/) - Sprints anteriores
4. [tests/](tests/) - Testes

### Quero Entender o Projeto

1. [docs/MANIFESTO.md](docs/MANIFESTO.md) - Filosofia
2. [STRATEGIC_VISION.md](STRATEGIC_VISION.md) - Visão estratégica
3. [docs/sessions/](docs/sessions/) - Histórico de desenvolvimento

---

## 🔍 Convenções

### Nomenclatura de Arquivos

**Documentação:**
- `UPPERCASE.md` - Documentos principais
- `lowercase.md` - Documentos secundários
- `SPRINT_N.md` - Documentação de sprint
- `SESSION_SUMMARY_vX.Y.Z.md` - Resumo de sessão

**Código:**
- `snake_case.rs` - Arquivos Rust
- `kebab-case/` - Diretórios de crates
- `snake_case.matter` - Programas Matter
- `snake_case.mbc` - Bytecode compilado

**Scripts:**
- `snake_case.ps1` - Scripts PowerShell
- `test_*.ps1` - Scripts de teste
- `install*.ps1` - Scripts de instalação

### Organização de Documentos

**docs/**
- Documentação técnica e especificações
- Guias de uso e integração
- Arquitetura e design

**docs/sprints/**
- Documentação de cada sprint
- Commits e mudanças
- Progresso incremental

**docs/sessions/**
- Resumos de sessões de desenvolvimento
- Relatórios executivos
- Histórico de evolução

**docs/releases/**
- Notas de release
- Estado atual do projeto
- Changelog

---

## 📦 Gestão de Arquivos

### Arquivos Importantes (Não Deletar)

- `README.md` - Entrada principal
- `Cargo.toml` - Configuração do workspace
- `docs/MANIFESTO.md` - Filosofia do projeto
- `docs/SPEC.md` - Especificação da linguagem
- `PROGRESS.md` - Roadmap

### Arquivos Gerados (Podem Deletar)

- `target/` - Build artifacts
- `bytecode/*.mbc` - Bytecode compilado (pode recompilar)
- `.matter_store.json` - Store backend (teste)

### Arquivos de Backup

Manter em `docs/sessions/` para histórico:
- Resumos de sessões antigas
- Relatórios executivos
- Documentação de integrações

---

## 🧹 Limpeza

### Limpar Build Artifacts

```bash
cargo clean
```

### Limpar Bytecode

```bash
rm bytecode/*.mbc
```

### Recompilar Exemplos

```bash
# Recompilar todos os exemplos
for file in examples/*.matter; do
    matter compile "$file" -o "bytecode/$(basename "$file" .matter).mbc"
done
```

---

## 🚀 Workflow de Desenvolvimento

### 1. Criar Nova Feature

```bash
# 1. Criar branch
git checkout -b feature/nome-da-feature

# 2. Desenvolver
# ... código ...

# 3. Testar
cargo test
.\scripts\test_all.ps1

# 4. Documentar
# Criar docs/sprints/SPRINT_N.md

# 5. Commit
git add .
git commit -m "feat: descrição da feature"
```

### 2. Adicionar Exemplo

```bash
# 1. Criar arquivo
# examples/novo_exemplo.matter

# 2. Testar
matter run examples/novo_exemplo.matter

# 3. Compilar
matter compile examples/novo_exemplo.matter -o bytecode/novo_exemplo.mbc

# 4. Documentar
# Adicionar em examples/README.md

# 5. Commit
git add examples/novo_exemplo.matter examples/README.md
git commit -m "docs: add novo_exemplo"
```

### 3. Atualizar Documentação

```bash
# 1. Editar documento
# docs/DOCUMENTO.md

# 2. Atualizar índice
# docs/INDEX.md

# 3. Commit
git add docs/
git commit -m "docs: update documentação"
```

---

## 📈 Evolução do Projeto

### v0.1.x - Protótipo
- Pipeline básico
- Funções e escopo
- Loops e controle de fluxo

### v0.2.x - Sistema Completo
- MBC1 Persistence
- Visual Backend
- Stdlib Expansion
- CLI Improvements
- REPL Interativo
- Showcase Examples

### v0.3.x - Ecossistema (Futuro)
- Error Integration
- Performance Optimization
- Package Manager
- LSP

---

## 🎓 Melhores Práticas

### Documentação
- Sempre documentar novas features
- Manter INDEX.md atualizado
- Criar resumos de sprint
- Documentar decisões de design

### Código
- Seguir convenções Rust
- Adicionar testes para novas features
- Manter cobertura > 80%
- Zero warnings

### Exemplos
- Testar antes de commitar
- Documentar em examples/README.md
- Usar apenas features implementadas
- Manter simples e focado

### Organização
- Arquivos na pasta correta
- Nomenclatura consistente
- Limpar arquivos temporários
- Manter estrutura limpa

---

**Última atualização:** 9 de Maio de 2026  
**Versão:** v0.2.1  
**Status:** ✅ ORGANIZADO
