# Matter Core - Optimization Quick Guide

> Status note (2026-05-19): este guia descreve uso de `-O0..-O3`.
> Nao trate speedups historicos como baseline oficial.
> Decisao de nivel deve ser por benchmark local com mediana.

## Comandos

```powershell
matter compile-native <file.matter> -o <output> -O0
matter compile-native <file.matter> -o <output> -O1
matter compile-native <file.matter> -o <output> -O2
matter compile-native <file.matter> -o <output> -O3

matter run-native <file.matter> -O0
matter run-native <file.matter> -O1
matter run-native <file.matter> -O2
matter run-native <file.matter> -O3
```

## Quando Usar

- `-O0`: depuracao e iteracao rapida.
- `-O1`: desenvolvimento com leve otimizacao.
- `-O2`: equilibrio geral.
- `-O3`: tentativa de melhor runtime, com possivel custo maior de build.

## Regra Pratica

1. Comece por `-O2`.
2. Compare `-O2` vs `-O3` no workload real.
3. Fique com o nivel que entregar melhor latencia total para seu caso.

## Medicao Minima

- Rodar cada teste 3 vezes.
- Usar mediana.
- Separar startup/compilacao de execucao quando aplicavel.

## Referencias

- `docs/status/REALIDADE_ATUAL_HONESTA.md`
- `docs/status/RESULTADOS_REAIS.md`
- `docs/technical/JIT_EXEC_EXPERIMENTAL.md`
