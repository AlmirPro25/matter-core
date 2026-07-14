# Current status — Matter Core 0.1.0 baseline

**Date:** 2026-07-14  
**Tag intent:** `matter-core-v0.1.0-baseline`  
**Edition focus:** language-only Core  

## What works

- Parse / check / compile / run / run-bytecode for Core programs  
- MBC1 load with structural validation  
- VM limits (stack, call depth, instructions, etc.)  
- Capability isolation: no shell/agent/net/polyglot on default binary  
- Portable package install/update/uninstall scripts  
- Automated suites: Core, Security, Portable  

## What does **not** claim readiness

| Claim | Status |
|-------|--------|
| `production_ready` | **false** |
| Release Candidate | **not granted** |
| External clean-Windows validation | **BLOCKED** |
| Experimental edition as sandbox | **explicitly not a sandbox** |

## Gates (recorded)

| Suite | Result |
|-------|--------|
| Core | 37/37 PASS |
| Security | 26/26 PASS |
| Portable | 20/20 PASS |
| production-readiness-v2 | **BLOCKED_EXTERNAL_VALIDATION** |

Evidence copies: `docs/evidence/releases/0.1.0/`.  
Full reports: `docs/status/`.

## Blocker to RC

Run `scripts/external-windows-validation.ps1` on a clean Windows host with only the frozen ZIP + script. See `docs/status/EXTERNAL_WINDOWS_VALIDATION_V1.md`.

## Phases completed (docs)

1. Language-only isolation  
2. Parser / MBC1 / VM hardening  
3. Capability security  
4. Portable package  
5. Production readiness audit V2  
6. External validation attempt → BLOCKED (build host)  
