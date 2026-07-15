# Matter LSP Delivery Merge Milestone

**Date:** 2026-07-15  
**Status:** **LSP_DELIVERED_IN_MAIN**  
**Not claimed:** LSP_RELEASED · release_candidate · production_ready  
**Remote:** `github-matter-core`  
**Merge type:** `git merge --no-ff`  
**Force push:** not used  

---

## Commits

| Role | SHA | Message |
|------|-----|---------|
| Feature delivery | `a558c68` | feat(lsp): deliver versioned matter-lsp.exe and wire VS Code packaging |
| Path scrub | `a465b10` | chore(lsp): scrub personal paths before delivery push |
| Parent main | `6f1aa61` | LSP engine merge milestone |
| **Merge commit** | `a492e84` | `merge: deliver standalone Matter LSP v1` |
| Milestone doc | (this commit) | delivery merge milestone |

---

## Delivered in main

| Item | Detail |
|------|--------|
| Binary | versioned **`matter-lsp.exe`** (`crates/matter-lsp` bin) — stdio only |
| CLI language-only | **unchanged** independence (`language_main` without `lsp` subcommand) |
| VS Code | auto-discover `matter-lsp.exe`; `matter.lsp.path` default empty; clear error if missing |
| Package script | includes `bin/matter-lsp.exe` + SHA256SUMS entry (temp 0.2.0-dev layout) |
| Permanent smoke | `scripts/test-matter-lsp-binary.ps1` |
| Portable | validates packaged LSP, space paths, dedicated start script |

---

## Pre-merge gates (PASS)

| Gate | Result |
|------|--------|
| Feature push (no secrets / no personal paths after scrub) | PASS → remote `a465b10` |
| Clean worktree build `matter-cli` language-only | PASS |
| Build `matter-lsp.exe` | PASS |
| `cargo test -p matter-lsp` | **15/15** |
| JSON-RPC on versioned binary only | PASS (no orphan) |
| Extension resolve explicit path | PASS |
| Package temp `0.2.0-dev` | PASS |
| Portable | **27/27** |
| Core / Semantic / Security | **37 / 37 / 26** |
| ZIP 0.1.0 | **`0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2`** |

### Executable hashes (pre-merge clean tree)

| Binary | SHA-256 | Size |
|--------|---------|------|
| `matter-cli.exe` | `DDBEB266D3C82FBC88D5DA06C78F7F02A70DBB0DAF1BF76FCFB2DE7D4B3724C6` | 3762486 |
| `matter-lsp.exe` | `2941C758ECDDA004CC854325D4A0470A8581275B94D4D728AAB37D9F825C3B16` | 7908159 |

Evidence: `target/validation/lsp_delivery_merge_gate/`

---

## Post-merge recheck (on merge commit)

| Gate | Result |
|------|--------|
| LSP binary smoke | PASS |
| Portable | **27/27** |
| Core / Semantic / Security | **37 / 37 / 26** |
| Extension resolve → matter-lsp.exe | PASS |

---

## Explicit non-claims

- **Not** LSP_RELEASED  
- **Not** a 0.2.0 release or tag  
- **Not** production_ready / release_candidate  
- Frozen **0.1.0 ZIP** not modified  
- No package-resolver, polyglot, network server, or closures feature work  

---

## Status claim

**Matter Core main** includes **standalone Matter LSP delivery** (binary + packaging hooks + VS Code wiring).  

Official **0.2.0 package release** remains a separate decision.
