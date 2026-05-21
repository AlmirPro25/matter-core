# Matter Native Compiler - Quick Start

> Status note (2026-05-19): este guia cobre comandos e fluxo de uso.
> Para status validado atual de performance e prontidao, use:
> `docs/status/REALIDADE_ATUAL_HONESTA.md`,
> `docs/technical/JIT_EXEC_EXPERIMENTAL.md` e
> `docs/roadmap/JIT_V2.md`.

## Uso Rapido

```powershell
cargo run --bin matter-cli compile-native examples/sprint26_native_test.matter -o test.exe -O3
cargo run --bin matter-cli run-native examples/sprint26_native_test.matter -O3
```

## Niveis de Otimizacao

```text
-O0  debug
-O1  basico
-O2  balanceado
-O3  agressivo
```

## Fluxo Recomendado

1. Desenvolver e validar com interpretador:

```powershell
cargo run --bin matter-cli run my_program.matter
```

2. Testar compilacao nativa no mesmo programa:

```powershell
cargo run --bin matter-cli compile-native my_program.matter -o my_program.exe -O3
./my_program.exe
```

3. Medir com benchmark reproduzivel (3+ execucoes, mediana).

## Escopo Atual

- Backend nativo util para experimentacao e evolucao incremental.
- Cobertura depende de feature e arquitetura; validar caso a caso.
- Caminho de producao atual: VM interpretada.

## Troubleshooting

Se aparecer instrucao nao implementada ou comportamento divergente no nativo, rode via interpretador:

```powershell
cargo run --bin matter-cli run my_program.matter
```

## Referencias

- `docs/status/REALIDADE_ATUAL_HONESTA.md`
- `docs/technical/JIT_EXEC_EXPERIMENTAL.md`
- `docs/roadmap/JIT_V2.md`
