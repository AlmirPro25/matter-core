# Realidade Atual - Matter Core

Atualizado em 2026-05-15.

## Estado Validado

Matter Core e um runtime/linguagem experimental real, com parser, AST, bytecode MBC1, VM, runtime, CLI, exemplos e documentacao extensa.

Validado neste workspace:

```powershell
cargo check -p matter-cli
cargo test -p matter-kernel-vm
cargo run -q -p matter-cli -- run examples\first_run.matter
```

Resultado observado para `examples\first_run.matter`:

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

## O Que Funciona Hoje

- O workspace compila pelo menos o pacote `matter-cli`.
- Programas `.matter` simples executam via VM.
- O pipeline `source -> parser -> bytecode -> VM -> output` funciona.
- O crate `matter-kernel-vm` passa seus testes locais.
- O problema antigo de espaco no caminho foi resolvido buildando no `target/` local do projeto no D: com MinGW-w64 (`D:\mingw64\mingw64`), sem `target-dir` no F:.
- A documentacao foi organizada em `docs/status`, `docs/guides`, `docs/sprints`, `docs/sessions`, `docs/vision`, `docs/technical` e `docs/archive`.

## O Que Ainda Precisa Cuidado

- O repo tem muita documentacao historica com claims antigos ou repetidos.
- Algumas areas sao prototipos/simulacoes, nao integracoes reais com hardware.
- Bridges FFI e features avancadas precisam ser verificadas caso a caso antes de qualquer claim de producao.
- A CLI ainda concentra muita logica em um unico arquivo grande.
- O Git mostra muitos moves como `D` + `??` ate que as mudancas sejam staged.

## FFI Bridges

Os bridges FFI foram auditados separadamente em [FFI_BRIDGE_AUDIT.md](FFI_BRIDGE_AUDIT.md). Resumo curto:

- Python tem bridge PyO3 basico validado.
- Node.js, Go, Java e Rust ja tem chamadas reais simples validadas nos testes, mas ainda sao bridges limitados.
- Rust FFI dinamico funciona para bibliotecas exportando o ABI JSON do bridge; isso nao significa chamada arbitraria para qualquer crate Rust.
- Go native e Java native existem por feature, mas o build padrao usa fallback.
- Node.js native compila, mas precisa de teste dentro de host Node real.

## Leitura Correta

Use como fonte principal:

- [README.md](../../README.md)
- [docs/INDEX.md](../INDEX.md)
- [docs/BUILD_STATUS.md](../BUILD_STATUS.md)
- [docs/status/FFI_BRIDGE_AUDIT.md](FFI_BRIDGE_AUDIT.md)
- Este arquivo

Trate `docs/archive/` como historico, nao como status atual.
