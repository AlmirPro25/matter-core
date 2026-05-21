# Matter Core Beta

Matter Core beta for Windows x64 is ready for controlled testing.

## What You Can Test

- Install without Rust.
- Verify the zip with SHA-256 before installing.
- Run `matter capabilities-json`.
- Run `examples\first_run.matter`.
- Diagnose an installed copy.
- Uninstall safely.
- Report results through the `Beta feedback` GitHub issue template.

## Install

Download these files into the same folder:

- `matter-core-windows-x64.zip`
- `install-release-zip.ps1`
- `release-checksums.json`
- `SHA256SUMS.txt`

Run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install-release-zip.ps1 -ZipPath .\matter-core-windows-x64.zip -ChecksumJsonPath .\release-checksums.json -Sha256Path .\SHA256SUMS.txt
matter run examples\first_run.matter
matter capabilities-json
```

## Diagnose

```powershell
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Matter\scripts\diagnose-local-install.ps1"
```

## Uninstall

```powershell
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Matter\scripts\uninstall-local.ps1"
```

## Limits

This is a beta. It is not a production guarantee and does not yet include a signed `.msi`, auto-update, or cross-platform installers.

## Feedback

Use `TESTER_GUIDE.md` for the full tester flow and open a GitHub issue with the `Beta feedback` template.
