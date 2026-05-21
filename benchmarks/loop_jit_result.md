# Loop JIT Result (Hot While > 1000)

## Configuracao
- Trigger: backedge de while com mais de 1000 iteracoes.
- Acao: compilar frame atual com matter-jit e tentar trocar para execucao nativa no mesmo loop hot.
- Benchmark: loop_sum (1..1_000_000).

## Resultado
- Baseline anterior (mediana): **5.017,094 ms**
- Resultado atual (mediana): **4.859,597 ms**
- Ganho absoluto: **157,497 ms**
- Ganho percentual: **3,14%**

## Leitura
- Houve melhora real no loop_sum apos a integracao do hot-loop path + ajustes de VM.
- A maior parte da melhoria continua vindo da reducao de custo no loop interpretado e lookup.
