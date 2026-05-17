# Instrucoes Finais - Matter Core

Este guia descreve o caminho atual para compilar, testar e executar o projeto.

## Build

O workspace pode ficar no diretorio atual, mesmo com espacos no nome. A saida de build e direcionada por [.cargo/config.toml](../../.cargo/config.toml) para um caminho externo sem espacos.

```powershell
cargo check -p matter-cli
cargo build -p matter-cli --release
```

## Executar Um Programa Matter

```powershell
cargo run -q -p matter-cli -- run examples\first_run.matter
```

Ou, depois do build release:

```powershell
F:\Users\almir\Desktop\matter_target\release\matter-cli.exe run examples\first_run.matter
```

## Testes

Teste rapido de uma parte pequena:

```powershell
cargo test -p matter-kernel-vm
```

Teste mais amplo do workspace:

```powershell
cargo test --workspace --all-targets
```

## Benchmarks Locais

Os arquivos de benchmark soltos foram movidos para `benchmarks/`:

```powershell
cargo run -q -p matter-cli -- run benchmarks\benchmark_fib.matter
python benchmarks\benchmark_fib.py
```

## Documentacao Atual

- [README.md](../../README.md) - entrada principal.
- [docs/INDEX.md](../INDEX.md) - mapa da documentacao.
- [docs/status/REALIDADE_ATUAL_HONESTA.md](../status/REALIDADE_ATUAL_HONESTA.md) - status tecnico atual.
- [docs/BUILD_STATUS.md](../BUILD_STATUS.md) - build validado.

## Observacao

Muitos documentos antigos ainda existem em `docs/archive/`, `docs/sprints/` e `docs/technical/`. Eles preservam historico, mas podem conter instrucoes antigas como renomear a pasta ou mover o projeto. Para operacao atual, use este guia, o README e `docs/status/`.
