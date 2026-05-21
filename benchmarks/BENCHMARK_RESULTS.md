# Benchmark Results: Matter vs Python vs Node vs Lua

- Rodadas por teste: 3
- Metrica usada: mediana (ms)
- Matter medido com `Measure-Command` no PowerShell
- Python/Node/Lua medidos com temporizador nativo dentro do codigo

## Loop simples (1..1.000.000)

| Linguagem | Run 1 (ms) | Run 2 (ms) | Run 3 (ms) | Mediana (ms) | Ranking |
|---|---:|---:|---:|---:|---:|
| matter | 6.295,619 | 6.679,244 | 6.524,723 | 6.679,244 | 3 |
| python | 139,402 | 196,964 | 207,158 | 207,158 | 2 |
| node | 13,354 | 5,335 | 15,867 | 15,867 | 1 |
| lua | - | - | - | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## Fibonacci recursivo fib(30)

| Linguagem | Run 1 (ms) | Run 2 (ms) | Run 3 (ms) | Mediana (ms) | Ranking |
|---|---:|---:|---:|---:|---:|
| matter | 7.741,486 | 7.695,673 | 8.366,406 | 8.366,406 | 3 |
| python | 296,308 | 280,008 | 289,791 | 296,308 | 2 |
| node | 23,844 | 18,535 | 14,350 | 23,844 | 1 |
| lua | - | - | - | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## Lista: criar, ordenar, somar 10.000

| Linguagem | Run 1 (ms) | Run 2 (ms) | Run 3 (ms) | Mediana (ms) | Ranking |
|---|---:|---:|---:|---:|---:|
| matter | 4.009,092 | 4.225,360 | 4.068,898 | 4.225,360 | 3 |
| python | 0,302 | 0,328 | 0,287 | 0,328 | 1 |
| node | 4,069 | 1,600 | 1,479 | 4,069 | 2 |
| lua | - | - | - | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## String concat 10.000 vezes

| Linguagem | Run 1 (ms) | Run 2 (ms) | Run 3 (ms) | Mediana (ms) | Ranking |
|---|---:|---:|---:|---:|---:|
| matter | 1.748,220 | 1.631,035 | 1.424,624 | 1.748,220 | 3 |
| python | 3,437 | 3,297 | 3,460 | 3,460 | 2 |
| node | 0,326 | 0,340 | 0,334 | 0,340 | 1 |
| lua | - | - | - | N/A | N/A |

Observacao: Matter nao liderou este teste (ranking 3).

## O que isso significa

- Matter venceu 0 de 4 testes comparaveis nesta maquina/ambiente.
- Se Matter ganhou em algum teste, isso indica que o caminho VM/execucao atual esta competitivo para esse padrao de carga.
- Onde Matter perdeu, os dados apontam oportunidade de otimizar runtime, chamadas de backend e custo de inicializacao do CLI.
- Compare tambem variancia entre runs: alta variacao sugere ruido de ambiente e necessidade de mais iteracoes para decisao de tuning.
- Este benchmark e um retrato local (hardware/OS atual); para decisoes de produto, rode em CI padronizado com maquina dedicada.
