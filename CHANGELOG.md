# Changelog

All notable project milestones for Matter. Format is chronological; versions below 1.0 are baselines, not stability guarantees.

## [0.1.0-baseline] — 2026-07-14

### Matter Core (language-only)

- Default binary `matter-cli` / package `matter.exe` without polyglot/agent/visual/device by default  
- Hardened lexer/parser (illegal tokens, limits, nesting)  
- MBC1 structural validation before execution  
- VM structured errors and configurable resource limits  
- Capability policy: deny shell/agent/net/package commands on Core; experimental local-command allowlist (**not a sandbox**)  
- Portable Windows package scripts: package / install / verify / update / uninstall  
- Automated suites: Core 37/37, Security 26/26, Portable 20/20  

### Audits

- Production Readiness Audit V1: not production-ready  
- Production Readiness Audit V2: software gates green; **BLOCKED_EXTERNAL_VALIDATION** (no clean Windows proof)  
- External Windows Validation V1: **BLOCKED** on build host  

### Frozen local package (not required in git)

- `dist/matter-core-0.1.0-windows-x64.zip`  
- SHA-256: `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2`  

### Not included in this baseline claim

- Release Candidate marketing  
- `production_ready: true`  
- Stable 1.0  
- Public GitHub Release asset publish (unless separately authorized)  

## Earlier history

Prior sprints and experimental features (polyglot, visual, frontier, Sentinel bridges, etc.) remain in the tree under optional crates/features. See git history and `docs/status/` for detail.
