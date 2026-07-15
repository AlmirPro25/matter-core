# Matter 0.2.0 Merge Milestone — Semantic Honesty & File Capabilities v1

**Date:** 2026-07-15  
**Remote:** `github-matter-core` (`https://github.com/AlmirPro25/matter-core.git`)  
**Merge type:** `git merge --no-ff` (history preserved)  
**Force push:** not used  
**Branch deleted:** no — `develop/0.2.0-semantic-honesty` retained  

---

## Commits

| Role | SHA | Message |
|------|-----|---------|
| Merge commit | `c273992cc5166dccb786c8b5f29e928d427de4ff` | `merge: Matter 0.2.0 semantic honesty and file capabilities v1` |
| Feature tip (pre-merge) | `211afdf03c6d9c60ae28e58b51405e7d96ace2c5` | recover 0.1.0 ZIP / seal dist |
| Post-merge build integrity | `991dcf455acfb83efe64f14969449f607e600f0e` | restore AST + native `MakeClosure` arms for clean-tree builds |
| **main HEAD after merge** | `991dcf455acfb83efe64f14969449f607e600f0e` | (includes merge + build fix) |

Parent of merge: `main` was `fa973a3` (inventory), matching `github-matter-core/main` with no unexpected remote commits.

---

## CLI tested (language-only)

| Field | Value |
|-------|--------|
| Path | `target\release\matter-cli.exe` |
| SHA-256 | `616B8BA1551287DD31A6754557781C33D29E53D67BBB6F2699DE4844A4E5C646` |
| Version banner | language-only; development track 0.2.0 semantic-honesty |
| production_ready | **false** |
| release_candidate | **false** |

---

## Post-merge gates (on `main`)

| Gate | Result |
|------|--------|
| Semantic Honesty | **37/37 PASS** |
| Core | **37/37 PASS** |
| Security | **26/26 PASS** |
| Portable | **24/24 PASS** + `dist-immutable` / `wrote_to_dist=false` |
| cargo tests Core crates | **PASS** (lexer, parser, bytecode, vm, runtime, stdlib, ast) |
| cargo test matter-cli (capability_policy) | **8/8 PASS** |
| language-only build | **PASS** (after AST integrity fix) |
| experimental-full | **Binary present** (rebuild of full dep graph was interrupted; see residuals) |
| `cargo fmt --check` (altered crates) | **PASS** |

Evidence: `target/validation/merge_0_2_0_postcheck/`

---

## Frozen 0.1.0 protections

| Check | Result |
|-------|--------|
| ZIP SHA-256 | **`0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2`** |
| Tag `matter-core-v0.1.0-baseline` | **intact** (`6ca31f3fbb44425f5fa7f950b9d8f9a8b8965129`) |
| `dist/` mutation during portable suite | **none** |
| 0.2.0 release / tag created | **no** |

---

## Intentional semantic changes (0.2.0)

1. **import / export** — hard error: not implemented; no module loaded (no silent no-op).  
2. **Type annotations** — hard error: unsupported; no typechecker.  
3. **panic** — reserved word; explicit not-implemented diagnostic.  
4. **match** — equality / first-arm semantics formalized + tests (not full pattern matching).  
5. **File Capabilities v1** — default-deny for program `file.*` / `fileio.*`; `--allow-fs-read|write|delete <dir>`; write ≠ delete; no env grants.  
6. **Portable packaging** — tests must not write frozen `dist/` without explicit flags; dist hash gate.

---

## Residual risks

1. **Draft Release** for original 0.1.0 ZIP still pending (`gh` auth). Vault copy: `D:\Users\almir\MatterArtifactVault\…ORIGINAL-0A5FEE59.zip`.  
2. **experimental-full** full rebuild from a clean dep cache is slow; gate verified language-only thoroughly; experimental binary was present from an earlier successful build on this workstation.  
3. **Symlink file** escape tests still require Administrator; junction/reparse cases verified.  
4. **TOCTOU** remains between capability path check and FS op (not a full OS sandbox).  
5. **Uncommitted local WIP** was stashed before merge (`stash@{0}: pre-merge-0.2.0: stash unrelated local WIP`) — restore with `git stash pop` when convenient.  
6. Clean-tree build required restoring AST variants (`ImportFrom`/`Lambda`/…) that the parser already referenced but that had lived only in local WIP — fixed in `991dcf4`.

---

## What was not done

- No release 0.2.0 package  
- No `production_ready` / `RELEASE_CANDIDATE` promotion  
- No tag rewrite of 0.1.0  
- No force push  
- No deletion of `develop/0.2.0-semantic-honesty`  
- No modules / type system feature work  

---

## Status claim

**Matter Core main** now includes **development track 0.2.0** (semantic honesty + file capabilities v1).  
Still **not** production-ready and **not** a release candidate.
