# JIT Exec Experimental Flag

`jit-exec` is experimental.

## Status

The JIT infrastructure exists (hot-path detection, compilation pipeline, code cache), but the native executor path is not yet competitive against the interpreter on current benchmarks.

## Production Guidance

Use the interpreter path for production workloads.

Builds without `jit-exec` keep JIT execution disabled by default.

## How to enable (experimental only)

```powershell
cargo run -p matter-cli --features jit-exec -- benchmark-json examples\first_run.matter --iterations 5
```

Use this only for research and profiling, not as default runtime strategy.
