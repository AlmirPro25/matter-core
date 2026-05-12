# Matter Core Performance Audit

Updated: 2026-05-12

## Environment

- OS: Windows
- CLI build: `cargo build -p matter-cli --release`
- Binary: `F:\Users\almir\Desktop\matter_target\release\matter-cli.exe`
- Benchmark mode: `matter-cli benchmark <file> --iterations N`

The CLI benchmark compiles source once, then measures repeated bytecode VM execution with stdout disabled. This is the best current signal for runtime speed because it avoids process startup and repeated parsing.

## Existing Benchmarks

Command pattern:

```powershell
matter-cli benchmark benchmarks/<name>.matter --iterations 30
```

| Benchmark | Workload | Average | Min | Max |
|---|---:|---:|---:|---:|
| `fibonacci.matter` | recursive `fib(20)` | 297.694 ms | 212.751 ms | 461.735 ms |
| `fibonacci_iterative.matter` | iterative `fib(30)` | 35.61 us | 31.9 us | 71 us |
| `sum_array.matter` | 1,000 loop additions | 446.55 us | 423.5 us | 892 us |
| `nested_loops.matter` | 100 x 100 loop | 6.811 ms | 4.331 ms | 30.605 ms |
| `function_calls.matter` | 1,000 nested function calls | 8.845 ms | 5.625 ms | 42.333 ms |
| `loop_intensive.matter` | 20,000 mixed loop iterations | 11.436 ms | 8.705 ms | 21.708 ms |
| `data_structures.matter` | list/map/struct-style workload | 815.623 us | 769.9 us | 1.504 ms |
| `backend_calls.matter` | math/string/store/time backend calls | 28.717 ms | 12.902 ms | 128.365 ms |
| `stress_test.matter` | recursion + loops + backend mix | 3.584 ms | 2.364 ms | 16.925 ms |

## Heavy Synthetic Benchmarks

Command pattern:

```powershell
matter-cli benchmark <temp-heavy-file>.matter --iterations 10
```

| Benchmark | Workload | Average | Min | Max |
|---|---:|---:|---:|---:|
| `heavy_loop_1m.matter` | 1,000,000 loop additions | 635.671 ms | 523.673 ms | 833.127 ms |
| `heavy_nested_300.matter` | 300 x 300 arithmetic loop | 65.884 ms | 51.741 ms | 80.953 ms |
| `heavy_function_10k.matter` | 10,000 calls through nested functions | 379.412 ms | 263.887 ms | 582.439 ms |
| `heavy_fib_22.matter` | recursive `fib(22)` | 851.833 ms | 613.260 ms | 1.130 s |
| `heavy_backend_5k.matter` | 5,000 math-backend iterations | 7.650 ms | 6.604 ms | 14.563 ms |

## Function Call Optimization Pass

Implemented on 2026-05-12:

- VM function calls no longer clone the whole `Function` body on every invocation.
- Call frames no longer clone and store the function name.
- Direct named calls now compile to `CallNamed`, avoiding the old `LoadGlobal(function) -> Value::Function -> Call` path for normal `foo(...)` calls.
- Bytecode serialization/deserialization and native codegen dispatch now understand `CallNamed`.
- Function parameters now compile to `LoadParam(index)` and execute through positional function-frame storage instead of fake `__param_N` hashmap locals.
- Hot arities 0, 1, and 2 now use specialized argument collection instead of the generic pop/reverse path.
- VM hot-path profiling now uses lightweight call counting instead of timing bookkeeping on every interpreted function call.
- Hot-path/JIT checks now run at controlled call-count milestones instead of every function call, and functions that fail JIT compilation are not retried on every hot call.

Measured after `cargo build -p matter-cli --release`:

| Benchmark | Before | After | Improvement |
|---|---:|---:|---:|
| `function_calls.matter` | 8.845 ms | 3.445 ms | 2.57x faster |
| `fibonacci.matter` | 297.694 ms | 13.000 ms | 22.90x faster |
| `heavy_function_10k.matter` | 379.412 ms | 35.744 ms | 10.62x faster |
| `heavy_fib_22.matter` | 851.833 ms | 29.039 ms | 29.34x faster |

The benchmark command runs repeated bytecode VM executions. After this pass, hot functions can enter the VM JIT cache during the repeated benchmark loop, while the separate LLVM-native benchmark still remains skipped unless LLVM is enabled.

## Process-Level Measurements

These numbers include Windows process startup, parsing/compilation for source mode, stdout handling, and runtime execution.

| Case | Mode | Average | Min | Max |
|---|---|---:|---:|---:|
| `sum_array` | run source process | 10.93 ms | 7.18 ms | 23.08 ms |
| `sum_array` | run bytecode process | 10.18 ms | 7.22 ms | 20.71 ms |
| `backend_calls` | run source process | 43.90 ms | 32.75 ms | 66.21 ms |
| `backend_calls` | run bytecode process | 72.80 ms | 55.02 ms | 98.25 ms |
| `heavy_loop_1m` | run source process | 707.03 ms | 559.97 ms | 819.75 ms |
| `heavy_loop_1m` | run bytecode process | 680.55 ms | 551.62 ms | 866.32 ms |

## Findings

### Where Matter Core shines

- Iterative arithmetic and straight-line VM execution are the current strong path.
- The VM can run a 1,000,000-iteration arithmetic loop in about 0.64 seconds.
- Small iterative workloads are very fast once already inside the VM. `fib(30)` iterative ran in about 35.61 microseconds.
- Math backend calls are strong in the isolated heavy math-backend test.
- Bytecode execution is stable under mixed stress workloads.
- The runtime/tooling path is now practical: `doctor`, `init`, package checks, project run, bytecode compile, event emit, and CI all work.

### Where Matter Core is weak today

- Recursive workloads are expensive. `fib(22)` took about 0.85 seconds, and `fib(20)` took about 0.30 seconds.
- Function call overhead is significant. The 10,000-call heavy function workload took about 0.38 seconds.
- Very small scripts are dominated by process startup and CLI overhead, not VM execution.
- Mixed backend workloads have high variance, especially when store/time/random-style calls are involved.
- Native/JIT acceleration is not part of the validated hot path in this run.

## Practical Interpretation

Matter Core is strongest today as an interpreted/bytecode VM for:

- Event-driven scripts.
- CLI-driven workflows.
- Reactive prototypes.
- Package/project experiments.
- Runtime/backend orchestration.
- Loops and iterative calculations that do not require deep recursion.

It is not yet ideal for:

- Heavy recursive algorithms.
- Tight numeric kernels where Rust/C-level performance is expected.
- High-frequency function-call-heavy inner loops.

## Optimization Targets

## Benchmark Telemetry

Implemented on 2026-05-12:

- `matter-cli benchmark <file> --iterations N --json`
- `matter-cli benchmark-json <file> --iterations N`
- `matter-cli benchmark-gate-json <benchmark.json> --max-median-ns N --max-p95-ns N --ci-exit-codes`
- JSON output includes average, median, p95, min, max, standard deviation, and native benchmark status.
- Text benchmark output now also includes median and p95, reducing dependence on noisy averages.
- Gate output is JSON and can fail CI with `--ci-exit-codes`.

Example:

```powershell
matter-cli benchmark-json benchmarks/fibonacci.matter --iterations 20
matter-cli benchmark-gate-json bench.json --max-median-ns 100000000 --max-p95-ns 200000000 --ci-exit-codes
```

## Optimization Targets

1. Add tail-call or recursion optimization.
2. Add call-frame reuse/pooling to reduce recursive allocation churn.
3. Add loop bytecode specialization for integer arithmetic.
4. Wire `benchmark-gate-json` into a manual/performance CI workflow with conservative budgets.
5. Revisit LLVM/native path after VM/JIT benchmark baselines are stable.
