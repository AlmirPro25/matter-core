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
| Milestone doc | `6c3a77e52e783466caa0b3a693f59e1e07364865` | merge milestone (postcheck) |
| **Residual close** | `da9b10f34c2f2dc4d94b755216d1b51442df64ab` | clean-tree Closure/AST/link fixes + permanent gates; **RESIDUAL_CLOSED** |

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
| experimental-full | **PASS** on clean worktree (see Residual close) |
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

## Residual close (post-merge) — **RESIDUAL_CLOSED**

Closed on clean worktree of **`6c3a77e`** with isolated `CARGO_TARGET_DIR` on `D:` (no `cargo clean` on primary tree; stash `pre-merge-0.2.0` untouched; ZIP/tag 0.1.0 untouched).

### Environment

| Field | Value |
|-------|--------|
| Worktree | `D:\matter-clean-6c3a77e` (detached `6c3a77e`) |
| `CARGO_TARGET_DIR` | `D:\matter-target-clean-6c3a77e` |
| Prebuilt CLI in isolated target before language-only | **none** (language-only built first) |
| Target space after both builds | **~1.87 GB** / ~4300 files |

### Clean builds

| Variant | Command | Exit | Duration | Binary SHA-256 | Size |
|---------|---------|------|----------|----------------|------|
| **language-only** | `cargo build -p matter-cli --release --bin matter-cli` | **0** | **31.0 s** | `B232B8F96D1A3EA672154C1B53A5C83A1891DA0C34819AFDCCA00323181F8682` | 3 764 158 |
| **experimental-full** | `cargo build -p matter-cli --release --features experimental-full --bin matter-cli-experimental` | **0** | **73.2 s** (incremental after compile-fix retries; first full graph ~552 s then blockers) | `FD091B8C5BE98EBABE759F85112E977511FDB6313F21B7003D77070B7B596410` | 48 141 741 |

Language-only system DLL surface (representative): `kernel32`, `ntdll`, `userenv`, `ws2_32`, CRT APIs — no `python3` / `opengl32`.  
Experimental additionally links GUI/polyglot deps (incl. `shlwapi`, `python3`, `opengl32`, DXGI/D3D family).

### Compile blockers fixed (residual-only)

1. **`Value::Closure` non-exhaustive** in bridges (`java`, `go`, `python`, `nodejs`, native variants, `rust`) and **`matter-visual`** `value_json`.  
2. **AST arms** for `ImportFrom` / `ImportAs` / `Export` and `Lambda` / `Ok`/`Err`/`Some`/`None` / `TryPropagate` in **`matter-linter`** and **`matter-formatter`**.  
3. **windows-gnu link**: missing `libshlwapi.a` in Rust self-contained sysroot for experimental-full — vendored `tools/windows-gnu-libs/libshlwapi.a` + gate installs into toolchain `self-contained` when needed.

### Suites on clean language-only CLI

| Gate | Result |
|------|--------|
| Semantic Honesty | **37/37 PASS** (exit 0) |
| Core | **37/37 PASS** (exit 0) |
| Security | **26/26 PASS** (exit 0) |
| ZIP 0.1.0 | **`0A5FEE59…332A2` match** |

CLI: `D:\matter-target-clean-6c3a77e\release\matter-cli.exe`  
SHA-256: `B232B8F9…1F8682`

### cargo tests (Core crates, isolated target)

| Crate | Result |
|-------|--------|
| matter-lexer / parser / bytecode / vm / runtime / stdlib / ast | **PASS** (all exit 0) |
| `surface_integrity_991dcf4` (ast) | **3/3 PASS** |
| `make_closure_surface` (bytecode) | **1/1 PASS** |
| `make_closure_native_surface` (native) | **1/1 PASS** |

### Permanent gates added

- `scripts/test-clean-checkout-build.ps1` — worktree + isolated target; optional `-IncludeExperimental`; surface tests; shlwapi ensure for gnu experimental.  
- Unit surfaces above so incremental caches cannot hide missing 991dcf4 / MakeClosure arms.

Evidence: `target/validation/residual_0_2_0_clean/` (`language_only.json`, `experimental_full.json`, `suite_results.json`, `cargo_core_tests.json`, `residual_report.json`, build logs).

---

## Residual risks (remaining, non-blocking)

1. **Draft Release** for original 0.1.0 ZIP still pending (`gh` auth). Vault copy: `D:\Users\almir\MatterArtifactVault\…ORIGINAL-0A5FEE59.zip`.  
2. **experimental-full** full cold rebuild remains slow (multi-minute).  
3. **Symlink file** escape tests still require Administrator; junction/reparse cases verified.  
4. **TOCTOU** remains between capability path check and FS op (not a full OS sandbox).  
5. **Uncommitted local WIP** stashed before merge (`stash@{0}: pre-merge-0.2.0`) — **not** popped; restore only when convenient.  
6. windows-gnu toolchains without `libshlwapi.a` need vendored import lib (documented under `tools/windows-gnu-libs/`).

---

## What was not done

- No release 0.2.0 package  
- No `production_ready` / `RELEASE_CANDIDATE` promotion  
- No tag rewrite of 0.1.0  
- No force push  
- No deletion of `develop/0.2.0-semantic-honesty`  
- No modules / type system feature work  
- No new product features beyond residual compile/surface gates  

---

## Status claim

**Matter Core main** includes **development track 0.2.0** (semantic honesty + file capabilities v1).  
**Post-merge residual close: RESIDUAL_CLOSED** (clean language-only + experimental-full builds proven; permanent gates landed).  
Still **not** production-ready and **not** a release candidate.
