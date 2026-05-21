# Comparativo JIT V2 e Decisão de Gate

Atualizado em 2026-05-21.

## 1. Escopo do Teste

*   **Benchmark Alvo:** `benchmarks/loop_sum_vm.matter` (Soma de `1..1_000_000`)
*   **Iterações:** 10
*   **Ambiente de Teste:**
    *   **Interpretador (Sem JIT):** Binário compilado em modo Release oficial (`dist/matter-core-windows-x64/matter-cli.exe`).
    *   **JIT (Com JIT):** Binário compilado com a flag `--features jit-exec` em modo de desenvolvimento (`dev` profile).

---

## 2. Resultados Coletados

| Modo de Execução | Tipo de Execução | Tempo Medido (Mediana) | Speedup vs. Interpretador |
| :--- | :--- | :--- | :--- |
| **Interpretador** | Tradicional (Release) | **421.70 ms** | *Baseline (1x)* |
| **JIT** | Execução Fria (Cold Run - inclui compilação) | **2.98 ms** | **~141.5x mais rápido** |
| **JIT** | Execução Quente (Warm Run) | **2.86 ms** | **~147.4x mais rápido** |

### Estatísticas Detalhadas do JIT (10 execuções):
*   **Média:** 2.91 ms
*   **Mínimo:** 2.86 ms
*   **Máximo:** 3.20 ms
*   **Desvio Padrão:** 0.10 ms

---

## 3. Análise de Desempenho

1.  **Overhead de Compilação Negligenciável:** A diferença entre a primeira execução fria (**2.98 ms**) e as execuções quentes seguintes (**2.86 ms**) é de apenas **0.12 ms**. Isso prova que o tempo de compilação da AST para código de máquina nativo e alocação da página de memória executável está na ordem de microsegundos, sendo totalmente imperceptível.
2.  **Ganho de Desempenho Massivo:** Mesmo comparando um binário JIT rodando sob a build `dev` contra o interpretador em modo `Release` otimizado, a execução nativa compilada pelo JIT obteve um ganho de **~147x**.

---

## 4. Decisão de Gate (Recomendação)

Com base nos dados coletados na Semana 4 do plano de 30 dias:

> [!TIP]
> **Decisão Recomendada:** Promover a feature `jit-exec` de **Experimental** para **Beta Interno** no Windows.

### Ações Recomendadas:
1.  **Habilitar por Padrão na Build Local:** Manter o fallback estável do interpretador como principal para portabilidade, mas habilitar a feature `--features jit-exec` por padrão nas builds locais do Windows para desenvolvedores de apps.
2.  **Continuar Refinamento de Código de Máquina:** O compilador nativo (MNC) mostrou-se extremamente rápido e correto para o conjunto de instruções do benchmark. Recomenda-se expandir testes de stress para mais funções e estruturas de dados sob a feature JIT.
