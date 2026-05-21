# Benchmark Results (fair_bytecode)

- Rodadas por teste: 7
- Warmup: 1 rodada por linguagem/teste (fora da medicao)
- Matter mode: run-bytecode

## Loop simples (1..1.000.000)

| Linguagem | Mediana (ms) | Ranking |
|---|---:|---:|
| matter | 7.156,864 | 3 |
| python | 176,562 | 2 |
| node | 5,982 | 1 |
| lua | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## Fibonacci recursivo fib(30)

| Linguagem | Mediana (ms) | Ranking |
|---|---:|---:|
| matter | 8.363,215 | 3 |
| python | 282,429 | 2 |
| node | 25,195 | 1 |
| lua | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## Lista: criar, ordenar, somar 10.000

| Linguagem | Mediana (ms) | Ranking |
|---|---:|---:|
| matter | 4.711,531 | 3 |
| python | 0,402 | 1 |
| node | 2,282 | 2 |
| lua | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## String concat 10.000 vezes

| Linguagem | Mediana (ms) | Ranking |
|---|---:|---:|
| matter | 1.663,965 | 3 |
| python | 3,392 | 2 |
| node | 0,348 | 1 |
| lua | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## O que isso significa

- Matter venceu 0 de 4 testes comparaveis neste modo fair.
- Dados de perda indicam foco em otimizar custo de execucao e overhead do caminho escolhido (source ou bytecode).
