# JIT V2 Roadmap

## Context from JIT V1

This document records what was validated in JIT V1 and what is required before enabling JIT by default.

## What is already ready

- `matter-jit` crate structure exists.
- Hot path profiler/detector exists.
- JIT compilation pipeline exists (`matter-jit` -> `matter-native` machine code buffer).
- Code cache exists (LRU behavior and lookup path).
- Feature flag gating exists (`jit-exec`), and default runtime keeps JIT execution off.

## What is still missing

- Native executor quality is not yet competitive for real workloads.
- Current hot-loop path is still experimental and shape-limited.
- There is no proven general break-even where native execution beats interpreter consistently.
- Need stronger validation for correctness under broader bytecode patterns.

## Measured reality from V1

- Interpreter path outperformed the experimental JIT path on the tested `loop_sum` scenario.
- Separating cold vs warm runs showed warm execution still not competitive.
- Conclusion: compile-cost separation was useful, but executor/runtime quality remains the bottleneck.

## JIT V2 goals

1. Make native execution correctness and ABI behavior robust across representative instruction mixes.
2. Improve executor performance for hot loops beyond the interpreter baseline.
3. Define explicit break-even thresholds (cold, warm, and steady-state).
4. Expand from one shape-specific loop to generalized hot-loop/function coverage.

## Break-even estimate target

JIT V2 should only be considered viable when:

- warm median runtime is consistently below interpreter baseline on representative workloads, and
- cold penalty is amortized in repeated execution scenarios.

Practical gate proposal:

- at least ~15% warm-speed improvement on core loop benchmarks,
- no correctness regressions in VM/runtime test suites,
- fallback behavior remains safe when JIT path is disabled or unavailable.
