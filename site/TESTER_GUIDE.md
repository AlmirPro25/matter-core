# Matter Core Beta Tester Guide

Use this guide on a Windows x64 machine. Rust is not required.

## Download

Simplest path: download `matter-core-beta-setup.exe` and run it.

Fallback path: download these files from the beta site into the same folder:

- `matter-core-windows-x64.zip`
- `matter-core-beta-setup.exe`
- `install-matter-beta.cmd`
- `install-release-zip.ps1`
- `release-checksums.json`
- `SHA256SUMS.txt`

## Install

Recommended path: double-click `matter-core-beta-setup.exe`.

Fallback path: double-click `install-matter-beta.cmd`.

Manual path: open PowerShell in the download folder and run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install-release-zip.ps1 -ZipPath .\matter-core-windows-x64.zip -ChecksumJsonPath .\release-checksums.json -Sha256Path .\SHA256SUMS.txt
```

## First Run

Open a new PowerShell window and run:

```powershell
matter run examples\first_run.matter
matter run examples\agent_policy_demo.matter
matter capabilities-json
```

## Demo That Shows The Point

Run the agent policy demo:

```powershell
matter run examples\agent_policy_demo.matter
matter reflect-json examples\agent_policy_demo.matter
matter reflexive-guard-json examples\agent_policy_demo.matter
```

This demonstrates the product idea: a small rule runs on the Matter VM, then
the same source can be inspected and guarded before another tool trusts it.

## Diagnose

Run:

```powershell
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Matter\scripts\diagnose-local-install.ps1"
```

Save this output if anything fails.

## Uninstall

Run:

```powershell
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Matter\scripts\uninstall-local.ps1"
```

## Send Feedback

Open a GitHub issue using the `Beta feedback` template.

Include:

- Windows version.
- PowerShell version.
- The command that failed.
- Diagnosis output.
- Any SmartScreen, antivirus, PATH, or permission warning.
