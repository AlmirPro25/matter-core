# VM Loop Optimization - Comparativo Interno

Atualizado em 2026-05-19.

## Escopo

Benchmark alvo: `benchmarks/loop_sum_vm.matter`  
Carga: soma de `1..1_000_000`  
Metodo: mediana de 3 execucoes por etapa (benchmark-json)

## Resultados

| Etapa | Baseline (ms) | Otimizado (ms) | Ganho |
|---|---:|---:|---:|
| Superinstruction global+global+add+store | 5190.61 | 4352.64 | 16.1% |
| Superinstruction global+const+add+store | 5038.36 | 4315.76 | 14.3% |
| Fast locals (loop canônico) | 4696.66 | 2990.70 | 36.3% |

## Leitura

- Etapa 1 e 2 reduziram custo por instrução no caminho interpretado.
- Etapa 3 trouxe o maior ganho ao evitar lookup repetido de globais no loop canônico.
- A VM manteve corretude: `cargo test -p matter-vm` passou com 32 testes.

## Observacao

Os numeros variam por maquina/carga. Este documento e interno e deve ser revalidado quando houver mudancas no dispatch da VM.
