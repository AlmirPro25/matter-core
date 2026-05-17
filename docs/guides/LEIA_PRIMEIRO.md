# Leia Primeiro - Matter Core

Este workspace ja foi organizado. A raiz deve ficar reservada para arquivos essenciais do projeto; a documentacao extensa fica dentro de `docs/`.

## Comece Por Aqui

1. Leia [README.md](../../README.md) para entender o projeto.
2. Leia [docs/INDEX.md](../INDEX.md) para navegar pela documentacao.
3. Leia [docs/status/REALIDADE_ATUAL_HONESTA.md](../status/REALIDADE_ATUAL_HONESTA.md) para o estado atual.
4. Leia [docs/guides/INSTRUCOES_FINAIS.md](INSTRUCOES_FINAIS.md) para compilar e executar.

## Estado Atual

O projeto nao precisa mais ser renomeado para compilar. A configuracao em `.cargo/config.toml` envia os artefatos do Cargo para `F:/Users/almir/Desktop/matter_target`, evitando problemas do toolchain com espacos no caminho do workspace.

Validado neste workspace:

```powershell
cargo check -p matter-cli
cargo test -p matter-kernel-vm
cargo run -q -p matter-cli -- run examples\first_run.matter
```

## Estrutura

- `crates/` - crates Rust do workspace.
- `examples/` - programas `.matter`.
- `docs/status/` - status e resultados atuais.
- `docs/guides/` - guias praticos.
- `docs/sprints/` - historico de sprints.
- `docs/sessions/` - resumos de sessoes.
- `docs/archive/` - documentos antigos ou substituidos.
- `.build-artifacts/` - artefatos locais antigos agrupados e ignorados pelo Git.

## Proximos Passos Recomendados

1. Manter `README.md`, `docs/INDEX.md` e `docs/status/` como fontes principais.
2. Tratar documentos em `docs/archive/` como historicos.
3. Antes de publicar, revisar claims antigos sobre performance, hardware real e status de producao.
