# Organizacao Completa - Matter Core

Atualizado em 2026-05-15.

## Resultado

A raiz do workspace foi reduzida aos arquivos essenciais e as massas de documentacao/artefatos foram movidas para pastas especificas.

## Raiz Atual

- `.gitignore`
- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `LICENSE`
- `CODE_OF_CONDUCT.md`
- `CONTRIBUTING.md`
- `SECURITY.md`
- `PROGRESS.md`
- `matter.toml`
- arquivos locais ignorados: `.matter_store.json`, `.matter_live_events.json`

## Pastas Principais

- `crates/` - codigo Rust.
- `examples/` - exemplos Matter.
- `benchmarks/` - benchmarks e comparacoes locais.
- `docs/` - documentacao organizada.
- `scripts/` - scripts ativos.
- `scripts/archive/` - scripts antigos de organizacao/movimentacao.
- `.build-artifacts/` - artefatos antigos `target*` e `tmp_*`, ignorados pelo Git.

## Documentacao

- `docs/status/` - status e validacoes atuais.
- `docs/guides/` - guias praticos atuais.
- `docs/sprints/` - historico de sprints.
- `docs/sessions/` - resumos de sessoes.
- `docs/vision/` - documentos de visao.
- `docs/technical/` - documentos tecnicos.
- `docs/archive/` - documentos antigos ou substituidos.

## Validacao

Comandos validados apos a organizacao:

```powershell
cargo check -p matter-cli
cargo test -p matter-kernel-vm
```

## Observacao Git

Como muitos arquivos foram movidos, `git status` mostra varios pares `D` + `??` ate que as mudancas sejam staged. Isso e esperado para uma reorganizacao grande.
