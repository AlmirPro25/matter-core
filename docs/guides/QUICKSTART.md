# Matter Core Quickstart

This guide uses the current organized workspace. You do not need to rename the project folder; `.cargo/config.toml` already sends build artifacts to a safe external target directory.

## Build The CLI

```powershell
cargo check -p matter-cli
cargo build -p matter-cli --release
```

## Run The First Program

```powershell
cargo run -q -p matter-cli -- run examples\first_run.matter
```

Expected output includes:

```text
Matter Core
language -> bytecode -> VM -> output
fib(8)
21
runtime: event handler executed
```

## Run A Small Program

Create `meu_programa.matter`:

```matter
let nome = "Mundo"
let numero = 42

print "Ola"
print nome
print numero

let resultado = 10 + 20
print resultado

if resultado > 25 {
    print "Resultado grande"
}
```

Run it:

```powershell
cargo run -q -p matter-cli -- run meu_programa.matter
```

## Evaluate Inline Code

```powershell
cargo run -q -p matter-cli -- eval "print 41 + 1"
cargo run -q -p matter-cli -- eval-json "print 41 + 1"
```

## Use Stdin

```powershell
"print 7 * 6" | cargo run -q -p matter-cli -- run -
"print 10" | cargo run -q -p matter-cli -- check -
"print 10" | cargo run -q -p matter-cli -- compile - -o target\programa.mbc
```

## JSON/Tooling Commands

```powershell
cargo run -q -p matter-cli -- capabilities-json
cargo run -q -p matter-cli -- tokens-json examples\first_run.matter
cargo run -q -p matter-cli -- imports-json examples\first_run.matter
cargo run -q -p matter-cli -- check-json examples\first_run.matter
cargo run -q -p matter-cli -- run-json examples\first_run.matter
```

## Bytecode

```powershell
cargo run -q -p matter-cli -- compile examples\first_run.matter -o target\first_run.mbc
cargo run -q -p matter-cli -- inspect-json target\first_run.mbc
cargo run -q -p matter-cli -- run-bytecode-json target\first_run.mbc
```

## Sentinel Kernel VM Check

```powershell
cargo run -q -p matter-cli -- sentinel-mbc1-kernel-check-json examples\sentinel_boot.matter --budget 10000
```

This compiles a Matter source file to MBC1, inspects it with `matter-kernel-vm`, and executes the supported kernel-safe subset with an instruction budget.

## Project Manifest Commands

```powershell
cargo run -q -p matter-cli -- package-json
cargo run -q -p matter-cli -- project-check-json
cargo run -q -p matter-cli -- project-verify-json
cargo run -q -p matter-cli -- project-run-json
cargo run -q -p matter-cli -- project-imports-json
cargo run -q -p matter-cli -- project-lock-json
```

## One-Command App Flow

For `create -> check -> run` in one script:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\app-bootstrap-smoke.ps1 -ProjectDir target\quick-app
```

Guide: [APP_BOOTSTRAP_SMOKE.md](APP_BOOTSTRAP_SMOKE.md)

## Tests

Fast focused validation:

```powershell
cargo test -p matter-kernel-vm
```

Larger workspace validation:

```powershell
cargo test --workspace --all-targets
```

## More Documentation

- [Documentation index](../INDEX.md)
- [Build status](../BUILD_STATUS.md)
- [Current reality/status](../status/REALIDADE_ATUAL_HONESTA.md)
- [Architecture](../ARCHITECTURE.md)
- [Language spec](../SPEC.md)
