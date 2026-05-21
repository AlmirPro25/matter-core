# Benchmark Results (fair_source)

- Rodadas por teste: 7
- Warmup: 1 rodada por linguagem/teste (fora da medicao)
- Matter mode: run source

## Loop simples (1..1.000.000)

| Linguagem | Mediana (ms) | Ranking |
|---|---:|---:|
| matter | 7.109,758 | 3 |
| python | 181,817 | 2 |
| node | 18,679 | 1 |
| lua | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## Fibonacci recursivo fib(30)

| Linguagem | Mediana (ms) | Ranking |
|---|---:|---:|
| matter | 8.486,687 | 3 |
| python | 221,854 | 2 |
| node | 18,583 | 1 |
| lua | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## Lista: criar, ordenar, somar 10.000

| Linguagem | Mediana (ms) | Ranking |
|---|---:|---:|
| matter | 4.761,813 | 3 |
| python | 0,376 | 1 |
| node | 1,689 | 2 |
| lua | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## String concat 10.000 vezes

| Linguagem | Mediana (ms) | Ranking |
|---|---:|---:|
| matter | 1.648,028 | 3 |
| python | 3,572 | 2 |
| node | 0,405 | 1 |
| lua | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## O que isso significa

- Matter venceu 0 de 4 testes comparaveis neste modo fair.
- Dados de perda indicam foco em otimizar custo de execucao e overhead do caminho escolhido (source ou bytecode).
