# Testes Completos - Matter Core

Atualizado em 2026-05-15.

## Validado Nesta Organizacao

```powershell
cargo check -p matter-cli
cargo test -p matter-kernel-vm
```

Resultados:

- `matter-cli`: check concluido com sucesso.
- `matter-kernel-vm`: 11 testes passaram.

## Teste Recomendado Para Validacao Total

Para uma validacao ampla do workspace:

```powershell
cargo test --workspace --all-targets
```

Esse comando pode demorar mais e pode exigir ambiente local completo para todos os crates opcionais.

## Risco Restante

Nem todos os claims historicos do projeto foram revalidados nesta passada. Os documentos antigos preservados em `docs/archive/`, `docs/sprints/` e `docs/technical/` devem ser lidos como historico ate que seus comandos sejam executados novamente.
