# Auditoria de realidade dos modulos frontier

Esta nota separa tres coisas que estavam misturadas na documentacao:

- **Funcional no runtime**: existe backend registrado na VM e chamadas Matter executam codigo Rust real.
- **Modelo/simulador funcional**: ha algoritmo, estado e testes, mas nao ha hardware externo real.
- **Stub**: chamada existe, mas nao executa comportamento substantivo.

## Veredito curto

| Modulo | Status honesto | Evidencia no codigo |
| --- | --- | --- |
| `quantum` | Simulador funcional, nao hardware quantico real | `QuantumState`, gates, circuitos, medida probabilistica e algoritmos em `crates/matter-quantum/src/lib.rs`; backend em `crates/matter-quantum/src/backend.rs` chama `bell_state`, `grover` e `qft`. |
| `photonic` | Modelo funcional de computacao fotonica, nao chip/fibra real | `OpticalSignal`, `Waveguide`, gates logicos, WDM, rede neural e processador em `crates/matter-photonic/src/lib.rs`; backend executa `and`, `or`, `not` e `metrics`. |
| `neuromorphic` | Simulador SNN funcional, nao hardware neuromorfico real | `LIFNeuron`, `Synapse`, `SpikingNetwork`, STDP e taxa de spikes em `crates/matter-neuromorphic/src/lib.rs`; backend mantem rede com `init`, `step`, `apply_learning`, `spike_rate`. |
| `wetware` | Simulacao declarada, nao wetware biologico real | O proprio crate diz "simulated Microelectrode Arrays"; `OrganoidCulture::stimulate` faz mapeamento sintetico de spikes e `DopamineSystem` simula concentracao/decay. |

## Evidencia de integracao

Os quatro modulos sao registrados como backends nativos em `register_stdlib_backends`:

- `wetware`
- `quantum`
- `photonic`
- `neuromorphic`

O CLI tambem expoe um contrato JSON de auditoria:

```powershell
cargo run -q -p matter-cli -- frontier-status-json
```

Esse comando retorna `summary.all_non_stub=true`, `summary.all_simulated=true` e `summary.any_hardware=false`, alem do status individual de cada backend.

O payload declara o schema repo-relativo `schemas/frontier-status.schema.json`, validado pelo contrato:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\test-frontier-status-contract.ps1
```

Para release, o mesmo contrato pode ser exportado como artefato:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\export-frontier-status.ps1 -Out target\frontier\frontier-status.json
```

O verificador de pacote exige `target\frontier\frontier-status.json`, entao o zip distribuido carrega uma prova machine-readable das flags `non-stub/simulated/no-hardware`.

O teste `frontier_backends_execute_through_runtime` em `crates/matter-runtime/src/lib.rs` chama esses backends via `Runtime::call_backend`, provando que nao sao apenas nomes em documentacao:

- `*.status()` retorna `stub=false`, `hardware=false`, `simulated=true`, `mode`, `model` e `capabilities`.
- `quantum.bell_state()` retorna dois bits medidos.
- `photonic.and(0.9, 0.8)` retorna `1.0`.
- `neuromorphic.init`, `add_synapse` e `step` executam uma rede SNN em estado interno.
- `wetware.stimulate([true, false, true])` retorna uma lista de resposta.

## Limites que devem aparecer no manifesto

Nao afirmar "hardware real" para esses modulos sem drivers, FFI, SDKs ou dispositivos externos conectados. A formulacao correta e:

- `quantum`: simulador de vetor de estado integrado ao runtime.
- `photonic`: modelo computacional fotonico integrado ao runtime.
- `neuromorphic`: simulador SNN/LIF/STDP integrado ao runtime.
- `wetware`: abstracao wetware simulada, explicitamente nao biologica.

## Como reproduzir

```powershell
cargo test -p matter-quantum -p matter-photonic -p matter-neuromorphic -p matter-wetware
cargo test -p matter-runtime frontier_backends_execute_through_runtime
cargo test -p matter-cli frontier_status_json_reports_simulated_non_stub_backends
powershell -ExecutionPolicy Bypass -File scripts\test-frontier-status-contract.ps1
powershell -ExecutionPolicy Bypass -File scripts\export-frontier-status.ps1 -Out target\frontier\frontier-status.json
```

Resultado esperado: todos os testes passam. O primeiro comando cobre os modelos internos; o segundo cobre a chamada real pelo runtime.
