# PHASE_3_CAPABILITY_SECURITY

**Date:** 2026-07-14  
**Scope:** Isolation and policy for dangerous capabilities (language-only default vs experimental).  
**Not in scope:** general cleanup, broad refactor, signing, installer, Phase 4.  
**Verdict:** Phase 3 **complete** for approved gates.  
**Explicit:** the experimental edition is **NOT a sandbox**.

---

## 0. Preconditions (Phase 2 freeze)

| Item | Result |
|---|---|
| Phase 2 report | `PHASE_2_CORE_HARDENING.md` |
| Phase 2 CLI SHA-256 | `A83233A7D5091CD5283F647A1162213A80ABF2627D516B138DEAD94F948DCAEF` (3 725 413 B) |
| Git HEAD at freeze (workspace) | `e9f91fbb642f5474abe600c93a44e109a623e809` (dirty tree; freeze recorded by hash files) |
| UTF-8 BOM policy | Documented — BOM (`U+FEFF`) → `Token::Illegal` → parse error (Phase 2 I8) — `target/validation/phase_3_capability_security/baseline_phase2/utf8-bom-policy.md` |
| Limit defaults (parser / MBC1 / VM) | `target/validation/phase_3_capability_security/baseline_phase2/limits-defaults.json` |
| Core suite re-run | **37 / 37 PASS** |

Baseline dir: `target/validation/phase_3_capability_security/baseline_phase2/`.

### 0.1 Default limits (frozen reference)

| Layer | Variable / constant | Default |
|---|---|---:|
| Parser | `MATTER_MAX_SOURCE_BYTES` | 1 048 576 |
| Parser | `MATTER_MAX_TOKENS` | 250 000 |
| Parser | `MAX_RECURSION_DEPTH` | 64 |
| MBC1 | `MATTER_MBC_MAX_FILE_BYTES` | 33 554 432 |
| MBC1 | `MATTER_MBC_MAX_CONSTANTS` | 100 000 |
| MBC1 | `MATTER_MBC_MAX_FUNCTIONS` | 50 000 |
| MBC1 | `MATTER_MBC_MAX_EVENT_HANDLERS` | 50 000 |
| MBC1 | `MATTER_MBC_MAX_INSTRUCTIONS_PER_BLOCK` | 500 000 |
| MBC1 | `MATTER_MBC_MAX_INSTRUCTIONS_TOTAL` | 2 000 000 |
| MBC1 | `MATTER_MBC_MAX_STRING_BYTES` | 1 048 576 |
| VM | `MATTER_VM_MAX_STACK` | 1 000 000 |
| VM | `MATTER_VM_MAX_CALL_DEPTH` | 10 000 |
| VM | `MATTER_VM_MAX_INSTRUCTIONS` | 100 000 000 |
| VM | `MATTER_VM_MAX_EVENT_DRAINS` | 10 000 |
| VM | `MATTER_VM_MAX_SCOPE_DEPTH` | 50 000 |

### 0.2 UTF-8 BOM

Leading UTF-8 BOM is **rejected** as illegal input (not silently skipped). Sources must be UTF-8 **without** BOM. Fixtures/suite already use no-BOM writers.

---

## 1. Objective

Ensure **default Matter Core (language-only)** does not execute shell, PowerShell, external processes, network, package install, or language bridges via direct or indirect paths reachable from the default binary.

---

## 2. Inventory summary

Full matrix: **`capability-matrix.json`** (also copied under evidence).

| Capability | Language-only | Experimental |
|---|---|---|
| Process spawn | **Absent** | Allowlist + injection filter + timeout + output cap; structured argv |
| PowerShell `-Command` | **Absent** | **Removed** (was `run_local_command_capture`) |
| Network / sockets | **Absent** (no `net` backend) | Present under `net` feature |
| pip/npm/go install | **Absent** | Possible only via polyglot bridges |
| Python/Node bridges | **Absent** (no `python3.dll`) | `experimental-full` |
| Agent UI / local tools | CLI **denied** (exit 2) | Present; `/run` allowlisted |
| `run_local_command_capture` | **No callers** | Hardened module `capability_policy.rs` |
| Filesystem write (stdlib/compile) | Present (language feature) | Present + agent file tools |

---

## 3. Changes implemented

### 3.1 Shared policy module

**File:** `crates/matter-cli/src/capability_policy.rs`

- `reject_injection` — blocks `; | & \` $ () {} quotes newlines redirection`
- Exact + prefix **allowlist** (git/cargo/matter project-*)
- `run_local_command_capture` / `run_whitelisted_command` — structured `Command` argv, **no PowerShell**
- Timeout (`MATTER_LOCAL_COMMAND_TIMEOUT_SECS`, default 60s)
- Output cap (`MATTER_LOCAL_COMMAND_MAX_OUTPUT`, default 256 KiB)
- Arbitrary non-allowlist only if `MATTER_ALLOW_LOCAL_COMMANDS=1` (still injection-blocked)
- Unit tests for injection + denylist (8 tests, permanent)

### 3.2 Language-only binary

**File:** `crates/matter-cli/src/language_main.rs`

- Expanded deny list for dangerous command names (`LANGUAGE_ONLY_DENIED_COMMANDS`)
- Messages state **action was NOT executed**
- Exit code **2** for denied capability commands; **1** for unknown
- `capabilities-json` includes `security` block (`is_sandbox: false`, spawn/net/agent false)
- No automatic fallback to experimental edition

### 3.3 Experimental binary

**File:** `crates/matter-cli/src/main.rs`

- Replaced PowerShell-based `run_local_command_capture` with `capability_policy`
- Call sites that quoted paths for PowerShell now pass plain paths for structured argv
- Header comment: **NOT a sandbox**
- Existing experimental features preserved behind `experimental-full` (not auto-enabled)

### 3.4 Runtime (unchanged contract from Phase 1, verified)

- `agent` / `net` / `polyglot` backends only with features
- Language-only: `agent.say` / `net.get` / `python.call` → **backend not found** (exit 1)

---

## 4. `run_local_command_capture` audit

| Item | Detail |
|---|---|
| Definition | `capability_policy::run_local_command_capture` |
| Callers | **Only** `matter-cli` experimental `main.rs` (agent-ui, tool run, scaffold validation, cargo/git helpers) |
| Language-only access | **None** |
| Injection tests | `;` `\|` `&` `$()` rejected in unit tests + suite |
| Sandbox claim | **Forbidden** — docs and error strings say NOT a sandbox |

---

## 5. Gates

| Gate | Result |
|---|:---:|
| Core suite 37/37 | **PASS** |
| Capability security suite | **26/26 PASS** (`scripts/test-capability-security.ps1`) |
| PATH mínimo | **PASS** |
| Default CLI denies shell/agent/net/package/bridge commands | **PASS** (exit 2, NOT executed) |
| Matter `agent`/`net`/`python` backend calls fail closed | **PASS** |
| Command-injection-style CLI names fail | **PASS** |
| Permanent Rust injection tests | **PASS** (8) |
| No secrets in deny diagnostics | **PASS** |
| Size near Phase 2 baseline | **PASS** (3 726 794 B, **+1 381 B**) |
| No `python3.dll` / OpenGL / MF | **PASS** |
| Valid Matter programs intact | **PASS** |

Evidence: `target/validation/phase_3_capability_security/`.

---

## 6. Risks

### Eliminated (default / language-only)

- PowerShell `-Command` execution from default binary
- CLI surface for agent-ui, shell, package-install, bridges, net-* on default binary
- Linked polyglot/network agent stacks on default binary
- Matter programs invoking unregistered `agent`/`net`/`python` backends (fail closed)

### Mitigated (experimental only)

- Local process execution: allowlist + injection reject + timeout + output limits + structured argv
- Non-allowlist requires **explicit** `MATTER_ALLOW_LOCAL_COMMANDS=1`
- Error/docs state **not a sandbox**

### Accepted residual

| Residual | Why accepted |
|---|---|
| Stdlib `file` / `fileio` write on language-only | Core language I/O; not shell/network |
| `compile` writes `.mbc` | Required language tool |
| `tool` backend stub (metadata only) | No OS spawn; no network |
| Experimental edition can still do powerful things when built and invoked | By design; not default; not sandbox |
| Allowlist is not OS isolation | Documented; list-block ≠ sandbox |
| Host process can still open files it is allowed to by OS | Outside Matter policy |

---

## 7. Experimental edition declaration

> **`matter-cli-experimental` is not a sandbox.**  
> It may spawn allowlisted (or explicitly override-enabled) host processes, use network backends, polyglot bridges, and agent tools when features are compiled in. Controls are **policy filters**, not isolation. Do not treat allowlists as multi-tenant security.

Activation remains **explicit**:

```powershell
cargo build -p matter-cli --release --target x86_64-pc-windows-gnu `
  --features experimental-full --bin matter-cli-experimental
```

Optional env:

| Env | Meaning |
|---|---|
| `MATTER_ALLOW_LOCAL_COMMANDS=1` | Allow non-allowlist commands (still no shell metacharacters) |
| `MATTER_LOCAL_COMMAND_TIMEOUT_SECS` | Spawn timeout (default 60) |
| `MATTER_LOCAL_COMMAND_MAX_OUTPUT` | Capture byte cap (default 262144) |

---

## 8. Binary metrics

| Metric | Phase 2 baseline | Phase 3 | Delta |
|---|---:|---:|---:|
| `matter-cli.exe` | 3 725 413 B | 3 726 794 B | **+1 381 B** |
| SHA-256 (post) | `A83233A7…` | `355F7406680A3D7B662A7728B92986DE53E87431794AD886310BBCDC1B3D6F22` | changed |
| DLL class | no python/opengl/mf | same | ok |

---

## 9. Permanent tests

| Test | Location |
|---|---|
| Injection / denylist unit tests | `crates/matter-cli/src/capability_policy.rs` `#[cfg(test)]` |
| Core language suite | `scripts/test-core-suite.ps1` |
| Capability security suite | `scripts/test-capability-security.ps1` |

```powershell
cargo test -p matter-cli --target x86_64-pc-windows-gnu --bin matter-cli -- capability_policy
cargo build -p matter-cli --release --target x86_64-pc-windows-gnu --bin matter-cli
.\scripts\test-core-suite.ps1
.\scripts\test-capability-security.ps1
```

---

## 10. Deliverables

| Deliverable | Path |
|---|---|
| This report | `PHASE_3_CAPABILITY_SECURITY.md` |
| Capability matrix | `capability-matrix.json` |
| Evidence root | `target/validation/phase_3_capability_security/` |
| Security suite results | `…/security-suite-results.json` |
| Post hashes | `…/sha256-post.json` |
| Permanent policy + tests | `crates/matter-cli/src/capability_policy.rs` |
| Security suite script | `scripts/test-capability-security.ps1` |

---

**Phase 3 status: COMPLETE.**  
**Stop here.** Do not start Phase 4 or unrelated cleanup without **new explicit approval**.
