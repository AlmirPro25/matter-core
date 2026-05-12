# Matter Reflexive Core

Matter Reflexive Core is the first concrete step toward a language that can inspect and improve its own programs.

This is not mystical consciousness. It is operational reflexivity:

1. Read source without executing it.
2. Convert code into structured data.
3. Detect functions, events, calls, backend usage, and bytecode shape.
4. Feed that structure into agents, optimizers, test generators, or future Matter code.
5. Apply changes only after validation.

## Current Primitive

The CLI exposes source reflection through:

```powershell
matter-cli reflect-json examples/reflexive_self.matter
```

The command returns JSON with:

- `ast.top_level_statements`
- `ast.total_statements`
- `ast.statement_kinds`
- `ast.calls`
- `ast.backend_calls`
- `bytecode.summary`
- `bytecode.functions`
- `bytecode.event_handlers`
- `bytecode.opcode_histogram`

That means Matter can now be treated as data before execution. This is the root of self-analysis.

## Current Guard

The CLI also exposes a conservative guard for reflexive workflows:

```powershell
matter-cli reflexive-guard-json examples/reflexive_self.matter
```

The guard checks:

- total statement budget
- function count budget
- backend-call policy
- direct recursion warnings

Use stricter budgets for generated patches:

```powershell
matter-cli reflexive-guard-json examples/reflexive_self.matter --max-statements 40 --max-functions 10
```

Use `--allow-backends` only when the reflexive loop is expected to touch external systems.

## Runtime Loop

A living runtime should be built as a strict loop:

```text
observe -> reflect -> propose -> test -> accept/reject -> remember
```

- `observe`: read source, tests, benchmarks, logs, and runtime output.
- `reflect`: call `reflect-json` and convert the program into facts.
- `guard`: call `reflexive-guard-json` to reject unsafe or overlarge mutation candidates.
- `propose`: generate a small patch or optimization plan.
- `test`: run `check-json`, `benchmark-json`, and project tests.
- `accept/reject`: keep only changes that pass the gate.
- `remember`: store benchmark and decision history.

## Why This Matters

Before this command, Matter could run programs. With reflection, Matter can inspect program shape. The next frontier is giving Matter safe write access to its own source through a guarded agent loop.

The hard rule: the language may propose its own mutation, but validation decides what survives.
