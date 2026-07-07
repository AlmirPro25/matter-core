# Frontier Simulation Refinement Map

This document maps the current Matter frontier simulations into an engineering plan. The goal is not to claim real quantum, photonic, neuromorphic, or wetware hardware. The goal is to make simulated models measurable, reproducible, useful, and fast enough to support real applications.

## Objective

Matter frontier simulation should become a verified laboratory for hybrid algorithms:

- generate useful features for AI and optimization workloads
- test frontier-inspired execution models before hardware exists
- expose quality metrics through JSON contracts
- keep claims honest: simulated, non-hardware, measurable
- improve practical speed through approximation, sparsity, caching, batching, and specialized kernels

## Current Surface

| Backend | Current model | Runtime methods exposed today | Honest status |
| --- | --- | --- | --- |
| `quantum` | State-vector simulator with gates, controlled gates, measurement, Grover/QFT helpers | `status`, `bell_state`, `bell_stats`, `grover`, `qft` | Functional simulator, no hardware |
| `neuromorphic` | LIF spiking neural network with synapses, STDP, spike history | `status`, `init`, `add_synapse`, `add_random_synapses`, `step`, `lif_threshold_probe`, `apply_learning`, `spike_rate`, `reset` | Functional SNN simulator, no hardware |
| `photonic` | Optical signal/waveguide model with threshold logic gates and metrics | `status`, `and`, `or`, `not`, `metrics`, `truth_table`, `waveguide_loss` | Functional optical model, no chip/fiber hardware |
| `wetware` | Simulated organoid/MEA abstraction with synthetic spike response and dopamine state | `status`, `stimulate`, `reward`, `punish`, `tick`, `health`, `synapses`, `neurons`, `dopamine`, `bounded_state_probe` | Declared biological-inspired simulation, no biological hardware |

## Quality Ladder

### Level 1: Correctness

The simulator produces known reference behavior.

| Backend | Required checks | Useful metrics |
| --- | --- | --- |
| `quantum` | Bell distribution, norm preservation, Grover target success, QFT norm check | fidelity, probability histogram, success rate, norm error |
| `neuromorphic` | LIF threshold response, refractory behavior, spike-rate window, STDP direction | spike latency, spike count, rate Hz, weight delta |
| `photonic` | deterministic gate truth table, waveguide loss curve, phase continuity | attenuation dB, throughput %, truth-table accuracy |
| `wetware` | deterministic seeded response, reward/punish bounded state, dopamine decay | response entropy, health delta, dopamine concentration |

### Level 2: Reproducibility

The same experiment can be rerun and compared.

Required additions:

- `seed` input for stochastic experiments
- `shots` input for repeated quantum/statistical measurement
- stable JSON result shapes
- explicit simulator version
- min/max safety limits for qubits, neurons, channels, and samples

### Level 3: Realism

The simulation includes calibrated imperfection, not only ideal math.

| Backend | Realism model to add | Why it matters |
| --- | --- | --- |
| `quantum` | bit-flip, phase-flip, depolarizing, amplitude damping, measurement error | approximates noisy quantum hardware behavior |
| `neuromorphic` | jitter, variable threshold, synaptic delay noise, dropped spikes | approximates event hardware and biological variability |
| `photonic` | phase noise, crosstalk, dispersion, imperfect couplers, wavelength drift | approximates optical compute and communication limits |
| `wetware` | culture fatigue, stimulation adaptation, noisy response, bounded plasticity | prevents unrealistic stable biological claims |

### Level 4: Application Utility

The simulator produces an output that can improve another task.

| Use case | Backend path | Metric |
| --- | --- | --- |
| Quantum-inspired feature map | classical vector -> small quantum circuit -> probability features | classification accuracy, feature entropy, latency |
| Grover-style candidate scoring | candidate space -> oracle-like scoring -> sampled candidates | hit rate, sample efficiency |
| Spiking event filter | sensor/event stream -> SNN -> sparse trigger | event reduction %, latency, false positive rate |
| Photonic matrix approximation | matrix/vector -> optical mesh approximation -> output vector | relative error, throughput estimate |
| Wetware-inspired adaptation | stimulus/reward loop -> adaptive response state | stability, bounded health, reward sensitivity |

### Level 5: Speed

The simulation becomes fast enough for repeated use.

Speed does not come from pretending simulated hardware is real. It comes from choosing efficient simulation strategies:

- batching shots instead of rerunning full setup per measurement
- sparse/event-driven updates for SNNs
- cached compiled circuits
- fixed-size specialized kernels for small qubit counts
- approximate feature maps instead of full universal simulation
- vectorized matrix operations for photonic models
- skipping inactive neurons, empty channels, and zero-amplitude branches

## API Refinement Map

### Quantum

Current progress: `bell_stats` now exposes seeded shots and a Bell histogram. Grover success rate, QFT norm error, fidelity, and noise models remain open.

Next APIs:

```text
quantum.bell_stats(shots, seed) -> map
quantum.grover_stats(target, shots, seed) -> map
quantum.qft_quality(qubits) -> map
quantum.feature_map(values, qubits, seed) -> list<float>
quantum.noisy_bell_stats(shots, seed, noise_rate) -> map
```

Minimum quality contract:

- Bell: only `00` and `11` dominate.
- Bell balance: `abs(p00 - p11) <= 0.08` for sufficient shots.
- Grover: target success rate must exceed random baseline.
- QFT: norm error must remain below tolerance.

### Neuromorphic

Current progress: `lif_threshold_probe` now exposes a deterministic single-neuron threshold experiment. Refractory, spike-train, STDP, and sparse-event probes remain open.

Next APIs:

```text
neuromorphic.lif_threshold_probe(input_current, steps) -> map
neuromorphic.spike_train_stats(input_currents, steps) -> map
neuromorphic.stdp_probe(pre_post_dt) -> map
neuromorphic.event_filter(values, threshold) -> map
```

Minimum quality contract:

- Strong current causes a spike within bounded steps.
- Refractory period suppresses immediate repeated spike.
- Positive STDP timing increases weight; negative timing decreases it.
- Sparse input produces sparse output.

### Photonic

Current progress: `truth_table` and `waveguide_loss` now expose deterministic logic and simplified attenuation experiments. Crosstalk and approximate matrix multiplication remain open.

Next APIs:

```text
photonic.truth_table() -> map
photonic.waveguide_loss(length_m, intensity) -> map
photonic.wdm_crosstalk(channels, spacing_nm) -> map
photonic.matrix_multiply_approx(matrix, vector) -> map
```

Minimum quality contract:

- AND/OR/NOT truth tables match threshold logic.
- Waveguide attenuation increases with length.
- Throughput stays within `[0, 100]`.
- Approximate matrix multiply reports relative error.

### Wetware

Current progress: `bounded_state_probe` exposes a deterministic reward, punishment, and dopamine-decay experiment. Seeded response and richer adaptation probes remain open.

Next APIs:

```text
wetware.stimulate_seeded(spikes, seed) -> list<bool>
wetware.adaptation_probe(rounds, reward, punish) -> map
wetware.response_stats(spikes, rounds, seed) -> map
```

Minimum quality contract:

- Health remains bounded between `0` and `1`.
- Dopamine decays after `tick`.
- Reward and punishment move state in expected directions.
- Seeded stimulation is reproducible.

## Unified Contract

Add a machine-readable command:

```powershell
matter-cli frontier-sim-quality-json
```

Target shape:

```json
{
  "$schema": "schemas/frontier-simulation-quality.schema.json",
  "schema_version": 1,
  "kind": "frontier_simulation_quality",
  "ok": true,
  "summary": {
    "all_simulated": true,
    "any_hardware": false,
    "quality_level": 1
  },
  "checks": {
    "quantum": [],
    "neuromorphic": [],
    "photonic": [],
    "wetware": []
  }
}
```

The quality artifact also reports per-probe execution time in nanoseconds, total probe time, and the slowest measured probe. Process startup remains tracked separately by the general performance baseline.

## Implementation Order

1. Add quantum `bell_stats(shots, seed)` and a contract test. (implemented)
2. Add `frontier-sim-quality-json` with quantum-only Level 1 output. (implemented)
3. Add neuromorphic `lif_threshold_probe`. (implemented)
4. Add photonic `truth_table` and `waveguide_loss`. (implemented)
5. Add wetware bounded-state/adaptation probes. (implemented)
6. Add `schemas/frontier-simulation-quality.schema.json`. (implemented)
7. Add `scripts/test-frontier-simulation-quality-contract.ps1`. (implemented)
8. Add performance measurements for each simulation probe. (implemented)
9. Add noise models after deterministic contracts are stable.
10. Build application demos: quantum feature map, SNN event filter, photonic approximate matrix multiply.

## Claim Boundaries

Allowed claims:

- simulated quantum/photonic/neuromorphic/wetware-inspired models
- useful for algorithm prototyping and hybrid application experiments
- measurable quality and performance contracts
- no hardware dependency

Blocked claims until real evidence exists:

- real quantum speedup
- real photonic chip acceleration
- biological wetware execution
- hardware-equivalent results
- production scientific accuracy

## North Star

Matter should make frontier simulation useful by being honest and measurable:

```text
small simulated frontier model
-> reproducible quality contract
-> application-facing feature or decision
-> benchmarked cost
-> optional stronger model/noise later
```
