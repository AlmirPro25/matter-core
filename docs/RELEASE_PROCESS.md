# Release process (Matter Core)

## Principles

1. **Honest maturity:** do not set `production_ready: true` or claim RC without external clean-Windows PASS.  
2. **Freeze packages:** once a ZIP SHA-256 is published in evidence, do not rebuild/replace silently.  
3. **Core first:** language-only binary is the default release surface.  
4. **No force-push** of protected history for baseline tags without explicit recovery approval.

## Baseline 0.1.0 checklist

- [x] Phases 1–5 documentation in `docs/status/`  
- [x] Core / Security / Portable suites green  
- [x] production-readiness-v2 recorded (may be `BLOCKED_EXTERNAL_VALIDATION`)  
- [ ] External Windows validation PASS (still BLOCKED as of 2026-07-14)  
- [x] Git milestone commit + tag `matter-core-v0.1.0-baseline`  
- [ ] Optional: GitHub Release asset upload (separate authorization)  

## Packaging

```powershell
.\scripts\build-matter-cli.ps1 -Release
.\scripts\package-matter-core.ps1 -SkipBuild
# produces dist\matter-core-<ver>-windows-x64\ and .zip + MANIFEST + SHA256SUMS
```

Record SHA-256 in evidence under `docs/evidence/releases/<ver>/`.

## Gates before push/tag

```powershell
.\scripts\test-core-suite.ps1
.\scripts\test-capability-security.ps1
.\scripts\test-portable-release.ps1 -PackageRoot .\dist\matter-core-0.1.0-windows-x64
.\scripts\production-readiness-v2.ps1
```

`production-readiness-v2` must **not** be falsified: external independent Windows may remain BLOCKED.

## Tagging convention

| Tag | Meaning |
|-----|---------|
| `matter-core-v0.1.0-baseline` | Docs + code snapshot of readiness baseline |
| `matter-core-v0.1.0` | Reserved for a future claim after external PASS (not automatic) |

## Remotes

- Keep existing remotes (e.g. `origin`) intact.  
- Add backup/publish remotes with distinct names (`github-matter-core`, etc.).  
- Prefer `git push <remote> <branch>` and `git push <remote> <tag>` without `--force`.
