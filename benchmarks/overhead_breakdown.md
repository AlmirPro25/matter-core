# Matter Overhead Breakdown

- Programa: `benchmarks/crosslang/matter/loop_sum.matter`
- Rodadas por medicao: 5

| Camada | Mediana (ms) |
|---|---:|
| cargo run + CLI + compile + VM | 32.392,037 |
| CLI binario + compile + VM | 5.202,561 |
| CLI binario + bytecode + VM | 5.208,891 |
| VM (benchmark interno) | 4.900,278 |

## Separacao estimada

- Overhead do `cargo run`: **27.189,476 ms**
- Overhead de startup+compile no CLI: **302,284 ms**
- Overhead de startup sobre bytecode: **308,614 ms**

## Nota metodologica

- O valor "VM (benchmark interno)" vem de `benchmark-json` e representa o miolo de execucao medido dentro do processo.
- A separacao e uma estimativa pratica para orientar otimização (startup/processo vs loop da VM).
