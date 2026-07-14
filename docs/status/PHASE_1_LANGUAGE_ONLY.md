# PHASE_1_LANGUAGE_ONLY

**Date:** 2026-07-14  
**Scope:** Binary isolation only (language-only default). No parser/VM hardening, no broad unwrap cleanup, no demos.  
**Verdict:** Phase 1 **complete** for language-only isolation gates. Still **not** production-ready overall.

---

## 1. What changed (minimal)

### 1.1 `matter-runtime` features (`crates/matter-runtime/Cargo.toml`)

| Feature | Default | Pulls |
|---|:---:|---|
| *(none)* | yes | stdlib + energy only |
| `polyglot` | no | Python/Node/Go/Java/Rust bridges (**python3.dll** when Python enabled) |
| `visual` | no | `matter-visual` (eframe/reqwest → OpenGL/network stack) |
| `device` | no | `matter-device` (cpal/nokhwa → MF/audio) |
| `frontier` | no | domain science backends |
| `agent` / `net` | no | AgentBackend / NetBackend registration |
| `experimental-full` | no | all of the above |

### 1.2 `matter-cli` binaries (`crates/matter-cli/Cargo.toml`)

| Binary | When built | Role |
|---|---|---|
| **`matter-cli`** (default) | `cargo build -p matter-cli` | Language-only entry (`src/language_main.rs`) |
| **`matter-cli-experimental`** | `--features experimental-full --bin matter-cli-experimental` | Previous full CLI (`src/main.rs`) |

Default features: `language-only` + `jit-exec`.  
Bridges/agent/visual/device/emnr/native/lsp are **not deleted** — only optional.

### 1.3 Files touched

- `crates/matter-runtime/Cargo.toml` — feature matrix + optional deps  
- `crates/matter-runtime/src/lib.rs` — `cfg`-gated registration/tests  
- `crates/matter-cli/Cargo.toml` — dual bin + features  
- `crates/matter-cli/src/language_main.rs` — **new** language-only entry  
- `scripts/build-matter-cli.ps1` — `-ExperimentalFull` switch  

No Matter syntax changes. Valid core programs behave as before under language-only (stdlib backends).

---

## 2. Feature / capability matrix

| Capability | language-only (`matter-cli`) | experimental-full (`matter-cli-experimental`) |
|---|:---:|:---:|
| `run` / `eval` / `check` / `compile` | yes | yes |
| `run-bytecode` / `inspect` | yes | yes |
| `core-status-json` / `capabilities-json` | yes | yes |
| lexer/parser/MBC1/VM | yes | yes |
| stdlib math/list/string/file/json… | yes | yes |
| Python / Node / Go / Java bridges | **no** | yes |
| `polyglot-status-json` | exit 2 disabled | yes |
| `agent-ui` / shell / curl agent tools | **no** (exit 2 / not linked) | yes |
| visual / device / frontier backends | **no** | yes |
| `python3.dll` linked | **no** | yes (when polyglot) |
| OpenGL / Media Foundation deps | **no** | likely yes |

---

## 3. Reproducible build commands

```powershell
cd "D:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
$env:PATH = "D:\mingw64\mingw64\bin;" + $env:PATH
$env:CC = "D:\mingw64\mingw64\bin\gcc.exe"
$env:CXX = "D:\mingw64\mingw64\bin\g++.exe"
$env:DLLTOOL = "D:\mingw64\mingw64\bin\dlltool.exe"
$env:LIBRARY_PATH = "D:\mingw64\mingw64\x86_64-w64-mingw32\lib;D:\mingw64\mingw64\lib"

# Language-only (default)
cargo build -p matter-cli --release --target x86_64-pc-windows-gnu --bin matter-cli
# or: .\scripts\build-matter-cli.ps1 -Release

# Full experimental edition
cargo build -p matter-cli --release --target x86_64-pc-windows-gnu `
  --features experimental-full --bin matter-cli-experimental
# or: .\scripts\build-matter-cli.ps1 -Release -ExperimentalFull
```

Output (language-only):

`target/x86_64-pc-windows-gnu/release/matter-cli.exe`

---

## 4. Gate results

Evidence directory: `target/validation/phase_1_language_only/`

| Gate | Result | Evidence |
|---|---|---|
| Binary starts with PATH = System32 only | **PASS** | `minimal-path-gates.json`, `package-minimal-path.json` |
| `--help` / `--version` | **PASS** | same |
| `core-status-json` | **PASS** (`ok:true`, `edition: language-only`, `production_ready: false`) | same |
| `run` hello.matter | **PASS** | same |
| `compile` + `run-bytecode` MBC1 | **PASS** | `hello.mbc` + gates |
| No `python3.dll` | **PASS** | `dll-after.txt` / `dll-inventory.txt` |
| No opengl/mf dependents | **PASS** | `dll-after.txt` |
| agent-ui / polyglot-status disabled | **PASS** (exit 2) | `minimal-path-gates.json` |
| Package folder without Rust/Cargo/GCC/Python/Node in PATH | **PASS** | `package-language-only/` + `package-minimal-path.json` |
| Core cargo tests (12 crates) | **PASS** (0 fails) | `cargo-core-tests.json` |
| Language core app suite (5 cases) | **PASS** | `language-core-suite.json` |
| experimental-full still builds | **PASS** | build log exit 0 |

### 4.1 Expected suite deltas (documented)

`scripts/test-real-apps.ps1` against language-only CLI:

| Case | Result | Reason |
|---|---|---|
| hello, first_run, fibonacci, diario, orcamento, agent_policy | PASS | pure language |
| polyglot_runtime, python_*, node_* | **FAIL** (backend not found) | polyglot off — **expected** |
| polyglot-status-json | **FAIL** exit 2 | command disabled — **expected** |
| core-status | PASS (JSON ok) | language-only edition |

Do **not** treat polyglot failures on language-only binary as regressions.

---

## 5. Size & DLL comparison

| Metric | Before (full experimental) | After (language-only default) |
|---|---:|---:|
| Binary size | **45.9 MB** (48,126,352 B) | **3.51 MB** (3,684,828 B) |
| DLL dependents (objdump lines) | **59** (incl. python3, opengl, mf*) | **16** (kernel/crt/ws2/userenv only) |
| `python3.dll` | present | **absent** |

Before snapshot: `dll-before.txt` (captured from previous full release).  
After: `dll-after.txt`, `size-compare.json`.

### 5.1 Language-only DLL inventory (objdump)

```
kernel32.dll, ntdll.dll, userenv.dll, ws2_32.dll,
bcryptprimitives.dll, api-ms-win-core-synch-l1-2-0.dll,
api-ms-win-crt-*.dll (environment, heap, math, private, runtime, stdio, string, time)
```

**Note:** `ws2_64`/`ws2_32` may still appear via std/tokio-less stacks or transitive CRT usage; no Python/OpenGL/MF. No DLL copying was done to mask missing deps.

---

## 6. Incompatibilities recorded (before “fixing” them away)

| Finding | Decision in Phase 1 |
|---|---|
| Full CLI defaulted to `agent-ui` with no args | Language-only prints usage instead (CLI entry only; not Matter syntax) |
| `polyglot-status-json` / `agent-ui` absent | Explicit exit **2** + rebuild hint (not silent no-op) |
| Programs using `python.*` / `node.*` backends fail with “backend not found” | **Expected** on language-only; use experimental-full |
| `test-real-apps.ps1` fails polyglot rows on language-only | Documented expected delta; do not “fix” by re-enabling polyglot in default |
| `core-status-json` schema slightly leaner (language-only evidence) | Still `ok:true`, `claim: experimental_language_runtime`, `production_ready: false` |
| Install scripts / docs may still mention full CLI paths | Not rewritten beyond build script; Phase 1 did not require full doc migration |
| `ws2_32.dll` still listed | System networking DLL; not Python/Node. Acceptable for Phase 1 gate “no python3.dll” |

---

## 7. Runtime path independence

Language-only binary does **not** require absolute C:/D:/F: paths at runtime for:

- `--help`, `core-status-json`, `run`, `compile`, `run-bytecode`

Verified by copying only `matter.exe` + `examples/hello.matter` into  
`target/validation/phase_1_language_only/package-language-only/`  
and executing with `PATH=C:\Windows\System32;C:\Windows`.

Build still uses MinGW on D: **at compile time only** (developer machine).

---

## 8. Stop line

Phase 1 ends here.

**Not started (per approval boundary):**

- parser/VM malformed-input hardening  
- broad unwrap removal  
- Phase 2+ of production readiness plan  

**Not claimed:** production-ready.

---

## 9. Quick verification checklist

```powershell
$cli = "target\x86_64-pc-windows-gnu\release\matter-cli.exe"
$env:PATH = "C:\Windows\System32;C:\Windows"
& $cli --help
& $cli core-status-json
& $cli run examples\hello.matter
& $cli compile examples\hello.matter -o out.mbc
& $cli run-bytecode out.mbc
& $cli agent-ui   # expect exit 2
```

Evidence root: `target/validation/phase_1_language_only/`.
