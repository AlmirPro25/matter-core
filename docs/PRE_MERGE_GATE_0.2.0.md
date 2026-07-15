# Pre-Merge Gate — Matter 0.2.0 Semantic Honesty

**Date:** 2026-07-15  
**Branch:** `develop/0.2.0-semantic-honesty`  
**Recommendation:** **MERGE_BLOCKED**  
**production_ready:** false  
**RELEASE_CANDIDATE:** false  

No merge was performed. No 0.2.0 release was produced.

---

## 1. Remote / commits

| Item | Result |
|------|--------|
| Remote | `github-matter-core` → `https://github.com/AlmirPro25/matter-core.git` |
| Feature commit | `08a2f9f4c998fc045ad849027d76f49eae3bb136` **pushed** |
| Build-compat commit (gate) | `de693a494887f515bf5a8a36df41fbec5bfce8fe` — `Token::Illegal` arm for experimental-full |
| Remote branch HEAD | `de693a4…` on `develop/0.2.0-semantic-honesty` |

Confirm command: `git ls-remote github-matter-core develop/0.2.0-semantic-honesty`

---

## 2. Executable under test (anti-stale)

Gate **refused** silent use of a stale binary.

| Path | SHA-256 | mtime (UTC) |
|------|---------|-------------|
| **`target\release\matter-cli.exe` (USED)** | `FA045DF7B76EC10FBE9F74C81478AD7282CEAA11C82CBDD83CC47BEF4A98AE08` | 2026-07-14T23:36:20Z |
| `target\x86_64-pc-windows-gnu\release\matter-cli.exe` (STALE) | `355F7406680A3D7B662A7728B92986DE53E87431794AD886310BBCDC1B3D6F22` | 2026-07-14T21:28:20Z |

- Dual binaries **differ**. Newest-by-mtime selected: `target\release\matter-cli.exe`.
- CLI version string reports: language-only, development track 0.2.0, `production_ready=false`.
- Fingerprint: `target/validation/pre_merge_0_2_0/cli_fingerprint.json`
- Git commit at gate start for feature: `08a2f9f`; HEAD after build fix: `de693a4`.

### Script path audit (risk)

| Script | Default preference |
|--------|-------------------|
| `test-core-suite.ps1` | **gnu_only** (stale risk) |
| `test-capability-security.ps1` | **gnu_only** |
| `test-portable-release.ps1` | **gnu_only** |
| `test-fuzz-stress-v2.ps1` | **gnu_only** |
| `production-readiness-v2.ps1` | **gnu_only** |
| `test-semantic-honesty-0.2.ps1` | **release_first** (correct) |
| `package-matter-core.ps1` | gnu_first |

Gate runs used explicit `-Cli` / `-CliPath` pointing at `target\release\matter-cli.exe`.  
Evidence: `target/validation/pre_merge_0_2_0/cli_script_audit.json`

---

## 3. Suites

| Suite | Expected | Observed | Status |
|-------|----------|----------|--------|
| Semantic Honesty | 37/37 | 37/37 | **PASS** |
| Core | 37/37 | 37/37 | **PASS** |
| Security | 26/26 | 26/26 | **PASS** |
| Portable | 20/20 | **21/21** (suite grew) | **PASS** (all green; count changed) |

Evidence:

- `target/validation/matter_0_2_semantic_honesty/summary.json`
- `target/validation/phase_2_core_hardening/core-suite-results.json`
- `target/validation/pre_merge_0_2_0/suite_security_raw.txt`
- `target/validation/pre_merge_0_2_0/suite_portable_raw.txt`

---

## 4. Cargo quality

| Check | Result |
|-------|--------|
| `cargo test` core crates (lexer, parser, bytecode, vm, runtime, stdlib, cli) | **PASS** (all exit 0) |
| `cargo fmt --all -- --check` | **FAIL** (pre-existing + workspace noise) |
| `cargo fmt` limited to 0.2.0 crates | **FAIL** (e.g. `matter-stdlib/src/vec.rs` whitespace) |
| `cargo clippy … -- -D warnings` | **FAIL** (bytecode/stdlib lints) |
| `cargo clippy` without `-D warnings` | **PASS** (exit 0) |

Logs under `target/validation/pre_merge_0_2_0/cargo_*`.

---

## 5. Builds

| Build | Result |
|-------|--------|
| language-only (`matter-cli`) | **PASS** |
| experimental-full (`matter-cli-experimental`) | **PASS after `de693a4`** (failed on pure `08a2f9f` with missing `Token::Illegal` match arm) |

Experimental fingerprint: `target/validation/pre_merge_0_2_0/experimental_fingerprint.json`  
SHA-256 (release): `29FDE11ED708C19C0763773A549840616371AB590145B9C98E549F3283B497F1`

---

## 6. File Capabilities edge audit

### Results (junction / path forms)

| Case | Result |
|------|--------|
| Read/write/delete via **junction** to outside root | **DENIED** (`capability_denied`) |
| New file under authorized root (parent canonicalize) | **PASS** |
| UNC-like `//server/share/...` | **DENIED** |
| Device path `//./C:/...` | **DENIED** |
| ADS / extra colon stream | **DENIED** |
| Case-fold access inside root | **PASS** (Windows allow or deny — both acceptable) |
| Similar prefix (`sandbox` vs `sandbox_evil`) | **DENIED** |
| Root missing at grant time | **DENIED** |
| Full sandbox claim | **None** (`is_sandbox: false`) |

### Symlink file (not admin)

Creating file symlinks failed: **requires Administrator** (`SeCreateSymbolicLinkPrivilege`).  
Symlink R/W/D escape tests = **NOT EXECUTED on this host**.  
Junction (directory reparse) is the verified Windows reparse case.

### Policy parity (run / eval / bytecode / JSON)

| Channel | capability_denied without allow? |
|---------|----------------------------------|
| `run` | YES |
| `run-json` | YES (`error` / `error_code`: `capability_denied`) |
| `run-bytecode` | YES |
| `run-bytecode-json` | YES |
| `eval` / `eval-json` | YES when argv preserves quotes (via `cmd`); same `Runtime::with_fs_policy` in code |

Code paths all call `Runtime::with_fs_policy` (`commands/run.rs`, `language_main.rs`).

Evidence: `fs_edge_summary.json`, `policy_parity.json`, `fs_edge2/`.

### Residual TOCTOU / non-sandbox risks (open)

1. **TOCTOU:** `check_path` resolves, then the backend performs a later `open`/`write`/`delete` — concurrent replacement of a path component can race.  
2. **Not a sandbox:** no AppContainer, integrity levels, or kernel isolation.  
3. **Symlink files** not host-proven without admin.  
4. **Root grant is a snapshot** at flag parse; replacing the root directory after grant is best-effort.  
5. **Case folding** on Windows can make alternate-case paths resolve inside a root.  
6. **PowerShell eval quoting** is fragile; does not change the shared policy implementation.

---

## 7. Frozen 0.1.0 artifacts — **CRITICAL FAILURE**

| Check | Expected | Observed |
|-------|----------|----------|
| Tag `matter-core-v0.1.0-baseline` | intact | **OK** (`6ca31f3fbb44425f5fa7f950b9d8f9a8b8965129`) |
| ZIP SHA-256 | `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2` | **MISMATCH** |
| Current ZIP SHA-256 | — | `938ACA2F9807B10DCC261B5801ED7907484C42CDA5EAC42952105D8C6BE4296D` |
| `dist/SHA256SUMS` | still records `0a5fee59…` | **out of sync with file** |
| `dist/*.meta.json` | — | rewritten with new hash |
| `D:\Matter` | not replaced by 0.2.0 | **OK** (still `matter-cli 0.1.0 language-only`) |

**Cause:** running `scripts/test-portable-release.ps1` during this gate **repackaged** `dist/matter-core-0.1.0-windows-x64.zip` with a newer CLI (size 2 676 017 → 2 707 058).  

This violates the 0.1.0 freeze guarantee for the working tree copy of the ZIP.  
**Restore from a known-good backup / rebuild from baseline tag is required before any merge or release.**

No secrets or temp host secrets found in commit `08a2f9f` content scan (paths under `C:\Users` not committed; policy errors use `capability_denied`).

---

## 8. Open risks and regressions

### Blockers (merge)

1. **Frozen ZIP 0.1.0 hash broken** on disk (see §7).  
2. **Validation scripts default to potentially stale gnu binary** (false confidence risk).  
3. **`cargo fmt --check` / `clippy -D warnings` not green.**  
4. **Symlink file escape not verified** on this machine (admin).  

### Residual / accepted for later

- TOCTOU (above).  
- Portable suite case count 21 vs historical 20.  
- experimental-full required `de693a4` after `08a2f9f`.  
- Not production_ready / not RC (by design).

### No regressions observed in

- Core 37, Security 26, Semantic Honesty 37.  
- Default-deny FS + distinct read/write/delete.  
- Baseline git tag.

---

## 9. Recommendation

# **MERGE_BLOCKED**

Do **not** merge `develop/0.2.0-semantic-honesty` into `main` until:

1. Restore or re-prove bit-identity of `dist/matter-core-0.1.0-windows-x64.zip` to SHA-256 `0A5FEE59…332A2` (or re-issue freeze docs with explicit new baseline — not recommended silently).  
2. Fix portable packaging so it **never** overwrites the frozen 0.1.0 ZIP during tests.  
3. Fix suite defaults to prefer `target\release\matter-cli.exe` (or newest-by-mtime + hash log).  
4. Optionally clean fmt/clippy `-D warnings` for Core crates.  
5. Optionally re-run symlink escape tests with admin privilege.

Language-only 0.2.0 semantics (honesty + file caps) themselves are **functionally validated** on the correct binary; the block is about **freeze integrity, process hazards, and quality gates**, not silent module/type lies.

---

## 10. Deliverables map

| Deliverable | Path |
|-------------|------|
| This report | `docs/PRE_MERGE_GATE_0.2.0.md` |
| Machine JSON | `pre-merge-gate-0.2.0.json` and `target/validation/pre_merge_0_2_0/pre-merge-gate-0.2.0.json` |
| Evidence root | `target/validation/pre_merge_0_2_0/` |

**Stop condition:** gate complete; **merge not performed**.
