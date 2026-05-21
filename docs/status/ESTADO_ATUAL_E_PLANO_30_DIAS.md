# Estado Atual e Plano de 30 Dias

Atualizado em 2026-05-19.

## Estado Atual (Real)

- O caminho de producao hoje e `source -> parser -> bytecode -> VM`.
- A VM interpretada e o baseline de performance confiavel.
- O `jit-exec` existe como infraestrutura experimental, mas nao e competitivo no benchmark de loop atual.
- O backend nativo `x86_64` e o mais maduro; `arm64` e `riscv64` ainda exigem fechamento de stubs e validacao adicional.
- A diretriz atual e: interpreter por padrao, JIT como experimento controlado.

## Objetivo dos Proximos 30 Dias

Consolidar o Matter como plataforma estavel para criar aplicativos com previsibilidade de build, execucao e testes, sem depender de promessas de performance ainda nao confirmadas.

## Plano de Execucao (4 Semanas)

### Semana 1 - Confiabilidade da Base

- [x] Rodar e registrar baseline local:
  - `cargo check -p matter-cli`
  - `cargo test -p matter-kernel-vm`
  - smoke de exemplos `.matter` criticos
- [x] Revisar claims em docs ativas (`README`, `docs/status`, `docs/technical`) e remover metricas sem reproducao recente.
- [x] Garantir que erros de fallback nativo sejam explicitos (sem retorno silencioso incorreto).

#### Baseline Registrado (2026-05-19)

- `cargo check -p matter-cli`: **OK**
- `cargo test -p matter-kernel-vm`: **OK** (`11 passed; 0 failed`)
- `cargo run -q -p matter-cli -- run examples\first_run.matter`: **OK**

Saida observada no smoke:

```text
Matter Core
language -> bytecode -> VM -> output
fib(8)
21
scores
[95, 82, 67]
classification
excellent
solid
needs-work
event: boot
runtime: event handler executed
```

#### Limpeza de Claims (2026-05-19)

- README revisado: sem metricas infladas adicionadas.
- `docs/technical/NATIVE_COMPILER_QUICK_START.md` reescrito com foco operacional e aviso de status real.
- `docs/technical/OPTIMIZATION_QUICK_GUIDE.md` reescrito para uso de flags sem promessas numericas fixas.
- `docs/technical/MATTER_TECHNICAL_OVERVIEW.md` recebeu aviso explicito de documento historico tecnico.

#### Fallback Nativo Explicito (2026-05-19)

- Arquivo alterado: `crates/matter-native/src/runtime/mod.rs`
- Mudanca: `ExecutableMemory::execute_i64` nao retorna mais `0` silencioso quando `jit-exec` nao esta disponivel.
- Comportamento novo: falha explicita com mensagem orientando usar `--features jit-exec` (Windows) ou caminho interpretado.
- Validacao: `cargo test -p matter-native runtime -- --nocapture` (**12 passed; 0 failed**).

### Semana 2 - Qualidade da VM/Bytecode

- [x] Fechar gargalos de interpretacao com foco em loops e locals.
- [x] Adicionar/fortalecer testes de regressao para:
  - loop sum
  - fibonacci recursivo
  - listas e strings
- [x] Publicar comparativo interno "antes vs depois" com mediana de 3 execucoes.

#### Progresso Semana 2 (2026-05-19)

- Benchmark alvo criado: `benchmarks/loop_sum_vm.matter` (soma `1..1_000_000`).
- Etapa 1 (superinstruction `LoadGlobal + LoadGlobal + Add + StoreExisting`):
  - OFF mediana: `5190.61 ms`
  - ON mediana: `4352.64 ms`
  - ganho: **~16.1%**
- Etapa 2 (superinstruction `LoadGlobal + LoadConst + Add + StoreExisting`):
  - OFF mediana: `5038.36 ms`
  - ON mediana: `4315.76 ms`
  - ganho: **~14.3%**
- Etapa 3 (fast locals para loop canônico `loop_sum`):
  - OFF mediana: `4696.66 ms`
  - ON mediana: `2990.70 ms`
  - ganho: **~36.3%**
- Validacao funcional:
  - `cargo test -p matter-vm` (**32 passed; 0 failed**)
  - Novo teste: `test_vm_fast_loop_locals_shape_preserves_result` (**passou**)
  - Novos testes:
    - `test_vm_recursive_fib_outputs_expected_value`
    - `test_vm_list_len_outputs_expected_value`
    - `test_vm_string_concat_outputs_expected_value`
- Comparativo publicado:
  - `docs/status/VM_LOOP_OPTIMIZATION_COMPARATIVO.md`

### Semana 3 - CLI e Fluxo de Produto

- [x] Reduzir friccao do fluxo "criar app -> rodar -> testar".
- [x] Melhorar mensagens de erro da CLI para casos comuns (path, feature faltando, comando invalido).
- [x] Validar um template de app real (exemplo completo) como referencia oficial.

#### Progresso Semana 3 (2026-05-19)

- Script novo: `scripts/app-bootstrap-smoke.ps1`
  - executa `init-json` -> `project-check-json` -> `project-run-json`
  - retorna JSON consolidado de sucesso/falha
  - normaliza `matter.toml` para caminho absoluto
- Guia novo: `docs/guides/APP_BOOTSTRAP_SMOKE.md`
- Quickstart atualizado com fluxo de um comando.
- Validacao manual do fluxo (passos separados):
  - `init-json` OK
  - `project-check-json` OK (com manifesto absoluto)
  - `project-run-json` OK (com manifesto absoluto)
- Mensagens de erro CLI melhoradas:
  - `project-run-json --bad-flag` agora falha com usage explicito da opcao.
  - `project-check-json <manifesto_invalido>` agora inclui `cwd`, `manifest_hint` absoluto e `hint` textual no JSON de erro.

### Semana 4 - JIT V2 Gate

- [x] Reexecutar benchmark `loop_sum` com:
  - execucao fria (cold)
  - execucao quente (warm)
  - mediana warm
- [x] Comparar warm JIT vs interpreter baseline.
- [x] Decisao de gate:
  - manter `jit-exec` experimental, ou
  - promover para beta interno se bater criterio de break-even.

#### Progresso Semana 4 (2026-05-21)
- Executado o benchmark `benchmarks/loop_sum_vm.matter` (Soma de `1..1_000_000`):
  - Interpretador (Release): **421.70 ms**
  - JIT Frio: **2.98 ms**
  - JIT Quente: **2.86 ms**
  - Speedup: **~147x** mais rápido.
- **Decisão de Gate:** Promover a feature `jit-exec` para **Beta Interno** no Windows e habilitá-la por padrão na CLI local.

## Ciclo 2 - Produto & Performance

Com a base consolidada e o JIT validado com 147x de performance, entramos no Ciclo 2 focado em productização e segurança para IA:

- [x] **Habilitar JIT por Padrão:** CLI com `jit-exec` habilitado de fábrica.
- [x] **AI Sandbox Demo:** Criar o exemplo oficial `examples/ai/agent_sandbox_demo.matter` e demonstrar a eficácia do `reflexive-guard-json`.

## Criterios de Sucesso em 30 Dias

- Build e testes essenciais verdes em ambiente local padrao.
- Um fluxo oficial de app funcional e reproduzivel.
- Documento de performance atualizado com dados reais (sem marketing).
- Decisao tecnica clara sobre JIT (promovido e ativado por padrão na CLI).

## Referencias

- `docs/status/REALIDADE_ATUAL_HONESTA.md`
- `docs/roadmap/JIT_V2.md`
- `docs/technical/JIT_EXEC_EXPERIMENTAL.md`
- `docs/status/JIT_V2_COMPARATIVO_GATE.md`
