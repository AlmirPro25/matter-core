# Matter Core Download Site

Static download page for the current Windows release.

Build the release first:

```powershell
.\scripts\build-release-package.ps1 -CliPath F:\Users\almir\Desktop\matter_target\release\matter-cli.exe
```

Then publish the site artifacts:

```powershell
.\scripts\beta-gate.ps1 -CliPath F:\Users\almir\Desktop\matter_target\release\matter-cli.exe
.\scripts\build-download-site.ps1
.\scripts\test-download-site-contract.ps1
.\scripts\test-beta-readiness-contract.ps1
.\scripts\test-beta-feedback-contract.ps1
.\scripts\export-beta-release-notes.ps1
.\scripts\test-beta-release-notes-contract.ps1
.\scripts\test-beta-site-workflow-contract.ps1
.\scripts\test-beta-gate-contract.ps1
```

The generated download files live in `site\downloads\` and are described by `site\release.json`.

GitHub Pages deployment is defined in `.github\workflows\beta-site.yml`. It rebuilds the site metadata, runs the site and beta contracts, uploads `site\`, and deploys only after those checks pass.
