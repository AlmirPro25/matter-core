# EXTERNAL_WINDOWS_VALIDATION_V1

**Product:** Matter Core 0.1.0 language-only  
**Phase:** 6 — external package validation  
**Date:** 2026-07-14  

## Verdict

| Field | Value |
|---|---|
| **Verdict** | **`BLOCKED`** |
| Meaning | No independent clean Windows environment was available to complete the official validation |
| Frozen ZIP | `dist/matter-core-0.1.0-windows-x64.zip` |
| Expected SHA-256 | `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2` |
| Package modified? | **No** |

**Not used:** `EXTERNAL_VALIDATION_PASS` (would require a real independent host).  
**Not used:** `EXTERNAL_VALIDATION_FAIL` (package was not shown to fail on a clean host; validation could not start).

---

## 1. Why this host is disqualified

The agent environment is the **same development/build Windows** used for Phases 1–5.

| Disqualifier | Observed |
|---|---|
| Build/dev host identity | Windows 10 Pro 10.0.19045 x64; computer_id_hash `e8e7407b19a9f727` (anonymized) |
| Rust / Cargo | **present** |
| Python | **present** |
| Node | **present** |
| MinGW tree (`D:\mingw64`) | **present** |
| Prior `D:\Matter` | **present** |
| Project tree / `target/` accessible | **yes** |
| Windows Sandbox EXE | **not found** / Hyper-V feature query needs elevation |

Per Phase 6 rules, this **cannot** yield `EXTERNAL_VALIDATION_PASS`, even if the ZIP runs here.

Evidence: `target/validation/external_windows_validation_v1/blocked_build_host_run/`  
JSON: `external-windows-validation-v1.json`

---

## 2. Frozen artifact preflight (build host only)

| Check | Result |
|---|---|
| ZIP present | yes |
| SHA-256 computed on build host | `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2` |
| Matches expected | **yes** |
| ZIP size | 2 676 017 bytes |

**Note:** Hash match on the build host is necessary but **not sufficient** for external validation.

File: `target/validation/external_windows_validation_v1/build-host-zip-hash.json`

---

## 3. Tests not executed as official external (blocked before extract pipeline)

The harness **stops** when independence fails (by design), so install/update/uninstall/PATH/SmartScreen steps were **not** claimed as external results.

What *would* run on a clean machine (script ready):

1. Hash before extract  
2. Extract under path with spaces  
3. `--help` / `--version` / `core-status-json` / run / compile / run-bytecode  
4. Install outside `D:\Matter`  
5. `verify-matter-core.ps1`  
6. User project + update preserve  
7. Uninstall selective (manifest vs user vs foreign files)  
8. Dangerous commands → exit 2 + NOT executed  

Script: `scripts/external-windows-validation.ps1`  
Runbook: `scripts/EXTERNAL_VALIDATION_RUNBOOK.md`

---

## 4. How to complete Phase 6 (operator action)

On a **different** Windows PC (or clean VM) **without** Rust/Cargo/GCC/Python/Node/`D:\Matter`:

```powershell
# Folder contains ONLY:
#   matter-core-0.1.0-windows-x64.zip
#   external-windows-validation.ps1

powershell -NoProfile -ExecutionPolicy Bypass -File .\external-windows-validation.ps1 `
  -ZipPath .\matter-core-0.1.0-windows-x64.zip
```

Return the generated `WorkRoot` (logs + `external-windows-validation-v1.json`).

| Script exit | Verdict to record |
|------------:|-------------------|
| 0 | EXTERNAL_VALIDATION_PASS → can update V2 to RELEASE_CANDIDATE |
| 1 | EXTERNAL_VALIDATION_FAIL → keep package frozen; fix in 0.1.0-rc.2 |
| 2 | BLOCKED |

Do **not** install toolchains to hide failures before recording them.

---

## 5. Impact on Production Readiness V2

| Field | Before Phase 6 | After Phase 6 |
|---|---|---|
| V2 verdict | `BLOCKED_EXTERNAL_VALIDATION` | **unchanged** `BLOCKED_EXTERNAL_VALIDATION` |
| production_ready | false | **false** |
| RELEASE_CANDIDATE | not granted | **not granted** |
| Stable 1.0 | not claimed | **not claimed** |

Controlled update: `production-readiness-v2.json` gains a `phase6_external` note pointing here; software gates remain as previously recorded.

---

## 6. Decision

| Decision | **BLOCKED** |
|---|---|
| Package integrity (hash on build host) | OK preflight only |
| External functional proof | **Not obtained** |
| Next step | Run official script on clean Windows; then re-open Phase 6 completion |

**No publish, no signing, no package alteration.**
