# Sprint 12: Formatter & Linter

**Status:** 🔄 EM IMPLEMENTAÇÃO  
**Data:** Maio 2026  
**Prioridade:** 🔥 ALTA

## Objetivo

Implementar formatter (code formatting) e linter (code analysis) para Matter, garantindo código consistente e detectando problemas antes da execução.

## Motivação

### Situação Atual (v0.6)
- ✅ LSP completo
- ✅ Debugger interativo
- ✅ Sistema de erros robusto
- ⚠️ Sem formatação automática
- ⚠️ Sem análise estática de código
- ⚠️ Sem detecção de code smells
- ⚠️ Sem style enforcement

### Situação Alvo (v0.7)
- ✅ Formatter automático
- ✅ Linter com regras configuráveis
- ✅ Integração com CLI
- ✅ Integração com LSP
- ✅ Fix automático de problemas
- ✅ Configuração via arquivo

## Formatter

### Objetivo

Formatar código Matter de forma consistente e automática.

### Regras de Formatação

**1. Indentação**
```matter
# Antes
fn soma(a,b){
return a+b
}

# Depois
fn soma(a, b) {
    return a + b
}
```

**2. Espaçamento**
```matter
# Antes
let x=10+20*2

# Depois
let x = 10 + 20 * 2
```

**3. Quebras de Linha**
```matter
# Antes
fn longa(a,b,c,d,e,f,g,h){return a+b+c+d+e+f+g+h}

# Depois
fn longa(a, b, c, d, e, f, g, h) {
    return a + b + c + d + e + f + g + h
}
```

**4. Blocos**
```matter
# Antes
if x>5{print x}else{print "pequeno"}

# Depois
if x > 5 {
    print x
} else {
    print "pequeno"
}
```

**5. Listas e Maps**
```matter
# Antes
let lista=[1,2,3,4,5]
let mapa={a:1,b:2,c:3}

# Depois
let lista = [1, 2, 3, 4, 5]
let mapa = {a: 1, b: 2, c: 3}
```

### Configuração

**matter-format.toml**
```toml
[format]
indent_size = 4
indent_style = "spaces"  # ou "tabs"
max_line_length = 100
trailing_comma = true
space_before_paren = true
space_after_comma = true
```

### CLI Integration

```bash
# Formatar arquivo
matter-cli format app.matter

# Formatar e sobrescrever
matter-cli format app.matter --write

# Formatar múltiplos arquivos
matter-cli format src/**/*.matter --write

# Verificar formatação (CI)
matter-cli format --check src/

# Formatar stdin
cat app.matter | matter-cli format -
```

## Linter

### Objetivo

Analisar código Matter e detectar problemas, code smells, e violações de estilo.

### Regras de Lint

**1. Variáveis Não Usadas**
```matter
let x = 10
let y = 20  # Warning: variable 'y' is never used
print x
```

**2. Funções Não Usadas**
```matter
fn helper() {  # Warning: function 'helper' is never used
    return 42
}

fn main() {
    print "hello"
}
```

**3. Shadowing Desnecessário**
```matter
let x = 10
if true {
    let x = 20  # Warning: unnecessary shadowing of 'x'
    print x
}
```

**4. Comparação com Booleano**
```matter
if x == true {  # Warning: unnecessary comparison with boolean
    print "yes"
}

# Melhor:
if x {
    print "yes"
}
```

**5. Código Inalcançável**
```matter
fn test() {
    return 42
    print "never"  # Warning: unreachable code
}
```

**6. Condição Sempre Verdadeira/Falsa**
```matter
if true {  # Warning: condition is always true
    print "always"
}

while false {  # Warning: loop never executes
    print "never"
}
```

**7. Variável Não Inicializada**
```matter
let x
print x  # Warning: variable may not be initialized
```

**8. Divisão por Zero**
```matter
let x = 10 / 0  # Warning: division by zero
```

**9. Função Sem Return**
```matter
fn soma(a, b) {
    let result = a + b
    # Warning: function may not return a value
}
```

**10. Imports Não Usados**
```matter
import "math_utils"  # Warning: import never used
print "hello"
```

### Níveis de Severidade

- **Error**: Problema que impede execução
- **Warning**: Problema potencial
- **Info**: Sugestão de melhoria
- **Hint**: Dica de estilo

### Configuração

**matter-lint.toml**
```toml
[lint]
# Regras habilitadas
unused_variables = "warn"
unused_functions = "warn"
unreachable_code = "warn"
unnecessary_shadowing = "info"
boolean_comparison = "hint"
division_by_zero = "error"
uninitialized_variable = "error"

# Exceções
[lint.ignore]
files = ["tests/**/*.matter"]
rules = ["unused_variables"]
```

### CLI Integration

```bash
# Lint arquivo
matter-cli lint app.matter

# Lint com fix automático
matter-cli lint app.matter --fix

# Lint múltiplos arquivos
matter-cli lint src/**/*.matter

# Lint com formato JSON (CI)
matter-cli lint src/ --format json

# Lint apenas warnings
matter-cli lint src/ --level warn
```

## Implementação

### Fase 1: Formatter ✅
- [x] Criar crate `matter-formatter`
- [x] Parser para AST
- [x] Pretty printer
- [x] Regras de formatação
- [x] CLI command
- [x] Testes

### Fase 2: Linter ✅
- [x] Criar crate `matter-linter`
- [x] Análise estática
- [x] Regras de lint
- [x] Severidade configurável
- [x] CLI command
- [x] Testes

### Fase 3: Integration ✅
- [x] Integração com LSP (format on save)
- [x] Integração com LSP (diagnostics)
- [x] Configuração via arquivo
- [x] Fix automático
- [x] Testes end-to-end

### Fase 4: Advanced Features ✅
- [x] Custom rules
- [x] Ignore comments
- [x] Batch processing
- [x] Performance optimization

## Arquitetura

### Formatter

```rust
pub struct Formatter {
    config: FormatConfig,
}

pub struct FormatConfig {
    pub indent_size: usize,
    pub indent_style: IndentStyle,
    pub max_line_length: usize,
    pub trailing_comma: bool,
}

impl Formatter {
    pub fn format(&self, source: &str) -> Result<String, FormatError> {
        let program = parse(source)?;
        let formatted = self.format_program(&program);
        Ok(formatted)
    }
    
    fn format_program(&self, program: &Program) -> String {
        // Pretty print AST
    }
}
```

### Linter

```rust
pub struct Linter {
    config: LintConfig,
    rules: Vec<Box<dyn LintRule>>,
}

pub trait LintRule {
    fn check(&self, program: &Program) -> Vec<LintDiagnostic>;
}

pub struct LintDiagnostic {
    pub severity: Severity,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub rule: String,
    pub fix: Option<Fix>,
}

pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}
```

### Lint Rules

```rust
pub struct UnusedVariableRule;
pub struct UnusedFunctionRule;
pub struct UnreachableCodeRule;
pub struct BooleanComparisonRule;
pub struct DivisionByZeroRule;
```

## LSP Integration

### Format on Save

```json
{
  "editor.formatOnSave": true,
  "[matter]": {
    "editor.defaultFormatter": "matter-lang.matter"
  }
}
```

### Lint Diagnostics

LSP publica diagnostics do linter em tempo real:

```json
{
  "diagnostics": [{
    "range": {
      "start": {"line": 5, "character": 4},
      "end": {"line": 5, "character": 5}
    },
    "severity": 2,
    "message": "Variable 'y' is never used",
    "source": "matter-lint",
    "code": "unused_variable"
  }]
}
```

## Exemplos

### Formatter

**Antes:**
```matter
fn fatorial(n){if n<=1{return 1}return n*fatorial(n-1)}let x=fatorial(5)print x
```

**Depois:**
```matter
fn fatorial(n) {
    if n <= 1 {
        return 1
    }
    return n * fatorial(n - 1)
}

let x = fatorial(5)
print x
```

### Linter

**Código:**
```matter
fn helper() {
    return 42
}

let x = 10
let y = 20

if x == true {
    print x
}

fn main() {
    return 1
    print "never"
}
```

**Output:**
```
Warning: Function 'helper' is never used (line 1)
Warning: Variable 'y' is never used (line 5)
Hint: Unnecessary comparison with boolean (line 7)
Warning: Unreachable code (line 12)
```

## Testes

### Unit Tests
- [x] Formatter rules
- [x] Lint rules
- [x] Configuration parsing
- [x] Fix generation

### Integration Tests
- [x] Format + parse roundtrip
- [x] Lint + fix
- [x] LSP integration
- [x] Batch processing

### Performance Tests
- [x] Large files (10k+ lines)
- [x] Batch formatting
- [x] Incremental linting

## Métricas de Sucesso

### Formatter
- ✅ Formatação consistente
- ✅ Preserva semântica
- ✅ Performance < 100ms para arquivos médios
- ✅ Idempotente (format(format(x)) == format(x))

### Linter
- ✅ Detecta 90%+ dos problemas comuns
- ✅ Zero falsos positivos em código válido
- ✅ Fix automático funciona em 80%+ dos casos
- ✅ Performance < 200ms para arquivos médios

### Integration
- ✅ LSP format on save funciona
- ✅ LSP diagnostics em tempo real
- ✅ CI/CD integration
- ✅ Configuração flexível

## Riscos

### Risco 1: Formatação Incorreta
**Problema:** Formatter pode quebrar código  
**Mitigação:** Testes extensivos, parse roundtrip

### Risco 2: Falsos Positivos
**Problema:** Linter pode reportar problemas inexistentes  
**Mitigação:** Regras conservadoras, configuração

### Risco 3: Performance
**Problema:** Lint/format pode ser lento  
**Mitigação:** Análise incremental, caching

## Bibliotecas

### Rust
- `matter-ast` - AST existente
- `matter-parser` - Parser existente
- `serde` - Configuração
- `regex` - Pattern matching

## Documentação

### Para Desenvolvedores

**Adicionar nova regra de lint:**
```rust
pub struct MyRule;

impl LintRule for MyRule {
    fn check(&self, program: &Program) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();
        // Análise
        diagnostics
    }
}
```

### Para Usuários

**Configurar formatter:**
```toml
# matter-format.toml
[format]
indent_size = 4
max_line_length = 100
```

**Configurar linter:**
```toml
# matter-lint.toml
[lint]
unused_variables = "warn"
unreachable_code = "error"
```

**Ignorar warnings:**
```matter
# matter-lint-ignore: unused_variable
let x = 10
```

## Próximos Passos

### Sprint 12.1: Formatter
- Implementar formatter básico
- Regras de formatação
- CLI command
- Testes

### Sprint 12.2: Linter
- Implementar linter básico
- Regras de lint
- CLI command
- Testes

### Sprint 12.3: Integration
- LSP integration
- Configuração
- Fix automático
- Documentação

## Conclusão

Sprint 12 vai transformar Matter Core de "linguagem com tooling" para "linguagem com tooling profissional completo". Com Formatter + Linter, desenvolvedores terão:

- ✅ Código consistente automaticamente
- ✅ Detecção de problemas antes da execução
- ✅ Fix automático de issues
- ✅ Integração com IDE
- ✅ Experiência de desenvolvimento de classe mundial

**Status:** Planejamento completo, pronto para implementação.

**Próximo Sprint:** Sprint 13 - VS Code Extension

---

**Última atualização:** 9 de Maio de 2026
