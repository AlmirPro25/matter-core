# WIP Preservation and Triage v1

**Date:** 2026-07-15  
**Base:** `github-matter-core/main` @ `7533efc`  
**Worktree:** `D:\matter-wip-triage-v1`  
**Vault:** `D:\Users\almir\MatterArtifactVault\wip-triage-v1\`  
**main branch:** **not modified**  
**stash@{0}:** **preserved** (not popped, not dropped)

---

## 1. Stash preservation

| Field | Value |
|-------|--------|
| Ref | `stash@{0}` — `pre-merge-0.2.0: stash unrelated local WIP` |
| Stash SHA | `afbdec2beeb67a7dfaae2366900d7c0402a78ba3` |
| Merge parents | `211afdf03c6d9c60ae28e58b51405e7d96ace2c5` (WIP base), `9ca4334609382f2d9d6a712ed81e936b52224c45` (index) |
| Safety branch | **`rescue/pre-merge-0.2.0`** → same SHA (local only; **not pushed**) |
| Vault exports | `stash0_metadata.txt`, `stash0_name_status.txt`, `stash0_stat.txt`, `stash0_full.patch`, `stash0_path_scan.txt` |

### Vault SHA-256 (preservation set)

See `D:\Users\almir\MatterArtifactVault\wip-triage-v1\SHA256SUMS.txt` (regenerate with `Get-FileHash` if missing).

Notable:

| File | SHA-256 (recorded at preserve) | Bytes |
|------|--------------------------------|-------|
| `stash0_full.patch` | `E0B808656C71BAB2EB068A736361B714190A45D8384FD4A0C07569CC08B4F43C` | 61602 |
| `stash0_metadata.txt` | `A6D8D88FC6F3512C35101712FEE0BB4A4856CAAD26E0FC16D439390A1449EDFD` | 405 |
| `stash0_name_status.txt` | `5CCA77571634B705B52CE1235A5BE2D77B91528849C72E2E1E21782F39A8B9D6` | 824 |
| `stash0_stat.txt` | `C2F3C3C6EFDA8CF3E64599219EF8C8D428DF61ABE4F557FE8743A90DE9658D10` | 1363 |

### Scans

| Scan | Result |
|------|--------|
| Secret patterns (API keys, private keys, ghp_, etc.) | **0 hits** |
| Absolute local paths in full patch | **3** — `F:\Users\almir\Desktop\MANIFESTO…`, `D:\Users\almir\Desktop\MANIFESTO…`, `F:\Users\almir\Desktop\MeuAppMatter\$AppFile` (from `.cargo/config.toml` / scripts) → **LOCAL_ONLY**; do **not** push `rescue/*` until scrubbed |

**Do not push** `rescue/pre-merge-0.2.0` without removing local paths and re-scanning.

---

## 2. Isolated worktree

| Field | Value |
|-------|--------|
| Path | `D:\matter-wip-triage-v1` |
| Base branch | `wip/triage-base` @ `7533efc` |
| Original main working tree | **untouched** (still has `Cargo.lock` drift + untracked WIP) |
| Cargo target | `D:\matter-target-wip-triage-v1` (isolated) |

---

## 3. Bucket: LSP / package-resolver

**Branch:** `wip/lsp-package-resolver-recovery` @ `30cf5d7`

### Selective extract (from stash only)

| Path | Classification | Notes |
|------|----------------|-------|
| `crates/matter-package-resolver/src/lib.rs` | **UNIQUE_USEFUL** | ~+336 lines vs main |
| `crates/matter-lsp/src/lib.rs` | **UNIQUE_USEFUL** | ~+395 lines vs main |
| `crates/matter-lsp/Cargo.toml` | **UNIQUE_USEFUL** | adds `matter-bytecode` dep |
| `tests/integration_test.rs` | **UNIQUE_USEFUL** + **REWORK** | +140 lines; 4 lambda tests fail on Core 0.2.0 |

### Explicitly **not** extracted (already on residual main)

bridges, visual, linter, formatter, AST, native → **ALREADY_RESOLVED** by `da9b10f` residual close (would be **CONFLICTING** if reapplied from stash).

### Commits

1. `b464b57` — package-resolver  
2. `dbe20c0` — matter-lsp  
3. `30cf5d7` — integration_test  

### Tests executed

| Command | Exit | Detail |
|---------|------|--------|
| `cargo check -p matter-lsp` | **0** | 1 unused import warning |
| `cargo check -p matter-package-resolver` | **0** | 1 unused import warning |
| `cargo test -p matter-package-resolver --lib` | **0** | 4/4 PASS |
| `cargo test -p matter-lsp --lib` | **0** | 0 unit tests |
| `cargo test --test integration_test` | **101** | **36 PASS / 4 FAIL** (`test_lambda_*`, `test_closure_*`: unknown function) |

### Secret scan (branch files)

**0 hits** (no absolute user paths in extracted crate sources).

### Merge?

**No.** Branch is recovery only.

---

## 4. Bucket: fuzz

**Branch:** `tooling/fuzz-harness-recovery` @ `b1f7944`

| Item | Result |
|------|--------|
| Source | Original untracked `fuzz/` (main tree) — not stash |
| Secret scan | **0 hits** |
| Corpus/artifacts | **none** present; `.gitignore` adds `target/`, `corpus/`, `artifacts/`, coverage |
| `Cargo.toml` | cargo-fuzz layout; deps lexer/parser/bytecode |
| Standalone | empty `[workspace]` so package is not forced into root workspace members |
| `cargo check --manifest-path fuzz/Cargo.toml` | **FAIL** (needs `g++` / libfuzzer C++ toolchain on this host) → **REWORK** (environment) |

### Commits

1. `fda6afb` — harness + gitignore  
2. `b1f7944` — standalone workspace table  

### Classification

**UNIQUE_USEFUL** (tooling) / **REWORK** for Windows compile.

---

## 5. Bucket: examples

**Branch:** `docs/core-examples-recovery` @ `5626324`

| File | Edition | Abs path | FS ops | language-only CLI | Disposition |
|------|---------|----------|--------|-------------------|-------------|
| `examples/apps/calculadora_orcamento.matter` | **Core** | no | no | **PASS** exit 0 | **KEEP** |
| `examples/apps/diario_tarefas.matter` | **Core** | no | no | **PASS** exit 0 | **KEEP** |
| `examples/new_features_demo.matter` | Experimental / **REWORK** | no | no | **FAIL** (unknown `multiplier` / closures) | **REWORK** |
| `examples/polyglot/node_path_smoke.matter` | **Experimental** | no (string data only) | no | FAIL backend not found on language-only | **KEEP** (needs experimental CLI) |
| `examples/polyglot/python_math_smoke.matter` | **Experimental** | no | no | FAIL backend not found | **KEEP** (needs experimental CLI) |
| `examples/apps/run_core_examples_smoke.ps1` | harness | — | — | Core apps smoke | **KEEP** |

Edition headers and repro notes added in-file. No File Caps required (no `file.*`).

---

## 6. Session documents

**Branch:** `docs/session-archive-2026-07-14` @ `b901338`

| Root untracked (main tree) | vs `docs/status/*` on main | Classification |
|----------------------------|----------------------------|----------------|
| `PHASE_1…4_*.md`, `PRODUCTION_READINESS_AUDIT_V1.md` | **Identical** to `docs/status/` counterparts | **ALREADY_RESOLVED** (canonical already on main) + **ARCHIVE** copy under `docs/session-archive/2026-07-14/` |

Root untracked copies are **DISCARD_CANDIDATE** after user confirms `docs/status/` is sufficient (do not delete in this phase).

No automatic promotion to main root docs.

---

## 7. Local-only items (not on recovery branches)

| Item | Classification | Action this phase |
|------|----------------|-------------------|
| `.vscode/settings.json` (absolute `matter.lsp.path`) | **LOCAL_ONLY** | Vault copy under `local_only/vscode/`; never push |
| `.vscode/extensions.json` (`matter-core.matter` recommendation) | **LOCAL_ONLY** / optional editor | Vault; evaluate separately if publishing extension |
| `my_app/` | **LOCAL_ONLY** scaffold | **Vaulted** to `local_only/my_app/` before any discard decision |
| Working tree `Cargo.lock` (1-line remove bytecode from matter-lsp) | Drift vs HEAD | **Diagnostic only** — do not restore/commit here. Main `Cargo.toml` has no bytecode; LSP recovery branch *adds* it. Regenerate lock on that branch later. See vault `cargo_lock_diagnosis.txt` |
| `vscode-extension/package-lock.json` | Optional tooling | Name appears consistent with package.json; full `npm ci` verification deferred → **REWORK**/optional |

---

## 8. Branch summary (local only — no push)

| Branch | Tip | Purpose |
|--------|-----|---------|
| `rescue/pre-merge-0.2.0` | `afbdec2` | Full stash safety ref |
| `wip/triage-base` | `7533efc` | Clean base pointer |
| `wip/lsp-package-resolver-recovery` | `30cf5d7` | LSP + resolver + integration tests |
| `tooling/fuzz-harness-recovery` | `b1f7944` | cargo-fuzz harness |
| `docs/core-examples-recovery` | `5626324` | Core apps + polyglot smokes |
| `docs/session-archive-2026-07-14` | `b901338` | Historical phase/audit copies |
| `docs/wip-triage-v1-report` | (this report commit) | Triage deliverables |

---

## 9. Final disposition table

| Item | KEEP | REWORK | ARCHIVE | DISCARD_CANDIDATE |
|------|:----:|:------:|:-------:|:-----------------:|
| matter-package-resolver (stash) | ✓ | | | |
| matter-lsp (stash) | ✓ | polish warnings | | |
| integration_test lambda/closure cases | | ✓ | | |
| residual-related stash arms (bridges/…) | | | | n/a (ALREADY_RESOLVED on main) |
| fuzz harness | ✓ | Windows g++/libfuzzer | | |
| calculadora / diario examples | ✓ | | | |
| new_features_demo | | ✓ | | |
| polyglot smokes | ✓ (experimental) | run on exp CLI | | |
| PHASE/AUDIT root untracked | | | ✓ (status + session-archive) | root dup after confirm |
| `.vscode` absolute paths | | | vault | never push |
| `my_app/` | | | vault | optional later |
| working tree Cargo.lock drift | | ✓ (ignore until LSP merge) | | |
| package-lock.json | | optional verify | | |
| stash@{0} | **KEEP forever until explicit drop** | | rescue branch | |

---

## 10. Protections verified

- No `git stash pop`  
- No `git add .`  
- No `git clean`  
- stash@{0} still present  
- `main` still @ `7533efc` matching remote  
- ZIP 0.1.0 still `0A5FEE59…332A2`  
- No modules / type-system feature work  
- No merge to main  
- No push of personal config or rescue branch  
- Original worktrees/untracked **not deleted**

---

## 11. Suggested next (not executed)

1. Review `wip/lsp-package-resolver-recovery` for product merge candidacy.  
2. Install g++/cargo-fuzz or document Windows fuzz limits.  
3. Merge `docs/core-examples-recovery` Core apps only (or cherry-pick) after review.  
4. Confirm discard of root PHASE duplicates once user accepts `docs/status/`.  
5. Only then choose modules vs types as 0.2.0 evolution.
