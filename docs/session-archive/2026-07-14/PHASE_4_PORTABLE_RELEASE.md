# PHASE_4_PORTABLE_RELEASE

**Date:** 2026-07-14  
**Scope:** Portability, packaging, and reproducible install of **Matter Core language-only**.  
**Not in scope:** GUI installer, paid code signing, public publish, architecture rewrite, Phase 5, demos/bridges.  
**Verdict:** Phase 4 **complete** for approved gates.

---

## 0. Preconditions

| Item | Result |
|---|---|
| Phase 3 frozen | SHA-256 CLI `355F7406680A3D7B662A7728B92986DE53E87431794AD886310BBCDC1B3D6F22` (3 726 794 B) |
| Baseline evidence | `target/validation/phase_4_portable_release/baseline_phase3/` |
| Core suite | **37 / 37 PASS** |
| Security suite | **26 / 26 PASS** |
| Experimental binary | **Preserved** (`matter-cli-experimental` + `experimental-full`; not in Core package) |
| Syntax / MBC1 / VM semantics | **Unchanged** |

---

## 1. Objective (achieved)

Matter Core can be **built** (on a build host), **packaged**, **installed**, and **run** outside the original tree **without** requiring destination `C:`/`D:`/`F:` paths, and **without** Rust/GCC/Python/Node on the destination machine.

---

## 2. Package artifact

| Artifact | Path |
|---|---|
| Folder | `dist/matter-core-0.1.0-windows-x64/` |
| ZIP | `dist/matter-core-0.1.0-windows-x64.zip` |
| ZIP SHA-256 | see `dist/SHA256SUMS` and evidence `sha256-post.json` |
| In-package manifest | `MANIFEST.json` |
| In-package sums | `SHA256SUMS` |

### 2.1 Contents (only necessary files)

```
bin/matter-cli.exe
bin/matter.exe          (same language-only binary)
examples/*.matter       (core samples only)
schemas/core-status.schema.json
scripts/install|verify|update|uninstall-matter-core.ps1
README.md, INSTALL.txt, LICENSE (if present)
MANIFEST.json, SHA256SUMS
```

**Excluded:** `target/`, source crates, `.cargo/`, caches, credentials, polyglot/FFI demos, experimental CLI.

### 2.2 Build (build host only)

```powershell
.\scripts\build-matter-cli.ps1 -Release
.\scripts\package-matter-core.ps1 -SkipBuild
# or rebuild inside packager without -SkipBuild
```

Toolchain (`MATTER_MINGW_BIN` / MinGW) is **build-host only** — not required after packaging.

---

## 3. Permanent scripts

| Script | Role |
|---|---|
| `scripts/package-matter-core.ps1` | Create versioned package + zip + MANIFEST + SHA256SUMS |
| `scripts/install-matter-core.ps1` | Install to any `-InstallRoot` (default `%LOCALAPPDATA%\Matter` or `$env:MATTER_HOME`) |
| `scripts/verify-matter-core.ps1` | Verify install/package (`--help`, version, core-status, run/compile/run-bytecode, optional PATH mínimo) |
| `scripts/update-matter-core.ps1` | Update product files; **preserve** `projects/` / `user/` |
| `scripts/uninstall-matter-core.ps1` | Remove **owned** files from `.matter-install-manifest.json` only |
| `scripts/test-portable-release.ps1` | Permanent portability suite |
| `scripts/start-matter-lsp.ps1` | LSP launcher without drive hardcodes |

---

## 4. Hardcodes

Full JSON: `target/validation/phase_4_portable_release/hardcodes-inventory.json`

### 4.1 Removed / fixed (runtime, package, install, LSP, Core path)

| Location | Before | After |
|---|---|---|
| `vscode-extension/package.json` `matter.lsp.path` | Absolute `D:/Users/.../matter-cli.exe` | Default `matter-cli` (PATH) |
| `scripts/start-matter-lsp.ps1` | `D:\Matter\bin\...` | `MATTER_CLI` → PATH → relative → `MATTER_HOME` → `%LOCALAPPDATA%\Matter` |
| `scripts/install-local.ps1` | Preferred `D:\Matter` | `MATTER_HOME` or `%LOCALAPPDATA%\Matter` |
| `scripts/build-matter-cli.ps1` | Hard fail unless `D:\mingw64` | `MATTER_MINGW_BIN` + discovery (**build host**) |
| `scripts/test-capability-security.ps1` | Fixed `D:\mingw64\...\objdump` | Env + discovery |
| New portable install flow | n/a | Explicit `-InstallRoot` / relative package paths only |

### 4.2 Remaining (justified)

| Location | Justification |
|---|---|
| `.cargo/config.toml` MinGW linker | **Build host** config; not in package; not destination runtime |
| `build-matter-cli.ps1` fallback MinGW candidates | Build machine convenience; overridable |
| Legacy `scripts/install.ps1` / `uninstall.ps1` (`C:\Program Files\Matter`) | Old admin installer; **not** Phase 4 path; superseded |
| Perf scripts / docs with `F:\...\matter_target` | Non-Core experimental tooling / historical docs |
| FFI example READMEs with absolute paths | Outside language-only package |

**Runtime of packaged Core does not require any `C:`/`D:`/`F:` reference.**

---

## 5. Install / update / uninstall behavior

| Operation | Behavior |
|---|---|
| Install | Copies product files; writes `.matter-install-manifest.json`; optional User PATH; creates `projects/` |
| Update | Re-runs install with **preserve** of `projects/`, `user/`, `workspace/` |
| Uninstall | Deletes only `owned_files` from manifest; keeps `projects/` unless `-RemoveProjects` |
| Verify | Runs CLI contracts with optional `PATH=System32` only |

---

## 6. Gates

| Gate | Result |
|---|:---:|
| Core 37/37 | **PASS** |
| Security 26/26 | **PASS** |
| Documented release build script | **PASS** (`build-matter-cli.ps1` + `package-matter-core.ps1`) |
| Package smoke PATH mínimo | **PASS** |
| Install path A + path B (spaces) | **PASS** |
| Run from extracted package / isolated copy | **PASS** |
| `--help`, `--version`, `core-status-json`, `compile`, `run`, `run-bytecode` | **PASS** |
| LSP resolver without drive hardcode | **PASS** |
| Update preserves user projects | **PASS** |
| Uninstall removes only Matter-owned files | **PASS** |
| No `target/` / caches / credentials in package | **PASS** |
| MANIFEST + SHA256SUMS present | **PASS** |
| DLLs without Python / OpenGL / MF | **PASS** (security suite) |
| Portable suite total | **20 / 20 PASS** |

Evidence: `target/validation/phase_4_portable_release/`  
(`portable-suite-results.json`, `sha256-post.json`, `MANIFEST.json`, `SHA256SUMS`, `hardcodes-inventory.json`)

---

## 7. Sizes (recorded)

| Item | Size |
|---|---:|
| `matter-cli.exe` (language-only) | 3 726 794 B (~3.55 MB) |
| Package folder total (files) | ~7.48 MB (two exe copies + examples + scripts) |
| ZIP | see `dist/matter-core-0.1.0-windows-x64.zip` + `dist/SHA256SUMS` |

---

## 8. Destination machine requirements

- Windows x64  
- System CRT DLLs only (no redistributed python3/opengl/mf)  
- **Not required:** Rust, GCC, Python, Node, `target/`, source tree, original drive letters  

---

## 9. Quick destination workflow

```powershell
# Extract zip anywhere
Expand-Archive matter-core-0.1.0-windows-x64.zip -DestinationPath .\MatterCore
cd .\MatterCore
.\bin\matter-cli.exe run .\examples\hello.matter

# Or install
.\scripts\install-matter-core.ps1 -PackageRoot . -InstallRoot "$env:LOCALAPPDATA\Matter"
.\scripts\verify-matter-core.ps1 -InstallRoot "$env:LOCALAPPDATA\Matter" -MinimalPath
```

---

## 10. Deliverables checklist

| Deliverable | Status |
|---|---|
| `PHASE_4_PORTABLE_RELEASE.md` | this file |
| Versioned package + ZIP | `dist/matter-core-0.1.0-windows-x64[.zip]` |
| `MANIFEST.json` + `SHA256SUMS` | in package + evidence copy |
| `dist/SHA256SUMS` | zip checksum |
| package/install/verify/update/uninstall scripts | permanent under `scripts/` |
| `scripts/test-portable-release.ps1` | permanent |
| Evidence dir | `target/validation/phase_4_portable_release/` |
| Hardcodes removed vs remaining | §4 + `hardcodes-inventory.json` |

---

**Phase 4 status: COMPLETE.**  
**Stop here.** Do not start Phase 5 without **new explicit approval**.
