# PRODUCTION_READINESS_AUDIT_V1

**Projeto:** Matter Core (linguagem + runtime + CLI)  
**Data da auditoria:** 2026-07-14  
**Escopo:** diagnóstico e evidências apenas — sem reescrita de arquitetura, sem alteração de comportamento público, sem novos exemplos/demos.  
**Veredito:** **NÃO production-ready** (alinha-se ao próprio `core-status-json`: `production_ready: false`, `claim: experimental_language_runtime`).

---

## 0. Evidências geradas (artefatos)

Todos sob:

`target/validation/audit/`

| Artefato | Conteúdo |
|---|---|
| `suite-summary.json` | Resultado agregado da suíte de scripts + cargo test |
| `suite-run.log` | Log textual dos passos |
| `cargo-tests.json` | Exit codes por crate testado |
| `crates-inventory.json` | Inventário de ~98 crates em `crates/` |
| `hazard-macros-summary.json` | Contagens panic/unwrap/expect/todo/unsafe |
| `hits-*.json` | Amostras de ocorrências por padrão |
| `cat-*.json` / `io-surface-summary.json` | Superfícies process/fs/rede/bridges |
| `absolute-paths.json` | Referências absolutas C:/D:/F:/matter_target |
| `malformed-input-probes.json` | Probes de entrada malformada no CLI release |
| `malformed/*.matter` | Fixtures usadas nos probes |
| `package-standalone.json` | Smoke do pacote Windows com PATH mínimo |

---

## 1. Suíte atual executada e registrada

### 1.1 Scripts / CLI (release `x86_64-pc-windows-gnu`)

| Passo | Exit | OK | Tempo (s) |
|---|---:|:---:|---:|
| `scripts/test-real-apps.ps1` | 0 | sim | ~4.7 |
| `scripts/test-runnable-examples.ps1` | 0 | sim | ~3.1 |
| `scripts/test-core-status-contract.ps1` | 0 | sim | ~0.5 |
| `scripts/test-status-triad-contract.ps1` | 0 | sim | ~0.8 |
| `core-status-json` | 0 | sim | ~0.1 |
| `world-status-json` | 0 | sim | ~0.1 |
| `frontier-status-json` | 0 | sim | ~0.1 |
| `polyglot-status-json` | 0 | sim | ~0.2 |
| `capabilities-json` | 0 | sim | ~0.1 |
| `scripts/verify-release-package.ps1` | 0 | sim | ~1.9 |

**Resumo scripts:** `steps_pass=10`, `steps_fail=0` (`suite-summary.json`).

### 1.2 `cargo test` (target `x86_64-pc-windows-gnu`)

Crates exercitados (todos exit 0 nesta corrida):

`matter-lexer`, `matter-parser`, `matter-ast`, `matter-bytecode`, `matter-compiler`, `matter-vm`, `matter-backend`, `matter-stdlib`, `matter-kernel-vm`, `matter-error`, `matter-optimizer`, `matter-bridge-python`, `matter-bridge-nodejs`, `matter-bridge-rust`, `matter-polyglot`, `matter-lsp`, `matter-runtime`

**Resumo cargo:** `cargo_pass=17`, `cargo_fail=0`.

### 1.3 Limitações da suíte atual (evidência de gap de cobertura)

- Muitos crates retornam **0 unit tests** (apenas doc-tests vazios) — “exit 0” ≠ cobertura de segurança.
- Não há bateria formal de fuzzing do parser/VM/MBC1 no CI observado.
- `verify-release-package` valida layout/artefatos no host de build; **não** prova execução em máquina sem Python/DLL extras (ver §7).
- Workspace tem **dezenas** de crates de domínio (física, bridges nativos, etc.) **fora** desta rodada de testes.

---

## 2. Crates e responsabilidades

Inventário automatizado: **98** entradas em `crates/` (`crates-inventory.json`). O workspace `Cargo.toml` também lista `emnr/*` e muitos backends de domínio.

### 2.1 Núcleo da linguagem (pipeline)

| Crate | Responsabilidade |
|---|---|
| `matter-lexer` | Tokenização |
| `matter-parser` | Parsing → AST |
| `matter-ast` | Tipos de AST |
| `matter-bytecode` | IR MBC1, serialize/deserialize |
| `matter-compiler` / builder em bytecode | Compilação AST → bytecode |
| `matter-optimizer` | Otimizações de bytecode |
| `matter-vm` | Execução de bytecode |
| `matter-kernel-vm` | Variante/kernel VM testável |
| `matter-backend` | Valores/`Backend` trait, runtime values |
| `matter-runtime` | Registro de backends, orquestração VM+stdlib+polyglot |
| `matter-stdlib` | Bibliotecas built-in (math, list, file, json, …) |
| `matter-error` | Tipos de erro |
| `matter-cli` | Interface pública CLI (binário monólito) |
| `matter-lsp` | Language Server Protocol |
| `matter-package` / `matter-package-resolver` | Pacotes/projeto |

### 2.2 Bridges / polyglot

| Crate | Responsabilidade |
|---|---|
| `matter-polyglot` | Resolução de runtimes, instalação de deps (pip/npm/go/mvn) |
| `matter-bridge-python` | Bridge PyO3 / Python |
| `matter-bridge-nodejs` | Bridge Node via processo `node` |
| `matter-bridge-nodejs-native` | Addon N-API |
| `matter-bridge-go` / `matter-bridge-go-native` | Go / cgo |
| `matter-bridge-java` / `matter-bridge-java-native` | Java / JNI |
| `matter-bridge-rust` | FFI Rust dinâmico + `cargo`/`rustc` em alguns caminhos |

### 2.3 Nativo / memória / tooling

| Crate | Responsabilidade |
|---|---|
| `matter-native` | Codegen nativo multi-arch, SIMD, LTO |
| `matter-jit` | JIT / cache executável |
| `matter-llvm` | Backend LLVM |
| `matter-memory` | Pool / RC / unsafe memory |
| `matter-visual` | UI visual / processo CLI auxiliar |
| `matter-debugger` / `matter-formatter` / `matter-linter` | Tooling dev |

### 2.4 Domínio “frontier” / ciência (ampla superfície)

Dezenas de crates (`matter-quantum`, `matter-photonic`, `matter-climate`, `matter-neuromorphic`, …) registráveis como backends. Status declarado no CLI: frontiers **simulados**, sem hardware (`frontier-status-json`: `all_simulated: true`, `any_hardware: false`).

---

## 3. `panic!` / `unwrap` / `expect` / `todo!` / `unimplemented!` / `unsafe`

Varredura em `crates/` + `emnr/` (somente `.rs`):

| Padrão | Total (aprox.) | Observação |
|---|---:|---|
| `panic!` | **75** | Concentrados em backends/visual/stdlib; alguns em parser/VM (tests + produção) |
| `.unwrap()` | **848** | Pico em `matter-cli/src/main.rs` (~75), package, native, bridges |
| `.expect()` | **384** | Inclui **falso positivo parcial**: `Parser::expect(Token::…)` (método) vs `expect!` |
| `todo!` | **0** | — |
| `unimplemented!` | **0** | — |
| `unsafe { … }` | **106** | Pico em `matter-native`, `matter-memory`, VM/FFI |
| `unsafe fn` | **2** | — |
| `unsafe impl` | **4** | — |

### 3.1 Exemplos concretos (produção, não só teste)

| Local | Tipo | Nota |
|---|---|---|
| `crates/matter-vm/src/lib.rs` ~1027 | `.unwrap()` | `scope_stack.last_mut().unwrap()` no caminho `Value::Closure` |
| `crates/matter-cli/src/main.rs` | muitos `.unwrap()` | Monólito CLI; falhas viram panic se não tratadas |
| `crates/matter-cli/src/main.rs` ~24671 | `Command::new` + exit | Compila/executa nativo temporário |
| `crates/matter-cli/src/main.rs` ~21958 | `Command::new("powershell")` | Execução de comando arbitrário (agent/local capture) |
| `crates/matter-memory`, `matter-native` | `unsafe` | Superfície de memory safety se mal usada |

**Impacto:** em release, `unwrap`/`panic` = **crash do processo** em entrada/estado inesperado — inaceitável para “production language runtime” sem política clara de falha controlada.

---

## 4. Processos, filesystem, rede, bridges

Contagens de ocorrências (padrão greppável; podem incluir testes):

| Superfície | Hits (aprox.) | Onde (principais) |
|---|---:|---|
| `Command::new` / `std::process` | ~96 | CLI, polyglot resolver, bridges Node/Java/Go/Rust, visual, llvm |
| FS read | ~133 | CLI, package, stdlib file, scripts embutidos |
| FS write | ~151 | CLI, package, temp files, memory agent |
| FS remove | ~153 | cleanup temp/native |
| Rede (`reqwest`/sockets/etc.) | ~10 | CLI agent/HTTP (`curl`/APIs) |
| Python/pyo3 | ~38 | `matter-bridge-python` |
| Node/napi/js | ~149 | bridges + strings |
| `libloading`/DLL | ~6 | FFI dinâmico |
| `env::var` | ~13 | configuração |

### 4.1 Execução de processos (amostra)

| Arquivo | Função / contexto | Processo |
|---|---|---|
| `matter-cli/src/main.rs` | `run_local_command_capture` | `powershell -Command <user/agent string>` |
| `matter-cli/src/main.rs` | HTTP helpers | `curl`, `powershell` |
| `matter-cli/src/main.rs` | native run path | executa `.matter_temp_native.exe` |
| `matter-polyglot/src/resolver.rs` | install/show packages | `pip`, `npm`, `go`, `mvn` |
| `matter-bridge-nodejs` | runtime calls | `node` |
| `matter-bridge-java` | compile/run | `javac`, `java` |
| `matter-bridge-rust` | build/load | `cargo`, `rustc` |
| `matter-visual` | agent fallback | `where`, CLI path |

### 4.2 Bridges Python / Node (estado observado)

`polyglot-status-json` (host de auditoria):

- **python:** `ready: true`, mode `real`
- **node:** `ready: true`, mode `real`
- **rust:** `ready: true`
- **go / java:** `ready: false` (binários ausentes)

**Risco de produto:** o binário de release **linka `python3.dll`** (ver §7) — Python deixa de ser opcional no loader do Windows.

---

## 5. Parser, MBC1 e VM vs entradas malformadas

Probes no CLI **release** (`malformed-input-probes.json`). Nenhum caso reportou string `panicked at` / stack overflow no stdout/stderr capturado.

| Caso | `run` exit | `check-json` exit | Comportamento observado |
|---|---:|---:|---|
| empty | 0 | 0 | Aceito como programa vazio (`ok:true`) |
| garbage `@@@###$$$` | 0 | 0 | **Aceito** (`ok:true`, 1 instruction) — sem erro de parse |
| unclosed brace/paren | 1 | 1 | Erro de parse estruturado |
| `let $x = 1` (bad token) | 0 | 0 | **Aceito** (possível engolir/`$` não rejeitado como esperado) |
| `1/0` | 1 | 0 | Runtime “division by zero”; **check não pega** |
| undefined var / unknown fn | 1 | 1 | Erro semântico |
| deep nest 200 parens | 0 | 0 | Executa `1` (OK funcional; sem limite de profundidade explícito) |
| string 100k chars | 0 | 0 | Aceito; imprime string enorme (sem limite de tamanho) |
| null bytes no fonte | 0 | 0 | Executa e imprime `1` e `2` |
| only comment | 0 | 0 | OK vazio |
| random `.mbc` via `run` | 1 | — | Falha leitura UTF-8 (tratado como fonte, não como MBC loader dedicado) |

### 5.1 MBC1 deserialize

`matter-bytecode/src/deserialize.rs` valida magic `MBC1` e major version; retorna `io::Error` em magic inválido.  
**Gap:** o caminho CLI `run <file>` parece tratar input como **texto fonte**, não como loader MBC robusto — bytecode aleatório vira erro de encoding, não “invalid bytecode section”.

### 5.2 VM

- Há `VmError` e caminhos `?` (bom).
- Ainda há `.unwrap()` em caminhos de execução (ex.: closure captures).
- Divisão por zero → erro de runtime (bom), mas não bloqueada no `check-json`.

---

## 6. Referências absolutas C: / D: / F:

Varredura em `crates`, `scripts`, `docs`, `.cargo`, `vscode-extension`, `examples` (sem `target/`):

| Classe | Hits (aprox.) |
|---|---:|
| F: / `matter_target` | **30** |
| D: (Users/mingw/Matter) | **8** |
| C: Users absolutos (neste filtro) | **0** (outros paths C: podem existir fora do padrão) |

### Exemplos F: ainda presentes (scripts)

- `scripts/run-performance-baseline.ps1` — paths `F:\Users\almir\Desktop\matter_target\...`
- `scripts/README.md` — vários exemplos `-CliPath F:\...`

### D: em tooling de desenvolvimento

- `.cargo/config.toml` — linker/CC em `D:/mingw64/...` (**build machine**, não runtime Matter)
- `vscode-extension` / settings — path absoluto do `matter-cli` release no Desktop D:
- `scripts/install-local.ps1` / `build-matter-cli.ps1` — preferência D:

**Impacto:** scripts de performance/docs ainda amarrados a layout antigo no F:; builds documentados dependem de MinGW no D:.

---

## 7. Pacote Windows sem Rust / Cargo / GCC

### 7.1 Conteúdo

- `dist/matter-core-windows-x64/` e `.zip` (~15.6 MB zip; `matter-cli.exe` ~48 MB) existem.
- `verify-release-package.ps1` passou no host de desenvolvimento.

### 7.2 Dependências nativas do `matter-cli.exe` (objdump)

Inclui, entre outras:

- **`python3.dll`** ← crítica
- `opengl32.dll`, `mf.dll` / Media Foundation, `uiautomationcore.dll`, `gdi32`, `user32`, `ws2_32`, CRTs `api-ms-win-crt-*`, etc.

### 7.3 Smoke com PATH mínimo (`C:\Windows\System32;C:\Windows` apenas)

Todos os comandos falharam com exit **`-1073741515`** (`0xC0000135` — **STATUS_DLL_NOT_FOUND**):

| Comando | Exit | OK |
|---|---:|:---:|
| `--help` | -1073741515 | não |
| `core-status-json` | -1073741515 | não |
| `capabilities-json` | -1073741515 | não |
| `run examples/first_run.matter` | -1073741515 | não |

**Conclusão:** o pacote **não** demonstra, nesta auditoria, execução em ambiente “só Windows” sem as DLLs não-sistema (notavelmente **Python3** e possivelmente stack gráfico/MF).  
**Não** falhou por falta de `cargo`/`rustc`/`gcc` no PATH — falhou **antes**, no loader do PE.

No host de desenvolvimento (PATH completo com Python em `C:\Program Files\Python312\`), o mesmo binário roda.

---

## 8–9. Riscos classificados (com arquivo, impacto, repro, teste desejado)

### CRÍTICO

#### R-C1 — Binário de release não carrega sem `python3.dll` (e outras DLLs não-sistema)
- **Arquivo / origem:** link do `matter-cli` com `matter-bridge-python` / PyO3 (dependents: `python3.dll` em `package-standalone.json`)
- **Função:** load-time do PE (antes de `main`)
- **Impacto:** instalação “copiar pasta e rodar” quebra em máquinas sem Python; falso sentido de standalone
- **Repro:** `PATH=C:\Windows\System32;C:\Windows` → executar `dist\...\matter-cli.exe --help` → exit `-1073741515`
- **Teste que deveria proteger:** CI “clean VM” / job sem Python no PATH; assert de dependents (lista branca de DLLs do sistema); build feature-flag de Python opcional sem link hard

#### R-C2 — Execução arbitrária de shell via PowerShell no CLI
- **Arquivo:** `crates/matter-cli/src/main.rs`
- **Função:** `run_local_command_capture` (~21957) — `Command::new("powershell").args(["-NoProfile","-Command", command])`
- **Impacto:** se exposto a input de agente/usuário/rede, **RCE local**
- **Repro:** acionar caminho de agent/local command com payload PowerShell (depende de subcomando agent habilitado e chaves)
- **Teste:** suite de segurança negando metacaracteres; allowlist de comandos; default deny; testes de integração que garantem que strings arbitrárias não chegam ao shell

#### R-C3 — Superfície agent/HTTP + side effects sem sandbox
- **Arquivo:** `crates/matter-cli/src/main.rs` (curl/powershell helpers ~23787+, memória em disco, etc.)
- **Impacto:** exfiltração/rede + FS + processo no mesmo binário “linguagem”
- **Repro:** subcomandos agent/live com API key e tools habilitadas
- **Teste:** modo “language-only” sem agent compilado; contract tests de capabilities sem network/process

### ALTO

#### R-A1 — `.unwrap()` em caminho quente da VM (closures)
- **Arquivo:** `crates/matter-vm/src/lib.rs` ~1021–1028
- **Função:** dispatch `Instruction::Call` / `Value::Closure`
- **Impacto:** panic se `scope_stack` vazio (estado inconsistente / bug de push_scope)
- **Repro:** forçar Call em Closure com stack de escopos corrompida (teste unitário interno) ou fuzz de bytecode
- **Teste:** property test / fuzz MBC; substituir unwrap por `VmError`; regression com closures + returns aninhados

#### R-A2 — Volume de `unwrap`/`expect` em `matter-cli` e bridges
- **Arquivo:** principalmente `crates/matter-cli/src/main.rs` (dezenas), bridges (`converter.rs`, java/go/rust)
- **Impacto:** crash em I/O, JSON, paths, índices
- **Repro:** entradas parciais em comandos `*-json`, arquivos inexistentes, JSON truncado
- **Teste:** golden tests de erro (`exit != 0`, JSON `ok:false`, sem panic); proibir unwrap em CI com `unwrap_used` lint no bin

#### R-A3 — Entrada lixo aceita como programa válido
- **Arquivo:** pipeline lexer/parser (`matter-lexer`, `matter-parser`) + CLI `run`/`check-json`
- **Evidência:** `garbage` e `bad_token` → `run_exit=0`, `check ok:true`
- **Impacto:** silêncio em arquivos corrompidos; dificulta tooling/CI de qualidade
- **Repro:** `matter-cli check-json` em arquivo `@@@###$$$` ou `let $x = 1`
- **Teste:** corpus “must reject”; assert `ok:false` + código de erro estável

#### R-A4 — Sem limites de recursos (string enorme / nesting)
- **Evidência:** string 100k e nesting 200 aceitos e executados
- **Impacto:** DoS local (memória/CPU) com fonte hostil
- **Repro:** `huge_string.matter` / `deep_nest.matter` nos fixtures de audit
- **Teste:** limites configuráveis + testes de rejeição acima do threshold

#### R-A5 — Polyglot resolver executa package managers
- **Arquivo:** `crates/matter-polyglot/src/resolver.rs`
- **Função:** `Command::new("pip"|"npm"|"go"|"mvn")`
- **Impacto:** side effects de rede/instalação se exposto
- **Repro:** fluxos de resolve/install de dependência polyglot
- **Teste:** mode offline; mock de Command; deny-by-default em release language-only

#### R-A6 — Scripts de performance ainda apontam para F:
- **Arquivo:** `scripts/run-performance-baseline.ps1` (e docs em `scripts/README.md`)
- **Impacto:** gates de performance falham ou usam binários errados em máquinas sem F:
- **Repro:** rodar script sem `F:\Users\almir\Desktop\matter_target\...`
- **Teste:** scripts resolvem CLI via argumento/env/`target/x86_64-pc-windows-gnu/release`

### MÉDIO

#### R-M1 — `check-json` não modela erros de runtime
- **Evidência:** `div_zero` → check `ok:true`, run exit 1
- **Impacto:** CI que só faz check deixa passar programas que sempre crasham em run
- **Teste:** estágio opcional `check --include-runtime-smoke` ou documentar limitação + testes de contrato

#### R-M2 — Loader MBC1 não é o caminho principal de `run`
- **Arquivo:** deserialize existe; CLI `run` em `.mbc` aleatório → erro UTF-8
- **Impacto:** artefatos bytecode não têm UX/segurança de loader dedicado
- **Teste:** `matter run-bytecode` / magic detect; corpus de MBC inválidos com erros tipados

#### R-M3 — Hardcoded D: em install/LSP/build scripts
- **Arquivos:** `.cargo/config.toml`, `scripts/build-matter-cli.ps1`, `scripts/install-local.ps1`, extension settings
- **Impacto:** portabilidade entre máquinas
- **Teste:** paths relativos + discovery; CI multi-drive

#### R-M4 — `unsafe` concentrado em native/memory
- **Arquivos:** `matter-native/**`, `matter-memory/**`
- **Impacto:** UB se invariants quebrarem
- **Teste:** miri (onde possível), fuzz de alloc/free, feature-gate native off por default

#### R-M5 — Dependências gráficas/MF no binário da linguagem
- **Evidência:** dependents `opengl32`, `mf*.dll`, etc. (visual/camera stack puxado para o CLI)
- **Impacto:** superfície e deps desnecessárias para “só linguagem”
- **Teste:** build `default-features` language-only sem visual/media

#### R-M6 — Cobertura de testes unitários baixa em vários crates
- **Evidência:** vários `cargo test` com 0 testes de unidade
- **Impacto:** regressões silenciosas
- **Teste:** metas de cobertura no núcleo (parser/vm/bytecode) + mutation testing seletivo

### BAIXO

#### R-B1 — `panic!` em backends de domínio / visual (muitos em asserts de desenvolvimento)
- **Impacto:** crash se backend experimental for exercitado
- **Mitigação:** não registrar backends experimentais em builds production-default

#### R-B2 — Docs históricas com claims e paths F:
- **Impacto:** onboarding confuso; não quebra runtime se não usado
- **Teste:** link checker + grep CI proibindo `F:\Users\almir\Desktop\matter_target` em scripts ativos

#### R-B3 — Self-claim experimental já exposto
- **Arquivo:** saída `core-status-json`
- **Impacto:** positivo (honestidade); não é bug — mas bloqueia marketing “prod ready”

---

## 10. Plano de correção (baseado no código real, por prioridade)

### Fase 0 — Critérios de saída (definition of “mais perto de prod”, ainda sem declarar ready)

1. Pacote Windows roda em VM limpa **sem** Python/Rust/Cargo/GCC no PATH (DLL set documentado ou static/feature-split).  
2. Nenhum `unwrap`/`expect` em caminhos `run`/`check`/`deserialize` do núcleo (lint + CI).  
3. Corpus malformado com políticas explícitas (reject vs accept).  
4. Agent/shell/network **desligados por default** no binário “language runtime”.  
5. Suíte automatizada inclui: host dev + VM clean package smoke.

### Fase 1 — Isolamento do runtime de linguagem (bloqueadores C1–C3)

| Ação | Alvo | Evidência atual |
|---|---|---|
| Feature flags: `python`, `visual`, `agent`, `native` default **off** no release language | `matter-cli` Cargo features / deps | dependents `python3.dll`, opengl/mf |
| Build release “slim” sem PyO3 link | workspace features | package-standalone fail |
| Remover ou gatear `run_local_command_capture` e curl helpers atrás de `--enable-agent-tools` | `main.rs` | R-C2 |
| Documentar DLLs necessárias no README do `dist/` | package | STATUS_DLL_NOT_FOUND |

### Fase 2 — Robustez do núcleo (parser / MBC / VM)

| Ação | Alvo | Evidência |
|---|---|---|
| Rejeitar tokens/garbage não-linguagem | lexer/parser | garbage/bad_token exit 0 |
| Substituir unwraps da VM por `VmError` | `matter-vm` | closure unwrap |
| Limites: tamanho fonte, profundidade AST, tamanho string/const | compiler/cli | huge_string/deep_nest |
| CLI: detectar magic `MBC1` e usar `Bytecode::deserialize` | cli run path | random_mbc UTF-8 error |
| Corpus `target/validation/audit/malformed` → testes oficiais | new tests only (fase correção) | probes atuais |

### Fase 3 — Hardening de I/O e polyglot

| Ação | Alvo | Evidência |
|---|---|---|
| Polyglot offline por default; sem `pip`/`npm` auto | `matter-polyglot` | Command::new pip/npm |
| Timeouts e allowlist de executáveis | bridges | node/java/go spawns |
| Não embutir package managers em release language | resolver | R-A5 |

### Fase 4 — Qualidade e portabilidade

| Ação | Alvo | Evidência |
|---|---|---|
| Eliminar F: de scripts ativos | `scripts/*.ps1`, README scripts | absolute-paths.json |
| Paths relativos no extension/install | vscode-extension, install-local | D: hardcodes |
| Expandir unit tests do núcleo além de smoke scripts | parser/vm/bytecode | cargo tests vazios |
| Lint `unwrap_used` / `expect_used` no bin e lib núcleo | CI | 848 unwraps |

### Fase 5 — Re-auditoria

Repetir este documento como **V2** somente após:

- package smoke em VM limpa = pass  
- corpus malformado com política explícita = pass  
- zero RCE shell no default binary  
- `core-status-json` ainda pode dizer `production_ready: false` até checklist humano de produto

---

## 11. O que a auditoria **não** afirma

- **Não** afirma production-ready.  
- **Não** afirma ausência de vulnerabilidades (não foi pentest completo nem fuzz contínuo).  
- **Não** reexecutou todos os crates do workspace (só o conjunto listado em §1.2).  
- **Não** alterou código de produção nesta fase (probes e relatórios apenas).

---

## 12. Resumo executivo

| Dimensão | Estado atual |
|---|---|
| Happy-path no host de dev | Forte (suítes scripts 10/10, cargo selecionado 17/17) |
| Pacote standalone Windows | **Falhou** no PATH mínimo (DLL, incl. Python) |
| Robustez a input hostil | Mista (parse estrutural ok; garbage/token/limites fracos) |
| Superfície de processo/rede | **Alta** (powershell, curl, package managers, temp exe) |
| Higiene de panics/unwraps | **Fraca** para produção (centenas de unwraps) |
| Acoplamento a drives | Scripts F: residuais; build D:/mingw |
| Posicionamento honesto do produto | Experimental (auto-declarado) |

**Conclusão:** Matter tem um **núcleo experimental utilizável** e boas suítes de smoke no ambiente de desenvolvimento, mas **não está pronto para produção** como linguagem/runtime distribuível de forma segura e portátil. Os bloqueadores principais são: **dependência dura de DLLs (Python/gráficas)**, **execução de shell/rede no CLI**, e **falta de hardening sistemático do pipeline parser→VM** sob entrada malformada e limites de recurso.

---

*Fim do PRODUCTION_READINESS_AUDIT_V1. Próximo passo recomendado (fora deste documento): implementação da Fase 1 apenas após aprovação explícita.*
