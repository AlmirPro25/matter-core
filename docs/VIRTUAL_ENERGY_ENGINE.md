# Matter Virtual Energy Engine

## Filosofia
Matter não cria energia do nada. Matter cria mais trabalho útil por unidade de energia física.

## O que é
`matter-energy` é uma camada de runtime para estimar e otimizar custo virtual/computacional:
- CPU
- memória
- IO
- rede
- backend calls

## Modos
- `eco`
- `balanced`
- `performance`
- `adaptive`
- `critical`

## API Matter
- `energy.cpu()`
- `energy.memory()`
- `energy.mode()`
- `energy.set_mode(mode)`
- `energy.configure(key, value)`
- `energy.score(name)`
- `energy.estimate(name)`
- `energy.defer(name)`
- `energy.cache(key, value)`
- `energy.reuse(key)`

## Eventos de energia
- `energy.low`
- `energy.high`
- `energy.spike`
- `performance.drop`
- `battery.low`
- `heat.high`

## Profile (suporte inicial)
Enquanto `energy profile { ... }` não está no parser, use:
- `energy.set_mode("adaptive")`
- `energy.configure("battery_aware", true)`
- `energy.configure("prefer_cache", true)`

## Sintaxe de Profile (suportada)
Agora voce pode usar diretamente:

energy profile {
    mode: "adaptive"
    battery_aware: true
    prefer_cache: true
    allow_defer: true
}

Exemplo completo: `examples/energy/profile_block_demo.matter`

## CLI Energy Report
- matter-cli run-energy <arquivo.matter>: executa e imprime output + custo energetico estimado.
- matter-cli run-energy-json <arquivo.matter>: executa e retorna JSON com bloco energy (instruction_cost, ackend_cost).


## CLI Visual + Energy
Comandos visuais com telemetria opcional:
- `matter-cli visual-step-json <file.matter|-> <events.json> <delta_ms> [--with-energy]`
- `matter-cli visual-run-json <file.matter|-> <events.json> <frames> <delta_ms> [--with-energy]`
- `matter-cli project-visual-step-build-json [matter.toml] <events.json> <delta_ms> [-o out] [--with-energy]`
- `matter-cli project-visual-run-build-json [matter.toml] <events.json> <frames> <delta_ms> [-o out] [--with-energy]`

Quando `--with-energy` estiver ativo, o JSON inclui:
- `energy.instruction_cost`
- `energy.backend_cost`

## Tool Calling para IA
Matter agora inclui backend `tool` para transformar a linguagem em protocolo de tool-calling.

APIs:
- `tool.list()`
- `tool.describe(name)`
- `tool.register(name, description, expensive)`
- `tool.call(name, payload)`

Integracao energetica:
- Chamadas `tool` sao classificadas como operacoes caras no estimador virtual.
- O scheduler pode combinar `energy.defer("tool")` com cache/reuse para reduzir custo.
