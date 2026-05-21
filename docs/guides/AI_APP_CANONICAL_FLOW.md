# AI -> Matter -> App (Fluxo Canonico)

Este guia define o caminho principal para transformar codigo Matter (gerado por IA ou humano) em software executavel com validacao.

## Objetivo

Padronizar um ciclo unico:
1. validar estrutura;
2. refletir sobre o programa;
3. aplicar gate reflexivo;
4. executar;
5. diagnosticar performance;
6. medir benchmark;
7. opcionalmente aplicar gate de performance.

## Script oficial

Use:

```powershell
.\scripts\ai-app-canonical-flow.ps1 -ProgramPath examples\first_run.matter
```

Por padrao, os artefatos sao gravados em `target\ai-flow\`.

## O que o script executa

- `check-json`
- `reflect-json`
- `reflexive-guard-json`
- `run-json`
- `perf-diagnose-json`
- `benchmark-json --iterations N`
- `benchmark-gate-json` (pode ser desativado)

## Parametros uteis

```powershell
.\scripts\ai-app-canonical-flow.ps1 `
  -ProgramPath examples\apps\counter_app.matter `
  -BenchmarkIterations 50 `
  -MaxMedianNs 30000000 `
  -MaxP95Ns 70000000
```

Para pular o gate de benchmark:

```powershell
.\scripts\ai-app-canonical-flow.ps1 -ProgramPath examples\first_run.matter -SkipBenchmarkGate
```

Para usar binario precompilado do CLI:

```powershell
.\scripts\ai-app-canonical-flow.ps1 `
  -ProgramPath examples\first_run.matter `
  -CliPath F:\Users\almir\Desktop\matter_target\release\matter-cli.exe
```

## Artefatos gerados

- `target\ai-flow\check.json`
- `target\ai-flow\reflect.json`
- `target\ai-flow\guard.json`
- `target\ai-flow\run.json`
- `target\ai-flow\perf.json`
- `target\ai-flow\benchmark.json`
- `target\ai-flow\benchmark-gate.json` (quando habilitado)
- `target\ai-flow\summary.json`
- `target\ai-flow\summary.md`

## Como interpretar

- `check.json`: sanidade de parse/compilacao.
- `reflect.json`: estrutura do programa como dados.
- `guard.json`: decisao de risco reflexivo antes de automacao agressiva.
- `run.json`: resultado funcional.
- `perf.json`: sinais de gargalo.
- `benchmark*.json`: criterio de orcamento de performance para CI/local.

## Recomendacao de uso no dia a dia

- Desenvolvimento rapido: rode com `-SkipBenchmarkGate` durante iteracao.
- PR/CI local: rode com gate habilitado e thresholds definidos por app.
- Release: combine este fluxo com `scripts\validate-full-workspace.ps1`.
