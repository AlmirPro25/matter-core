# VM Optimization Report

## Objective
Separate startup overhead from VM execution and reduce VM instruction-loop cost.

## What was optimized
- Removed instruction vector clone in `Vm::run`.
- Removed event instruction clone in `emit_event_now`.
- Added global fast-path in variable load/update when no scope stack exists.
- Added fast-path in `LoadGlobal` and `LoadLocal` VM instruction handlers.

## Measured VM impact (loop_sum)
- Before optimization (`benchmark-json`, median, 3 iterations): **5429.976 ms**
- After optimization (`benchmark-json`, median, 3 iterations): **5017.094 ms**
- Improvement: **~412.884 ms** (**~7.60% faster**)

## Overhead separation (current session)
Reliable layers measured:
- CLI binary + source execution median: **5219.234 ms**
- CLI binary + bytecode execution median: **5011.376 ms**
- VM internal benchmark median (7 iterations): **4979.988 ms**

Estimated overhead from those layers:
- CLI startup+compile overhead vs VM internal: **~239.246 ms**
- CLI startup overhead on bytecode path: **~31.388 ms**

`cargo run` layer:
- In this session, direct separation of `cargo run` startup became **unreliable** because Windows keeps `matter-cli.exe` locked (`os error 5: Access denied`) for the debug target, causing command failures.
- Keep previously collected cargo-layer numbers as provisional only.

## Files
- Raw report: `benchmarks/vm_optimization_report.json`
- Overhead breakdown: `benchmarks/overhead_breakdown.json`
