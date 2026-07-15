# LSP Recovery Assessment v1

**Date:** 2026-07-15  
**Base:** `github-matter-core/main` @ `7533efc`  
**Source WIP (untouched):** `wip/lsp-package-resolver-recovery` @ `30cf5d7`  
**Branch:** `feature/lsp-recovery-v1`  
**Merge:** **not performed**

---

## Separation of concerns

| Area | Branch / location | Action this assessment |
|------|-------------------|------------------------|
| Matter LSP | `feature/lsp-recovery-v1` | Extracted + honesty-filtered |
| Package resolver | WIP only | **Not integrated** — see `PACKAGE_RESOLVER_ASSESSMENT_V1.md` |
| Lambda/closure tests | WIP `tests/integration_test.rs` | **Not merged** — see `LAMBDA_CLOSURE_FAILURE_ANALYSIS_V1.md` |

---

## What was taken from WIP

From `30cf5d7` (selectively rewritten, not blind copy):

- Document map + `did_open` / `did_change`
- Completions (keywords + language-only builtins)
- Hover + goto definition (same-file)
- Diagnostics via parse + `BytecodeBuilder::build_checked`
- `matter-bytecode` path dependency in `Cargo.toml`

## What was **changed** for Semantic Honesty / safety

| WIP behavior | Recovery v1 |
|--------------|-------------|
| Hover claimed “Import a module” / “Export names” | **Hard honesty**: not implemented in Core 0.2.0 |
| Hover claimed full “pattern matching” | **Equality / first-arm** semantics only |
| `panic` only as keyword | Explicit **reserved / not implemented** hover |
| `document_formatting_provider: true` without handler | **false** (do not advertise) |
| `inter_file_dependencies: true` | **false** (no module resolution) |
| Hardcoded version `"0.5.0"` | `env!("CARGO_PKG_VERSION")` |
| `file.*` free completions | Kept with **File Caps v1 default-deny** detail |
| Unused `Lexer` import | Removed |
| No unit tests | **15** recovery tests on `analyze_source` / honesty hovers |

## What was **not** included

- Package resolver crate changes  
- Integration-test lambda/closure cases  
- Shell / network / package install  
- Absolute paths  
- Module enablement or import/export implementation  

---

## Tests executed (LSP)

| Gate | Result |
|------|--------|
| `cargo check -p matter-lsp` | **PASS** (exit 0) |
| `cargo test -p matter-lsp --lib` | **15/15 PASS** |
| initialize/shutdown (logic) | shutdown clears document map; initialize returns capabilities |
| open/change pipeline | re-analyzes full document text |
| diagnostics invalid code | parse-error with line/column |
| import / export unsupported | compile-error messages contain honesty text |
| type annotations unsupported | compile-error |
| panic unsupported | parse-error reserved word |
| match equality | no diagnostic (compiles) |
| Unicode source | no diagnostic |
| hang after shutdown | no LSP loop in unit tests; clear-map path exercised |

---

## Cargo.lock

- Main `Cargo.toml` for matter-lsp **lacked** `matter-bytecode` while **lock already listed it** (stale consistency).  
- Recovery v1 **adds** the dep to `Cargo.toml` so lock matches manifests.  
- Regenerating the whole lock via `cargo generate-lockfile` produced a large unrelated churn (rejected).  
- Final branch: **no `Cargo.lock` delta** vs `7533efc` after restoring lock from base — workspace lock already encoded `matter-bytecode` for matter-lsp.  
- **Did not** copy the original working-tree 1-line lock drift.

---

## Regression / product gates

| Gate | Result |
|------|--------|
| Core suite | **37/37** (clean language-only CLI `B232…` path) |
| Semantic Honesty | **37/37** |
| Security | **26/26** |
| `cargo build -p matter-cli --release --bin matter-cli` on branch | **PASS** (exit 0) |
| New dangerous capabilities | **none** |
| ZIP 0.1.0 | **`0A5FEE59…332A2` intact** |

Language-only binary built on this branch (artifact fingerprint of *this* build, not a release):  
`7D0A3947685B539662F34E3057FBA50D194EACB144716CF7B7F6916E7B3D8FEE`

---

## Recommendation

### **LSP_MERGE_READY** (with review)

- Honesty-aligned diagnostics share CLI validation path.  
- No module/type false advertising in hover.  
- Unit tests lock honesty contracts.  
- Product suites green; ZIP untouched.

Optional before merge: wire VS Code extension to this LSP binary; add an integration smoke that spawns `start_server` over stdio if desired (out of scope here).

**Do not** merge package resolver or lambda tests with this branch.
