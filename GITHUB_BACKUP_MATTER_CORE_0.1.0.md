# GitHub backup — Matter Core 0.1.0 baseline

**Date:** 2026-07-14  
**Purpose:** Private GitHub snapshot of Matter Core production-readiness **baseline** (not a stability release).

---

## Remote

| Field | Value |
|-------|--------|
| Remote name | `github-matter-core` |
| URL | `https://github.com/AlmirPro25/matter-core.git` |
| Notes | Prefer **no trailing period** in the repo name. Empty private repo at push time. |
| Existing remote preserved | `origin` → `https://github.com/AlmirPro25/LIGUAGEM.git` (**not removed**) |

## Branch / commit / tag

| Field | Value |
|-------|--------|
| Branch pushed | `main` |
| Commit | `fa6e3aa737c382f3699ddd31c075754cb5a8e695` |
| Message | `docs/release: snapshot Matter Core 0.1.0 production-readiness baseline` |
| Annotated tag | `matter-core-v0.1.0-baseline` |
| Force push | **No** |

## Maturity claims (honest)

| Claim | Status |
|-------|--------|
| production_ready | **false** |
| RELEASE_CANDIDATE | **not declared** |
| Stable 1.0 | **not released** |
| production-readiness-v2 | **BLOCKED_EXTERNAL_VALIDATION** |
| External Windows validation | **BLOCKED** (build host only) |

## Gates run before push

| Gate | Result |
|------|--------|
| Core suite | **37 / 37 PASS** |
| Security suite | **26 / 26 PASS** |
| Portable suite | **20 / 20 PASS** |
| production-readiness-v2 | **BLOCKED_EXTERNAL_VALIDATION** (not falsified) |
| Pre-push secret scan (docs/core sources) | **0 secret hits** |
| Large local tool zips | Present under `.tools/` locally; **gitignored**, not pushed |

## Frozen package (local only)

| Field | Value |
|-------|--------|
| Path | `dist/matter-core-0.1.0-windows-x64.zip` (gitignored) |
| SHA-256 | `0A5FEE59F07A0C09E74992A62CDE95EA4C1DCA9AE6529B12AE47E9F23E1332A2` |
| Modified during this backup? | **No** |
| Uploaded as GitHub Release asset? | **No** (not authorized) |

## What was versioned

- Core language/runtime/CLI sources and scripts  
- Docs: README, CURRENT_STATUS, CHANGELOG, SECURITY, architecture/install/release  
- Phase 1–6 reports under `docs/status/`  
- Small evidence JSON under `docs/evidence/releases/0.1.0/`  
- Invalid fixtures under `tests/fixtures/invalid/`  
- `Cargo.toml` / `Cargo.lock`  

## What was **not** versioned

- Entire `target/`  
- `dist/` ZIP packages  
- MinGW / `.tools` toolchains  
- `.env` / credentials  
- Local large caches  

## Post-push verification (remote)

| Check | Result |
|-------|--------|
| `main` on `github-matter-core` | pushed |
| Tag `matter-core-v0.1.0-baseline` | pushed |
| `origin` still LIGUAGEM | yes |
| GitHub Release published | **no** |

Manual UI checks recommended: open README render, confirm `crates/`, `scripts/`, `docs/`, `Cargo.toml`, tag list.

## Limitations

1. External clean-Windows validation remains **BLOCKED**.  
2. Workspace may still have unstaged local edits outside the milestone commit.  
3. `cargo fmt` / clippy warning debt remains (WARN in readiness V2).  
4. Experimental edition is **not** a sandbox.  

## Clone

```powershell
git clone https://github.com/AlmirPro25/matter-core.git
cd matter-core
git checkout matter-core-v0.1.0-baseline
```
