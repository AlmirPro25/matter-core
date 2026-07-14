# Matter

**Matter** is a programming language and runtime: you write `.matter` source, the toolchain parses it, compiles to **MBC1** bytecode, and executes it on a stack-based **VM**.

> Matter Core runs small programs on its own VM without requiring Python, Node, or a full experimental stack.

This repository is the development tree for Matter. The **language-only Core** is the production-oriented slice; **Experimental** is the full optional surface.

---

## Pipeline

```
.matter source  →  lexer  →  parser  →  AST
       →  bytecode builder  →  MBC1 (.mbc)
       →  Matter VM / runtime  →  output
```

| Stage | Role |
|-------|------|
| Lexer | Tokens; illegal characters are rejected |
| Parser | AST with size/depth limits |
| MBC1 | Portable bytecode (validate before execute) |
| VM | Stack machine with structured errors and resource limits |
| CLI | `run`, `check`, `compile`, `run-bytecode`, JSON tools |

---

## Matter Core vs Matter Experimental

| | **Matter Core** (default) | **Matter Experimental** |
|--|---------------------------|-------------------------|
| Binary | `matter-cli` / `matter.exe` | `matter-cli-experimental` |
| Features | Language, MBC1, VM, stdlib essentials | + polyglot bridges, agent UI, visual, net, devices… |
| Shell / PowerShell spawn | **Not available** | Allowlisted / explicit override only (**not a sandbox**) |
| Network / agent / package install | Denied | Optional when built with features |
| Typical size | ~3.5 MB | Much larger; may link extra DLLs |

Build Core (language-only):

```powershell
.\scripts\build-matter-cli.ps1 -Release
# → target\x86_64-pc-windows-gnu\release\matter-cli.exe
```

Build Experimental (explicit):

```powershell
cargo build -p matter-cli --release --target x86_64-pc-windows-gnu `
  --features experimental-full --bin matter-cli-experimental
```

---

## Basic commands (Core)

```powershell
# From a built binary or installed package:
matter-cli --help
matter-cli --version
matter-cli core-status-json
matter-cli check examples\hello.matter
matter-cli run examples\hello.matter
matter-cli compile examples\hello.matter -o hello.mbc
matter-cli run-bytecode hello.mbc
```

Portable package (no Rust on the destination machine):

```powershell
.\scripts\package-matter-core.ps1 -SkipBuild
# → dist\matter-core-0.1.0-windows-x64.zip
.\scripts\install-matter-core.ps1 -PackageRoot .\dist\matter-core-0.1.0-windows-x64 -InstallRoot $env:LOCALAPPDATA\Matter
```

See [docs/INSTALL_WINDOWS.md](docs/INSTALL_WINDOWS.md).

---

## Maturity status (honest)

| Claim | Status |
|-------|--------|
| Production ready | **No** (`production_ready: false`) |
| Release Candidate | **Not granted** |
| Stable 1.0 | **Not released** |
| Baseline snapshot | **Matter Core 0.1.0 production-readiness baseline** |

### Gates (latest recorded)

| Gate | Result |
|------|--------|
| Core suite | **37 / 37 PASS** |
| Security suite | **26 / 26 PASS** |
| Portable suite | **20 / 20 PASS** |
| Production readiness V2 | **`BLOCKED_EXTERNAL_VALIDATION`** |

### Blocker

**External Windows validation** of the frozen ZIP on a **clean** machine (no Rust/Cargo/GCC/Python/Node, no prior `D:\Matter`) was **BLOCKED** on the build host. Until that passes, the project does **not** claim Release Candidate.

- Report: [docs/status/EXTERNAL_WINDOWS_VALIDATION_V1.md](docs/status/EXTERNAL_WINDOWS_VALIDATION_V1.md)
- Harness: [scripts/external-windows-validation.ps1](scripts/external-windows-validation.ps1)

Frozen package SHA-256 (local `dist/`, not required in git):

`0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2`

---

## Repository layout

| Path | Content |
|------|---------|
| `crates/` | Language, VM, CLI, optional bridges |
| `scripts/` | Build, package, install, security, readiness |
| `examples/` | Sample `.matter` programs |
| `tests/` | Integration tests and invalid fixtures |
| `docs/` | Architecture, install, release, status reports |
| `docs/evidence/releases/0.1.0/` | Small JSON evidence snapshots |

---

## Documentation

- [CURRENT_STATUS.md](CURRENT_STATUS.md) — snapshot of maturity and gates  
- [CHANGELOG.md](CHANGELOG.md)  
- [SECURITY.md](SECURITY.md)  
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)  
- [docs/INSTALL_WINDOWS.md](docs/INSTALL_WINDOWS.md)  
- [docs/RELEASE_PROCESS.md](docs/RELEASE_PROCESS.md)  
- Phase reports: [docs/status/](docs/status/)  

---

## License

See [LICENSE](LICENSE).

---

## Development notes

- Default Windows GNU target: `x86_64-pc-windows-gnu` (MinGW on the **build** machine only).  
- Destination machines running the portable ZIP do **not** need Rust or GCC.  
- Do not treat experimental allowlists as a security sandbox.
