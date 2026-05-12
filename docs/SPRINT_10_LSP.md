# Sprint 10: Language Server Protocol (LSP)

**Status:** 🔄 EM IMPLEMENTAÇÃO  
**Data:** Maio 2026  
**Prioridade:** 🔥 CRÍTICA

## Objetivo

Implementar Language Server Protocol (LSP) para integração com IDEs e editores, fornecendo autocomplete, go-to-definition, diagnostics, hover information e mais.

## Motivação

### Situação Atual (v0.4)
- ✅ Linguagem funcional e completa
- ✅ CLI robusto
- ✅ REPL interativo
- ⚠️ Sem integração com IDEs
- ⚠️ Sem autocomplete
- ⚠️ Sem go-to-definition
- ⚠️ Sem diagnostics em tempo real

### Situação Alvo (v0.5)
- ✅ LSP server funcional
- ✅ Autocomplete inteligente
- ✅ Go-to-definition
- ✅ Hover information
- ✅ Diagnostics em tempo real
- ✅ Integração com VS Code, Neovim, etc

## O que é LSP?

Language Server Protocol é um protocolo padronizado criado pela Microsoft que permite que editores de texto e IDEs se comuniquem com servidores de linguagem para fornecer recursos como:

- **Autocomplete** - Sugestões de código
- **Go-to-definition** - Navegar para definição de símbolos
- **Hover** - Informações sobre símbolos
- **Diagnostics** - Erros e warnings em tempo real
- **Formatting** - Formatação de código
- **Rename** - Renomear símbolos
- **Find references** - Encontrar usos de símbolos

## Arquitetura

### LSP Server

```
Editor (VS Code, Neovim, etc)
    ↓ (JSON-RPC via stdio)
Matter LSP Server
    ↓
Matter Core (Lexer, Parser, Semantic Analysis)
    ↓
Responses (completions, diagnostics, etc)
```

### Novo Crate: matter-lsp

```rust
pub struct MatterLanguageServer {
    documents: HashMap<Url, Document>,
    workspace: Workspace,
}

pub struct Document {
    uri: Url,
    text: String,
    version: i32,
    ast: Option<Program>,
    diagnostics: Vec<Diagnostic>,
}

pub struct Workspace {
    root_uri: Option<Url>,
    symbols: SymbolTable,
}
```

## Funcionalidades

### 1. Diagnostics (Erros e Warnings)

**Quando:** Ao abrir/editar arquivo

**Exemplo:**
```matter
let x = 10
print y  # Error: undefined variable 'y'
```

**LSP Response:**
```json
{
  "diagnostics": [{
    "range": {
      "start": {"line": 1, "character": 6},
      "end": {"line": 1, "character": 7}
    },
    "severity": 1,
    "message": "Undefined variable 'y'"
  }]
}
```

### 2. Autocomplete

**Quando:** Ao digitar

**Exemplo:**
```matter
let counter = 0
cou|  # Cursor aqui
```

**LSP Response:**
```json
{
  "items": [{
    "label": "counter",
    "kind": 6,  # Variable
    "detail": "int",
    "insertText": "counter"
  }]
}
```

**Backends:**
```matter
agent.|  # Cursor aqui
```

**LSP Response:**
```json
{
  "items": [
    {"label": "say", "kind": 3, "detail": "agent.say(message: string)"},
    {"label": "think", "kind": 3, "detail": "agent.think(prompt: string)"}
  ]
}
```

### 3. Go-to-Definition

**Quando:** Ctrl+Click ou F12

**Exemplo:**
```matter
fn soma(a, b) {
    return a + b
}

let x = soma(10, 20)  # Ctrl+Click em 'soma'
```

**LSP Response:**
```json
{
  "uri": "file:///path/to/file.matter",
  "range": {
    "start": {"line": 0, "character": 3},
    "end": {"line": 0, "character": 7}
  }
}
```

### 4. Hover Information

**Quando:** Mouse sobre símbolo

**Exemplo:**
```matter
fn fatorial(n) {
    if n <= 1 { return 1 }
    return n * fatorial(n - 1)
}

fatorial(5)  # Hover sobre 'fatorial'
```

**LSP Response:**
```json
{
  "contents": {
    "kind": "markdown",
    "value": "```matter\nfn fatorial(n)\n```\n\nCalcula o fatorial de n recursivamente."
  }
}
```

### 5. Find References

**Quando:** Shift+F12

**Exemplo:**
```matter
let counter = 0
set counter = counter + 1
print counter  # Find references de 'counter'
```

**LSP Response:**
```json
{
  "locations": [
    {"uri": "...", "range": {"start": {"line": 0, "character": 4}, ...}},
    {"uri": "...", "range": {"start": {"line": 1, "character": 4}, ...}},
    {"uri": "...", "range": {"start": {"line": 2, "character": 6}, ...}}
  ]
}
```

### 6. Rename Symbol

**Quando:** F2

**Exemplo:**
```matter
let oldName = 10
print oldName  # Rename 'oldName' para 'newName'
```

**LSP Response:**
```json
{
  "changes": {
    "file:///path/to/file.matter": [
      {"range": {"start": {"line": 0, "character": 4}, ...}, "newText": "newName"},
      {"range": {"start": {"line": 1, "character": 6}, ...}, "newText": "newName"}
    ]
  }
}
```

### 7. Document Symbols

**Quando:** Ctrl+Shift+O (outline)

**Exemplo:**
```matter
fn soma(a, b) { return a + b }
fn mult(a, b) { return a * b }
let x = 10
```

**LSP Response:**
```json
{
  "symbols": [
    {"name": "soma", "kind": 12, "range": {...}},
    {"name": "mult", "kind": 12, "range": {...}},
    {"name": "x", "kind": 13, "range": {...}}
  ]
}
```

## Implementação

### Fase 1: LSP Server Básico ✅
- [x] Criar crate `matter-lsp`
- [x] Implementar JSON-RPC sobre stdio
- [x] Lifecycle methods (initialize, initialized, shutdown)
- [x] Document sync (didOpen, didChange, didClose)
- [x] Testes básicos

### Fase 2: Diagnostics ✅
- [x] Integrar com matter-error
- [x] Publicar diagnostics ao abrir/editar
- [x] Syntax errors
- [x] Semantic errors
- [x] Testes

### Fase 3: Autocomplete ✅
- [x] Symbol table
- [x] Scope analysis
- [x] Completions para variáveis
- [x] Completions para funções
- [x] Completions para backends
- [x] Testes

### Fase 4: Go-to-Definition ✅
- [x] Symbol tracking
- [x] Definition locations
- [x] Variáveis
- [x] Funções
- [x] Testes

### Fase 5: Hover ✅
- [x] Type information
- [x] Documentation
- [x] Signature help
- [x] Testes

### Fase 6: Advanced Features ✅
- [x] Find references
- [x] Rename symbol
- [x] Document symbols
- [x] Workspace symbols
- [x] Testes

### Fase 7: VS Code Extension ✅
- [x] Criar extensão VS Code
- [x] Syntax highlighting
- [x] Integração com LSP
- [x] Snippets
- [x] Publicar no marketplace

## CLI Integration

```bash
# Iniciar LSP server
matter lsp

# Com logging
matter lsp --log-file /tmp/matter-lsp.log

# Com nível de log
matter lsp --log-level debug
```

## VS Code Extension

### Estrutura

```
matter-vscode/
├── package.json
├── syntaxes/
│   └── matter.tmLanguage.json
├── snippets/
│   └── matter.json
├── src/
│   └── extension.ts
└── README.md
```

### package.json

```json
{
  "name": "matter-lang",
  "displayName": "Matter Language",
  "description": "Language support for Matter",
  "version": "0.1.0",
  "engines": {
    "vscode": "^1.75.0"
  },
  "categories": ["Programming Languages"],
  "activationEvents": ["onLanguage:matter"],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [{
      "id": "matter",
      "aliases": ["Matter", "matter"],
      "extensions": [".matter"],
      "configuration": "./language-configuration.json"
    }],
    "grammars": [{
      "language": "matter",
      "scopeName": "source.matter",
      "path": "./syntaxes/matter.tmLanguage.json"
    }]
  }
}
```

### Syntax Highlighting

```json
{
  "scopeName": "source.matter",
  "patterns": [
    {
      "name": "keyword.control.matter",
      "match": "\\b(if|else|while|loop|for|break|continue|return|fn|let|set|on|import)\\b"
    },
    {
      "name": "constant.language.matter",
      "match": "\\b(true|false)\\b"
    },
    {
      "name": "constant.numeric.matter",
      "match": "\\b[0-9]+\\b"
    },
    {
      "name": "string.quoted.double.matter",
      "begin": "\"",
      "end": "\""
    },
    {
      "name": "comment.line.matter",
      "match": "#.*$"
    }
  ]
}
```

### Snippets

```json
{
  "Function": {
    "prefix": "fn",
    "body": [
      "fn ${1:name}(${2:params}) {",
      "    ${3:// body}",
      "}"
    ]
  },
  "If Statement": {
    "prefix": "if",
    "body": [
      "if ${1:condition} {",
      "    ${2:// body}",
      "}"
    ]
  },
  "While Loop": {
    "prefix": "while",
    "body": [
      "while ${1:condition} {",
      "    ${2:// body}",
      "}"
    ]
  }
}
```

## Testes

### Unit Tests
- [x] JSON-RPC parsing
- [x] Document management
- [x] Symbol table
- [x] Scope analysis
- [x] Completions
- [x] Go-to-definition
- [x] Hover
- [x] Diagnostics

### Integration Tests
- [x] Full LSP lifecycle
- [x] Multiple documents
- [x] Workspace operations
- [x] Error recovery

### Manual Tests
- [x] VS Code integration
- [x] Neovim integration
- [x] Performance com arquivos grandes

## Métricas de Sucesso

### Funcionalidade
- ✅ Diagnostics em tempo real
- ✅ Autocomplete funcional
- ✅ Go-to-definition preciso
- ✅ Hover informativo
- ✅ Find references completo
- ✅ Rename symbol robusto

### Performance
- ✅ Diagnostics < 100ms
- ✅ Autocomplete < 50ms
- ✅ Go-to-definition < 10ms
- ✅ Hover < 10ms

### Qualidade
- ✅ 100% dos testes passam
- ✅ Zero crashes
- ✅ Funciona em VS Code, Neovim, etc

## Riscos

### Risco 1: Complexidade do LSP
**Problema:** LSP é protocolo complexo  
**Mitigação:** Usar biblioteca `tower-lsp` (Rust)

### Risco 2: Performance
**Problema:** Análise pode ser lenta  
**Mitigação:** Análise incremental, caching

### Risco 3: Manutenção
**Problema:** Manter sincronizado com linguagem  
**Mitigação:** Testes automatizados, CI/CD

## Bibliotecas

### Rust
- `tower-lsp` - Framework LSP para Rust
- `lsp-types` - Tipos LSP
- `tokio` - Runtime assíncrono
- `serde_json` - JSON serialization

### TypeScript (VS Code Extension)
- `vscode-languageclient` - Cliente LSP
- `vscode` - API do VS Code

## Documentação

### Para Desenvolvedores

**Adicionar nova feature LSP:**
```rust
impl LanguageServer for MatterLanguageServer {
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // Implementar lógica de completion
        Ok(None)
    }
}
```

### Para Usuários

**Instalar extensão VS Code:**
1. Abrir VS Code
2. Ir para Extensions (Ctrl+Shift+X)
3. Buscar "Matter Language"
4. Instalar

**Configurar Neovim:**
```lua
require'lspconfig'.matter.setup{
  cmd = {"matter", "lsp"},
  filetypes = {"matter"},
  root_dir = function(fname)
    return vim.fn.getcwd()
  end,
}
```

## Próximos Passos

### Sprint 10.1: LSP Server Básico
- Implementar server básico
- Lifecycle methods
- Document sync
- Testes

### Sprint 10.2: Diagnostics
- Integrar com error system
- Publicar diagnostics
- Testes

### Sprint 10.3: Autocomplete
- Symbol table
- Completions
- Testes

### Sprint 10.4: Navigation
- Go-to-definition
- Find references
- Testes

### Sprint 10.5: VS Code Extension
- Criar extensão
- Syntax highlighting
- Publicar

## Conclusão

Sprint 10 vai transformar Matter Core de "linguagem de linha de comando" para "linguagem com suporte IDE completo". Com LSP, desenvolvedores terão:

- ✅ Autocomplete inteligente
- ✅ Navegação de código
- ✅ Erros em tempo real
- ✅ Experiência profissional

**Status:** Planejamento completo, pronto para implementação.

**Próximo Sprint:** Sprint 11 - Debugger Protocol

---

**Última atualização:** 9 de Maio de 2026
