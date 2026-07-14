# PRODUCTION_READINESS_AUDIT_V2

**Product:** Matter Core 0.1.0 (language-only)  
**Date:** 2026-07-14  
**Scope:** Stabilization audit after Phases 1–4 — no new features, demos, bridges, or cosmetic refactors.  
**Machine role:** Development host (toolchains present).  

## Verdict

| Field | Value |
|---|---|
| **Verdict** | **`BLOCKED_EXTERNAL_VALIDATION`** |
| **production_ready** | **false** (forbidden to claim without clean independent Windows run) |
| **Near-RC software gates** | Core / Security / Portable / cargo tests / fuzz-stress / package / deps language-only: **PASS** |
| **Blocking for full RC claim** | Independent clean Windows validation **BLOCKED** (not simulated) |
| JSON | `production-readiness-v2.json` |
| Evidence root | `target/validation/production_readiness_v2/` |

Allowed verdicts used correctly: not `PRODUCTION_READY`.

---

## 0. Baseline freeze (Phase 4)

| Artifact | Value |
|---|---|
| CLI SHA-256 | `355F7406680A3D7B662A7728B92986DE53E87431794AD886310BBCDC1B3D6F22` |
| CLI size | 3 726 794 bytes |
| ZIP | `dist/matter-core-0.1.0-windows-x64.zip` |
| ZIP SHA-256 | `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2` |
| Baseline copy | `target/validation/production_readiness_v2/baseline_phase4/` |
| Pre-gates | Core 37/37, Security 26/26, Portable 20/20 (reconfirmed in V2 run) |

Syntax / MBC1 / VM semantics: **unchanged** in Phase 5.

---

## 1. Unified gate

**Script:** `scripts/production-readiness-v2.ps1`  
**Output:** `target/validation/production_readiness_v2/production-readiness-v2.json` (+ root copy)

| Gate | Mandatory | Status |
|---||:---:|---|
| core_suite | yes | **PASS** |
| security_suite | yes | **PASS** |
| portable_suite | yes | **PASS** |
| cargo_core_tests | yes | **PASS** |
| cargo_fmt_check | no | **WARN** (style drift; non-blocking policy) |
| cargo_clippy_core | no | **WARN** (`-D warnings` fails; clean compile with warnings allowed) |
| fuzz_stress | yes | **PASS** |
| package_integrity | yes | **PASS** |
| package_repro | yes | **PASS_LOGICAL** |
| independent_windows | yes | **BLOCKED** |
| dependency_audit | yes | **PASS** (0 critical/high in language-only tree) |
| dangerous_caps_denied | yes | **PASS** |

Fail-fast applies to mandatory **FAIL** only; **BLOCKED** is explicit and does not become a silent PASS.

---

## 2. Static analysis

### 2.1 `cargo fmt --check`

- **Result:** exit 1 (formatting drift remains in workspace).  
- **Policy:** non-mandatory for RC; recorded as WARN. Mass reformat deferred (would be cosmetic / out of Phase 5 “no cosmetic changes” spirit).

### 2.2 `cargo clippy` (core crates)

- Crates: `matter-lexer`, `matter-parser`, `matter-bytecode`, `matter-vm`, `matter-runtime`.  
- `-D warnings`: fails (exit 101).  
- Without deny: **compiles** (exit 0).  
- Logs: `clippy-core.log`, `clippy-core-allow.log`.  
- **Policy:** warnings allowed for this audit; compile errors would FAIL.

### 2.3 unwrap / expect / panic / unsafe inventory

**File:** `unwrap-unsafe-inventory.json`

| Metric | Count (approx.) |
|---|---:|
| Scanned hits (core crates src) | 244 |
| Filtered “core path” samples | 149 |
| Residual categories | justified list J1–J7 |

**Corrected in earlier phases (not re-broken):**

- Lexer garbage → `Illegal`  
- Parser limits / trailing tokens  
- VM closure `unwrap` → structured error  
- MBC1 validate-before-run  
- PowerShell shell path removed from default binary  

**Remaining (justified, not mechanical removal):**

| ID | Item | Why kept |
|---|---|---|
| J1 | `Parser::expect` method | Returns `ParseResult`, not panic API |
| J2 | Lexer `parse().unwrap_or(0)` | Soft fallback on digit runs |
| J3–J4 | Test-module unwraps | Harness only |
| J5 | Some CLI JSON encode unwraps | Self-built values; partial hardening done |
| J6 | VM `unsafe` instruction pointers | Pre-existing design; structural MBC validation reduces OOB |
| J7 | Experimental `main.rs` volume | Not linked into language-only binary |

No new user-reachable panic path found in Phase 5 fuzz corpus.

---

## 3. Robustness (fuzz / adversarial / limits)

**Script:** `scripts/test-fuzz-stress-v2.ps1`  
**Results:** `fuzz-stress-results.json`

| Check | Result |
|---|---|
| Deterministic fuzz 128 seeds (`0xC0FFEE42`) | **0 panic, 0 hang**, max check ~1.1 s |
| Adversarial sources (garbage, illegal, deep nest, truncate, div0, huge ident) | no crash |
| MBC empty/bad/random | reject exit ≠ 0 |
| `MATTER_VM_MAX_INSTRUCTIONS=50000` on long loop | limit hit (exit ≠ 0) |
| Soak compile+run-bytecode ×40 | 40/40 OK; WS samples ~68–71 MB (no runaway growth observed) |
| Repeat core programs | 15/15 OK |

**Not claimed:** formal absence of all leaks from a single short sample — soak is multi-cycle with memory samples only on the host process.

---

## 4. Dependencies / SBOM / licenses

| Artifact | Path |
|---|---|
| Dependency audit | `dependency-audit.json` |
| Package list (tree heuristic) | `sbom-packages.json` |
| cargo tree | `cargo-tree-matter-cli.txt` |
| cargo-audit raw | `cargo-audit.txt` / `.json` |

| Metric | Value |
|---|---|
| Tool | **cargo-audit** available |
| Language-only tree packages (approx.) | 32 |
| Critical/high **in language-only tree** | **0** |
| Workspace-wide advisories (lockfile) | 7 (e.g. **pyo3**, quick-xml) — **not in language-only tree** |
| Licenses | Project `LICENSE` + crates.io deps statically linked; CRT system |

**Policy:** critical/high in language-only tree → FAIL; tool missing → BLOCKED; workspace-only (polyglot) noted, not ignored, not blocking language-only RC software gate.

---

## 5. Reproducibility

**Script:** `scripts/test-package-repro-v2.ps1`  
**Results:** `repro-compare.json`

| Criterion | Result |
|---|---|
| Two package dirs from same CLI | file set equal (16 files) |
| Per-file SHA-256 (excl. MANIFEST/SHA256SUMS timestamps) | **equal** |
| ZIP byte-identical | **Not claimed** |

**Why ZIP bytes may differ:** `System.IO.Compression.ZipFile` writes entry timestamps / metadata; Windows packaging is not bit-reproducible without fixed DOS times.  
**RC criterion used:** logical content hashes (**PASS_LOGICAL**), honestly labeled — not “byte-reproducible build”.

---

## 6. Independent Windows environment

| Item | Status |
|---|---|
| Clean Windows without Rust/Cargo/GCC/Python/Node | **Not available in this session** |
| Gate | **BLOCKED** (never PASS by simulation) |
| Host | Development machine with MinGW/Rust present |

**Blocker for `RELEASE_CANDIDATE` / any stronger claim:** run `scripts/test-portable-release.ps1` + ZIP hash verify on a clean VM and flip this gate to PASS with evidence under `production_readiness_v2/independent_windows/`.

---

## 7. Disposition of Audit V1 findings

| ID | Title | Disposition | Evidence |
|---|---|---|---|
| **R-C1** | Release needs `python3.dll` | **Resolved** (language-only) | Phase 1 isolation; DLL inventory no python; PATH mínimo PASS |
| **R-C2** | PowerShell `run_local_command_capture` | **Resolved** default; **Mitigated** experimental | Phase 3: no PS in language-only; experimental allowlist+injection+timeout; NOT a sandbox |
| **R-C3** | Agent/HTTP surface in default | **Resolved** default | Features off; CLI deny list exit 2 |
| **R-A1** | VM closure unwrap | **Resolved** | Phase 2 → `VmError` |
| **R-A2** | unwrap volume CLI/bridges | **Mitigated** | language-only path reduced; residual inventory J*; experimental still large |
| **R-A3** | Garbage accepted | **Resolved** | Phase 2 Illegal + full consumption |
| **R-A4** | No resource limits | **Resolved** (defaults) | Phase 2 limits + env overrides; V2 limit test |
| **R-A5** | Polyglot package managers | **Resolved** default | Polyglot not in language-only |
| **R-A6** | Perf scripts F: | **Accepted residual** | Non-Core scripts/docs; not package runtime |
| **R-M1** | check vs runtime | **Accepted** | Documented: check is static; div0 fails at run |
| **R-M2** | MBC loader UX on `run` | **Mitigated** | `run-bytecode` + validate; `run` still source-oriented |
| **R-M3** | D: hardcodes install/LSP | **Resolved** for Core path | Phase 4 portable scripts + LSP discovery |
| **R-M4** | unsafe native/memory | **Accepted residual** | Not default language-only surface; still present in workspace |
| **R-M5** | OpenGL/MF in language bin | **Resolved** | Language-only DLL set |
| **R-M6** | Low unit test coverage | **Mitigated** | Core crates + suites expanded; not full coverage metric |
| **R-B1** | panic in domain backends | **Mitigated** | Not registered language-only |
| **R-B2** | Historical F: docs | **Accepted residual** | Docs only |
| **R-B3** | experimental self-claim | **Kept intentional** | `production_ready:false` still honest |

### New risks introduced in Phases 1–4 (checked)

| Risk | Status |
|---|---|
| False sense of sandbox from allowlists | **Documented** (Phase 3 declaration) |
| ZIP non-bit-reproducible packaging | **Documented** (PASS_LOGICAL) |
| Stdlib file write still available | **Accepted** language I/O residual |
| Independent install not proven on clean VM | **Open / BLOCKED gate** |

---

## 8. Production blockers (if aiming beyond this audit)

1. **Execute and archive independent clean Windows validation** of ZIP + install + update + uninstall.  
2. Optional: fix `cargo fmt` workspace drift and reduce clippy warnings for stricter CI.  
3. Optional: upgrade/workspace-prune crates that still trigger workspace-wide audit noise (pyo3 etc.) even if not in language-only tree.  
4. Do not market as production-ready until (1) is green and product checklist is signed.

---

## 9. Deliverables

| Deliverable | Path |
|---|---|
| This report | `PRODUCTION_READINESS_AUDIT_V2.md` |
| Gate JSON | `production-readiness-v2.json` |
| Unified script | `scripts/production-readiness-v2.ps1` |
| SBOM / deps | `target/validation/production_readiness_v2/dependency-audit.json`, `sbom-packages.json` |
| Unwrap inventory | `…/unwrap-unsafe-inventory.json` |
| Fuzz/stress | `…/fuzz-stress-results.json` |
| Repro compare | `…/repro-compare.json` |
| Phase 4 baseline | `…/baseline_phase4/` |

---

## 10. Final statement

Software gates for Matter Core **0.1.0 language-only** (suites, cargo tests, adversarial fuzz smoke, package integrity, logical package repro, dependency posture for the slim binary, capability isolation) are **green**.

**External validation on a clean Windows system was not performed** → overall verdict is **`BLOCKED_EXTERNAL_VALIDATION`**, not `RELEASE_CANDIDATE`, and **not** `PRODUCTION_READY`.

**Phase 5 complete.** No publish, signing, GUI installer, or next phase without new explicit approval.

---

## 11. Phase 6 follow-up (external Windows) — 2026-07-14

| Item | Result |
|---|---|
| Attempt | Official external validation harness executed |
| Independent host available? | **No** (agent host is build/dev machine: Rust, Cargo, Python, Node, `D:\mingw64`, `D:\Matter`) |
| Frozen ZIP hash | Still `0A5FEE59…` — package **not modified** |
| Phase 6 verdict | **`BLOCKED`** — see `EXTERNAL_WINDOWS_VALIDATION_V1.md` |
| Overall V2 verdict | Remains **`BLOCKED_EXTERNAL_VALIDATION`** |
| RELEASE_CANDIDATE | **Not granted** |
| production_ready | **false** |

Harness for a future clean machine: `scripts/external-windows-validation.ps1` + `scripts/EXTERNAL_VALIDATION_RUNBOOK.md`.
