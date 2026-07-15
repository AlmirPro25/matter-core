# Matter LSP Delivery Assessment and Implementation v1

**Date:** 2026-07-15  
**Branch:** `feature/lsp-delivery-v1`  
**Base:** `main` @ `6f1aa61` (LSP **engine merged**)  
**Merge to main:** **not performed** (requires separate approval)  
**Release / tag:** **not created**

---

## Recommendation

### **LSP_DELIVERY_READY**

Gates on this branch passed. Ready for review/merge after explicit approval.  
Do **not** call the product “LSP released” until a real 0.2.0 package is published (this work enables delivery; frozen **0.1.0 ZIP is unchanged**).

---

## 1. Audit

### VS Code extension (before)

| Item | Finding |
|------|---------|
| Start command | `matter.lsp.path` default **`matter-cli`** with args **`['lsp']`** |
| Reality | language-only `matter-cli` (**`language_main.rs`**) has **no** `lsp` subcommand |
| Docs | README / QUICK_START / SUMMARY / install.md claimed `matter-cli` + `lsp` |
| Personal paths | User-local `.vscode` could point at absolute CLI; **defaults** used `matter-cli` on PATH (not a hard-coded drive) |

### Docs claiming availability

| Location | Claim | Correction |
|----------|--------|------------|
| `docs/MATTER_LSP_V1_MERGE_MILESTONE.md` | merge status | Clarified: **LSP engine merged**, not delivered |
| `vscode-extension/*` | matter-cli lsp | Updated to **matter-lsp.exe** + auto-discover |
| Archive docs (`docs/archive`, sprints) | historical `matter-cli lsp` | Left as history; delivery doc is source of truth |

### Dependencies of `matter-lsp` (engine)

Path crates: `matter-lexer`, `matter-parser`, `matter-ast`, `matter-bytecode`, `matter-error`.  
External: `tower-lsp`, `tokio` (rt-multi-thread/full), `serde`/`serde_json`.  
**Not** linked: package-resolver, polyglot bridges, Python, GUI.

### Size / DLL impact (measured release binary)

| Binary | Size (approx) | Notes |
|--------|---------------|--------|
| `matter-cli.exe` language-only | ~3.6 MB | Unchanged build path |
| `matter-lsp.exe` | ~7.5 MB (`7908069` bytes) | Separate package; optional |

ASCII PE import scan of `matter-lsp.exe` (not a full dumpbin inventory):  
`kernel32`, `ntdll`, `msvcrt`, `userenv`, `ws2_32`, `bcryptprimitives`, `api-ms-win-core-synch-*`.  
`ws2_32` comes with Tokio/runtime linkage; the LSP **does not open network listeners** (stdio only). No shell, no package install.

### Independence of language-only CLI

- Default `matter-cli` remains `language_main.rs` without LSP.  
- `matter-lsp` is a **separate crate binary**.  
- Packaging can include both without enabling `experimental-full`.

---

## 2. Implementation

| Deliverable | Detail |
|-------------|--------|
| Binary | `crates/matter-lsp` → `[[bin]] name = "matter-lsp"` → `src/main.rs` |
| Entry | `matter_lsp::start_server().await` |
| Transport | **stdio** (tower-lsp Server) |
| Shutdown | `shutdown` clears document map; `exit` ends process |
| Forbidden | no package resolver, no absolute defaults, no auto-download |

### Extension

- `matter.lsp.path` default **`""`** (auto-discover `matter-lsp.exe`)  
- `matter.cli.path` for Run/Compile (still language-only CLI)  
- Clear error if server missing; **no** auto download  
- Resolve: config → `MATTER_LSP` → PATH → adjacent to matter-cli → package `bin/` → `LOCALAPPDATA\Matter\bin` / `MATTER_HOME`

### Packaging (`package-matter-core.ps1`)

- Builds/copies `bin/matter-lsp.exe` unless `-SkipLsp`  
- Listed in `SHA256SUMS` / package layout README  
- Default output still under `target/validation/` (does **not** write frozen `dist/` 0.1.0)  
- Temp package version used in gates: **`0.2.0-dev`**

### Scripts

- `scripts/start-matter-lsp.ps1` — dedicated binary discovery  
- `scripts/test-matter-lsp-binary.ps1` — permanent JSON-RPC smoke against **versioned** binary  
- `scripts/test-portable-release.ps1` — package must include `matter-lsp.exe`, space-path copy, SHA256SUMS entry  

---

## 3. Gates (this branch)

| Gate | Result |
|------|--------|
| `cargo build -p matter-lsp --release --bin matter-lsp` | **PASS** |
| SHA-256 | `D9D4A51B999E67B345B8DE9659281A88810BCDA73ECEA336048D4DD2334637BD` |
| `cargo test -p matter-lsp --lib` | **15/15 PASS** |
| JSON-RPC smoke (versioned `matter-lsp.exe` only) | **PASS** (initialize / didOpen diags / shutdown / exit, process gone) |
| Extension resolve → real binary | **PASS** (`target\release\matter-lsp.exe`) |
| Temp package `0.2.0-dev` + SHA256SUMS includes LSP | **PASS** |
| Path with spaces (package copy + smoke) | **PASS** |
| Portable suite | **27/27 PASS** |
| Core / Semantic / Security | **37 / 37 / 26 PASS** |
| Dangerous new capabilities | **none** (stdio LSP only) |
| ZIP 0.1.0 | **`0A5FEE59…332A2` intact** (`dist-immutable`) |

Evidence: `target/validation/lsp_delivery_v1/`

---

## 4. Documentation language

| Term | Meaning |
|------|---------|
| **LSP engine merged** | Library + honesty diagnostics on `main` (merge milestone) |
| **LSP delivered** (this branch, after gates) | Versioned `matter-lsp.exe` + packaging + extension wiring ready to ship in a future package |
| **LSP released** | **Not** claimed — no 0.2.0 release/tag |

---

## 5. Not done (by design)

- No merge to `main` without new approval  
- No 0.2.0 release or tag  
- No modification of frozen 0.1.0 ZIP  
- No package-resolver / modules / type system  
- Full interactive VS Code UI launch not automated (resolve + LanguageClient wiring verified in code + node resolve smoke)

---

## 6. Suggested next (after approval)

1. Merge `feature/lsp-delivery-v1`  
2. Include `matter-lsp.exe` in official 0.2.0 package when that release is intentionally cut  
3. Optionally add `matter-cli` help note pointing users to `matter-lsp.exe`  
