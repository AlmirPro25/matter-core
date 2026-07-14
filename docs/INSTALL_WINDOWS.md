# Install Matter Core on Windows

## Option A — Portable ZIP (no Rust required)

1. Obtain `matter-core-0.1.0-windows-x64.zip` (SHA-256 must match the release notes / evidence).  
2. Extract anywhere (paths with spaces are supported).  
3. Run without install:

```powershell
cd "<extracted>\matter-core-0.1.0-windows-x64"
.\bin\matter.exe --version
.\bin\matter.exe run .\examples\hello.matter
```

4. Optional install to a user directory (not required to be `D:\Matter`):

```powershell
.\scripts\install-matter-core.ps1 -PackageRoot . -InstallRoot "$env:LOCALAPPDATA\Matter"
.\scripts\verify-matter-core.ps1 -InstallRoot "$env:LOCALAPPDATA\Matter" -MinimalPath
```

User projects should live under `<InstallRoot>\projects\` (preserved on update).

Update / uninstall:

```powershell
.\scripts\update-matter-core.ps1 -PackageRoot . -InstallRoot "$env:LOCALAPPDATA\Matter"
.\scripts\uninstall-matter-core.ps1 -InstallRoot "$env:LOCALAPPDATA\Matter"
# projects kept unless -RemoveProjects
```

Environment:

| Variable | Purpose |
|----------|---------|
| `MATTER_HOME` | Preferred install root |
| `MATTER_CLI` | Explicit path for LSP/tools |

## Option B — Build from source (developers)

Requirements: Rust toolchain, MinGW-w64 for `x86_64-pc-windows-gnu` (build host only).

```powershell
git clone <this-repo>
cd <repo>
.\scripts\build-matter-cli.ps1 -Release
.\target\x86_64-pc-windows-gnu\release\matter-cli.exe --version
```

Optional: set `MATTER_MINGW_BIN` to your `mingw64\bin` directory.

## LSP / editor

- VS Code setting `matter.lsp.path` defaults to `matter-cli` on PATH.  
- Or set `MATTER_CLI` / install to `%LOCALAPPDATA%\Matter\bin`.  
- Helper: `scripts/start-matter-lsp.ps1` (no hardcoded drive letters required).

## What destination machines do **not** need

- Rust, Cargo, GCC/MinGW  
- Python, Node  
- This git tree or `target/`  

## Troubleshooting

| Symptom | Check |
|---------|--------|
| Exit codes / “NOT executed” on agent-ui | Expected on Core; experimental not installed |
| Hash mismatch | Do not extract; re-download frozen ZIP |
| PowerShell execution policy | `Bypass` only for the install script invocation, or adjust policy deliberately |
