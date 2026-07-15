# Artifact Recovery and Preservation Hotfix — Matter Core 0.1.0

**Date:** 2026-07-15  
**Branch:** `develop/0.2.0-semantic-honesty`  
**Recommendation:** **MERGE_APPROVED** (with residual open actions below)  
**Tag `matter-core-v0.1.0-baseline`:** not modified  
**Release 0.2.0:** not generated  
**Merge to main:** not performed  

---

## 1. Quarantine (overwritten ZIP)

| Field | Value |
|-------|--------|
| Quarantine path | `dist/matter-core-0.1.0-windows-x64.OVERWRITTEN-938ACA2F.zip` |
| SHA-256 | `938ACA2F9807B10DCC261B5801ED7907484C42CDA5EAC42952105D8C6BE4296D` |
| Size | 2 707 058 bytes |
| mtime (UTC) | 2026-07-15T00:15:37Z |
| Probable origin | Pre-Merge Gate 0.2.0: `test-portable-release.ps1` → `package-matter-core.ps1` rewrote `dist/matter-core-0.1.0-windows-x64.zip` with a newer CLI |
| Deleted? | **No** (kept for investigation) |
| Claimed as original? | **No** |

Evidence: `target/validation/artifact_recovery_0_1_0/quarantine.json`

---

## 2. Search for original (full SHA only)

**Required SHA-256:**

`0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2`

| Location | Result |
|----------|--------|
| GitHub Draft Release (private) | **Not accessible** (`gh` not authenticated) |
| `dist/matter-core-0.1.0-repro-windows-x64.zip` | Different hash (`4F9059C6…`) — rejected |
| Downloads / Desktop / TEMP | Scanned; no original except via repo validation |
| **Found** | `target/validation/production_readiness_v2/baseline_phase4/matter-core-0.1.0-windows-x64.zip` |

Full SHA match confirmed (size 2 676 017).  
Evidence: `target/validation/artifact_recovery_0_1_0/search_results.json`

**Status:** `ARTIFACT_ORIGINAL_RECOVERED`  
**Not** `ARTIFACT_ORIGINAL_NOT_RECOVERED`.

---

## 3. Restoration

| Step | Status |
|------|--------|
| Restore to `dist/matter-core-0.1.0-windows-x64.zip` | **Done** |
| Re-verify SHA-256 | **Match** `0A5FEE59…332A2` |
| Read-only attribute | **Set** (`IsReadOnly=true`) |
| Second copy outside build tree | **Done:** `D:\Users\almir\MatterArtifactVault\matter-core-0.1.0-windows-x64.ORIGINAL-0A5FEE59.zip` (read-only) |
| Sidecars vaulted | `original-MANIFEST.json`, `original-SHA256SUMS`, `sha256-baseline.json` under vault |
| Draft Release private on baseline tag | **Not done** — `gh auth` unavailable; **manual follow-up required** |

Evidence: `target/validation/artifact_recovery_0_1_0/recovery.json`

The overwritten file was **never** relabeled as original.

---

## 4. Portable suite permanent fix

### `scripts/package-matter-core.ps1`
- Default output under `target/validation/packages/.../root` (not `dist/`)
- Writing under `dist/` requires **`-AllowDistWrite`**
- Existing outputs refused without **`-ForceOverwrite`**
- ZIP path must **not** sit inside package content root
- CLI selection: **newest by mtime** among `target\release` and `gnu\release`; logs path + SHA-256
- Records `cli_path` / `cli_sha256` in package meta

### `scripts/test-portable-release.ps1`
- Packages only into `target/validation/phase_4_portable_release/temp-package-*/`
- **Never** writes to `dist/`
- Snapshots all `dist/*` hashes before/after; **`dist-immutable` gate fails if any change**
- Fingerprints CLI used
- Relative “package-clean” check (does not false-positive on `...\target\validation\...`)
- Verifies frozen ZIP hash remains `0A5FEE59…`

### CLI defaults
- `test-core-suite.ps1` and `test-capability-security.ps1` now pick freshest binary (not gnu-only stale)

---

## 5. Format / clippy

| Check | Result |
|-------|--------|
| `cargo fmt` on branch crates (bytecode, parser, runtime, stdlib, cli) | Applied |
| `cargo fmt --check` (same packages) | **PASS** (exit 0) |
| Clippy **introduced by 0.2.0** | Fixed: `fs_capability` useless `format!`; removed unused `strip_fs_flags_keep_rest` |
| Clippy **pre-existing** (not cleaned en masse) | Still present under `-D warnings`: e.g. `tensor.rs`, string `repeat().take()`, `map_or` in stdlib, bytecode validate/recursion patterns |

No broad historical cleanup.

---

## 6. Retest (post-recovery)

| Gate | Result |
|------|--------|
| Semantic Honesty | **37/37 PASS** |
| Core | **37/37 PASS** |
| Security | **26/26 PASS** |
| Portable | **24/24 PASS** (suite expanded with immutability/fingerprint; previously ~21) |
| cargo test Core crates | **PASS** |
| language-only build | **PASS** |
| experimental-full build | **PASS** |
| fmt --check (branch packages) | **PASS** |
| dist/ immutability during portable | **PASS** (`changes=0`, `wrote_to_dist=false`) |
| Frozen ZIP before/after retest | **Unchanged** `0A5FEE59…332A2` |

CLI under test: `target\release\matter-cli.exe`  
SHA-256: `616B8BA1551287DD31A6754557781C33D29E53D67BBB6F2699DE4844A4E5C646`

---

## 7. Open residuals

1. **Draft Release:** attach vault ZIP + MANIFEST + SHA256SUMS to a **private** draft on tag `matter-core-v0.1.0-baseline` after `gh auth login` (or web UI).  
2. Pre-existing clippy `-D warnings` noise (not introduced zero-tolerance failures after branch fixes).  
3. Quarantined OVERWRITTEN zip may be deleted **only after** investigation closed and draft release (or vault) confirmed durable.

---

## 8. Recommendation

# **MERGE_APPROVED**

Rationale:
- Original 0.1.0 ZIP recovered by full SHA match, restored to `dist/`, verified, vaulted, read-only.  
- Portable packaging no longer mutates `dist/`.  
- Suites and fmt gate green on the freshest CLI.  
- Tag and 0.1.0 freeze semantics restored for the working tree.

**Residual (non-blocking for code merge, required for full archival):** private Draft Release attachment still pending authentication.

**Not done (by design of this hotfix):** merge to `main`, release 0.2.0, tag rewrite, claiming OVERWRITTEN zip as original.

---

## 9. Deliverables

| Item | Path |
|------|------|
| This report | `docs/ARTIFACT_RECOVERY_0.1.0.md` |
| Machine JSON | `artifact-recovery-0.1.0.json` |
| Evidence | `target/validation/artifact_recovery_0_1_0/` |
| Vault | `D:\Users\almir\MatterArtifactVault\` |
