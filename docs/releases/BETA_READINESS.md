# Matter Core Beta Readiness

Status: beta-ready for Windows x64 experimental users.

Matter Core is ready for a controlled beta when these gates are green:

- Windows release zip exists.
- SHA-256 checksums exist in JSON and `SHA256SUMS.txt`.
- Zip installer verifies checksums before installation.
- Local installer writes `INSTALL_MANIFEST.json`.
- Diagnosis script validates installed files, manifest, hashes, PATH, capabilities, and first-run execution.
- Uninstaller refuses unsafe directories without a valid Matter install manifest.
- Static download site serves the zip, checksum files, installer, and beta notes.
- Feedback channel is defined through the GitHub `Beta feedback` issue template.
- Release body is generated from beta metadata.
- GitHub Pages workflow builds and validates the beta site before deploy.
- Release package contract passes.
- Download site contract passes.
- Beta site workflow contract passes.
- Beta gate contract passes.
- Beta feedback contract passes.
- Beta release notes contract passes.
- Rust tests pass.

## Beta Scope

Included:

- Windows x64 zip distribution.
- User-local installation without Rust.
- Verified install path using SHA-256.
- CLI smoke path with `matter capabilities-json`.
- First-run example execution.
- Installation diagnosis.
- Safe uninstall.
- Static download page.
- Tester guide and feedback issue template.

Not included yet:

- `.msi` installer.
- Code signing certificate.
- Auto-update.
- Linux/macOS installers.
- General production guarantee.

## Beta User Promise

Matter Core beta is for developers who want to test an experimental embeddable language runtime with bytecode, VM execution, JSON automation commands, and guarded reflection.

The beta should not claim production readiness.
