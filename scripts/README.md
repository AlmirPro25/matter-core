# Matter Core Scripts

Active automation and validation scripts live in this folder. Older one-off organization scripts are kept in `scripts/archive/`.

## Install Scripts

```powershell
.\scripts\install.ps1
.\scripts\install-local.ps1
.\scripts\install-release-local.ps1
.\scripts\install-release-zip.ps1
.\scripts\diagnose-local-install.ps1
.\scripts\uninstall.ps1
.\scripts\uninstall-local.ps1
```

- `install.ps1` installs the Matter CLI globally.
- `install-local.ps1` builds from source and installs the Matter CLI for the current user.
- `install-release-local.ps1` installs a prebuilt release package for the current user without requiring Rust, runs a CLI smoke check, and writes `INSTALL_MANIFEST.json` with binary hashes.
- `install-release-zip.ps1` verifies a release zip with `release-checksums.json` and `SHA256SUMS.txt`, extracts it, and installs it for the current user.
- `diagnose-local-install.ps1` checks an installed Matter release: files, manifest schema, binary hashes, PATH, CLI capabilities, and first-run execution.
- `uninstall.ps1` removes the global install.
- `uninstall-local.ps1` removes the local install only when `INSTALL_MANIFEST.json` proves the target is a Matter release install; use `-Force` only for manual recovery.

## Validation Scripts

```powershell
.\scripts\preflight-env.ps1
.\scripts\release-doctor.ps1
.\scripts\release-doctor.ps1 -Json
.\scripts\ai-app-canonical-flow.ps1
.\scripts\run-ai-canonical-profile.ps1
.\scripts\validate-full-workspace.ps1
.\scripts\test-runnable-examples.ps1
.\scripts\test_all.ps1
.\scripts\test_api_bridge.ps1
.\scripts\test_bytecode_equivalence.ps1
.\scripts\test_repl_simple.ps1
.\scripts\test_repl_persistent.ps1
.\scripts\ffi-smoke-all.ps1
.\scripts\ffi-smoke-all.ps1 -IncludeJava
.\scripts\rust-ffi-plugin-smoke.ps1
.\scripts\rust-ffi-plugin-smoke.ps1 -CliPath F:\Users\almir\Desktop\matter_target\debug\matter-cli.exe
.\scripts\rust-ffi-plugin-smoke.ps1 -JsonOut target\ffi\rust-smoke.json
.\scripts\native-ffi-smoke.ps1
.\scripts\native-ffi-smoke.ps1 -IncludeJava
.\scripts\native-ffi-smoke.ps1 -JsonOut target\ffi\native-smoke.json
.\scripts\verify-ffi-smoke-summaries.ps1
.\scripts\verify-ffi-smoke-summaries.ps1 -CheckMatrix
.\scripts\export-ffi-validation-matrix.ps1
.\scripts\export-ffi-validation-report.ps1
.\scripts\export-release-readiness.ps1
.\scripts\build-release-package.ps1
.\scripts\build-release-package.ps1 -CliPath F:\Users\almir\Desktop\matter_target\release\matter-cli.exe
.\scripts\build-download-site.ps1
.\scripts\beta-gate.ps1 -CliPath F:\Users\almir\Desktop\matter_target\release\matter-cli.exe
.\scripts\export-beta-release-notes.ps1
.\scripts\export-release-artifact-checksums.ps1
.\scripts\verify-release-artifact-checksums.ps1
.\scripts\export-release-package-manifest.ps1 -PackageRoot dist\matter-core-windows-x64
.\scripts\test-ffi-validation-matrix-contract.ps1
.\scripts\test-ffi-validation-report-contract.ps1
.\scripts\test-release-readiness-contract.ps1
.\scripts\test-release-package-contract.ps1
.\scripts\test-release-install-contract.ps1 -PackageRoot dist\matter-core-windows-x64
.\scripts\test-release-install-contract.ps1 -ZipPath dist\matter-core-windows-x64.zip
.\scripts\test-release-artifact-checksums-contract.ps1
.\scripts\test-release-zip-installer-contract.ps1
.\scripts\test-download-site-contract.ps1
.\scripts\test-beta-readiness-contract.ps1
.\scripts\test-beta-feedback-contract.ps1
.\scripts\test-beta-release-notes-contract.ps1
.\scripts\test-beta-site-workflow-contract.ps1
.\scripts\test-beta-gate-contract.ps1
.\scripts\verify-release-package.ps1 -PackageRoot dist\matter-core-windows-x64
.\scripts\verify-release-package.ps1 -ZipPath dist\matter-core-windows-x64.zip
```

- `preflight-env.ps1` checks tools, disk space, and LLVM readiness.
- `release-doctor.ps1` provides a release-oriented PASS/WARN/FAIL diagnosis (toolchain/runtime readiness + recommended validation command). Use `-Json` for CI consumption.
- `ai-app-canonical-flow.ps1` runs the canonical AI -> Matter -> App loop (`check-json`, `reflect-json`, `reflexive-guard-json`, `run-json`, `perf-diagnose-json`, `benchmark-json`, and optional `benchmark-gate-json`) and writes artifacts under `target\ai-flow\`.
- `run-ai-canonical-profile.ps1` runs a quick validation profile with the canonical AI flow enabled and emits a JSON summary (`target\validation\ai-canonical-profile-summary.json` by default). Use `-FlowOnly` to run only the AI flow (without workspace clippy/tests gates) when you want a focused demo/iteration loop.
- `validate-full-workspace.ps1` runs format, clippy, and workspace tests.
- `test-runnable-examples.ps1` validates the stable runnable example contract with `check-json`, `perf-diagnose-json`, and `run-json`.
- `test_all.ps1` runs the project test script.
- `test_api_bridge.ps1` validates API/CLI JSON behavior.
- `test_bytecode_equivalence.ps1` checks source vs bytecode execution.
- `test_repl_simple.ps1` and `test_repl_persistent.ps1` validate REPL behavior.
- `ffi-smoke-all.ps1` runs the Rust FFI smoke, native FFI smoke, JSON summary verifier, validation matrix/report export, validation checks, and release package contract test as one command. Use `-IncludeJava` only on machines with a working JDK/JNI setup.
- `rust-ffi-plugin-smoke.ps1` builds the Rust FFI example plugin and calls its exported ABI symbols through `matter-cli`. By default it uses `cargo run -p matter-cli`; use `-CliPath` to validate an already built executable and `-JsonOut` to save a machine-readable summary.
- `native-ffi-smoke.ps1` validates Node native through a real Node.js host and Go native through `cgo-native` feature tests with a real generated Go shared library. Use `-IncludeJava` only on machines with `java`, `javac`, and a working JDK/JNI setup; use `-JsonOut` to save a machine-readable summary.
- `verify-ffi-smoke-summaries.ps1` validates the minimum contract of generated FFI smoke JSON summaries. Use `-CheckMatrix` after exporting the validation matrix, and use `-RequireJava` when the native smoke was run with Java enabled.
- `export-ffi-validation-matrix.ps1` writes `target\ffi\ffi-validation-matrix.json`, a machine-readable matrix of validated bridge paths and remaining production blockers. The matrix references `schemas\ffi-validation-matrix.schema.json`.
- `export-ffi-validation-report.ps1` writes `target\ffi\ffi-validation-report.md`, a human-readable report generated from the validation matrix.
- `export-release-readiness.ps1` writes `target\ffi\release-readiness.json`, deciding whether the current matrix is an experimental release candidate while still blocking general production claims. It records matrix timestamp/status linkage for package verification.
- `build-release-package.ps1` builds or accepts a release CLI binary, assembles `dist\matter-core-windows-x64`, writes the package manifest, creates `dist\matter-core-windows-x64.zip`, and verifies both outputs.
- `build-download-site.ps1` copies the current release zip, checksum files, and verified zip installer into `site\downloads\`, then writes `site\release.json` for the static download page.
- `beta-gate.ps1` is the one-command beta publication gate: it builds the release/site, runs package/checksum/install/site/beta workflow contracts, and optionally runs `cargo test -q`.
- `export-beta-release-notes.ps1` writes `docs\releases\BETA_RELEASE_BODY.md` from `site\release.json` so GitHub release text matches the actual beta artifacts.
- `export-release-artifact-checksums.ps1` writes `dist\release-checksums.json` and `dist\SHA256SUMS.txt` for final release artifacts.
- `verify-release-artifact-checksums.ps1` validates `dist\release-checksums.json` and `dist\SHA256SUMS.txt` against the actual release artifacts.
- `export-release-package-manifest.ps1` writes `target\ffi\release-package-manifest.json` for a prepared release folder, with relative paths, sizes, and SHA-256 hashes.
- `test-ffi-validation-matrix-contract.ps1` verifies that invalid matrices fail, including accidental production-claim enablement, missing blockers, and missing or invalid schema references.
- `test-ffi-validation-report-contract.ps1` verifies that the human-readable report contains the expected bridge rows, relative example paths, and no production claim allowance.
- `test-release-readiness-contract.ps1` verifies that readiness allows only an experimental release candidate and fails on production claims, missing required smoke, missing blockers, absolute matrix paths, or missing matrix linkage.
- `test-release-package-contract.ps1` verifies that invalid release package folders fail, and that valid package folders and final zips pass the release package verifier, including manifest and report drift checks.
- `test-release-install-contract.ps1` installs a prepared release folder or zip into a temporary directory without touching PATH, then verifies the install manifest, installed CLI capabilities, first-run example, and uninstaller behavior.
- `test-release-artifact-checksums-contract.ps1` verifies that valid checksum files pass and corrupted hash, size, path, or SHA256SUMS data fails.
- `test-release-zip-installer-contract.ps1` verifies the user-facing zip installer path: checksum verification, extraction, local install, CLI smoke check, and uninstall cleanup.
- `test-download-site-contract.ps1` verifies that the static download site points to existing artifacts and that copied zip hashes match `site\release.json` and checksum JSON.
- `test-beta-readiness-contract.ps1` verifies that release artifacts, download site metadata, beta notes, and readiness docs are consistent and do not claim production readiness.
- `test-beta-feedback-contract.ps1` verifies the beta tester guide, beta notes, and GitHub feedback issue template.
- `test-beta-release-notes-contract.ps1` verifies that the generated beta release body includes install, diagnosis, uninstall, hashes, feedback path, and beta limits.
- `test-beta-site-workflow-contract.ps1` verifies that the GitHub Pages beta-site workflow builds and validates the site before deploying.
- `test-beta-gate-contract.ps1` verifies that `beta-gate.ps1` includes the required beta publication checks in the right order.
- `verify-release-package.ps1` checks that a prepared Windows release folder or final zip contains the expected binary, release installer, docs, FFI examples, scripts, schema, generated FFI JSON summaries, report, and release manifest.

`validate-full-workspace.ps1` and `test_all.ps1` generate FFI smoke summaries under `target\ffi\` and verify them automatically unless the related smoke steps are skipped.

Useful validation options:

```powershell
.\scripts\validate-full-workspace.ps1 -JsonSummary
.\scripts\validate-full-workspace.ps1 -RunPreflight
.\scripts\validate-full-workspace.ps1 -RunDoctor
.\scripts\validate-full-workspace.ps1 -RunDoctor -RequireDoctorPass
.\scripts\validate-full-workspace.ps1 -CiMode
.\scripts\validate-full-workspace.ps1 -RequireLLVM
.\scripts\validate-full-workspace.ps1 -SkipRunnableExamples
.\scripts\validate-full-workspace.ps1 -SkipRustFfiSmoke
.\scripts\validate-full-workspace.ps1 -SkipNativeFfiSmoke
.\scripts\validate-full-workspace.ps1 -IncludeJavaNativeSmoke
.\scripts\validate-full-workspace.ps1 -IncludeNodeNativeUnitTests
.\\scripts\\validate-full-workspace.ps1 -RunAiCanonicalFlow
.\\scripts\\validate-full-workspace.ps1 -RunAiCanonicalFlow -AiFlowProgramPath examples\\apps\\counter_app.matter -AiFlowBenchmarkIterations 50
.\\scripts\\validate-full-workspace.ps1 -RunAiCanonicalFlow -AiFlowSkipBenchmarkGate
.\scripts\run-ai-canonical-profile.ps1
.\scripts\run-ai-canonical-profile.ps1 -ProgramPath examples\apps\counter_app.matter -SkipFmt -SkipBenchmarkGate
.\scripts\run-ai-canonical-profile.ps1 -CiLike
.\scripts\run-ai-canonical-profile.ps1 -FlowOnly -ProgramPath examples\apps\counter_app.matter
.\scripts\test_all.ps1 -SkipRustFfiSmoke
.\scripts\test_all.ps1 -SkipNativeFfiSmoke
.\scripts\test_all.ps1 -SkipRunnableExamples
.\scripts\test_all.ps1 -IncludeJavaNativeSmoke
.\scripts\test_all.ps1 -IncludeNodeNativeUnitTests
.\scripts\test_all.ps1 -CiMode
.\scripts\test_all.ps1 -RunAiCanonicalFlow
.\scripts\test_all.ps1 -RunAiCanonicalFlow -AiFlowProgramPath examples\apps\counter_app.matter -AiFlowBenchmarkIterations 50
.\scripts\test_all.ps1 -RunAiCanonicalFlow -AiFlowSkipBenchmarkGate
.\scripts\preflight-env.ps1 -MinFreeGB 20
```

## Archived Scripts

`scripts/archive/` contains older workspace organization and move scripts. They are preserved for history and should not be used for normal development.

## PowerShell Policy

If PowerShell blocks script execution:

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```
